using System;

namespace VolumeProfileManager.Domain.Entities;

public sealed class DeviceChangedEventArgs : EventArgs
{
    public string PreviousDeviceId { get; set; } = string.Empty;
    public string NewDeviceId { get; set; } = string.Empty;
    public DateTime Timestamp { get; set; }
}
