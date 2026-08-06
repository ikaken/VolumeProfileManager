using System;
using Microsoft.Extensions.DependencyInjection;
using Serilog;
using Serilog.Events;
using VolumeProfileManager.Core.Interfaces;
using VolumeProfileManager.Infrastructure.Persistence;
using VolumeProfileManager.Infrastructure.Services;
using static VolumeProfileManager.TrayApp.NativeMethods;

namespace VolumeProfileManager.TrayApp;

public static class Program
{
    [STAThread]
    public static int Main()
    {
        Log.Logger = new LoggerConfiguration()
            .MinimumLevel.Debug()
            .WriteTo.File("logs/vpm-tray-.log", rollingInterval: RollingInterval.Day, retainedFileCountLimit: 30, restrictedToMinimumLevel: LogEventLevel.Information)
            .CreateLogger();

        try
        {
            Log.Information("VolumeProfileManager TrayApp starting...");

            var services = new ServiceCollection();
            services.AddSingleton<IDeviceMonitorService, DeviceMonitorService>();
            services.AddSingleton<VolumeProfileManager.Infrastructure.Interfaces.IAudioEnumeratorAdapter, VolumeProfileManager.Infrastructure.Adapters.AudioEnumeratorAdapter>();
            services.AddSingleton<IAudioDeviceService, AudioDeviceService>();
            services.AddSingleton<IProfileRepository, ProfileRepository>();
            services.AddSingleton<IProfileService, ProfileService>();
            services.AddSingleton<IAudioVolumeService, AudioVolumeService>();
            services.AddSingleton<IProfileCaptureService, ProfileCaptureService>();
            services.AddSingleton<IDeviceMonitorOrchestrator, DeviceMonitorOrchestrator>();

            var provider = services.BuildServiceProvider();
            var orchestrator = provider.GetRequiredService<IDeviceMonitorOrchestrator>();

            using var trayIcon = new TrayIconWindow();

            orchestrator.ProfileApplied += (s, e) =>
            {
                var message = e.IsNewProfile
                    ? $"新しいプロファイルを作成しました: {e.MasterVolume:P0}{(e.IsMuted ? " (ミュート)" : string.Empty)}"
                    : $"音量 {e.MasterVolume:P0}{(e.IsMuted ? " (ミュート)" : string.Empty)} を適用しました";
                trayIcon.ShowBalloon(e.DeviceName, message);
            };

            trayIcon.MenuCommandSelected += cmd =>
            {
                switch (cmd)
                {
                    case TrayIconWindow.CmdStatus:
                        ShowStatus(trayIcon, provider);
                        break;
                    case TrayIconWindow.CmdUpdateProfile:
                        UpdateProfile(trayIcon, provider);
                        break;
                    case TrayIconWindow.CmdToggleStartup:
                        ToggleStartup(trayIcon);
                        break;
                    case TrayIconWindow.CmdExit:
                        orchestrator.Stop();
                        trayIcon.RequestExit();
                        break;
                }
            };

            orchestrator.Start();

            RunMessageLoop();

            orchestrator.Stop();
            Log.Information("VolumeProfileManager TrayApp exiting.");
            return 0;
        }
        catch (Exception ex)
        {
            Log.Fatal(ex, "Fatal error");
            return 1;
        }
        finally
        {
            Log.CloseAndFlush();
        }
    }

    private static void ShowStatus(TrayIconWindow trayIcon, IServiceProvider provider)
    {
        var deviceSvc = provider.GetRequiredService<IAudioDeviceService>();
        var volSvc = provider.GetRequiredService<IAudioVolumeService>();

        var device = deviceSvc.GetDefaultPlaybackDeviceAsync().GetAwaiter().GetResult();
        var vol = volSvc.GetMasterVolumeAsync().GetAwaiter().GetResult();
        var mute = volSvc.GetMuteStateAsync().GetAwaiter().GetResult();

        trayIcon.ShowBalloon(
            device?.DeviceName ?? "(unknown device)",
            $"音量: {vol:P0} / ミュート: {(mute ? "ON" : "OFF")}");
    }

    private static void UpdateProfile(TrayIconWindow trayIcon, IServiceProvider provider)
    {
        var captureSvc = provider.GetRequiredService<IProfileCaptureService>();
        var profile = captureSvc.CaptureCurrentProfileAsync().GetAwaiter().GetResult();

        if (profile == null)
        {
            trayIcon.ShowBalloon("VolumeProfileManager", "現在の再生デバイスを特定できなかったため、プロファイルを更新できませんでした");
            return;
        }

        trayIcon.ShowBalloon(
            profile.DeviceName,
            $"プロファイルを更新しました。音量: {profile.MasterVolume:P0} / ミュート: {(profile.IsMuted ? "ON" : "OFF")}");
    }

    private static void ToggleStartup(TrayIconWindow trayIcon)
    {
        var exePath = Environment.ProcessPath ?? System.Reflection.Assembly.GetExecutingAssembly().Location;
        var registered = StartupRegistration.Toggle(exePath);
        trayIcon.ShowBalloon(
            "VolumeProfileManager",
            registered ? "スタートアップに登録しました" : "スタートアップ登録を解除しました");
        Log.Information("Startup registration toggled: {Registered}", registered);
    }

    private static void RunMessageLoop()
    {
        while (GetMessage(out var msg, IntPtr.Zero, 0, 0) > 0)
        {
            TranslateMessage(ref msg);
            DispatchMessage(ref msg);
        }
    }
}
