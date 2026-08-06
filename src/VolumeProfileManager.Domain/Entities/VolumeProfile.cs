using System;

namespace VolumeProfileManager.Domain.Entities;

public sealed class VolumeProfile
{
    public string DeviceId { get; set; } = string.Empty;
    public string DeviceName { get; set; } = string.Empty;
    public float MasterVolume { get; set; }
    public bool IsMuted { get; set; }
    public DateTime CreatedAt { get; set; }
    public DateTime LastApplied { get; set; }
}
