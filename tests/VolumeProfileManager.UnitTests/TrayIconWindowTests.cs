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

    [Theory]
    [InlineData(1, 2, 0, "v1.2.0")]
    [InlineData(0, 1, 2, "v0.1.2")]
    public void FormatVersion_WithVersion_ShouldReturnMajorMinorBuild(int major, int minor, int build, string expected)
    {
        // Act
        var actual = TrayIconWindow.FormatVersion(new Version(major, minor, build));

        // Assert
        Assert.Equal(expected, actual);
    }

    [Fact]
    public void FormatVersion_WithNull_ShouldReturnDefault()
    {
        // Act
        var actual = TrayIconWindow.FormatVersion(null);

        // Assert
        Assert.Equal("v1.0.0", actual);
    }
}
