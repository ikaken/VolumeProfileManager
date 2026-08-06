using System;
using System.Collections.Generic;
using System.Linq;
using VolumeProfileManager.Domain.Entities;

namespace VolumeProfileManager.Infrastructure.Utilities;

public static class DeviceProfileMatcher
{
    public static VolumeProfile? Match(IEnumerable<VolumeProfile> profiles, string deviceId, string? deviceName = null)
    {
        if (profiles == null) return null;

        var profileList = profiles.ToList();
        if (profileList.Count == 0) return null;

        // 1. DeviceId 完全一致 (スコア: 100)
        if (!string.IsNullOrWhiteSpace(deviceId))
        {
            var matchById = profileList.FirstOrDefault(p =>
                string.Equals(p.DeviceId, deviceId, StringComparison.OrdinalIgnoreCase));
            if (matchById != null) return matchById;
        }

        // デバイス名が指定されていない場合は終了
        if (string.IsNullOrWhiteSpace(deviceName)) return null;

        var cleanTargetName = NormalizeName(deviceName);

        // 2. DeviceName 完全一致 (スコア: 80)
        var matchByNameExact = profileList
            .Where(p => string.Equals(NormalizeName(p.DeviceName), cleanTargetName, StringComparison.OrdinalIgnoreCase))
            .OrderByDescending(p => p.LastApplied)
            .ThenByDescending(p => p.CreatedAt)
            .FirstOrDefault();
        if (matchByNameExact != null) return matchByNameExact;

        // 3. DeviceName 部分一致・包含一致 (スコア: 50)
        var matchByNamePartial = profileList
            .Where(p => !string.IsNullOrWhiteSpace(p.DeviceName) &&
                        (cleanTargetName.Contains(NormalizeName(p.DeviceName), StringComparison.OrdinalIgnoreCase) ||
                         NormalizeName(p.DeviceName).Contains(cleanTargetName, StringComparison.OrdinalIgnoreCase)))
            .OrderByDescending(p => p.LastApplied)
            .ThenByDescending(p => p.CreatedAt)
            .FirstOrDefault();

        return matchByNamePartial;
    }

    private static string NormalizeName(string name)
    {
        if (string.IsNullOrWhiteSpace(name)) return string.Empty;
        return string.Join(" ", name.Split(new[] { ' ' }, StringSplitOptions.RemoveEmptyEntries)).Trim();
    }
}
