using System.Threading.Tasks;
using VolumeProfileManager.Domain.Entities;

namespace VolumeProfileManager.Core.Interfaces;

/// <summary>
/// 現在のデバイスの音量・ミュート状態をプロファイルとして保存（新規作成/更新）する。
/// CLI の save-profile コマンドとタスクトレイの「プロファイルを更新」で共通利用する。
/// </summary>
public interface IProfileCaptureService
{
    /// <param name="deviceId">対象デバイスID。null または空の場合は現在のデフォルト再生デバイスを対象とする</param>
    /// <returns>保存したプロファイル。対象デバイスを特定できなかった場合は null</returns>
    Task<VolumeProfile?> CaptureCurrentProfileAsync(string? deviceId = null);
}
