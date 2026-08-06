using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Threading.Tasks;
using Serilog;
using VolumeProfileManager.Domain.Entities;

namespace VolumeProfileManager.Infrastructure.Persistence;

public class ProfileRepository : IProfileRepository
{
    private readonly string _filePath;
    private readonly ILogger _logger = Log.ForContext<ProfileRepository>();
    private readonly object _lock = new();

    public ProfileRepository()
    {
        var localAppData = Environment.GetFolderPath(Environment.SpecialFolder.LocalApplicationData);
        var folder = Path.Combine(localAppData, "VolumeProfileManager");
        if (!Directory.Exists(folder)) Directory.CreateDirectory(folder);
        _filePath = Path.Combine(folder, "profiles.json");
    }

    public Task<VolumeProfile?> GetByIdentifierAsync(string deviceIdentifier)
    {
        var all = ReadAll();
        var found = all.FirstOrDefault(p => string.Equals(p.DeviceId, deviceIdentifier, StringComparison.OrdinalIgnoreCase)
                                           || string.Equals(p.DeviceName, deviceIdentifier, StringComparison.OrdinalIgnoreCase));
        return Task.FromResult(found);
    }

    public Task<IReadOnlyList<VolumeProfile>> GetAllAsync()
    {
        var all = ReadAll();
        return Task.FromResult((IReadOnlyList<VolumeProfile>)all);
    }

    public Task SaveAsync(VolumeProfile profile)
    {
        lock (_lock)
        {
            var all = ReadAll();
            var existing = all.FirstOrDefault(p => string.Equals(p.DeviceId, profile.DeviceId, StringComparison.OrdinalIgnoreCase));
            if (existing != null)
            {
                existing.DeviceName = profile.DeviceName;
                existing.MasterVolume = profile.MasterVolume;
                existing.IsMuted = profile.IsMuted;
                existing.LastApplied = profile.LastApplied;
            }
            else
            {
                profile.CreatedAt = DateTime.UtcNow;
                all.Add(profile);
            }

            var json = JsonSerializer.Serialize(all, new JsonSerializerOptions { WriteIndented = true });
            BackupAndWrite(json);
        }

        return Task.CompletedTask;
    }

    public Task DeleteAsync(string deviceIdentifier)
    {
        lock (_lock)
        {
            var all = ReadAll();
            all.RemoveAll(p => string.Equals(p.DeviceId, deviceIdentifier, StringComparison.OrdinalIgnoreCase)
                               || string.Equals(p.DeviceName, deviceIdentifier, StringComparison.OrdinalIgnoreCase));
            var json = JsonSerializer.Serialize(all, new JsonSerializerOptions { WriteIndented = true });
            BackupAndWrite(json);
        }

        return Task.CompletedTask;
    }

    private List<VolumeProfile> ReadAll()
    {
        lock (_lock)
        {
            try
            {
                if (!File.Exists(_filePath)) return new List<VolumeProfile>();
                var text = File.ReadAllText(_filePath);
                var list = JsonSerializer.Deserialize<List<VolumeProfile>>(text) ?? new List<VolumeProfile>();
                return list;
            }
            catch (JsonException ex)
            {
                _logger.Error(ex, "Failed to deserialize profiles.json. Attempting to recover from backup.");
                TryRecoverFromBackup();
                return new List<VolumeProfile>();
            }
            catch (Exception ex)
            {
                _logger.Error(ex, "Failed to read profiles.json");
                return new List<VolumeProfile>();
            }
        }
    }

    private void BackupAndWrite(string json)
    {
        try
        {
            if (File.Exists(_filePath))
            {
                var bak = _filePath + ".bak";
                File.Copy(_filePath, bak, true);
            }

            File.WriteAllText(_filePath, json);
        }
        catch (Exception ex)
        {
            _logger.Error(ex, "Failed to write profiles.json");
            throw;
        }
    }

    private void TryRecoverFromBackup()
    {
        try
        {
            var bak = _filePath + ".bak";
            if (File.Exists(bak))
            {
                File.Copy(bak, _filePath, true);
            }
        }
        catch (Exception ex)
        {
            _logger.Error(ex, "Failed to recover profiles.json from backup.");
        }
    }
}
