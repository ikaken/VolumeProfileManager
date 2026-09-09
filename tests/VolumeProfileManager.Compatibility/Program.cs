using System.Text;
using System.Text.Json;
using VolumeProfileManager.Domain.Entities;
using VolumeProfileManager.Infrastructure.Utilities;

Console.InputEncoding = Encoding.UTF8;
Console.OutputEncoding = new UTF8Encoding(false);
var options = new JsonSerializerOptions { WriteIndented = false };
switch (args[0])
{
    case "mappings":
        var mappings = new List<int[]>();
        for (var value = 0; value <= 0x10ffff; value++)
        {
            if (!Rune.IsValid(value)) continue;
            var rune = new Rune(value);
            var upper = Rune.ToUpperInvariant(rune);
            if (rune != upper && string.Equals(rune.ToString(), upper.ToString(), StringComparison.OrdinalIgnoreCase))
                mappings.Add(new[] { value, upper.Value });
        }
        Console.Write(JsonSerializer.Serialize(mappings));
        break;
    case "fixtures":
        Console.Write(JsonSerializer.Serialize(Fixtures(), options));
        break;
    case "verify":
        var actual = JsonSerializer.Deserialize<VolumeProfile[]>(Console.In.ReadToEnd())!;
        var expected = Fixtures();
        if (actual.Length != expected.Length) throw new Exception("profile count changed");
        for (var i = 0; i < expected.Length; i++)
        {
            var a = actual[i];
            var e = expected[i];
            if (a.DeviceId != e.DeviceId || a.DeviceName != e.DeviceName ||
                BitConverter.SingleToInt32Bits(a.MasterVolume) != BitConverter.SingleToInt32Bits(e.MasterVolume) ||
                a.IsMuted != e.IsMuted || a.CreatedAt.Ticks != e.CreatedAt.Ticks ||
                a.LastApplied.Ticks != e.LastApplied.Ticks || a.CreatedAt.Kind != e.CreatedAt.Kind ||
                a.LastApplied.Kind != e.LastApplied.Kind)
                throw new Exception($"profile {i} changed");
        }
        Console.Write("ok");
        break;
    case "compare":
        var pairs = JsonSerializer.Deserialize<string[][]>(Console.In.ReadToEnd())!;
        Console.Write(JsonSerializer.Serialize(pairs.Select(p => new[] {
            string.Equals(p[0], p[1], StringComparison.OrdinalIgnoreCase),
            p[0].Contains(p[1], StringComparison.OrdinalIgnoreCase)
        })));
        break;
    case "match":
        using (var document = JsonDocument.Parse(Console.In.ReadToEnd()))
        {
            var root = document.RootElement;
            var profiles = root.GetProperty("profiles").Deserialize<VolumeProfile[]>()!;
            var id = root.GetProperty("id").GetString()!;
            var name = root.GetProperty("name").GetString();
            var match = DeviceProfileMatcher.Match(profiles, id, name);
            Console.Write(match == null ? "-1" : Array.IndexOf(profiles, match).ToString());
        }
        break;
    default:
        throw new ArgumentException("unknown probe mode");
}

static VolumeProfile[] Fixtures()
{
    var profiles = new List<VolumeProfile> { new() };
    float[] volumes = { 0f, -0f, 1f, 0.1f, float.Epsilon, float.MaxValue, float.MinValue, 0.33333334f };
    for (var i = 0; i < volumes.Length; i++)
    {
        profiles.Add(new VolumeProfile {
            DeviceId = $"synthetic-{i}", DeviceName = "合成 スピーカー Σ 𐐀",
            MasterVolume = volumes[i], IsMuted = i % 2 == 0,
            CreatedAt = new DateTime(2025, 3, 4, 5, 6, 7, DateTimeKind.Utc).AddTicks(i == 0 ? 0 : (long)Math.Pow(10, 7 - i)),
            LastApplied = i == 0 ? DateTime.MinValue : DateTime.SpecifyKind(DateTime.MaxValue, DateTimeKind.Utc)
        });
    }
    return profiles.ToArray();
}
