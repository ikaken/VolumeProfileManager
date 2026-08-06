using System.Collections.Generic;
using VolumeProfileManager.Domain.Entities;

namespace VolumeProfileManager.Infrastructure.Interfaces;

public interface IAudioEnumeratorAdapter
{
    IReadOnlyList<AudioDeviceInfo> EnumeratePlaybackDevices();
    AudioDeviceInfo? GetDefaultPlaybackDevice();
}
