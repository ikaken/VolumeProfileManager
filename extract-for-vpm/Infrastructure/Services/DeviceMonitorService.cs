using System;
using VolumeProfileManager.Core.Interfaces;
using NAudio.CoreAudioApi;
using NAudio.CoreAudioApi.Interfaces;
using Serilog;

namespace VolumeProfileManager.Infrastructure.Services;

/// <summary>
/// IMMNotificationClient を実装し、オーディオデバイスの変更を監視するサービス。
/// </summary>
public class DeviceMonitorService : IDeviceMonitorService, IMMNotificationClient, IDisposable
{
    private readonly MMDeviceEnumerator _enumerator;
    private readonly ILogger _logger = Log.ForContext<DeviceMonitorService>();
    private bool _isDisposed;

    /// <inheritdoc />
    public event Action? DeviceChanged;

    public DeviceMonitorService()
    {
        _enumerator = new MMDeviceEnumerator();
        _enumerator.RegisterEndpointNotificationCallback(this);
        _logger.Information("DeviceMonitorService を初期化し、通知コールバックを登録しました。");
    }

    #region IMMNotificationClient Implementation

    void IMMNotificationClient.OnDeviceStateChanged(string deviceId, DeviceState newState)
    {
        _logger.Debug("デバイスの状態が変更されました: {DeviceId}, 新状態: {NewState}", deviceId, newState);
        DeviceChanged?.Invoke();
    }

    void IMMNotificationClient.OnDeviceAdded(string pwstrDeviceId)
    {
        _logger.Debug("デバイスが追加されました: {DeviceId}", pwstrDeviceId);
        DeviceChanged?.Invoke();
    }

    void IMMNotificationClient.OnDeviceRemoved(string deviceId)
    {
        _logger.Debug("デバイスが削除されました: {DeviceId}", deviceId);
        DeviceChanged?.Invoke();
    }

    void IMMNotificationClient.OnDefaultDeviceChanged(DataFlow flow, Role role, string defaultDeviceId)
    {
        // 既定の再生デバイス（Render/Multimedia）の変更を特に重視
        if (flow == DataFlow.Render && role == Role.Multimedia)
        {
            _logger.Information("既定の再生デバイスが変更されました: {DeviceId}", defaultDeviceId);
            DeviceChanged?.Invoke();
        }
    }

    void IMMNotificationClient.OnPropertyValueChanged(string pwstrDeviceId, NAudio.CoreAudioApi.PropertyKey key)
    {
        // プロパティ変更通知（音量名など）は必要に応じて処理
    }

    #endregion

    public void Dispose()
    {
        if (_isDisposed) return;

        try
        {
            _enumerator.UnregisterEndpointNotificationCallback(this);
            _enumerator.Dispose();
            _logger.Information("DeviceMonitorService を破棄し、通知コールバックを解除しました。");
        }
        catch (Exception ex)
        {
            _logger.Error(ex, "DeviceMonitorService の破棄中にエラーが発生しました。");
        }
        finally
        {
            _isDisposed = true;
        }

        GC.SuppressFinalize(this);
    }
}
