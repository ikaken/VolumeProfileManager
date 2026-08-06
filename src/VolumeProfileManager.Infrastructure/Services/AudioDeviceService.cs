using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using NAudio.CoreAudioApi;
using NAudio.CoreAudioApi.Interfaces;
using Serilog;
using VolumeProfileManager.Core.Interfaces;
using VolumeProfileManager.Domain.Entities;

namespace VolumeProfileManager.Infrastructure.Services;

public class AudioDeviceService : IAudioDeviceService, IDisposable
{
    private readonly VolumeProfileManager.Infrastructure.Interfaces.IAudioEnumeratorAdapter _adapter;
    private readonly ILogger _logger = Log.ForContext<AudioDeviceService>();
    private bool _disposed;

    public AudioDeviceService(VolumeProfileManager.Infrastructure.Interfaces.IAudioEnumeratorAdapter adapter)
    {
        _adapter = adapter;
        _logger.Information("AudioDeviceService initialized.");
    }

    public Task<IReadOnlyList<AudioDeviceInfo>> GetPlaybackDevicesAsync()
    {
        CheckDisposed();
        var list = _adapter.EnumeratePlaybackDevices();
        return Task.FromResult((IReadOnlyList<AudioDeviceInfo>)list);
    }

    public Task<AudioDeviceInfo?> GetDefaultPlaybackDeviceAsync()
    {
        CheckDisposed();
        var device = _adapter.GetDefaultPlaybackDevice();
        return Task.FromResult(device);
    }

    public void Dispose()
    {
        if (!_disposed)
        {
            try
            {
                if (_adapter is IDisposable d) d.Dispose();
                _logger.Information("AudioDeviceService disposed.");
            }
            catch (Exception ex)
            {
                _logger.Error(ex, "Error disposing AudioDeviceService.");
            }
            finally
            {
                _disposed = true;
            }
        }
    }

    private void CheckDisposed()
    {
        if (_disposed) throw new ObjectDisposedException(nameof(AudioDeviceService));
    }

    // NoopNotification placeholder removed; adapter now encapsulates NAudio responsibilities.
}
