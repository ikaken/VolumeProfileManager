using System;
using System.Collections.Generic;
using VolumeProfileManager.Domain.Entities;
using VolumeProfileManager.Infrastructure.Utilities;
using Xunit;

namespace VolumeProfileManager.UnitTests.Utilities;

public class DeviceProfileMatcherTests
{
    [Fact]
    public void Match_DeviceIdExactMatch_ReturnsProfile()
    {
        // Arrange
        var profiles = new List<VolumeProfile>
        {
            new() { DeviceId = "device-1", DeviceName = "Speaker", MasterVolume = 0.5f },
            new() { DeviceId = "device-2", DeviceName = "Headphones", MasterVolume = 0.8f }
        };

        // Act
        var result = DeviceProfileMatcher.Match(profiles, "device-2", "Dummy Name");

        // Assert
        Assert.NotNull(result);
        Assert.Equal("device-2", result.DeviceId);
        Assert.Equal(0.8f, result.MasterVolume);
    }

    [Fact]
    public void Match_DeviceNameExactMatch_FallbackWhenIdDiffers()
    {
        // Arrange
        var profiles = new List<VolumeProfile>
        {
            new() { DeviceId = "old-device-id", DeviceName = "Realtek HD Audio", MasterVolume = 0.4f, LastApplied = DateTime.UtcNow }
        };

        // Act - IDは違うが名前が完全一致
        var result = DeviceProfileMatcher.Match(profiles, "new-device-id", "Realtek HD Audio");

        // Assert
        Assert.NotNull(result);
        Assert.Equal("old-device-id", result.DeviceId);
        Assert.Equal(0.4f, result.MasterVolume);
    }

    [Fact]
    public void Match_DeviceNamePartialMatch_ReturnsProfile()
    {
        // Arrange
        var profiles = new List<VolumeProfile>
        {
            new() { DeviceId = "id-100", DeviceName = "Sony WH-1000XM4 Stereo", MasterVolume = 0.6f }
        };

        // Act - 名前の一部・包含一致
        var result = DeviceProfileMatcher.Match(profiles, "new-id", "Sony WH-1000XM4");

        // Assert
        Assert.NotNull(result);
        Assert.Equal("id-100", result.DeviceId);
        Assert.Equal(0.6f, result.MasterVolume);
    }

    [Fact]
    public void Match_NoMatch_ReturnsNull()
    {
        // Arrange
        var profiles = new List<VolumeProfile>
        {
            new() { DeviceId = "device-1", DeviceName = "Speaker", MasterVolume = 0.5f }
        };

        // Act
        var result = DeviceProfileMatcher.Match(profiles, "unknown-id", "Unknown Device");

        // Assert
        Assert.Null(result);
    }
}
