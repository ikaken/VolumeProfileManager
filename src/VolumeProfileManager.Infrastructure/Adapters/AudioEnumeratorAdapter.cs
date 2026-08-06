using System;
using System.Collections.Generic;
using System.Linq;
using NAudio.CoreAudioApi;
using Serilog;
using VolumeProfileManager.Domain.Entities;
using VolumeProfileManager.Infrastructure.Interfaces;

namespace VolumeProfileManager.Infrastructure.Adapters;

public class AudioEnumeratorAdapter : IAudioEnumeratorAdapter, IDisposable
{
    private readonly MMDeviceEnumerator _enumerator = new();
    private readonly ILogger _logger = Log.ForContext<AudioEnumeratorAdapter>();

    public IReadOnlyList<AudioDeviceInfo> EnumeratePlaybackDevices()
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

        return devices.Select(d => new AudioDeviceInfo
        {
            DeviceId = d.ID,
            DeviceName = d.FriendlyName,
            IsDefault = d.ID == defaultId
        }).ToList();
    }

    public AudioDeviceInfo? GetDefaultPlaybackDevice()
    {
        try
        {
            using var device = _enumerator.GetDefaultAudioEndpoint(DataFlow.Render, Role.Multimedia);
            return new AudioDeviceInfo
            {
                DeviceId = device.ID,
                DeviceName = device.FriendlyName,
                IsDefault = true
            };
        }
        catch (Exception ex)
        {
            _logger.Warning(ex, "Failed to get default device.");
            return null;
        }
    }

    public void Dispose()
    {
        try
        {
            _enumerator.UnregisterEndpointNotificationCallback(new NoopNotification());
            _enumerator.Dispose();
        }
        catch (Exception ex)
        {
            _logger.Error(ex, "Error disposing AudioEnumeratorAdapter.");
        }
    }

    private class NoopNotification : NAudio.CoreAudioApi.Interfaces.IMMNotificationClient
    {
        void NAudio.CoreAudioApi.Interfaces.IMMNotificationClient.OnDeviceStateChanged(string deviceId, DeviceState newState) { }
        void NAudio.CoreAudioApi.Interfaces.IMMNotificationClient.OnDeviceAdded(string pwstrDeviceId) { }
        void NAudio.CoreAudioApi.Interfaces.IMMNotificationClient.OnDeviceRemoved(string deviceId) { }
        void NAudio.CoreAudioApi.Interfaces.IMMNotificationClient.OnDefaultDeviceChanged(DataFlow flow, Role role, string defaultDeviceId) { }
        void NAudio.CoreAudioApi.Interfaces.IMMNotificationClient.OnPropertyValueChanged(string pwstrDeviceId, PropertyKey key) { }
    }
}
