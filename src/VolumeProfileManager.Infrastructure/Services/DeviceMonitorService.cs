using System;
using System.Threading.Tasks;
using NAudio.CoreAudioApi;
using NAudio.CoreAudioApi.Interfaces;
using Serilog;
using VolumeProfileManager.Core.Interfaces;
using VolumeProfileManager.Domain.Entities;

namespace VolumeProfileManager.Infrastructure.Services;

public class DeviceMonitorService : IDeviceMonitorService, IMMNotificationClient, IDisposable
{
    private readonly MMDeviceEnumerator _enumerator;
    private readonly ILogger _logger = Log.ForContext<DeviceMonitorService>();
    private bool _isDisposed;

    public event EventHandler<DeviceChangedEventArgs>? DeviceChanged;

    public DeviceMonitorService()
    {
        _enumerator = new MMDeviceEnumerator();
        _enumerator.RegisterEndpointNotificationCallback(this);
        _logger.Information("DeviceMonitorService initialized and registered callback.");
    }

    public Task<IReadOnlyList<AudioDeviceInfo>> GetAvailableDevicesAsync()
    {
        var devices = _enumerator.EnumerateAudioEndPoints(DataFlow.Render, DeviceState.Active);
        string defaultId = string.Empty;
        try
        {
            using var defaultDevice = _enumerator.GetDefaultAudioEndpoint(DataFlow.Render, Role.Multimedia);
            defaultId = defaultDevice.ID;
        }
        catch (Exception ex)
        {
            _logger.Warning(ex, "Failed to get default device.");
        }

        var list = new System.Collections.Generic.List<AudioDeviceInfo>();
        foreach (var d in devices)
        {
            list.Add(new AudioDeviceInfo
            {
                DeviceId = d.ID,
                DeviceName = d.FriendlyName,
                IsDefault = d.ID == defaultId
            });
        }

        return Task.FromResult((IReadOnlyList<AudioDeviceInfo>)list);
    }

    public Task<AudioDeviceInfo?> GetCurrentDefaultDeviceAsync()
    {
        try
        {
            using var device = _enumerator.GetDefaultAudioEndpoint(DataFlow.Render, Role.Multimedia);
            return Task.FromResult<AudioDeviceInfo?>(new AudioDeviceInfo
            {
                DeviceId = device.ID,
                DeviceName = device.FriendlyName,
                IsDefault = true
            });
        }
        catch (Exception ex)
        {
            _logger.Warning(ex, "Failed to get default device.");
            return Task.FromResult<AudioDeviceInfo?>(null);
        }
    }

    #region IMMNotificationClient Implementation

    void IMMNotificationClient.OnDeviceStateChanged(string deviceId, DeviceState newState)
    {
        _logger.Debug("Device state changed: {DeviceId} {NewState}", deviceId, newState);
        DeviceChanged?.Invoke(this, new DeviceChangedEventArgs { PreviousDeviceId = string.Empty, NewDeviceId = deviceId, Timestamp = DateTime.UtcNow, ChangeType = DeviceChangeType.DeviceStateChanged });
    }

    void IMMNotificationClient.OnDeviceAdded(string pwstrDeviceId)
    {
        _logger.Debug("Device added: {DeviceId}", pwstrDeviceId);
        DeviceChanged?.Invoke(this, new DeviceChangedEventArgs { PreviousDeviceId = string.Empty, NewDeviceId = pwstrDeviceId, Timestamp = DateTime.UtcNow, ChangeType = DeviceChangeType.DeviceAdded });
    }

    void IMMNotificationClient.OnDeviceRemoved(string deviceId)
    {
        _logger.Debug("Device removed: {DeviceId}", deviceId);
        DeviceChanged?.Invoke(this, new DeviceChangedEventArgs { PreviousDeviceId = deviceId, NewDeviceId = string.Empty, Timestamp = DateTime.UtcNow, ChangeType = DeviceChangeType.DeviceRemoved });
    }

    void IMMNotificationClient.OnDefaultDeviceChanged(DataFlow flow, Role role, string defaultDeviceId)
    {
        if (flow == DataFlow.Render && role == Role.Multimedia)
        {
            _logger.Information("Default playback device changed: {DeviceId}", defaultDeviceId);
            DeviceChanged?.Invoke(this, new DeviceChangedEventArgs { PreviousDeviceId = string.Empty, NewDeviceId = defaultDeviceId, Timestamp = DateTime.UtcNow, ChangeType = DeviceChangeType.DefaultDeviceChanged });
        }
    }

    void IMMNotificationClient.OnPropertyValueChanged(string pwstrDeviceId, PropertyKey key)
    {
        // Optional: handle property changes if needed
    }

    #endregion

    public void Dispose()
    {
        if (_isDisposed) return;

        try
        {
            _enumerator.UnregisterEndpointNotificationCallback(this);
            _enumerator.Dispose();
            _logger.Information("DeviceMonitorService disposed and callback unregistered.");
        }
        catch (Exception ex)
        {
            _logger.Error(ex, "Error disposing DeviceMonitorService.");
        }
        finally
        {
            _isDisposed = true;
        }

        GC.SuppressFinalize(this);
    }
}
