using System;
using System.Threading.Tasks;
using Microsoft.Extensions.DependencyInjection;
using Microsoft.Extensions.Hosting;
using Serilog;
using Serilog.Events;
using VolumeProfileManager.Core.Interfaces;
using VolumeProfileManager.Infrastructure.Persistence;
using VolumeProfileManager.Infrastructure.Services;

namespace VolumeProfileManager.Cli;

public static class Program
{
	public static async Task<int> Main(string[] args)
	{
		Log.Logger = new LoggerConfiguration()
			.MinimumLevel.Debug()
			.WriteTo.Console(restrictedToMinimumLevel: LogEventLevel.Information)
			.WriteTo.File("logs/vpm-.log", rollingInterval: RollingInterval.Day, retainedFileCountLimit: 30)
			.CreateLogger();

		try
		{
			Log.Information("VolumeProfileManager starting...");

			var host = Host.CreateDefaultBuilder(args)
				.UseSerilog()
				.ConfigureServices((context, services) =>
				{
					services.AddSingleton<IDeviceMonitorService, DeviceMonitorService>();
					services.AddSingleton<VolumeProfileManager.Infrastructure.Interfaces.IAudioEnumeratorAdapter, VolumeProfileManager.Infrastructure.Adapters.AudioEnumeratorAdapter>();
					services.AddSingleton<IAudioDeviceService, AudioDeviceService>();
					services.AddSingleton<IProfileRepository, ProfileRepository>();
					services.AddSingleton<IProfileService, ProfileService>();
					services.AddSingleton<IAudioVolumeService, AudioVolumeService>();
					services.AddSingleton<IProfileCaptureService, ProfileCaptureService>();
					services.AddSingleton<IDeviceMonitorOrchestrator, DeviceMonitorOrchestrator>();
					services.AddSingleton<ICliHost, CliHost>();
				})
				.Build();

			var cli = host.Services.GetRequiredService<ICliHost>();
			await cli.ExecuteAsync(args);
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
}
