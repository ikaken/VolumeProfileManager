# VolumeProfileManager 設計書

## 文書の対象

- 第 1～7 章と付録 A/B は V1 の設計を記載する。付録 B の拡張候補は Issue #16 の対象外。
- V2 の詳細設計は [付録 C: V2 再構築設計案](#v2-design) を参照する。`rewrite/v2` での計画であり、設計承認済み・実装中。
- 関連文書: [V2 要件](VolumeProfileManager_spec.md#v2-spec)、[V2 検証計画](test_specification.md#v2-tests)、[V2 タスク](task_list.md#v2-tasks)。

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

<a id="v2-design"></a>

### C. Issue #16 V2 再構築設計案（設計承認済み）

本節は 2026-09-05 時点の設計案。既存の第 1～7 章と付録 A/B は V1 の説明として維持する。本節の追記は実装開始・タグ付け・プッシュ・リリースの承認を意味しない。

#### C.1 目的・開発ライン

- 機能追加をせず、Rust + windows-rs + Windows ネイティブトレイで再構築する。egui は初期依存に含めず、将来必要になった場合に別途検討する。
- `main` は V1 安定版、`rewrite/v2` は V2 開発用とする。V1 の修正は原則として V2 にマージせず、必要な修正のみ内容を評価して個別に移植する。
- V2 ベータは手動インストールのみ。同じ PC では V1/V2 を共存させず置き換える。ソースの並行開発と、アプリの共存は区別する。
- V2 安定後に人間が `rewrite/v2` を `main` にマージし、正式版 `v2.0.0` へ移行する。今回の作業でマージは実行しない。
- Windows x64 のユーザーローカルインストールと管理者権限不要の運用を維持する。現行仕様の Windows 10/11 対応は、採用依存と実機の両方で検証する。

#### C.2 構成・依存方向

Cargo workspace を導入し、次の責務に分割する。以下のパスは実装時に作成する候補であり、現時点では未作成。

| パス候補 | 責務 | 依存方向 |
|---|---|---|
| `crates/vpm-core/` | プロファイル、照合、適用・保存ユースケース、時刻・音量操作・保存の trait | Windows/UI 実装に依存しない |
| `crates/vpm-platform/` | windows-rs による Core Audio、JSON I/O、スタートアップ、排他制御 | core に依存 |
| `crates/vpm-tray/` | エントリーポイント、Win32 メニュー・トレイ・通知、構成の組み立て | core/platform に依存 |

- NAudio は windows-rs の Core Audio COM バインディングに置き換える。トレイは `Shell_NotifyIconW` と Win32 メニューを直接利用し、Tauri/WebView/egui/汎用トレイライブラリは初期導入しない。
- JSON は `serde` / `serde_json` を候補とし、日時・ログの依存も必要最小限にする。いずれも現行リポジトリには未導入であり、実装時に互換性・ライセンス・公開日を確認する。原則として公開後 7 日以上のリリースを選び、`Cargo.lock` と Rust toolchain を固定する。
- Windows API、COM、ハンドル操作の `unsafe` は platform とトレイの境界に限定する。ハンドル・コールバック・COM 初期化の寿命を所有者に結び付け、終了時に解放する。
- 独立した UI スレッドで Win32 メッセージループを実行する。Core Audio と永続化は COM を初期化した専用ワーカーで直列処理する。コールバックは処理要求を通知するだけとし、ファイル I/O や待機を行わない。
- 結果通知はメッセージで UI スレッドに戻す。待機にはイベント/タイムアウトを使用し、常時ポーリングや描画を行わない。終了時は新規要求を止め、通知登録解除、処理完了、ワーカー終了、リソース解放の順序を管理する。
- 配布する実行ファイル名は既存の `VolumeProfileManager.TrayApp.exe` を維持し、コンソールを表示しない。アプリ情報・アイコンを Windows リソースに埋め込む。
- 既存 .NET ソース/テストは互換性検証用に当面残すが、V2 インストーラーには同梱しない。既存ファイルの削除は対象を提示して別途確認を得る。

#### C.3 維持する機能・挙動

| 項目 | V2 の互換性要件 |
|---|---|
| 監視対象 | 既定の再生デバイス `eRender` / `eMultimedia`。既定変更は即時処理、追加・削除・状態変更は静定後に既定デバイスを再取得 |
| 適用タイミング | 有効なデバイス変更通知に対し末尾デバウンス 800ms、既定変更には即時パスも設ける。同一デバイスへの成功済み適用は 3 秒間抑制 |
| 起動時 | V1 同様、監視開始のみを理由に新たな音量適用を追加しない |
| 処理対象 | イベント内の一時的 ID ではなく、処理時の既定デバイスを取得。音量とミュートは同じ取得済みエンドポイントに対して扱う |
| 既存プロファイル | 音量、ミュートの順に適用。音量は 0～1 に制限。成功時のみ重複抑制状態を更新し、通知する |
| 未登録デバイス | 現在の音量・ミュートで自動作成し、作成通知を表示。既存プロファイルの自動適用だけでは `LastApplied` を書き換えない |
| 照合 | ID 完全一致 → 正規化名完全一致 → 双方向の名前包含一致。名前一致の候補は `LastApplied`、`CreatedAt` の降順、同順位は元の順序。ID 一致は先頭を採用 |
| 文字列互換 | 大小文字を無視する比較は .NET `OrdinalIgnoreCase` との互換性をテスト。ASCII のみの比較や一律の Unicode 小文字化で代用しない。内部の連続半角スペースと前後の空白の扱いも V1 に合わせる |
| トレイ | アプリ名・バージョン、ステータス表示、現在のプロファイル更新、スタートアップ登録/解除、終了。既存アイコンと日本語のバルーン通知を維持 |
| 障害・終了 | 即時適用失敗時は成功扱いにせず検証パスで再試行。デバイスなしでは保存・適用しない。終了後に遅延適用や通知を残さない |

時計と OS 操作を差し替え可能にし、デバウンス/抑制のテストでは実時間の sleep に依存しない。経過時間には単調時計、保存日時には UTC を用いる。

#### C.4 データ互換性と保護

- 保存先は `%LOCALAPPDATA%\VolumeProfileManager\profiles.json`、ルートは JSON 配列を維持する。
- 実装の `ProfileRepository` と `VolumeProfile` を基準に、キーは **`DeviceId`, `DeviceName`, `MasterVolume`, `IsMuted`, `CreatedAt`, `LastApplied`（PascalCase）** を維持する。仕様書の JSON サンプルも実装に合わせて PascalCase に統一する。
- 日時は V1 が読み戻せる ISO 8601 形式とし、UTC、可変長の秒以下、V1 の未設定日時 `0001-01-01T00:00:00` を含む fixture で検証する。数値は V1 の float と互換な範囲・精度で扱う。
- V1 が生成する正常な JSON について V1 → V2 → V1 の往復をテストする。新規のデータラッパーや必須キーを追加しない。既存項目更新時の `CreatedAt` 保持、ID/名前の照合と削除条件も維持する。
- 初回移行前にプロファイルと通常バックアップを、通常の `.bak` ローテーションと独立した退避先へコピーする。既存退避を上書きせず、コピーが検証できなければ移行を進めない。実ユーザーのデータをテスト fixture として取得しない。
- 保存は一時ファイルへ書き込み・検証後、同一ディレクトリ内で置換する。直前の正常データを `profiles.json.bak` に保持し、保存失敗を成功通知しない。
- **承認対象の安全性改善**：V1 は JSON 破損時にバックアップをコピーしても当該読み込みで空リストを返すため、その後の保存で復旧データを失う可能性がある。V2 はバックアップを検証・再読込してから復旧し、正常データが得られなければ自動作成/上書きを中断する。
- **承認対象の安全性改善**：音量取得の失敗を `0` / `false` という実測値に置き換えて保存しない。エラーとして扱い、既存設定を保護する。通常の音量取得・保存機能は変えない。
- ログは仕様書にある `%LOCALAPPDATA%\VolumeProfileManager\logs\` を候補とし、日次ローテーション・30 ファイル保持を維持する。V1 実装のカレントディレクトリ相対パスから変わる点は動作確認対象とする。

#### C.5 インストール置換・起動制御・V1 への復帰

- Inno Setup と既存 AppId `8F2B6C6E-3D2E-4C7A-9A8B-1E5D6F7A9C10` を維持する。インストール先とアンインストール登録を一本化し、ベータ専用の別アプリは作らない。
- V1 → V2 は「旧版検出 → 終了確認 → 設定/スタートアップ状態退避 → ユーザー確認付きで旧版アンインストール → 同じ場所へ V2 インストール → 状態引き継ぎ」の置換とする。V1 の .NET 配布物を上書きだけで残さない。
- アンインストールは確認済みの本アプリの登録に限定する。任意ディレクトリの一括削除や、同名というだけのプロセス強制終了はしない。旧版終了・退避・アンインストールに失敗した場合は先へ進まない。
- スタートアップは既存の `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` の `VolumeProfileManager` を維持し、引用符付きの置換後実行パスを登録する。旧版で無効なら自動的に有効化しない。
- V2 は同一ユーザーの二重起動を名前付きミューテックスで抑止する。V1 はミューテックスに参加しないため、旧版プロセスの検出・終了確認とセットで扱い、ミューテックスだけで V1 排他を保証したとはしない。
- サポートする運用はインストーラーによる単一インストール。別ディレクトリへ手動コピーした V1 の後発起動まで完全に封じるには V1 側の協調が必要であり、対応範囲に含めない。実機テスト前にも旧版の停止を確認する。
- V1 に戻す場合は V2 を終了・アンインストールし、V1 を再インストールする。必要に応じて移行前退避からユーザー確認付きで復元する。復元するとベータ利用中の変更が失われることを案内する。
- インストール失敗時も退避データは削除しない。自動ロールバックの成功は前提とせず、V1 再インストールによる復帰手順を実機検証する。

#### C.6 ビルド・ベータ配布

- `rewrite/v2` 上の `.github/workflows/release.yml` を Rust ビルドへ置き換える。`main` 上の V1 workflow は維持し、各タグが指すコミットの workflow でそれぞれ配布する。
- ベータは `rewrite/v2`、正式版は安定化・人間のマージ後の `main` を基準とする。release-launcher スキルの一般例にある `develop` は Issue #16 には採用しない。
- 初回ベータの候補は `v2.0.0-beta`、正式版は Issue 指定の `v2.0.0`。ローカル最新タグは `v1.2.0-beta`、Issue ラベルは `enhancement` だが、今回は Issue に明示された major 変更を優先する。実際のリリース時に最新タグを再取得し、番号・タグ付け・プッシュは別途承認を得る。
- Cargo、トレイ表示、Windows バージョンリソース、インストーラーの表示バージョンを整合させる。Windows の数値版とベータ接尾辞付き表示版は区別する。現行インストーラーの `1.0.0` 固定値は維持しない。
- `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets --locked -- -D warnings`、`cargo test --workspace --locked`、`cargo build --release --locked --target x86_64-pc-windows-msvc` を検証候補とする。
- 合格後に Inno Setup で V2 の成果物だけを梱包する。ベータでは GitHub Release の `prerelease: true` と `make_latest: false` を明示する。現行 workflow にこの分岐はなく、タグ名だけでは自動分類されない。
- 自動アップデートは Issue #15 の別スコープであり、今回追加しない。将来導入する更新側でも Pre-release を除外する必要がある。

#### C.7 実装順序・検証・承認ゲート

1. V1 の合成 fixture と既存テストを基に、JSON 往復、照合順序、日時、大小文字/日本語/空白の互換性テストを用意する。
2. Cargo workspace、core と JSON 永続化を実装し、正常系・破損・バックアップなし・書込失敗・退避保持を自動テストする。
3. Core Audio とイベント処理を実装し、既定変更即時適用、800ms 静定、3 秒抑制、連続切替、失敗後の再試行、終了時キャンセルをモックと仮想時計で検証する。
4. ネイティブトレイ・日本語通知・バージョン表示・アイコン・スタートアップ・排他を実装する。Explorer 再起動時のトレイ再登録も検証する。
5. 置換インストーラーと Rust release workflow を実装し、V1 → V2、V2 → 次のベータ、V2 → V1、通常アンインストール、旧版実行中/退避失敗時の中断を検証する。
6. Windows 10/11 で、スピーカー/USB/Bluetooth の切替・未登録デバイス・ミュート・デバイスなし・再ログオンを確認する。常駐時 CPU/メモリ/成果物サイズも V1 と比較し、未計測の改善値を断定しない。

- 設計承認後に実装を開始し、テスト結果を提示してユーザーへ動作確認を依頼する。動作確認承認までは完了フェーズに進まない。
- 本設計段階ではアプリ実装、依存追加、旧版削除、インストール、タグ付け、プッシュ、PR 作成、リリースは行わない。
- PR 作成・タグ付け/プッシュはそれぞれ別途承認を得る。`main` へのマージと V1 終了判断は人間が行う。
