namespace VolumeProfileManager.Domain.Entities;

public sealed class AudioDeviceInfo
{
    public string DeviceId { get; set; } = string.Empty;
    public string DeviceName { get; set; } = string.Empty;
    public bool IsDefault { get; set; }
}
