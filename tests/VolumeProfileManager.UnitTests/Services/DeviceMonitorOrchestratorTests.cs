using System;
using System.Collections.Generic;
using System.Threading;
using System.Threading.Tasks;
using Moq;
using VolumeProfileManager.Core.Interfaces;
using VolumeProfileManager.Domain.Entities;
using VolumeProfileManager.Infrastructure.Services;

namespace VolumeProfileManager.UnitTests.Services;

public class DeviceMonitorOrchestratorTests
{
    [Fact]
    public async Task DefaultDeviceChanged_AppliesProfileWithoutWaitingForDebounce()
    {
        var monitor = new FakeDeviceMonitorService();
        var device = new AudioDeviceInfo { DeviceId = "device-1", DeviceName = "Headphones", IsDefault = true };
        var profile = new VolumeProfile { DeviceId = device.DeviceId, DeviceName = device.DeviceName, MasterVolume = 0.25f, IsMuted = true };
        var profileService = new Mock<IProfileService>();
        var volumeService = new Mock<IAudioVolumeService>();
        var applied = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);

        profileService.Setup(s => s.GetProfileAsync(device.DeviceId, device.DeviceName)).ReturnsAsync(profile);
        volumeService.Setup(s => s.SetMuteStateAsync(true)).Returns(() =>
        {
            applied.TrySetResult();
            return Task.CompletedTask;
        });

        var orchestrator = new TestOrchestrator(monitor, profileService.Object, volumeService.Object, device, TimeSpan.FromSeconds(1));
        orchestrator.Start();

        monitor.Raise(DeviceChangeType.DefaultDeviceChanged, device.DeviceId);

        var completed = await Task.WhenAny(applied.Task, Task.Delay(TimeSpan.FromMilliseconds(500)));

        Assert.Same(applied.Task, completed);
        volumeService.Verify(s => s.SetMasterVolumeAsync(profile.MasterVolume), Times.Once);
        volumeService.Verify(s => s.SetMuteStateAsync(profile.IsMuted), Times.Once);
    }

    [Fact]
    public async Task DeviceStateChanged_WaitsForDebounceBeforeApplyingProfile()
    {
        var monitor = new FakeDeviceMonitorService();
        var device = new AudioDeviceInfo { DeviceId = "device-1", DeviceName = "Headphones", IsDefault = true };
        var profileService = new Mock<IProfileService>();
        var volumeService = new Mock<IAudioVolumeService>();
        var applied = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);

        profileService.Setup(s => s.GetProfileAsync(device.DeviceId, device.DeviceName)).ReturnsAsync(
            new VolumeProfile { DeviceId = device.DeviceId, MasterVolume = 0.25f });
        volumeService.Setup(s => s.SetMasterVolumeAsync(It.IsAny<float>())).Returns(() =>
        {
            applied.TrySetResult();
            return Task.CompletedTask;
        });

        var orchestrator = new TestOrchestrator(monitor, profileService.Object, volumeService.Object, device, TimeSpan.FromMilliseconds(100));
        orchestrator.Start();

        monitor.Raise(DeviceChangeType.DeviceStateChanged, device.DeviceId);
        await Task.Delay(TimeSpan.FromMilliseconds(20));

        Assert.False(applied.Task.IsCompleted);
        var completed = await Task.WhenAny(applied.Task, Task.Delay(TimeSpan.FromMilliseconds(500)));

        Assert.Same(applied.Task, completed);
    }

    [Fact]
    public async Task DefaultDeviceChanged_DuplicateSettledEvent_IsAppliedOnce()
    {
        var monitor = new FakeDeviceMonitorService();
        var device = new AudioDeviceInfo { DeviceId = "device-1", DeviceName = "Headphones", IsDefault = true };
        var profileService = new Mock<IProfileService>();
        var volumeService = new Mock<IAudioVolumeService>();
        profileService.Setup(s => s.GetProfileAsync(device.DeviceId, device.DeviceName)).ReturnsAsync(
            new VolumeProfile { DeviceId = device.DeviceId, MasterVolume = 0.25f });

        var orchestrator = new TestOrchestrator(monitor, profileService.Object, volumeService.Object, device, TimeSpan.FromMilliseconds(50));
        orchestrator.Start();

        monitor.Raise(DeviceChangeType.DefaultDeviceChanged, device.DeviceId);
        await Task.Delay(TimeSpan.FromMilliseconds(200));

        volumeService.Verify(s => s.SetMasterVolumeAsync(0.25f), Times.Once);
        volumeService.Verify(s => s.SetMuteStateAsync(false), Times.Once);
    }

    [Fact]
    public async Task ImmediateResolutionFailure_IsRecoveredBySettledPath()
    {
        var monitor = new FakeDeviceMonitorService();
        var device = new AudioDeviceInfo { DeviceId = "device-1", DeviceName = "Headphones", IsDefault = true };
        var profileService = new Mock<IProfileService>();
        var volumeService = new Mock<IAudioVolumeService>();
        var applied = new TaskCompletionSource(TaskCreationOptions.RunContinuationsAsynchronously);
        var calls = 0;
        var deviceService = new Mock<IAudioDeviceService>();
        deviceService.Setup(s => s.GetDefaultPlaybackDeviceAsync()).Returns(() =>
            Task.FromResult<AudioDeviceInfo?>(++calls == 1 ? null : device));
        profileService.Setup(s => s.GetProfileAsync(device.DeviceId, device.DeviceName)).ReturnsAsync(
            new VolumeProfile { DeviceId = device.DeviceId, MasterVolume = 0.25f });
        volumeService.Setup(s => s.SetMasterVolumeAsync(It.IsAny<float>())).Returns(() =>
        {
            applied.TrySetResult();
            return Task.CompletedTask;
        });

        var orchestrator = new DeviceMonitorOrchestrator(
            monitor, profileService.Object, volumeService.Object, deviceService.Object, TimeSpan.FromMilliseconds(50));
        orchestrator.Start();

        monitor.Raise(DeviceChangeType.DefaultDeviceChanged, device.DeviceId);
        var completed = await Task.WhenAny(applied.Task, Task.Delay(TimeSpan.FromMilliseconds(500)));

        Assert.Same(applied.Task, completed);
        deviceService.Verify(s => s.GetDefaultPlaybackDeviceAsync(), Times.Exactly(2));
    }

    private sealed class FakeDeviceMonitorService : IDeviceMonitorService
    {
        public event EventHandler<DeviceChangedEventArgs>? DeviceChanged;

        public Task<IReadOnlyList<AudioDeviceInfo>> GetAvailableDevicesAsync() =>
            Task.FromResult<IReadOnlyList<AudioDeviceInfo>>(Array.Empty<AudioDeviceInfo>());

        public Task<AudioDeviceInfo?> GetCurrentDefaultDeviceAsync() => Task.FromResult<AudioDeviceInfo?>(null);

        public void Raise(DeviceChangeType changeType, string deviceId)
        {
            DeviceChanged?.Invoke(this, new DeviceChangedEventArgs
            {
                NewDeviceId = deviceId,
                ChangeType = changeType,
                Timestamp = DateTime.UtcNow
            });
        }
    }

    private sealed class TestOrchestrator : DeviceMonitorOrchestrator
    {
        public TestOrchestrator(
            IDeviceMonitorService monitor,
            IProfileService profileService,
            IAudioVolumeService volumeService,
            AudioDeviceInfo device,
            TimeSpan debounceWindow)
            : base(
                monitor,
                profileService,
                volumeService,
                CreateDeviceService(device),
                debounceWindow)
        {
        }

        private static IAudioDeviceService CreateDeviceService(AudioDeviceInfo device)
        {
            var service = new Mock<IAudioDeviceService>();
            service.Setup(s => s.GetDefaultPlaybackDeviceAsync()).ReturnsAsync(device);
            return service.Object;
        }
    }
}
