using System;
using System.Threading.Tasks;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Serilog;
using VolumeProfileManager.Core.Interfaces;
using VolumeProfileManager.Domain.Entities;

namespace VolumeProfileManager.Cli;

public class CliHost : ICliHost
{
    private readonly IServiceProvider _provider;
    private readonly ILogger _logger = Log.ForContext<CliHost>();

    public CliHost(IServiceProvider provider)
    {
        _provider = provider;
    }

    public async Task ExecuteAsync(string[] args)
    {
        if (args == null || args.Length == 0)
        {
            PrintHelp();
            return;
        }

        var cmd = args[0].ToLowerInvariant();
        switch (cmd)
        {
            case "list-devices":
                await ListDevices();
                break;
            case "status":
                await Status();
                break;
            case "set-volume":
                await SetVolume(args);
                break;
            case "set-mute":
                await SetMute(args);
                break;
            case "save-profile":
                await SaveProfile(args);
                break;
            case "delete-profile":
                await DeleteProfile(args);
                break;
            case "run":
                await Run();
                break;
            default:
                PrintHelp();
                break;
        }
    }

    private void PrintHelp()
    {
        Console.WriteLine("VolumeProfileManager CLI\nCommands:\n  list-devices\n  status\n  set-volume <0-100>\n  set-mute <true|false>\n  save-profile [deviceId]\n  delete-profile [deviceId]\n  run");
    }

    private async Task ListDevices()
    {
        var svc = _provider.GetRequiredService<IAudioDeviceService>();
        var list = await svc.GetPlaybackDevicesAsync();
        foreach (var d in list)
        {
            Console.WriteLine($"{d.DeviceId}: {d.DeviceName}{(d.IsDefault ? " [DEFAULT]" : string.Empty)}");
        }
    }

    private async Task Status()
    {
        var deviceSvc = _provider.GetRequiredService<IAudioDeviceService>();
        var volSvc = _provider.GetRequiredService<IAudioVolumeService>();
        var device = await deviceSvc.GetDefaultPlaybackDeviceAsync();
        var vol = await volSvc.GetMasterVolumeAsync();
        var mute = await volSvc.GetMuteStateAsync();
        Console.WriteLine($"Device: {device?.DeviceName} ({device?.DeviceId})\nVolume: {vol:P0}\nMuted: {mute}");
    }

    private async Task SetVolume(string[] args)
    {
        if (args.Length < 2 || !int.TryParse(args[1], out var percent) || percent < 0 || percent > 100)
        {
            Console.WriteLine("Usage: set-volume <0-100>");
            return;
        }

        var deviceSvc = _provider.GetRequiredService<IAudioDeviceService>();
        var volSvc = _provider.GetRequiredService<IAudioVolumeService>();

        var device = await deviceSvc.GetDefaultPlaybackDeviceAsync();
        var before = await volSvc.GetMasterVolumeAsync();

        float newVolume = percent / 100f;
        await volSvc.SetMasterVolumeAsync(newVolume);

        var after = await volSvc.GetMasterVolumeAsync();
        Console.WriteLine($"Device : {device?.DeviceName}");
        Console.WriteLine($"Volume : {before:P0} -> {after:P0}");
        _logger.Information("Volume changed {Before:P0} -> {After:P0} on {DeviceName}", before, after, device?.DeviceName);
    }

    private async Task SetMute(string[] args)
    {
        if (args.Length < 2 || !bool.TryParse(args[1], out var isMuted))
        {
            Console.WriteLine("Usage: set-mute <true|false>");
            return;
        }

        var deviceSvc = _provider.GetRequiredService<IAudioDeviceService>();
        var volSvc = _provider.GetRequiredService<IAudioVolumeService>();

        var device = await deviceSvc.GetDefaultPlaybackDeviceAsync();
        var before = await volSvc.GetMuteStateAsync();

        await volSvc.SetMuteStateAsync(isMuted);

        var after = await volSvc.GetMuteStateAsync();
        Console.WriteLine($"Device : {device?.DeviceName}");
        Console.WriteLine($"Muted  : {before} -> {after}");
        _logger.Information("Mute changed {Before} -> {After} on {DeviceName}", before, after, device?.DeviceName);
    }

    private async Task SaveProfile(string[] args)
    {
        var captureSvc = _provider.GetRequiredService<IProfileCaptureService>();

        var profile = await captureSvc.CaptureCurrentProfileAsync(args.Length > 1 ? args[1] : null);
        if (profile == null)
        {
            Console.WriteLine("Failed to resolve target device. Specify a valid device ID or set a default playback device.");
            return;
        }

        Console.WriteLine($"Profile saved.");
        Console.WriteLine($"  Device  : {profile.DeviceName} ({profile.DeviceId})");
        Console.WriteLine($"  Volume  : {profile.MasterVolume:P0}");
        Console.WriteLine($"  Muted   : {profile.IsMuted}");
        Console.WriteLine($"  Saved at: {profile.LastApplied.ToLocalTime():yyyy-MM-dd HH:mm:ss}");
    }

    private async Task DeleteProfile(string[] args)
    {
        var profileSvc = _provider.GetRequiredService<VolumeProfileManager.Core.Interfaces.IProfileService>();
        if (args.Length < 2)
        {
            Console.WriteLine("Usage: delete-profile <deviceId>");
            return;
        }

        await profileSvc.DeleteProfileAsync(args[1]);
        Console.WriteLine($"Profile deleted.");
        Console.WriteLine($"  DeviceId: {args[1]}");
        _logger.Information("Profile deleted: {DeviceId}", args[1]);
    }

    private Task Run()
    {
        var orchestrator = _provider.GetRequiredService<IDeviceMonitorOrchestrator>();

        using var mre = new System.Threading.ManualResetEvent(false);

        orchestrator.ProfileApplied += (s, e) =>
        {
            Console.WriteLine($"[{DateTime.Now:HH:mm:ss}] Device changed detected");
            Console.WriteLine($"  DeviceId  : {e.DeviceId}");
            Console.WriteLine($"  DeviceName: {e.DeviceName}");
            if (e.IsNewProfile)
            {
                Console.WriteLine($"  -> No profile found. Auto-created new profile: Volume={e.MasterVolume:P0}, Muted={e.IsMuted}");
            }
            else
            {
                Console.WriteLine($"  -> Profile applied: Volume={e.MasterVolume:P0}, Muted={e.IsMuted}");
            }
        };

        orchestrator.Start();

        Console.WriteLine("Running device monitor. Press Ctrl+C to exit.");
        Console.CancelKeyPress += (s, e) => { mre.Set(); };
        mre.WaitOne();

        orchestrator.Stop();
        return Task.CompletedTask;
    }

}
