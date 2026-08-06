using System.Collections.Generic;
using System.Threading.Tasks;
using Serilog;
using VolumeProfileManager.Core.Interfaces;
using VolumeProfileManager.Domain.Entities;
using VolumeProfileManager.Infrastructure.Persistence;

using VolumeProfileManager.Infrastructure.Utilities;

namespace VolumeProfileManager.Infrastructure.Services;

public class ProfileService : IProfileService
{
    private readonly IProfileRepository _repository;
    private readonly ILogger _logger = Log.ForContext<ProfileService>();

    public ProfileService(IProfileRepository repository)
    {
        _repository = repository;
    }

    public async Task<VolumeProfile?> GetProfileAsync(string deviceIdentifier, string? deviceName = null)
    {
        var allProfiles = await _repository.GetAllAsync();
        var matched = DeviceProfileMatcher.Match(allProfiles, deviceIdentifier, deviceName);
        if (matched != null)
        {
            _logger.Information("Matched profile {MatchedDeviceName} ({MatchedDeviceId}) for input ID '{DeviceIdentifier}' / Name '{DeviceName}'",
                matched.DeviceName, matched.DeviceId, deviceIdentifier, deviceName);
        }
        else
        {
            _logger.Information("No matching profile found for input ID '{DeviceIdentifier}' / Name '{DeviceName}'",
                deviceIdentifier, deviceName);
        }
        return matched;
    }

    public Task<IReadOnlyList<VolumeProfile>> GetAllProfilesAsync()
    {
        return _repository.GetAllAsync();
    }

    public Task SaveProfileAsync(VolumeProfile profile)
    {
        return _repository.SaveAsync(profile);
    }

    public Task DeleteProfileAsync(string deviceIdentifier)
    {
        return _repository.DeleteAsync(deviceIdentifier);
    }
}
