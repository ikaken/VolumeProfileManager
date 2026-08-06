using System.Collections.Generic;
using System.Threading.Tasks;
using VolumeProfileManager.Domain.Entities;

namespace VolumeProfileManager.Infrastructure.Persistence;

public interface IProfileRepository
{
    Task<VolumeProfile?> GetByIdentifierAsync(string deviceIdentifier);
    Task<IReadOnlyList<VolumeProfile>> GetAllAsync();
    Task SaveAsync(VolumeProfile profile);
    Task DeleteAsync(string deviceIdentifier);
}
