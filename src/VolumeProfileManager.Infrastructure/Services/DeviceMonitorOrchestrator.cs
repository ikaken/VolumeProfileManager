using System;
using System.Linq;
using System.Threading;
using System.Threading.Tasks;
using Serilog;
using VolumeProfileManager.Core.Interfaces;
using VolumeProfileManager.Domain.Entities;

namespace VolumeProfileManager.Infrastructure.Services;

public class DeviceMonitorOrchestrator : IDeviceMonitorOrchestrator
{
    private readonly IDeviceMonitorService _monitor;
    private readonly IProfileService _profileSvc;
    private readonly IAudioVolumeService _volSvc;
    private readonly IAudioDeviceService _deviceSvc;
    private readonly ILogger _logger = Log.ForContext<DeviceMonitorOrchestrator>();
    private readonly object _debounceLock = new();
    private bool _started;
    private CancellationTokenSource? _debounceCts;
    private static readonly TimeSpan DebounceWindow = TimeSpan.FromMilliseconds(800);
    private readonly object _lastAppliedLock = new();
    private string? _lastAppliedDeviceId;
    private DateTime _lastAppliedUtc = DateTime.MinValue;
    private static readonly TimeSpan SameDeviceSuppressWindow = TimeSpan.FromSeconds(3);

    public event EventHandler<ProfileAppliedEventArgs>? ProfileApplied;

    public DeviceMonitorOrchestrator(
        IDeviceMonitorService monitor,
        IProfileService profileSvc,
        IAudioVolumeService volSvc,
        IAudioDeviceService deviceSvc)
    {
        _monitor = monitor;
        _profileSvc = profileSvc;
        _volSvc = volSvc;
        _deviceSvc = deviceSvc;
    }

    public void Start()
    {
        if (_started)
        {
            return;
        }

        _monitor.DeviceChanged += OnDeviceChanged;
        _started = true;
        _logger.Information("DeviceMonitorOrchestrator started.");
    }

    public void Stop()
    {
        if (!_started)
        {
            return;
        }

        _monitor.DeviceChanged -= OnDeviceChanged;
        _started = false;
        _logger.Information("DeviceMonitorOrchestrator stopped.");
    }

    private void OnDeviceChanged(object? sender, DeviceChangedEventArgs? e)
    {
        var deviceId = e?.NewDeviceId ?? string.Empty;
        if (string.IsNullOrEmpty(deviceId))
        {
            return;
        }

        // Windowsの COM コールバック（OnDeviceStateChanged / OnDefaultDeviceChanged 等）は
        // 1度のデバイス切り替え操作に対して、ネゴシエーション中の一時的なエンドポイントIDを含め
        // 短時間に複数回・異なるデバイスIDで発火することがある。
        // そのため「最後のイベントから静定期間が経過したら、その時点の実際のデフォルトデバイスを
        // 問い合わせて1回だけ処理する」末尾デバウンス方式を採用する。
        CancellationTokenSource cts;
        lock (_debounceLock)
        {
            _debounceCts?.Cancel();
            _debounceCts = new CancellationTokenSource();
            cts = _debounceCts;
        }

        _ = ProcessSettledDeviceChangeAsync(cts.Token);
    }

    private async Task ProcessSettledDeviceChangeAsync(CancellationToken token)
    {
        try
        {
            await Task.Delay(DebounceWindow, token);
        }
        catch (TaskCanceledException)
        {
            // より新しいイベントに置き換えられたため、このイベントの処理は行わない
            return;
        }

        if (token.IsCancellationRequested)
        {
            return;
        }

        try
        {
            // 静定後、実際に確定しているデフォルトデバイスを問い合わせる
            // （イベントに含まれる一時的なデバイスIDは名前解決できない場合があるため、確定後の情報を使う）
            var defaultDevice = await _deviceSvc.GetDefaultPlaybackDeviceAsync();
            if (defaultDevice == null)
            {
                _logger.Warning("Failed to resolve settled default playback device.");
                return;
            }

            var deviceId = defaultDevice.DeviceId;
            var deviceName = defaultDevice.DeviceName;

            // OnDeviceStateChanged / OnDefaultDeviceChanged など複数のCOMコールバックが
            // デバウンス期間(800ms)よりも間隔を空けて同一デバイスへの切り替えを重複通知することがあるため、
            // 直近に適用済みの同一デバイスであれば再適用・再通知しない
            lock (_lastAppliedLock)
            {
                var now = DateTime.UtcNow;
                if (deviceId == _lastAppliedDeviceId && (now - _lastAppliedUtc) < SameDeviceSuppressWindow)
                {
                    _logger.Debug("Suppressed duplicate settled event for already-applied device {DeviceId}", deviceId);
                    return;
                }

                _lastAppliedDeviceId = deviceId;
                _lastAppliedUtc = now;
            }

            _logger.Information("Device changed detected: {DeviceName} ({DeviceId})", deviceName, deviceId);

            // 対応プロファイルがあれば音量・ミュートを適用
            var profile = await _profileSvc.GetProfileAsync(deviceId, deviceName == "(unknown)" ? null : deviceName);
            if (profile != null)
            {
                await _volSvc.SetMasterVolumeAsync(profile.MasterVolume);
                await _volSvc.SetMuteStateAsync(profile.IsMuted);
                _logger.Information("Profile applied: Volume={Volume:P0}, Muted={IsMuted} for {DeviceName}", profile.MasterVolume, profile.IsMuted, deviceName);

                ProfileApplied?.Invoke(this, new ProfileAppliedEventArgs
                {
                    DeviceId = deviceId,
                    DeviceName = deviceName,
                    MasterVolume = profile.MasterVolume,
                    IsMuted = profile.IsMuted,
                    IsNewProfile = false
                });
            }
            else
            {
                // プロファイルが存在しない場合、現在の音量・ミュート状態を取得して自動プロファイルを作成・保存
                try
                {
                    var currentVol = await _volSvc.GetMasterVolumeAsync();
                    var currentMute = await _volSvc.GetMuteStateAsync();
                    var now = DateTime.UtcNow;

                    var newProfile = new VolumeProfile
                    {
                        DeviceId = deviceId,
                        DeviceName = deviceName == "(unknown)" ? string.Empty : deviceName,
                        MasterVolume = currentVol,
                        IsMuted = currentMute,
                        CreatedAt = now,
                        LastApplied = now
                    };

                    await _profileSvc.SaveProfileAsync(newProfile);
                    _logger.Information("Auto-created profile for new device {DeviceName} ({DeviceId}): Volume={Volume:P0}, Muted={IsMuted}",
                        deviceName, deviceId, currentVol, currentMute);

                    ProfileApplied?.Invoke(this, new ProfileAppliedEventArgs
                    {
                        DeviceId = deviceId,
                        DeviceName = deviceName,
                        MasterVolume = currentVol,
                        IsMuted = currentMute,
                        IsNewProfile = true
                    });
                }
                catch (Exception ex)
                {
                    _logger.Warning(ex, "Failed to auto-create profile for {DeviceId}", deviceId);
                }
            }
        }
        catch (Exception ex)
        {
            _logger.Error(ex, "Unhandled error in DeviceChanged handler.");
        }
    }
}
