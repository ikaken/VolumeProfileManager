using System;
using System.Collections.Generic;
using System.Linq;
using VolumeProfileManager.Core.Interfaces;
using NAudio.CoreAudioApi;
using NAudio.CoreAudioApi.Interfaces;
using Serilog;

namespace VolumeProfileManager.Infrastructure.Services;

/// <summary>
/// NAudio を使用してオーディオデバイスを管理する実装。
/// </summary>
public class AudioDeviceService : IAudioDeviceService, IDisposable, IMMNotificationClient
{
    private readonly MMDeviceEnumerator _enumerator = new();
    private readonly ILogger _logger = Log.ForContext<AudioDeviceService>();
    private bool _disposed;

    public event Action? DeviceChanged;

    public AudioDeviceService()
    {
        _enumerator.RegisterEndpointNotificationCallback(this);
        _logger.Information("AudioDeviceService を初期化しました。");
    }

    public IEnumerable<AudioDeviceInfo> GetPlaybackDevices()
    {
        CheckDisposed();
        var devices = _enumerator.EnumerateAudioEndPoints(DataFlow.Render, DeviceState.Active);
        string defaultId = string.Empty;
        try
        {
            using var defaultDevice = _enumerator.GetDefaultAudioEndpoint(DataFlow.Render, Role.Multimedia);
            defaultId = defaultDevice.ID;
        }
        catch (Exception ex)
        {
            _logger.Warning(ex, "既定デバイスの取得に失敗しました。");
        }

        return devices.Select(d => new AudioDeviceInfo
        {
            Id = d.ID,
            Name = d.FriendlyName,
            IsDefault = d.ID == defaultId
        }).ToList();
    }

    public AudioDeviceInfo GetDefaultPlaybackDevice()
    {
        CheckDisposed();
        using var device = _enumerator.GetDefaultAudioEndpoint(DataFlow.Render, Role.Multimedia);
        return new AudioDeviceInfo
        {
            Id = device.ID,
            Name = device.FriendlyName,
            IsDefault = true
        };
    }

    public void SetDefaultPlaybackDevice(string deviceId)
    {
        CheckDisposed();
        _logger.Information("既定デバイスを設定: {DeviceId}", deviceId);
        // TODO: PolicyConfig を使用して既定のデバイスを設定する実装を追加する
    }

    private void CheckDisposed()
    {
        if (_disposed) throw new ObjectDisposedException(nameof(AudioDeviceService));
    }

    public void Dispose()
    {
        if (!_disposed)
        {
            try
            {
                _enumerator.UnregisterEndpointNotificationCallback(this);
                _enumerator.Dispose();
                _logger.Information("AudioDeviceService を破棄しました。");
            }
            catch (Exception ex)
            {
                _logger.Error(ex, "AudioDeviceService の破棄中にエラーが発生しました。");
            }
            finally
            {
                _disposed = true;
            }
        }
    }

    #region IMMNotificationClient Implementation

    void IMMNotificationClient.OnDeviceStateChanged(string deviceId, DeviceState newState) => DeviceChanged?.Invoke();
    void IMMNotificationClient.OnDeviceAdded(string pwstrDeviceId) => DeviceChanged?.Invoke();
    void IMMNotificationClient.OnDeviceRemoved(string deviceId) => DeviceChanged?.Invoke();
    void IMMNotificationClient.OnDefaultDeviceChanged(DataFlow flow, Role role, string defaultDeviceId)
    {
        if (flow == DataFlow.Render && role == Role.Multimedia)
        {
            DeviceChanged?.Invoke();
        }
    }
    void IMMNotificationClient.OnPropertyValueChanged(string pwstrDeviceId, NAudio.CoreAudioApi.PropertyKey key) { }

    #endregion
}
