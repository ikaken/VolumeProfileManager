# VolumeProfileManager 仕様書

## 文書の対象

第 1～7 章は V1 の仕様、第 8 章はロードマップ、第 9 章は [Issue #16 の V2 要件案](#v2-spec) を記載する。V2 は設計承認済み・実装中であり、V1 の .NET/NAudio 構成を既に置き換えたことを意味しない。

---

## 目次

1. [概要](#1-概要)
2. [プロジェクト構成・命名規則](#2-プロジェクト構成命名規則)
3. [機能要件](#3-機能要件)
4. [非機能要件](#4-非機能要件)
5. [UI仕様](#5-ui仕様)
6. [技術構成](#6-技術構成)
7. [制約事項とリスク](#7-制約事項とリスク)
8. [ロードマップ](#8-ロードマップ)
9. [V2 再構築要件（Issue #16）](#v2-spec)

---

## 1. 概要

### 1.1 アプリケーション名称

| 項目 | 内容 |
|------|------|
| 正式名称 | VolumeProfileManager |
| 略称 | VPM |
| コンセプト | オーディオデバイス切り替え時に音量・ミュート状態を自動調整する常駐型ユーティリティ |

### 1.2 目的

Windows環境において以下の機能を提供するスタンドアロン型アプリケーションを開発する。

- オーディオデバイスの自動監視
- デバイス切り替え検知
- デバイスごとの音量プロファイル管理
- デバイス変更時の自動音量調整
- タスクトレイ常駐による最小限の UI

### 1.3 対象OS・環境

| 項目 | 内容 |
|------|------|
| 対応OS | Windows 10（1903以降）、Windows 11（全バージョン） |
| ランタイム | .NET 10.0（Self-contained、ランタイム同梱） |
| 権限 | 管理者権限不要 |
| 形式 | タスクトレイ常駐アプリ（`VolumeProfileManager.TrayApp`）。CLI版（`VolumeProfileManager.Console`）は廃止し、TrayAppに一本化（Issue #1） |

### 1.4 スコープ

このプロジェクトは以下を**含まない**：

- アプリケーション単位の音量制御（AudioPilot の機能）
- デバイス割り当て・ルーティング（AudioPilot の機能）
- GUI での プロファイル作成・編集ツール
- プロセス監視・自動切り替え

---

## 2. プロジェクト構成・命名規則

### 2.1 ディレクトリ構成

```
VolumeProfileManager/
 ├─ src/
 │   ├─ VolumeProfileManager.Core           # ユースケース・サービスインターフェース定義
 │   ├─ VolumeProfileManager.Infrastructure # Core Audio API・設定ファイル実装
 │   ├─ VolumeProfileManager.TrayApp        # エントリーポイント・タスクトレイ常駐ホスト
 │   └─ VolumeProfileManager.Domain         # エンティティ・値オブジェクト
 ├─ tests/
 │   └─ VolumeProfileManager.UnitTests
 ├─ installer/                                # Inno Setup インストーラースクリプト
 ├─ docs/                                    # 仕様書・ユーザーマニュアル
 └─ profiles/                                # （不使用）
```

> `VolumeProfileManager.Console`（CLI版）は Issue #1 対応により廃止済み。CLIコマンド相当の機能はTrayAppのメニュー（ステータス表示・プロファイル更新）に統合されている。

### 2.2 各レイヤーの責務

| プロジェクト | 責務 |
|-------------|------|
| VolumeProfileManager.Domain | `AudioDeviceInfo`, `VolumeProfile`, `DeviceChangedEventArgs` 等のエンティティ定義。外部依存なし |
| VolumeProfileManager.Core | ユースケース実装・サービスインターフェース定義 |
| VolumeProfileManager.Infrastructure | Core Audio API・ファイルI/O・Serilog 等の具体的実装 |
| VolumeProfileManager.TrayApp | タスクトレイ常駐ホスト・DI コンテナ・エントリーポイント・Win32 API直接呼び出しによるトレイUI |

### 2.3 命名規則

| 種別 | ルール |
|------|--------|
| ソリューション | VolumeProfileManager |
| 名前空間 | VolumeProfileManager.* |
| アセンブリ | VolumeProfileManager.* |
| 設定ファイル | `profiles.json` (ユーザーディレクトリ: `%LOCALAPPDATA%\VolumeProfileManager\`) |

---

## 3. 機能要件

### 3.1 デバイス監視・検知

#### 3.1.1 既定の再生デバイス変更を検知

Windows Core Audio API を用いてオーディオデバイス変更をリアルタイム監視する。

| 項目 | 仕様 |
|-----|------|
| 監視対象 | 既定のマルチメディア再生デバイス（DataFlow=Render, Role=Multimedia） |
| 検知イベント | デバイス追加・削除・状態変更・既定デバイス変更 |
| 応答時間 | `OnDefaultDeviceChanged` 検知時は即時にプロファイル適用を開始。状態変更等は静定後に処理 |
| デバウンス | 末尾デバウンス 800ms。即時適用と検証パスを併用し、安定性を確保 |
| 重複抑制 | 同一デバイスは 3 秒間再適用・再通知しない |
| ログ | 全イベントを Serilog で記録 |

#### 3.1.2 デバイス一覧取得

```csharp
public interface IDeviceMonitorService
{
    event EventHandler<DeviceChangedEventArgs>? DeviceChanged;
    Task<IReadOnlyList<AudioDeviceInfo>> GetAvailableDevicesAsync();
    Task<AudioDeviceInfo?> GetCurrentDefaultDeviceAsync();
}
```

### 3.2 音量プロファイル管理

#### 3.2.1 プロファイルデータ構造

デバイスごとに以下の設定を保存・復元する：

```csharp
public class VolumeProfile
{
    public string DeviceId { get; set; }           // Core Audio デバイス ID
    public string DeviceName { get; set; }         // 表示名（例:"Realtek Audio"）
    public float MasterVolume { get; set; }        // マスターボリューム (0.0 - 1.0)
    public bool IsMuted { get; set; }              // ミュート状態
    public DateTime CreatedAt { get; set; }        // プロファイル作成日時
    public DateTime LastApplied { get; set; }      // 最後に適用した日時
}
```

#### 3.2.1.1 プロファイル保存サービス

`IProfileService` は `profiles.json` の読み書きとプロファイル管理を担当する。

```csharp
public interface IProfileService
{
    Task<VolumeProfile?> GetProfileAsync(string deviceIdentifier, string? deviceName = null);
    Task<IReadOnlyList<VolumeProfile>> GetAllProfilesAsync();
    Task SaveProfileAsync(VolumeProfile profile);
    Task DeleteProfileAsync(string deviceIdentifier);
}
```

- `GetProfileAsync(string deviceIdentifier, string? deviceName = null)`
  - `deviceIdentifier` でプロファイルを検索する
  - 一致しなければ `deviceName` をフォールバックとして使用する
  - どちらも一致しなければ `null` を返す
- `GetAllProfilesAsync()`
  - 全プロファイルを取得する
- `SaveProfileAsync(VolumeProfile profile)`
  - `deviceId` をキーとして新規登録または上書き保存する
- `DeleteProfileAsync(string deviceIdentifier)`
  - 指定したデバイスのプロファイルを削除する

#### 3.2.2 設定ファイル管理

- **保存先**：`%LOCALAPPDATA%\VolumeProfileManager\profiles.json`
- **形式**：JSON 配列。プロパティ名は V1 実装の保存形式である PascalCase とし、V2 でも維持する
- **初期化**：V1 はリポジトリ生成時にディレクトリを作成し、初回保存時に `profiles.json` を作成する

```json
[
  {
    "DeviceId": "{0.0.1.00000000}.{xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}",
    "DeviceName": "Realtek Audio",
    "MasterVolume": 0.75,
    "IsMuted": false,
    "CreatedAt": "2026-07-03T12:00:00Z",
    "LastApplied": "2026-07-03T13:45:00Z"
  }
]
```

- 書き込み前に `.json.bak` へバックアップを作成する

### 3.3 自動音量調整

#### 3.3.1 デバイス切り替え時の動作

1. **デバイス変更を検知** → `DeviceMonitorService` が `DeviceChanged` イベント発火（`ChangeType` 付き）
2. **即時適用**（`DefaultDeviceChanged` のみ）→ `DeviceMonitorOrchestrator` が待たずに処理を開始
3. **検証パス**（全イベント共通）→ 800ms 末尾デバウンス後に確定デフォルトデバイスを取得して処理
4. **新しい既定デバイスを取得** → `AudioDeviceService.GetDefaultPlaybackDeviceAsync()`
5. **プロファイルを検索** → `ProfileService.GetProfileAsync(deviceId, deviceName)`
6. **プロファイルが存在する場合** → 保存された音量・ミュートを適用
7. **プロファイルが存在しない場合** → 新規作成し、現在の音量を記録

```
デバイス A (Volume=70%) → デバイス B に切り替え
  ├─ DefaultDeviceChanged を即時処理
  ├─ B のプロファイル存在
  │   └─ 保存された Volume=50% を自動適用
  └─ B のプロファイル未作成
      └─ 新規作成、現在の音量を記録
```

#### 3.3.2 音量調整の実装

NAudio を用いてマスターボリュームとミュート状態を制御する。
`IAudioVolumeService` は `Core` 層で定義し、`Infrastructure` 層で NAudio を使った実装を提供する。

```csharp
public interface IAudioVolumeService
{
    Task<float> GetMasterVolumeAsync();
    Task SetMasterVolumeAsync(float volume); // 0.0 - 1.0
    Task<bool> GetMuteStateAsync();
    Task SetMuteStateAsync(bool isMuted);
}
```

- `GetMasterVolumeAsync()`
  - 現在のマスターボリュームを 0.0〜1.0 の範囲で取得する
- `SetMasterVolumeAsync(float volume)`
  - 指定したマスターボリュームを適用する（`Math.Clamp` で 0.0〜1.0 に制限）
- `GetMuteStateAsync()`
  - 現在のミュート状態を取得する
- `SetMuteStateAsync(bool isMuted)`
  - ミュート/ミュート解除を適用する

#### 3.3.2.1 実装の考え方

- `Core` ではインターフェースのみを定義し、依存を分離する
- `Infrastructure` では NAudio の `MMDevice` や `AudioEndpointVolume` を利用して実装する
- `SetMasterVolumeAsync` では、0.0〜1.0 の範囲に収まるよう Clamp する
- `IAudioVolumeService` には、将来的にアプリケーション単位の音量制御を追加するための拡張余地を残す

### 3.4 タスクトレイ操作

> CLI版（`vpm run` / `vpm status` / `vpm list-devices` / `vpm save-profile` / `vpm delete-profile` 等）は廃止された。同等の機能はタスクトレイアイコンの右クリックメニューに統合されている。

#### 3.4.1 トレイメニュー

| メニュー項目 | 説明 |
|---------|------|
| ステータス表示 | 現在のデバイス・音量・ミュート状態をバルーン通知で表示 |
| プロファイルを更新（現在の音量を保存） | 現在のデフォルトデバイスの音量・ミュート状態をプロファイルとして保存 |
| スタートアップ登録/解除 | Windowsログオン時の自動起動をトグル |
| 終了 | アプリケーションを終了 |

デバイス切り替えの検知・プロファイル自動適用・新規プロファイルの自動作成はバックグラウンドで常時動作し、ユーザー操作は不要。適用結果はバルーン通知で表示される。

### 3.5 開発フェーズ

本プロジェクトは以下のフェーズで段階的に実装を進める。フェーズ 1〜8 は完了している。

1. **デバイス判別方法を調査する**（完了）
2. **取得済みデバイスの音量・ミュート操作を確認する**（完了）
3. **常驻してデバイス切り替わりを自動で検出し、デバイス情報を取得・表示する**（完了）
4. **切り替わったデバイスの情報を自動で取得する**（完了、フェーズ 3 に統合）
5. **現在のデバイス情報を手動でプロファイルとして保存する**（完了）
6. **デバイスが切り替わった時に自動で対応するプロファイルを認識する**（完了）
7. **デバイスが切り替わった時に自動的に対応するプロファイル情報を適用する**（完了）
8. **Console版廃止・TrayApp一本化・インストーラー対応（Issue #1）**（完了）

---

## 4. 非機能要件

### 4.1 パフォーマンス

| 要件 | 基準 |
|-----|------|
| デバイス変更検知応答時間 | `OnDefaultDeviceChanged` は即時適用開始。状態変更系は 800ms 静定後 |
| 音量調整応答時間 | 200ms 以内 |
| メモリ使用量 | 30MB 以下（アイドル時） |
| CPU 使用率 | 1% 以下（アイドル時） |

### 4.2 信頼性

| 要件 | 仕様 |
|-----|------|
| 設定ファイル破損対策 | バックアップファイル自動作成（`.json.bak`） |
| エラーハンドリング | 全ての例外を Serilog で記録、アプリ継続実行 |
| Core Audio API エラー | 例外をキャッチしてログ出力、即時適用失敗時は検証パスで再試行の機会を残す |
| 重複適用防止 | 同一デバイス 3 秒抑制 + 適用処理の直列化 |

### 4.3 セキュリティ

| 要件 | 対応 |
|-----|------|
| 管理者権限 | 不要 |
| 設定ファイル保護 | ユーザーの `%LOCALAPPDATA%` 内に保存（OS ユーザー分離） |
| ログ情報 | 機密情報（シークレット等）を含めない |

### 4.3 ログ設定

- ファイル: `INFO` 以上を記録する
- ログ出力先: `%LOCALAPPDATA%\VolumeProfileManager\logs\`
- ローテーション: 日次
- 保持期間: 30 日程度（運用ポリシーによって調整可能）

ファイルログには詳細なデバッグ情報を含め、Serilog の最小レベルは `Debug`、ファイル出力は `Information` 以上とする。

### 4.4 互換性

- .NET 10.0 自己包含実行ファイル（SCD）で配布
- Windows 10 1903 以降で動作確認

---

## 5. UI仕様

### 5.1 ログ出力例

#### 5.1.1 起動時ログ

```
[13:45:00 INF] VolumeProfileManager TrayApp starting...
[13:45:00 INF] DeviceMonitorService initialized and registered callback.
[13:45:00 INF] AudioDeviceService initialized.
[13:45:00 INF] DeviceMonitorOrchestrator started.
```

#### 5.1.2 デバイス変更ログ

```
[13:50:23 INF] Default playback device changed: {0.0.0.00000000}.{xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}
[13:50:23 INF] Device changed detected: Headphones ({0.0.0.00000000}.{...})
[13:50:23 INF] Matched profile Headphones (...) for input ID '...'
[13:50:23 INF] Profile applied: Volume=50 %, Muted=false for Headphones
```

### 5.2 トレイアイコン

Windows トレイアイコンを主UIとして採用（Win32 API直接呼び出し、WinForms不使用）：

- 右クリックメニュー: ステータス表示 / プロファイルを更新 / スタートアップ登録/解除 / 終了
- プロファイル自動適用時のバルーン通知（音量制御非対応デバイスの場合はエラー通知）

---

## 6. 技術構成

### 6.1 スタック

| レイヤー | 技術 | バージョン |
|---------|------|-----------|
| ランタイム | .NET | 10.0 |
| 言語 | C# | 13.0 |
| Core Audio API | NAudio | 2.2.0 |
| ロギング | Serilog | 3.1.1 |
| DI | Microsoft.Extensions.DependencyInjection | 8.0.0 |
| テスト | xUnit | 2.9.3 |
| ビルド | MSBuild | .NET 10.0 SDK 付属 |

### 6.2 外部依存

```xml
<!-- VolumeProfileManager.Infrastructure -->
<PackageReference Include="NAudio" Version="2.2.0" />
<PackageReference Include="Serilog" Version="3.1.1" />

<!-- VolumeProfileManager.TrayApp -->
<PackageReference Include="Microsoft.Extensions.DependencyInjection" Version="8.0.0" />
<PackageReference Include="Serilog" Version="3.1.1" />
<PackageReference Include="Serilog.Sinks.File" Version="5.0.0" />
```

### 6.3 ログ設定

ログレベル：

```
[DEBUG]   - 関数呼び出し、パラメータ値
[INFO]    - デバイス変更、音量調整
[WARNING] - 即時適用失敗、非推奨 API 使用
[ERROR]   - 例外発生、失敗した操作
```

ログ出力先：

- **ファイル**：`%LOCALAPPDATA%\VolumeProfileManager\logs\` （ローテーション：日次）

---

## 7. 制約事項とリスク

### 7.1 制約事項

| 項目 | 内容 |
|-----|------|
| ユーザーセッション | 通常ユーザーセッション内でのみ動作（NT AUTHORITY\SYSTEM での実行は非対応） |
| Core Audio API 非公式 | Windows 上の Core Audio API は Microsoft の正式な公開 API ではない |
| デバイス ID | デバイス ID は Windows 再起動後に変更される可能性がある |
| 並行実行 | VolumeProfileManager インスタンスの複数実行は非推奨 |

### 7.2 リスク・対策

| リスク | 対策 |
|--------|------|
| Core Audio API 仕様変更 | NAudio で抽象化、テストカバレッジ 80% 以上を維持 |
| デバイス ID 変更 | デバイス名を含む多段階マッチングを実装 |
| 音量調整の失敗 | 例外をキャッチしログ出力、即時失敗は検証パスで救済 |
| リソースリーク | using 宣言・Dispose パターンの徹底 |
| 即時適用と検証パスの競合 | `SemaphoreSlim` による直列化、3 秒間の重複抑制 |

---

## 8. ロードマップ

### v1.0（リリース済み）

- [x] デバイス監視機能
- [x] 音量プロファイル管理（CRUD）
- [x] 自動音量調整
- [x] ユニット・統合テスト
- [x] タスクトレイ常駐
- [x] インストーラー対応

### v1.1.0-beta（リリース済み）

- [x] プロファイル適用レスポンス向上（Issue #6）
- [x] `OnDefaultDeviceChanged` 即時適用
- [x] 通知種別（`DeviceChangeType`）の導入

### v1.1.0（正式版予定）

- [ ] `v1.1.0-beta` の動作確認とフィードバック反映
- [ ] README ダウンロードリンク更新

### v2.0.0 - 新技術スタックによる全面再構築（Issue #16）

- [ ] Rust + windows-rs + Windows ネイティブトレイへ移行（機能追加なし）
- [ ] V1 の正常系動作・プロファイル形式との互換性を検証
- [ ] V1 と共存しない置換インストーラーを実装
- [ ] `rewrite/v2` からベータを手動配布（Pre-release、自動アップデート対象外）
- [ ] 安定化後に人間が `main` へマージし、`v2.0.0` を正式リリースして V1 を終了

### 将来の拡張候補（Issue #16 の対象外・バージョン未定）

以下は旧ロードマップの候補であり、V2 の受け入れ条件には含めない。

- [ ] タイムスケジュール機能（朝：60%, 夜：30%）
- [ ] アプリケーション別プロファイル
- [ ] Bluetooth デバイスの検出・接続状態管理
- [ ] REST API（他のツール連携用）

---

<a id="v2-spec"></a>

## 9. V2 再構築要件（Issue #16・設計承認済み）

詳細な構成・処理方式は [設計書の V2 設計](design.md#v2-design)、検証条件は [V2 テスト計画](test_specification.md#v2-tests)、作業順序は [V2 タスク](task_list.md#v2-tasks) を参照する。

### 9.1 対象と技術構成

| 項目 | 要件 |
|---|---|
| 開発ライン | `main` は V1 安定版、`rewrite/v2` は V2。V1 修正は原則マージしない |
| 技術 | Rust、windows-rs、Win32 トレイ/メニュー/通知、Inno Setup。egui は必要になった場合のみ別途検討し、初期版には含めない |
| 利用環境 | Windows x64、管理者権限不要。Windows 10/11 対応を依存確認と実機で検証 |
| 機能 | 既定再生デバイス監視、音量/ミュートの保存と自動適用、未登録デバイスの自動作成、現行トレイ操作を維持 |
| 適用 | `eRender` / `eMultimedia`、既定変更の即時適用、800ms 末尾デバウンス、成功済み同一デバイスへの 3 秒間の重複抑制 |
| データ | 同じ `profiles.json` の形式（PascalCase の 6 項目、JSON 配列、V1 が読める日時/数値形式）。通常モードは `%LOCALAPPDATA%\VolumeProfileManager\`、ポータブルモード（`portable.flag` 存在時）は `data\` 配下に保存 |
| 対象外 | 新しい画面、スケジュール、アプリ別音量、REST API、自動アップデート実装（Issue #15） |

### 9.2 置換・復帰・配布

- V1/V2 を別アプリとして同時にインストール・実行する運用にはしない。既存 AppId、インストール先、スタートアップ登録名を維持する。
- 旧版終了を確認し、設定とスタートアップ状態を退避してから、ユーザー確認付きで旧版をアンインストールし V2 を導入する。確認や退避に失敗したら置換を中断する。
- スタートアップの有効/無効を引き継ぐ。V2 は二重起動を抑止し、V1 との排他は旧版検出・終了確認と組み合わせる。任意の場所へ手動コピーした V1 の後発起動までの完全な抑止は対象外とする。
- 移行前退避は通常の `.bak` と分離して保持する。V1 への復帰は V2 終了・アンインストール → V1 再インストール → 必要に応じて確認付き設定復元とし、ベータ中の変更を失う可能性を案内する。
- ベータはインストーラー付きの手動配布およびポータブル版 ZIP の同時配布。GitHub Release の `prerelease: true` / `make_latest: false` を明示し、正式版扱いにしない。タグ名だけに依存しない。
- 正式版は `v2.0.0`。ベータの番号・タグ付け・プッシュ・公開、PR 作成はそれぞれ実施前に承認を得る。`main` へのマージは人間が行う。

### 9.3 エラー時の挙動変更案

通常機能の追加とは区別し、次を設計レビューの承認対象とする。

- JSON 破損時は正常なバックアップを検証・再読込し、復旧不能なら空データでの自動作成/上書きを中断する。
- 音量取得失敗を `0` / `false` として保存せず、エラーとして既存データを保護する。
- 保存には一時ファイルと置換を用い、失敗時に正常な元データを保持する。移行前退避を通常保存で上書きしない。
- ログはカレントディレクトリ相対ではなく、`%LOCALAPPDATA%\VolumeProfileManager\logs\` を候補とする。日次ローテーション・30 ファイル保持を検証する。

---

## 付録

### A. Core Audio API イベントフロー

```
IMMNotificationClient
  ├─ OnDeviceAdded(deviceId)
  ├─ OnDeviceRemoved(deviceId)
  ├─ OnDeviceStateChanged(deviceId, state)
  ├─ OnDefaultDeviceChanged(flow, role, deviceId)  ← 監視対象・即時適用対象
  └─ OnPropertyValueChanged(deviceId, key)
```

### B. 既存プロジェクト（AudioPilot）との差異

| 項目 | AudioPilot | VolumeProfileManager |
|-----|-----------|----------------------|
| UI | WinUI 3 | タスクトレイアイコン（Win32 API） |
| 対象 | アプリケーション単位 | デバイス単位 |
| スコープ | 統合制御 | シンプル・シングルタスク |
| 配布 | 1つの実行ファイル | Self-contained インストーラー |
