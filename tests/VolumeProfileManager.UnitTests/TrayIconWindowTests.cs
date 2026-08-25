using VolumeProfileManager.TrayApp;
using Xunit;

namespace VolumeProfileManager.UnitTests;

public class TrayIconWindowTests
{
    [Fact]
    public void AppVersionString_ShouldReturnFormattedVersion()
    {
        // Act
        var versionString = TrayIconWindow.AppVersionString;

        // Assert
        Assert.NotNull(versionString);
        Assert.StartsWith("v", versionString);
    }
}
