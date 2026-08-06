using System.Collections.Generic;
using System.Threading.Tasks;
using VolumeProfileManager.Domain.Entities;

namespace VolumeProfileManager.Core.Interfaces;

public interface IProfileService
{
    Task<VolumeProfile?> GetProfileAsync(string deviceIdentifier, string? deviceName = null);
    Task<IReadOnlyList<VolumeProfile>> GetAllProfilesAsync();
    Task SaveProfileAsync(VolumeProfile profile);
    Task DeleteProfileAsync(string deviceIdentifier);
}
