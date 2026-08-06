using System;
using System.Linq;
using System.Threading.Tasks;
using Serilog;
using VolumeProfileManager.Core.Interfaces;
using VolumeProfileManager.Domain.Entities;

namespace VolumeProfileManager.Infrastructure.Services;

public class ProfileCaptureService : IProfileCaptureService
{
    private readonly IAudioDeviceService _deviceSvc;
    private readonly IAudioVolumeService _volSvc;
    private readonly IProfileService _profileSvc;
    private readonly ILogger _logger = Log.ForContext<ProfileCaptureService>();

    public ProfileCaptureService(
        IAudioDeviceService deviceSvc,
        IAudioVolumeService volSvc,
        IProfileService profileSvc)
    {
        _deviceSvc = deviceSvc;
        _volSvc = volSvc;
        _profileSvc = profileSvc;
    }

    public async Task<VolumeProfile?> CaptureCurrentProfileAsync(string? deviceId = null)
    {
        AudioDeviceInfo? device;
        if (string.IsNullOrEmpty(deviceId))
        {
            device = await _deviceSvc.GetDefaultPlaybackDeviceAsync();
        }
        else
        {
            var devices = await _deviceSvc.GetPlaybackDevicesAsync();
            device = devices.FirstOrDefault(d => d.DeviceId == deviceId);
        }

        if (device == null)
        {
            _logger.Warning("Failed to resolve target device for profile capture. DeviceId={DeviceId}", deviceId);
            return null;
        }

        var vol = await _volSvc.GetMasterVolumeAsync();
        var mute = await _volSvc.GetMuteStateAsync();
        var now = DateTime.UtcNow;

        var profile = new VolumeProfile
        {
            DeviceId = device.DeviceId,
            DeviceName = device.DeviceName,
            MasterVolume = vol,
            IsMuted = mute,
            CreatedAt = now,
            LastApplied = now
        };

        await _profileSvc.SaveProfileAsync(profile);

        _logger.Information("Profile captured: {DeviceName} ({DeviceId}) Volume={Volume:P0} Muted={IsMuted}",
            profile.DeviceName, profile.DeviceId, profile.MasterVolume, profile.IsMuted);

        return profile;
    }
}
