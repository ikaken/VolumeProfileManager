using System;
using System.Threading.Tasks;
using NAudio.CoreAudioApi;
using Serilog;
using VolumeProfileManager.Core.Interfaces;

namespace VolumeProfileManager.Infrastructure.Services;

public class AudioVolumeService : IAudioVolumeService, IDisposable
{
    private readonly MMDeviceEnumerator _enumerator = new();
    private readonly ILogger _logger = Log.ForContext<AudioVolumeService>();
    private bool _disposed;

    public Task<float> GetMasterVolumeAsync()
    {
        try
        {
            using var device = _enumerator.GetDefaultAudioEndpoint(DataFlow.Render, Role.Multimedia);
            var vol = device.AudioEndpointVolume.MasterVolumeLevelScalar;
            return Task.FromResult(vol);
        }
        catch (Exception ex)
        {
            _logger.Warning(ex, "Failed to get master volume.");
            return Task.FromResult(0f);
        }
    }

    public Task SetMasterVolumeAsync(float volume)
    {
        try
        {
            using var device = _enumerator.GetDefaultAudioEndpoint(DataFlow.Render, Role.Multimedia);
            device.AudioEndpointVolume.MasterVolumeLevelScalar = Math.Clamp(volume, 0f, 1f);
            return Task.CompletedTask;
        }
        catch (Exception ex)
        {
            _logger.Error(ex, "Failed to set master volume.");
            throw;
        }
    }

    public Task<bool> GetMuteStateAsync()
    {
        try
        {
            using var device = _enumerator.GetDefaultAudioEndpoint(DataFlow.Render, Role.Multimedia);
            return Task.FromResult(device.AudioEndpointVolume.Mute);
        }
        catch (Exception ex)
        {
            _logger.Warning(ex, "Failed to get mute state.");
            return Task.FromResult(false);
        }
    }

    public Task SetMuteStateAsync(bool isMuted)
    {
        try
        {
            using var device = _enumerator.GetDefaultAudioEndpoint(DataFlow.Render, Role.Multimedia);
            device.AudioEndpointVolume.Mute = isMuted;
            return Task.CompletedTask;
        }
        catch (Exception ex)
        {
            _logger.Error(ex, "Failed to set mute state.");
            throw;
        }
    }

    public void Dispose()
    {
        if (!_disposed)
        {
            try
            {
                _enumerator.Dispose();
            }
            catch (Exception ex)
            {
                _logger.Error(ex, "Error disposing AudioVolumeService.");
            }
            finally
            {
                _disposed = true;
            }
        }
    }
}
