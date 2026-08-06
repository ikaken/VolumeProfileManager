using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using VolumeProfileManager.Domain.Entities;

namespace VolumeProfileManager.Core.Interfaces;

public interface IDeviceMonitorService
{
    event EventHandler<DeviceChangedEventArgs>? DeviceChanged;
    Task<IReadOnlyList<AudioDeviceInfo>> GetAvailableDevicesAsync();
    Task<AudioDeviceInfo?> GetCurrentDefaultDeviceAsync();
}
