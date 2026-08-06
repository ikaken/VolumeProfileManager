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
- **VolumeProfileManager.Console** - CLI・ホスト・DI コンテナ

---

## 2. アーキテクチャ

```
┌──────────────────────────────────────┐
│   Console (CLI, ホスト)               │
├──────────────────────────────────────┤
│   Core (インターフェース定義)          │
│  - IDeviceMonitorService             │
│  - IAudioDeviceService               │
│  - IProfileService                   │
│  - IAudioVolumeService               │
├──────────────────────────────────────┤
│   Infrastructure (実装)               │
│  - DeviceMonitorService              │
│  - AudioDeviceService                │
│  - ProfileService                    │
│  - AudioVolumeService                │
│  - ProfileRepository                 │
├──────────────────────────────────────┤
│   Domain (エンティティ)                │
│  - VolumeProfile                     │
│  - AudioDeviceInfo                   │
│  - DeviceChangedEventArgs            │
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
public class DeviceChangedEventArgs : EventArgs
{
    public string PreviousDeviceId { get; set; }
    public string NewDeviceId { get; set; }
    public DateTime Timestamp { get; set; }
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

// IAudioDeviceService
public interface IAudioDeviceService
{
    Task<IReadOnlyList<AudioDeviceInfo>> GetPlaybackDevicesAsync();
    Task<AudioDeviceInfo?> GetDefaultPlaybackDeviceAsync();
}

// IProfileService
public interface IProfileService
{
    Task<VolumeProfile?> GetProfileAsync(string deviceIdentifier);
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
```

### 3.3 Infrastructure レイヤー

Core インターフェースの実装。NAudio、Serilog、JSON I/O に依存。

```csharp
// DeviceMonitorService.cs
public class DeviceMonitorService : IDeviceMonitorService
{
    private IMMNotificationClient _notificationClient;
    private ILogger<DeviceMonitorService> _logger;
    
    public event EventHandler<DeviceChangedEventArgs>? DeviceChanged;
    
    public async Task<IReadOnlyList<AudioDeviceInfo>> GetAvailableDevicesAsync()
    {
        // NAudio を使用してデバイス一覧を取得
    }
    
    public async Task<AudioDeviceInfo?> GetCurrentDefaultDeviceAsync()
    {
        // 既定デバイスを取得
    }
}

// ProfileService.cs
public class ProfileService : IProfileService
{
    private readonly IProfileRepository _repository;
    private readonly ILogger<ProfileService> _logger;
    
    public async Task<VolumeProfile?> GetProfileAsync(string deviceIdentifier)
    {
        return await _repository.GetByIdentifierAsync(deviceIdentifier);
    }
    
    public async Task SaveProfileAsync(VolumeProfile profile)
    {
        await _repository.SaveAsync(profile);
    }
    
    // その他のメソッド...
}

// ProfileRepository.cs
public class ProfileRepository : IProfileRepository
{
    private readonly string _filePath;
    private readonly ILogger<ProfileRepository> _logger;
    
    public async Task<VolumeProfile?> GetByIdentifierAsync(string deviceIdentifier)
    {
        // profiles.json から読み込み、デバイス識別子で検索
    }
    
    public async Task SaveAsync(VolumeProfile profile)
    {
        // profiles.json に保存（再試行ロジック付き）
    }
}

// AudioVolumeService.cs
public class AudioVolumeService : IAudioVolumeService
{
    private ILogger<AudioVolumeService> _logger;
    
    public async Task<float> GetMasterVolumeAsync()
    {
        // NAudio で現在のマスターボリュームを取得
    }
    
    public async Task SetMasterVolumeAsync(float volume)
    {
        // NAudio でマスターボリュームを設定（再試行ロジック付き）
    }
    
    // その他のメソッド...
}
```

### 3.4 Console レイヤー

CLI ホスト、DI コンテナ、エントリーポイント。

```csharp
// Program.cs
public class Program
{
    public static async Task Main(string[] args)
    {
        var services = new ServiceCollection();
        ConfigureServices(services);
        
        var provider = services.BuildServiceProvider();
        var cli = provider.GetRequiredService<ICli>();
        
        await cli.ExecuteAsync(args);
    }
    
    private static void ConfigureServices(IServiceCollection services)
    {
        // Serilog 設定
        var logger = new LoggerConfiguration()
            .MinimumLevel.Debug()
            .WriteTo.Console(restrictedToMinimumLevel: LogEventLevel.Information)
            .WriteTo.File("path/to/logs", rollingInterval: RollingInterval.Day)
            .CreateLogger();
        
        services.AddSingleton(logger);
        
        // サービス登録
        services.AddSingleton<IDeviceMonitorService, DeviceMonitorService>();
        services.AddSingleton<IAudioDeviceService, AudioDeviceService>();
        services.AddSingleton<IProfileService, ProfileService>();
        services.AddSingleton<IAudioVolumeService, AudioVolumeService>();
        
        services.AddSingleton<ICli, CliHost>();
    }
}

// CliHost.cs
public class CliHost : ICli
{
    private readonly IDeviceMonitorService _monitorService;
    private readonly IProfileService _profileService;
    private readonly IAudioVolumeService _volumeService;
    
    public async Task ExecuteAsync(string[] args)
    {
        var command = ParseCommand(args);
        await HandleCommand(command);
    }
    
    private async Task HandleCommand(CliCommand command)
    {
        switch(command.Name)
        {
            case "run":
                await StartMonitoring();
                break;
            case "stop":
                await StopMonitoring();
                break;
            case "list-devices":
                await ListDevices();
                break;
            case "save-profile":
                await SaveProfile(command.Args[0]);
                break;
            // その他のコマンド...
        }
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

### 4.2 IAudioDeviceService

- **責責務**: 利用可能なデバイス情報の取得
- **依存**: NAudio の `MMDeviceEnumerator`
- **ライフタイム**: Singleton
- **スレッドセーフ**: 読み取り専用、スレッドセーフ

### 4.3 IProfileService

- **責務**: プロファイルの CRUD 操作
- **依存**: `IProfileRepository`
- **ライフタイム**: Singleton
- **スレッドセーフ**: リポジトリがファイルロックで保証

### 4.4 IAudioVolumeService

- **責務**: マスターボリューム・ミュート状態の取得・設定
- **依存**: NAudio の `AudioEndpointVolume`
- **ライフタイム**: Singleton
- **スレッドセーフ**: NAudio 実装に準ずる

---

## 5. データフロー

### 5.1 デバイス切り替え時のフロー

```
[1] IMMNotificationClient
    ↓ OnDefaultDeviceChanged イベント発火
[2] IDeviceMonitorService
    ↓ DeviceChanged イベント発火
[3] CliHost (メインループ)
    ↓ GetCurrentDefaultDeviceAsync() 呼び出し
[4] IAudioDeviceService
    ↓ 新デバイス情報を返す
[5] IProfileService
    ↓ GetProfileAsync(deviceIdentifier) で検索
[6] ProfileRepository
    ↓ profiles.json から読み込み
[7] IProfileService (結果判定)
    ├─ プロファイル存在
    │   ↓ IAudioVolumeService.SetMasterVolumeAsync()
    └─ プロファイル未存在
        ↓ 新規作成、現在の音量を記録
```

### 5.2 プロファイル保存時のフロー

```
[1] CLI コマンド: save-profile {device-identifier}
    ↓
[2] CliHost
    ↓ IAudioDeviceService.GetDefaultPlaybackDeviceAsync()
    ↓ IAudioVolumeService.GetMasterVolumeAsync()
    ↓ IAudioVolumeService.GetMuteStateAsync()
[3] VolumeProfile オブジェクト生成
    ↓
[4] IProfileService.SaveProfileAsync()
    ↓
[5] ProfileRepository
    ↓ profiles.json に追記/更新（再試行ロジック）
    ↓ .json.bak にバックアップ作成
```

---

## 6. エラーハンドリング

### 6.1 再試行戦略

- **初期遅延**: 100ms
- **乗数**: 1.5×
- **最大遅延**: 5,000ms
- **最大試行回数**: 5 回
- **対象**: Core Audio API 呼び出し、ファイル I/O

実装例:

```csharp
public class RetryPolicy
{
    public static async Task<T> ExecuteAsync<T>(
        Func<Task<T>> operation, 
        ILogger logger)
    {
        int attempt = 0;
        int delayMs = 100;
        
        while(attempt < 5)
        {
            try
            {
                return await operation();
            }
            catch(Exception ex)
            {
                attempt++;
                if(attempt >= 5)
                {
                    logger.LogError($"操作失敗（{attempt}回の再試行後）: {ex.Message}");
                    throw;
                }
                
                logger.LogWarning($"再試行 {attempt}/5 (遅延: {delayMs}ms)");
                await Task.Delay(delayMs);
                delayMs = Math.Min((int)(delayMs * 1.5), 5000);
            }
        }
    }
}
```

### 6.2 例外分類

| 例外タイプ | 対応 | ログレベル |
|-----------|------|----------|
| `COMException` (Core Audio API) | 再試行 | WARNING → ERROR |
| `IOException` (profiles.json) | 再試行後、バックアップから復帰 | WARNING → ERROR |
| `JsonException` (profiles.json 破損) | バックアップから復帰 | ERROR |
| その他の予期しない例外 | ログ記録、継続実行 | ERROR |

---

## 7. DI コンテナ構成

```csharp
public class ServiceConfiguration
{
    public static void ConfigureServices(IServiceCollection services)
    {
        // Logging
        var logger = CreateLogger();
        services.AddSingleton(logger);
        services.AddSingleton<ILogger>(logger);
        
        // Infrastructure
        services.AddSingleton<IDeviceMonitorService, DeviceMonitorService>();
        services.AddSingleton<IAudioDeviceService, AudioDeviceService>();
        services.AddSingleton<IProfileRepository, ProfileRepository>();
        services.AddSingleton<IProfileService, ProfileService>();
        services.AddSingleton<IAudioVolumeService, AudioVolumeService>();
        
        // Application
        services.AddSingleton<IDeviceProfileManager, DeviceProfileManager>();
        services.AddSingleton<ICli, CliHost>();
    }
    
    private static ILogger CreateLogger()
    {
        return new LoggerConfiguration()
            .MinimumLevel.Debug()
            .WriteTo.Console(restrictedToMinimumLevel: LogEventLevel.Information)
            .WriteTo.File(
                Path.Combine(
                    Environment.GetFolderPath(Environment.SpecialFolder.ApplicationData),
                    "VolumeProfileManager", "logs", "app-.txt"),
                rollingInterval: RollingInterval.Day,
                retainedFileCountLimit: 30)
            .CreateLogger();
    }
}
```

---

## 付録

### A. プロジェクト参照図

```
Console → Core + Infrastructure
Infrastructure → Core + Domain
Core → Domain
Domain (外部依存なし)
```

### B. 今後の拡張ポイント

- `IAudioSessionService` - アプリケーション別音量制御
- `IScheduleService` - スケジュール機能
- `IBluetoothDeviceService` - Bluetooth デバイス対応
