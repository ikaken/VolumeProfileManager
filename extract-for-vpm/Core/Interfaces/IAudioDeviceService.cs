using System;
using System.Collections.Generic;

namespace VolumeProfileManager.Core.Interfaces;

/// <summary>
/// オーディオデバイスを管理するサービスのインターフェース。
/// </summary>
public interface IAudioDeviceService
{
    /// <summary>
    /// 利用可能なすべての再生デバイスを取得します。
    /// </summary>
    IEnumerable<AudioDeviceInfo> GetPlaybackDevices();

    /// <summary>
    /// 現在の既定の再生デバイスを取得します。
    /// </summary>
    AudioDeviceInfo GetDefaultPlaybackDevice();

    /// <summary>
    /// 既定の再生デバイスを設定します。
    /// </summary>
    void SetDefaultPlaybackDevice(string deviceId);

    /// <summary>
    /// デバイスが変更されたときに発生します。
    /// </summary>
    event Action? DeviceChanged;
}

/// <summary>
/// オーディオデバイスの情報。
/// </summary>
public record AudioDeviceInfo
{
    public string Id { get; init; } = string.Empty;
    public string Name { get; init; } = string.Empty;
    public bool IsDefault { get; init; }
}
