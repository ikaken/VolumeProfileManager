using System.Collections.Generic;
using System.Threading.Tasks;
using Moq;
using VolumeProfileManager.Core.Interfaces;
using VolumeProfileManager.Domain.Entities;
using VolumeProfileManager.Infrastructure.Services;
using Xunit;

namespace VolumeProfileManager.UnitTests.Services;

public class ProfileCaptureServiceTests
{
    [Fact]
    public async Task CaptureCurrentProfileAsync_NoDeviceId_SavesDefaultDeviceCurrentState()
    {
        var deviceSvc = new Mock<IAudioDeviceService>();
        var volSvc = new Mock<IAudioVolumeService>();
        var profileSvc = new Mock<IProfileService>();

        deviceSvc.Setup(s => s.GetDefaultPlaybackDeviceAsync())
            .ReturnsAsync(new AudioDeviceInfo { DeviceId = "dev-1", DeviceName = "Speakers", IsDefault = true });
        volSvc.Setup(s => s.GetMasterVolumeAsync()).ReturnsAsync(0.42f);
        volSvc.Setup(s => s.GetMuteStateAsync()).ReturnsAsync(true);

        var service = new ProfileCaptureService(deviceSvc.Object, volSvc.Object, profileSvc.Object);

        var profile = await service.CaptureCurrentProfileAsync();

        Assert.NotNull(profile);
        Assert.Equal("dev-1", profile.DeviceId);
        Assert.Equal("Speakers", profile.DeviceName);
        Assert.Equal(0.42f, profile.MasterVolume);
        Assert.True(profile.IsMuted);
        profileSvc.Verify(s => s.SaveProfileAsync(It.Is<VolumeProfile>(p =>
            p.DeviceId == "dev-1" && p.MasterVolume == 0.42f && p.IsMuted)), Times.Once);
    }

    [Fact]
    public async Task CaptureCurrentProfileAsync_WithDeviceId_UsesSpecifiedDevice()
    {
        var deviceSvc = new Mock<IAudioDeviceService>();
        var volSvc = new Mock<IAudioVolumeService>();
        var profileSvc = new Mock<IProfileService>();

        deviceSvc.Setup(s => s.GetPlaybackDevicesAsync()).ReturnsAsync(new List<AudioDeviceInfo>
        {
            new() { DeviceId = "dev-1", DeviceName = "Speakers" },
            new() { DeviceId = "dev-2", DeviceName = "Headset" }
        });
        volSvc.Setup(s => s.GetMasterVolumeAsync()).ReturnsAsync(0.2f);
        volSvc.Setup(s => s.GetMuteStateAsync()).ReturnsAsync(false);

        var service = new ProfileCaptureService(deviceSvc.Object, volSvc.Object, profileSvc.Object);

        var profile = await service.CaptureCurrentProfileAsync("dev-2");

        Assert.NotNull(profile);
        Assert.Equal("dev-2", profile.DeviceId);
        Assert.Equal("Headset", profile.DeviceName);
        deviceSvc.Verify(s => s.GetDefaultPlaybackDeviceAsync(), Times.Never);
    }

    [Fact]
    public async Task CaptureCurrentProfileAsync_DeviceNotFound_ReturnsNullAndDoesNotSave()
    {
        var deviceSvc = new Mock<IAudioDeviceService>();
        var volSvc = new Mock<IAudioVolumeService>();
        var profileSvc = new Mock<IProfileService>();

        deviceSvc.Setup(s => s.GetDefaultPlaybackDeviceAsync()).ReturnsAsync((AudioDeviceInfo?)null);

        var service = new ProfileCaptureService(deviceSvc.Object, volSvc.Object, profileSvc.Object);

        var profile = await service.CaptureCurrentProfileAsync();

        Assert.Null(profile);
        profileSvc.Verify(s => s.SaveProfileAsync(It.IsAny<VolumeProfile>()), Times.Never);
    }
}
