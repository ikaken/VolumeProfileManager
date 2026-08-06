using System.Collections.Generic;
using System.Threading.Tasks;
using VolumeProfileManager.Domain.Entities;

namespace VolumeProfileManager.Core.Interfaces;

public interface IAudioDeviceService
{
    Task<IReadOnlyList<AudioDeviceInfo>> GetPlaybackDevicesAsync();
    Task<AudioDeviceInfo?> GetDefaultPlaybackDeviceAsync();
}
