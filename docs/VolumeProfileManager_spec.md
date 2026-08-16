# VolumeProfileManager 仕様書

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

---

## 1. 概要

### 1.1 アプリケーション名称

| 項目 | 内容 |
|------|------|
| 正式名称 | VolumeProfileManager |
| 略称 | VPM |
| コンセプト | オーディオデバイス切り替え時に音量・音声設定を自動調整する常駐型ユーティリティ |

### 1.2 目的

Windows環境において以下の機能を提供するスタンドアロン型アプリケーションを開発する。

- オーディオデバイスの自動監視
- デバイス切り替え検知
- デバイスごとの音量プロファイル管理
- デバイス変更時の自動音量調整
- 最小限の UI で軽量実行

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
 └─ profiles/                                # デフォルト設定ファイル
```

> `VolumeProfileManager.Console`（CLI版）は Issue #1 対応により廃止済み。CLIコマンド相当の機能はTrayAppのメニュー（ステータス表示・プロファイル更新）に統合されている。

### 2.2 各レイヤーの責務

| プロジェクト | 責務 |
|-------------|------|
| VolumeProfileManager.Domain | `AudioDevice`, `VolumeProfile`, `DeviceSetting` 等のエンティティ定義。外部依存なし |
| VolumeProfileManager.Core | ユースケース実装・サービスインターフェース定義 |
| VolumeProfileManager.Infrastructure | Core Audio API・ファイルI/O・Serilog 等の具体的実装 |
| VolumeProfileManager.TrayApp | タスクトレイ常駐ホスト・DI コンテナ・エントリーポイント・Win32 API直接呼び出しによるトレイUI |

### 2.3 命名規則

| 種別 | ルール |
|------|--------|
| ソリューション | VolumeProfileManager |
| 名前空間 | VolumeProfileManager.* |
| アセンブリ | VolumeProfileManager.* |
| 設定ファイル | `profiles.json` (ユーザーディレクトリ: `%APPDATA%\VolumeProfileManager\`) |

---

## 3. 機能要件

### 3.1 デバイス監視・検知

#### 3.1.1 既定の再生デバイス変更を検知

Windows Core Audio API を用いてオーディオデバイス変更をリアルタイム監視する。

| 項目 | 仕様 |
|-----|------|
| 監視対象 | 既定のマルチメディア再生デバイス（DataFlow=Render, Role=Multimedia） |
| 検知イベント | デバイス追加・削除・状態変更・既定デバイス変更 |
| 応答時間 | 100ms 以内 |
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

`IProfileService` は `profiles.json` の読み書きとプロファイル管理を担当する。シンプルかつ拡張性のある契約として、以下のメソッドを定義する。

```csharp
public interface IProfileService
{
    Task<VolumeProfile?> GetProfileAsync(string deviceIdentifier);
    Task<IReadOnlyList<VolumeProfile>> GetAllProfilesAsync();
    Task SaveProfileAsync(VolumeProfile profile);
    Task DeleteProfileAsync(string deviceIdentifier);
}
```

- `GetProfileAsync(string deviceIdentifier)`
  - `deviceIdentifier` でプロファイルを検索し、存在しなければ `null` を返す
- `GetAllProfilesAsync()`
  - 全プロファイルを取得する
- `SaveProfileAsync(VolumeProfile profile)`
  - `deviceId` をキーとして新規登録または上書き保存する
- `DeleteProfileAsync(string deviceId)`
  - 指定したデバイスのプロファイルを削除する

#### 3.2.2 設定ファイル管理

- **保存先**：`%APPDATA%\VolumeProfileManager\profiles.json`
- **形式**：JSON
- **初期化**：アプリ起動時に自動作成

```json
{
  "version": "1.0",
  "profiles": [
    {
      "deviceId": "{0.0.1.00000000}.{xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx}",
      "deviceName": "Realtek Audio",
      "masterVolume": 0.75,
      "isMuted": false,
      "createdAt": "2026-07-03T12:00:00Z",
      "lastApplied": "2026-07-03T13:45:00Z"
    }
  ]
}
```

### 3.3 自動音量調整

#### 3.3.1 デバイス切り替え時の動作

1. **デバイス変更を検知** → DeviceMonitorService が DeviceChanged イベント発火
2. **新しい既定デバイスを取得** → AudioDeviceService.GetCurrentDefaultDeviceAsync()
3. **プロファイルを検索** → ProfileService.GetProfileAsync(deviceId)
4. **プロファイルが存在する場合** → 保存された音量を適用
5. **プロファイルが存在しない場合** → 新規作成し、現在の音量を記録

```
デバイス A (Volume=70%) → デバイス B に切り替え
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
  - 指定したマスターボリュームを適用する
- `GetMuteStateAsync()`
  - 現在のミュート状態を取得する
- `SetMuteStateAsync(bool isMuted)`
  - ミュート/ミュート解除を適用する

#### 3.3.2.1 実装の考え方

- `Core` ではインターフェースのみを定義し、依存を分離する
- `Infrastructure` では NAudio の `MMDevice` や `AudioEndpointVolume` を利用して実装する
- `SetMasterVolumeAsync` では、0.0〜1.0 の範囲に収まる入力値を前提とする
- `IAudioVolumeService` には、将来的にアプリケーション単位の音量制御を追加するための拡張余地を残す

### 3.4 タスクトレイ操作（旧: コマンドライン操作）

> **廃止（Issue #1）**: CLI版（`vpm run` / `vpm status` / `vpm list-devices` / `vpm save-profile` / `vpm delete-profile` 等）は廃止された。同等の機能はタスクトレイアイコンの右クリックメニューに統合されている。

#### 3.4.1 トレイメニュー

| メニュー項目 | 説明 |
|---------|------|
| ステータス表示 | 現在のデバイス・音量・ミュート状態をバルーン通知で表示 |
| プロファイルを更新（現在の音量を保存） | 現在のデフォルトデバイスの音量・ミュート状態をプロファイルとして保存 |
| スタートアップ登録/解除 | Windowsログオン時の自動起動をトグル |
| 終了 | アプリケーションを終了 |

デバイス切り替えの検知・プロファイルの自動適用・新規プロファイルの自動作成はバックグラウンドで常時動作し、ユーザー操作は不要。適用結果はバルーン通知で表示される。

## 3.5 開発フェーズ

本プロジェクトは以下のフェーズで段階的に実装を進める。

1. **デバイス判別方法を調査する**
   - 目的: Windows のデバイス ID と識別子の挙動を確認し、安定したプロファイル紐付け方式を決定する
   - 検証内容:
     - デバイス切り替え時に `DeviceId` がどのように変化するか
     - 同一デバイスの再接続・再起動後の `DeviceId` の変化
     - `DeviceName` やその他識別子が安定して使えるか
     - 実際に使える識別情報の組み合わせ
   - 調査対象の識別子候補:
     - Core Audio の `DeviceId`
     - デバイスの表示名 (`DeviceName`)
     - ハードウェア ID / 製品名・製造元情報
     - `DataFlow` / `Role` を含む条件
   - フォールバック戦略:
     - まず `DeviceId` で検索する
     - `DeviceId` が一致しない場合は `DeviceName` など補助識別子で再検索する
     - それでも一致しない場合は新しいプロファイルとして扱う
   - 成功基準: プロファイル検索に使う識別方法とフォールバック方式を仕様として確定できる

2. **現在のデバイス情報を手動で取得する**
   - 目的: 既定再生デバイスの取得機能を確認する
   - 取得内容: `DeviceId`, `DeviceName`, `MasterVolume`, `IsMuted`, `DataFlow/Role`
   - 成功基準: CLI で現在の既定再生デバイス情報を表示できる

3. **常駐してデバイス切り替わりを自動で検出する**
   - 目的: 既定再生デバイスの変更イベントを受け取る監視基盤を構築する
   - 監視対象: `Render` / `Multimedia` の既定再生デバイス
   - 成功基準: デバイス切り替え時にイベントが発火する

4. **切り替わったデバイスの情報を自動で取得する**
   - 目的: 監視イベント後に最新の既定デバイス情報を自動取得する
   - 動作: イベント受信 → `GetCurrentDefaultDeviceAsync()` 実行
   - 成功基準: 切り替え後のデバイス情報を状態に反映できる

5. **現在のデバイス情報を手動でプロファイルとして保存する**
   - 目的: プロファイル保存・読み書きの基盤を構築する
   - 仕様: `profiles.json` に `deviceId`, `deviceName`, `masterVolume`, `isMuted`, `createdAt`, `lastApplied` を保存
   - 成功基準: CLI でプロファイルを保存・確認できる

5. **デバイスが切り替わった時に自動で対応するプロファイルを認識する**
   - 目的: 新しい既定デバイスに対応するプロファイルを検索する
   - 検索ルール: `DeviceId` を主キーとし、必要に応じて `DeviceName` でフォールバック
   - 成功基準: 対応プロファイルの有無を判定できる

6. **現在のデバイスの情報を対応するプロファイルの情報で更新する**
   - 目的: プロファイルを最新状態に維持する
   - 動作: プロファイルが未存在なら新規作成、既存なら更新
   - 成功基準: 切り替え時にプロファイルの登録・更新が行える

7. **デバイスが切り替わった時に自動的に対応するプロファイル情報を適用する**
   - 目的: 切り替え時に音量・ミュート状態を自動で復元する
   - 動作: プロファイルの `masterVolume` / `isMuted` を適用、失敗時はリトライ・ログ出力
   - 成功基準: 既定デバイス切り替え時に自動で設定を反映できる
---

## 4. 非機能要件

### 4.1 パフォーマンス

| 要件 | 基準 |
|-----|------|
| デバイス変更検知応答時間 | 100ms 以内 |
| 音量調整応答時間 | 200ms 以内 |
| メモリ使用量 | 30MB 以下（アイドル時） |
| CPU 使用率 | 1% 以下（アイドル時） |

### 4.2 信頼性

| 要件 | 仕様 |
|-----|------|
| 設定ファイル破損対策 | バックアップファイル自動作成（`.json.bak`） |
| エラーハンドリング | 全ての例外を Serilog で記録、アプリ継続実行 |
| Core Audio API エラー | 再試行ロジック実装（指数バックオフ） |

#### 4.2.1 再試行パラメータ

- 初期遅延: 100ms
- 乗数: 1.5×
- 最大遅延: 5,000ms
- 最大試行回数: 5回
- 対象: Core Audio API 呼び出し、デバイス取得・音量設定の操作

再試行は、短時間の transient エラーやデバイス状態の遷移による失敗に耐えるために使う。
5 回失敗しても復旧しない場合はエラーをログに記録し、アプリケーションは継続動作する。

### 4.3 セキュリティ

| 要件 | 対応 |
|-----|------|
| 管理者権限 | 不要 |
| 設定ファイル保護 | ユーザーの %APPDATA% 内に保存（OS ユーザー分離） |
| ログ情報 | 機密情報（シークレット等）を含めない |

### 4.3 ログ設定

- コンソール: INFO 以上を出力する
- ファイル: DEBUG 以上を記録する
- ログ出力先: `%APPDATA%\VolumeProfileManager\logs\`
- ローテーション: 日次
- 保持期間: 30 日程度（運用ポリシーによって調整可能）

ファイルログには詳細なデバッグ情報を含め、コンソールには運用上必要な情報のみを表示する。

### 4.4 互換性

- .NET 8.0 自己包含実行ファイル（SCD）で配布
- Windows 10 1903 以降で動作確認

---

## 5. UI仕様

### 5.1 コンソール出力

#### 5.1.1 起動時ログ

```
[13:45:00 INF] VolumeProfileManager v1.0.0 起動しました
[13:45:00 INF] 設定ファイル: C:\Users\User\AppData\Roaming\VolumeProfileManager\profiles.json
[13:45:00 INF] 既定デバイス: Realtek Audio
[13:45:00 INF] マスターボリューム: 75%
[13:45:00 INF] デバイス監視を開始しました
```

#### 5.1.2 デバイス変更ログ

```
[13:50:23 INF] デバイスが変更されました: NVIDIA HDMI Audio
[13:50:24 INF] プロファイル適用: Volume 60% → 60%
[13:50:24 INF] 音量調整完了 (適用時間: 150ms)
```

### 5.2 トレイアイコン（実装済み）

Windows トレイアイコンを主UIとして採用（Win32 API直接呼び出し、WinForms不使用）：

- 右クリックメニュー: ステータス表示 / プロファイルを更新 / スタートアップ登録/解除 / 終了
- プロファイル自動適用時のバルーン通知（音量制御非対応デバイスの場合はエラー通知）

---

## 6. 技術構成

### 6.1 スタック

| レイヤー | 技術 | バージョン |
|---------|------|-----------|
| ランタイム | .NET | 8.0 |
| 言語 | C# | 12.0 |
| Core Audio API | NAudio | 2.2+ |
| ロギング | Serilog | 4.0+ |
| テスト | xUnit | 2.6+ |
| ビルド | MSBuild | .NET 8.0 付属 |

### 6.2 外部依存

```xml
<!-- VolumeProfileManager.Core -->
<PackageReference Include="Serilog" Version="4.0.*" />

<!-- VolumeProfileManager.Infrastructure -->
<PackageReference Include="NAudio" Version="2.2.*" />
<PackageReference Include="System.Text.Json" Version="8.0.*" />

<!-- VolumeProfileManager.TrayApp -->
<PackageReference Include="Microsoft.Extensions.DependencyInjection" Version="8.0.*" />
<PackageReference Include="Serilog.Sinks.File" Version="5.0.*" />
```

### 6.3 ログ設定

ログレベル：

```
[DEBUG]   - 関数呼び出し、パラメータ値
[INFO]    - デバイス変更、音量調整
[WARNING] - リトライ発生、非推奨 API 使用
[ERROR]   - 例外発生、失敗した操作
```

ログ出力先：

- **コンソール**：INFO レベル以上
- **ファイル**：`%APPDATA%\VolumeProfileManager\logs\` （ローテーション：日次）

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
| デバイス ID 変更 | ハードウェア ID + デバイス名での二重管理を検討 |
| 音量調整の失敗 | リトライロジック実装、エラーハンドリング強化 |
| リソースリーク | using 宣言・Dispose パターンの徹底 |

---

## 8. ロードマップ

### v1.0 - MVP（リリース予定：2026年Q3）

- [x] デバイス監視機能
- [x] 音量プロファイル管理（CRUD）
- [x] 自動音量調整
- [x] コマンドラインインターフェース
- [x] ユニット・統合テスト

### v1.1 - 安定性向上

- [x] トレイアイコン UI（Issue #5）
- [x] Console版廃止・TrayApp一本化（Issue #1）
- [ ] インストーラー（Inno Setup）（Issue #1、対応中）
- [ ] 設定ウィザード
- [ ] UX 改善（設定の簡潔化）

### v2.0 - 拡張機能

- [ ] タイムスケジュール機能（朝：60%, 夜：30%）
- [ ] アプリケーション別プロファイル
- [ ] Bluetooth デバイスの検出・接続状態管理
- [ ] REST API（他のツール連携用）

---

## 付録

### A. Core Audio API イベントフロー

```
IMMNotificationClient
  ├─ OnDeviceAdded(deviceId)
  ├─ OnDeviceRemoved(deviceId)
  ├─ OnDeviceStateChanged(deviceId, state)
  ├─ OnDefaultDeviceChanged(flow, role, deviceId)  ← 監視対象
  └─ OnPropertyValueChanged(deviceId, key)
```

### B. 既存プロジェクト（AudioPilot）との差異

| 項目 | AudioPilot | VolumeProfileManager |
|-----|-----------|----------------------|
| UI | WinUI 3 | コンソール |
| 対象 | アプリケーション単位 | デバイス単位 |
| スコープ | 統合制御 | シンプル・シングルタスク |
| 配布 | 1つの実行ファイル | スタンドアロン実行ファイル |

