using System.Collections.Generic;
using System.Threading.Tasks;
using Moq;
using VolumeProfileManager.Domain.Entities;
using VolumeProfileManager.Infrastructure.Persistence;
using VolumeProfileManager.Infrastructure.Services;
using Xunit;

namespace VolumeProfileManager.UnitTests.Services;

public class ProfileServiceMatchingTests
{
    [Fact]
    public async Task GetProfileAsync_DelegatesToMatcher_ReturnsMatchedProfile()
    {
        // Arrange
        var mockRepo = new Mock<IProfileRepository>();
        var profiles = new List<VolumeProfile>
        {
            new() { DeviceId = "dev-1", DeviceName = "USB Audio Headset", MasterVolume = 0.35f, IsMuted = false }
        };
        mockRepo.Setup(r => r.GetAllAsync()).ReturnsAsync(profiles);

        var service = new ProfileService(mockRepo.Object);

        // Act - ID違い、名前完全一致
        var matched = await service.GetProfileAsync("dev-changed-id", "USB Audio Headset");

        // Assert
        Assert.NotNull(matched);
        Assert.Equal("dev-1", matched.DeviceId);
        Assert.Equal(0.35f, matched.MasterVolume);
        mockRepo.Verify(r => r.GetAllAsync(), Times.Once);
    }
}
