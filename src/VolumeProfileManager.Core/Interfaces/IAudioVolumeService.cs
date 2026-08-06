using System.Threading.Tasks;

namespace VolumeProfileManager.Core.Interfaces;

public interface IAudioVolumeService
{
    Task<float> GetMasterVolumeAsync();
    Task SetMasterVolumeAsync(float volume);
    Task<bool> GetMuteStateAsync();
    Task SetMuteStateAsync(bool isMuted);
}
