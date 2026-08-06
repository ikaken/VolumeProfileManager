using System;

namespace VolumeProfileManager.Core.Interfaces;

/// <summary>
/// オーディオデバイスの変更を監視するサービスのインターフェース。
/// </summary>
public interface IDeviceMonitorService
{
    /// <summary>
    /// デバイス構成や既定のデバイスが変更されたときに発生します。
    /// </summary>
    event Action? DeviceChanged;
}
