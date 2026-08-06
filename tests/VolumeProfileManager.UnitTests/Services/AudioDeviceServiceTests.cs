using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using VolumeProfileManager.Domain.Entities;
using VolumeProfileManager.Infrastructure.Interfaces;
using VolumeProfileManager.Infrastructure.Services;

namespace VolumeProfileManager.UnitTests.Services;

public class AudioDeviceServiceTests
{
    private class FakeAdapter : IAudioEnumeratorAdapter
    {
        private readonly List<AudioDeviceInfo> _devices;

        public FakeAdapter()
        {
            _devices = new List<AudioDeviceInfo>
            {
                new AudioDeviceInfo { DeviceId = "dev1", DeviceName = "Speakers", IsDefault = false },
                new AudioDeviceInfo { DeviceId = "dev2", DeviceName = "Headphones", IsDefault = true }
            };
        }

        public IReadOnlyList<AudioDeviceInfo> EnumeratePlaybackDevices() => _devices;

        public AudioDeviceInfo? GetDefaultPlaybackDevice() => _devices.FirstOrDefault(d => d.IsDefault);
    }

    [Fact]
    public async Task GetPlaybackDevicesAsync_ReturnsDevices()
    {
        var adapter = new FakeAdapter();
        var svc = new AudioDeviceService(adapter);

        var list = await svc.GetPlaybackDevicesAsync();

        Assert.NotNull(list);
        Assert.Equal(2, list.Count);
        Assert.Contains(list, d => d.DeviceId == "dev2" && d.IsDefault);
    }

    [Fact]
    public async Task GetDefaultPlaybackDeviceAsync_ReturnsDefault()
    {
        var adapter = new FakeAdapter();
        var svc = new AudioDeviceService(adapter);

        var device = await svc.GetDefaultPlaybackDeviceAsync();

        Assert.NotNull(device);
        Assert.Equal("dev2", device!.DeviceId);
        Assert.True(device.IsDefault);
    }
}
