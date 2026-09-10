# VolumeProfileManager 設計書

---

## 目次

1. [概要](#1-概要)
2. [アーキテクチャ](#2-アーキテクチャ)
3. [レイヤー設計](#3-レイヤー設計)
4. [サービス仕様](#4-サービス仕様)
5. [データフロー](#5-データフロー)
6. [エラーハンドリング](#6-エラーハンドリング)
7. [DI コンテナ構成](#7-di-コンテナ構成)

---

## 1. 概要

VolumeProfileManager は、クリーンアーキテクチャに基づいた 4 層構成で実装される。

- **VolumeProfileManager.Domain** - エンティティ・値オブジェクト
- **VolumeProfileManager.Core** - ユースケース・サービスインターフェース
- **VolumeProfileManager.Infrastructure** - 具体的実装（NAudio、JSON I/O）
- **VolumeProfileManager.TrayApp** - タスクトレイ常駐ホスト・DI コンテナ・エントリーポイント

---

## 2. アーキテクチャ

```
┌──────────────────────────────────────┐
│   TrayApp (ホスト、Win32 トレイ UI)     │
├──────────────────────────────────────┤
│   Core (インターフェース定義)            │
│  - IDeviceMonitorService              │
│  - IDeviceMonitorOrchestrator         │
│  - IAudioDeviceService                │
│  - IProfileService                    │
│  - IAudioVolumeService                │
│  - IProfileCaptureService             │
├──────────────────────────────────────┤
│   Infrastructure (実装)                │
│  - DeviceMonitorService               │
│  - DeviceMonitorOrchestrator          │
│  - AudioDeviceService                 │
│  - ProfileService                     │
│  - AudioVolumeService                 │
│  - ProfileCaptureService              │
│  - ProfileRepository                  │
├──────────────────────────────────────┤
│   Domain (エンティティ)                 │
│  - VolumeProfile                      │
│  - AudioDeviceInfo                    │
│  - DeviceChangedEventArgs             │
│  - ProfileAppliedEventArgs            │
└──────────────────────────────────────┘
```

---

## 3. レイヤー設計

### 3.1 Domain レイヤー

エンティティと値オブジェクトを定義する。外部依存なし。

```csharp
// VolumeProfile.cs
public class VolumeProfile
{
    public string DeviceId { get; set; }
    public string DeviceName { get; set; }
    public float MasterVolume { get; set; }
    public bool IsMuted { get; set; }
    public DateTime CreatedAt { get; set; }
    public DateTime LastApplied { get; set; }
}

// AudioDeviceInfo.cs
public class AudioDeviceInfo
{
    public string DeviceId { get; set; }
    public string DeviceName { get; set; }
    public bool IsDefault { get; set; }
}

// DeviceChangedEventArgs.cs
public enum DeviceChangeType
{
    DefaultDeviceChanged,
    DeviceStateChanged,
    DeviceAdded,
    DeviceRemoved
}

public sealed class DeviceChangedEventArgs : EventArgs
{
    public string PreviousDeviceId { get; set; }
    public string NewDeviceId { get; set; }
    public DateTime Timestamp { get; set; }
    public DeviceChangeType ChangeType { get; set; }
}
```

### 3.2 Core レイヤー

インターフェース定義とユースケース実装。Infrastructure への依存なし。

```csharp
// IDeviceMonitorService
public interface IDeviceMonitorService
{
    event EventHandler<DeviceChangedEventArgs>? DeviceChanged;
    Task<IReadOnlyList<AudioDeviceInfo>> GetAvailableDevicesAsync();
    Task<AudioDeviceInfo?> GetCurrentDefaultDeviceAsync();
}

// IDeviceMonitorOrchestrator
public interface IDeviceMonitorOrchestrator
{
    event EventHandler<ProfileAppliedEventArgs>? ProfileApplied;
    void Start();
    void Stop();
}

// IAudioDeviceService
public interface IAudioDeviceService
{
    Task<IReadOnlyList<AudioDeviceInfo>> GetPlaybackDevicesAsync();
    Task<AudioDeviceInfo?> GetDefaultPlaybackDeviceAsync();
}

// IProfileService
public interface IProfileService
{
    Task<VolumeProfile?> GetProfileAsync(string deviceIdentifier, string? deviceName = null);
    Task<IReadOnlyList<VolumeProfile>> GetAllProfilesAsync();
    Task SaveProfileAsync(VolumeProfile profile);
    Task DeleteProfileAsync(string deviceIdentifier);
}

// IAudioVolumeService
public interface IAudioVolumeService
{
    Task<float> GetMasterVolumeAsync();
    Task SetMasterVolumeAsync(float volume);
    Task<bool> GetMuteStateAsync();
    Task SetMuteStateAsync(bool isMuted);
}

// IProfileCaptureService
public interface IProfileCaptureService
{
    Task<VolumeProfile?> CaptureCurrentProfileAsync();
}
```

### 3.3 Infrastructure レイヤー

Core インターフェースの実装。NAudio、Serilog、JSON I/O に依存。

```csharp
// DeviceMonitorService.cs
public class DeviceMonitorService : IDeviceMonitorService, IMMNotificationClient, IDisposable
{
    public event EventHandler<DeviceChangedEventArgs>? DeviceChanged;

    public Task<IReadOnlyList<AudioDeviceInfo>> GetAvailableDevicesAsync()
    {
        // NAudio を使用してデバイス一覧を取得
    }

    public Task<AudioDeviceInfo?> GetCurrentDefaultDeviceAsync()
    {
        // 既定デバイスを取得
    }

    // IMMNotificationClient 実装
    void OnDefaultDeviceChanged(DataFlow flow, Role role, string defaultDeviceId)
    {
        // DeviceChangeType.DefaultDeviceChanged を設定してイベント発火
    }
}

// DeviceMonitorOrchestrator.cs
public class DeviceMonitorOrchestrator : IDeviceMonitorOrchestrator
{
    public event EventHandler<ProfileAppliedEventArgs>? ProfileApplied;

    public void Start()
    {
        // イベント購読
    }

    public void Stop()
    {
        // イベント購読解除
    }

    private void OnDeviceChanged(object? sender, DeviceChangedEventArgs e)
    {
        // 800ms 末尾デバウンスをスケジュール
        // DefaultDeviceChanged の場合は即時適用も開始
    }

    private async Task ResolveAndApplyAsync()
    {
        // デフォルトデバイス取得 → プロファイル検索 → 音量・ミュート適用
        // 同一デバイスの重複適用を 3 秒間抑制
    }
}

// ProfileService.cs
public class ProfileService : IProfileService
{
    private readonly IProfileRepository _repository;

    public async Task<VolumeProfile?> GetProfileAsync(string deviceIdentifier, string? deviceName = null)
    {
        var allProfiles = await _repository.GetAllAsync();
        return DeviceProfileMatcher.Match(allProfiles, deviceIdentifier, deviceName);
    }
}

// ProfileRepository.cs
public class ProfileRepository : IProfileRepository
{
    private readonly string _filePath;

    public async Task SaveAsync(VolumeProfile profile)
    {
        // profiles.json へ書き込み（.json.bak バックアップ付き）
    }
}

// AudioVolumeService.cs
public class AudioVolumeService : IAudioVolumeService, IDisposable
{
    public async Task<float> GetMasterVolumeAsync()
    {
        // NAudio で現在のマスターボリュームを取得
    }

    public async Task SetMasterVolumeAsync(float volume)
    {
        // NAudio でマスターボリュームを設定（Math.Clamp）
    }
}

// ProfileCaptureService.cs
public class ProfileCaptureService : IProfileCaptureService
{
    public async Task<VolumeProfile?> CaptureCurrentProfileAsync()
    {
        // 既定デバイス・音量・ミュートを取得して保存
    }
}
```

### 3.4 TrayApp レイヤー

タスクトレイ常駐ホスト、DI コンテナ、エントリーポイント。

```csharp
// Program.cs
public static class Program
{
    [STAThread]
    public static int Main()
    {
        // Serilog 設定
        // DI コンテナ構築
        // トレイアイコン初期化
        // Orchestrator 開始 → メッセージループ
    }

    private static void ShowStatus(TrayIconWindow trayIcon, IServiceProvider provider)
    {
        // 現在のデバイス・音量・ミュートをバルーン通知
    }

    private static void UpdateProfile(TrayIconWindow trayIcon, IServiceProvider provider)
    {
        // IProfileCaptureService で現在のプロファイルを保存
    }

    private static void ToggleStartup(TrayIconWindow trayIcon)
    {
        // HKCU\...\Run へのスタートアップ登録をトグル
    }
}
```

---

## 4. サービス仕様

### 4.1 IDeviceMonitorService

- **責務**: デバイス変更イベントの監視、デバイス一覧取得
- **依存**: NAudio の `IMMNotificationClient`
- **ライフタイム**: Singleton
- **スレッドセーフ**: イベント発火時にロックなし（登録者の責任）

### 4.2 IDeviceMonitorOrchestrator

- **責務**: デバイス変更通知に応じたプロファイル適用の調整
- **依存**: `IDeviceMonitorService`, `IProfileService`, `IAudioVolumeService`, `IAudioDeviceService`
- **ライフタイム**: Singleton
- **スレッドセーフ**: 適用処理を `SemaphoreSlim` で直列化、重複抑制状態をロック保護

### 4.3 IAudioDeviceService

- **責務**: 利用可能なデバイス情報の取得
- **依存**: NAudio の `MMDeviceEnumerator`
- **ライフタイム**: Singleton
- **スレッドセーフ**: 読み取り専用、スレッドセーフ

### 4.4 IProfileService

- **責務**: プロファイルの CRUD 操作と多段階照合
- **依存**: `IProfileRepository`
- **ライフタイム**: Singleton
- **スレッドセーフ**: リポジトリがファイルロックで保証

### 4.5 IAudioVolumeService

- **責務**: マスターボリューム・ミュート状態の取得・設定
- **依存**: NAudio の `AudioEndpointVolume`
- **ライフタイム**: Singleton
- **スレッドセーフ**: NAudio 実装に準ずる

### 4.6 IProfileCaptureService

- **責務**: 現在のデフォルトデバイスの音量・ミュート状態をプロファイルとして保存
- **依存**: `IAudioDeviceService`, `IAudioVolumeService`, `IProfileService`
- **ライフタイム**: Singleton

---

## 5. データフロー

### 5.1 デバイス切り替え時のフロー

```
[1] IMMNotificationClient
    ↓ OnDefaultDeviceChanged イベント発火（ChangeType = DefaultDeviceChanged）
[2] IDeviceMonitorService
    ↓ DeviceChanged イベント発火（ChangeType 付き）
[3] IDeviceMonitorOrchestrator
    ├─ 即時パス: 待たずに ResolveAndApplyAsync()
    └─ 検証パス: 800ms デバウンス後に ResolveAndApplyAsync()
[4] IAudioDeviceService
    ↓ GetDefaultPlaybackDeviceAsync() で確定デバイスを取得
[5] IProfileService
    ↓ GetProfileAsync(deviceId, deviceName) で検索
[6] ProfileRepository
    ↓ profiles.json から読み込み
[7] IProfileService (結果判定)
    ├─ プロファイル存在
    │   ↓ IAudioVolumeService.SetMasterVolumeAsync()
    │   ↓ IAudioVolumeService.SetMuteStateAsync()
    └─ プロファイル未存在
        ↓ 新規作成、現在の音量を記録
```

### 5.2 プロファイル保存時のフロー

```
[1] トレイメニュー: プロファイルを更新
    ↓
[2] TrayApp.Program.UpdateProfile()
    ↓ IProfileCaptureService.CaptureCurrentProfileAsync()
       ├─ IAudioDeviceService.GetDefaultPlaybackDeviceAsync()
       ├─ IAudioVolumeService.GetMasterVolumeAsync()
       ├─ IAudioVolumeService.GetMuteStateAsync()
       └─ VolumeProfile オブジェクト生成
    ↓ IProfileService.SaveProfileAsync(profile)
[3] ProfileRepository
    ↓ profiles.json に追記/更新（.json.bak バックアップ）
```

---

## 6. エラーハンドリング

### 6.1 即時適用失敗時の対応

- 即時パスでデバイス解決やプロファイル適用に失敗した場合はログを記録し、処理を中断する
- 直近適用済み状態は更新しない
- 800ms 後の検証パスで同じ処理が再実行されるため、最終的な状態を救済する

### 6.2 例外分類

| 例外タイプ | 対応 | ログレベル |
|-----------|------|----------|
| `COMException` (Core Audio API) | ログ出力、検証パスに委ねる | WARNING → ERROR |
| `IOException` (profiles.json) | バックアップから復帰 | ERROR |
| `JsonException` (profiles.json 破損) | バックアップから復帰 | ERROR |
| その他の予期しない例外 | ログ記録、継続実行 | ERROR |

---

## 7. DI コンテナ構成

```csharp
public static class Program
{
    public static int Main()
    {
        Log.Logger = new LoggerConfiguration()
            .MinimumLevel.Debug()
            .WriteTo.File(
                "logs/vpm-tray-.log",
                rollingInterval: RollingInterval.Day,
                retainedFileCountLimit: 30,
                restrictedToMinimumLevel: LogEventLevel.Information)
            .CreateLogger();

        var services = new ServiceCollection();
        services.AddSingleton<IDeviceMonitorService, DeviceMonitorService>();
        services.AddSingleton<IAudioEnumeratorAdapter, AudioEnumeratorAdapter>();
        services.AddSingleton<IAudioDeviceService, AudioDeviceService>();
        services.AddSingleton<IProfileRepository, ProfileRepository>();
        services.AddSingleton<IProfileService, ProfileService>();
        services.AddSingleton<IAudioVolumeService, AudioVolumeService>();
        services.AddSingleton<IProfileCaptureService, ProfileCaptureService>();
        services.AddSingleton<IDeviceMonitorOrchestrator, DeviceMonitorOrchestrator>();

        var provider = services.BuildServiceProvider();
        // トレイアイコンと Orchestrator の連携
    }
}
```

---

## 付録

### A. プロジェクト参照図

```
TrayApp → Core + Infrastructure
Infrastructure → Core + Domain
Core → Domain
Domain (外部依存なし)
```

### B. 今後の拡張ポイント

- `IAudioSessionService` - アプリケーション別音量制御
- `IScheduleService` - スケジュール機能
- `IBluetoothDeviceService` - Bluetooth デバイス対応
