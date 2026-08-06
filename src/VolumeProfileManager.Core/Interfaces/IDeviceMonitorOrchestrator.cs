using System;

namespace VolumeProfileManager.Core.Interfaces;

public class ProfileAppliedEventArgs : EventArgs
{
    public required string DeviceId { get; init; }
    public required string DeviceName { get; init; }
    public float MasterVolume { get; init; }
    public bool IsMuted { get; init; }
    public bool IsNewProfile { get; init; }
}

public interface IDeviceMonitorOrchestrator
{
    event EventHandler<ProfileAppliedEventArgs>? ProfileApplied;

    void Start();

    void Stop();
}
