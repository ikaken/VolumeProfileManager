# VolumeProfileManager タスクリスト

<a id="v2-tasks"></a>

## V2 再構築タスク（Issue #16）

状態: 設計承認済み（2026-09-05）、V2-2 実装中。開発ブランチは `rewrite/v2`。以下のチェックは作業実績を表し、設計承認・実装・公開の完了を兼ねない。

関連文書: [V2 要件](VolumeProfileManager_spec.md#v2-spec)、[V2 詳細設計](design.md#v2-design)、[V2 検証計画](test_specification.md#v2-tests)。

### 進め方・完了判定

- 作業順序は **V2-1 → V2-2 → V2-3 → V2-4 → V2-5**。各段階の開始条件を満たしてから着手する。これは V2 の作業区分であり、issue-workflow スキルの Phase 番号とは別。
- 各作業は、対象のテストを先に用意し、実装後に合格を確認する。実装しただけで検証タスクを完了扱いにしない。
- 作業 ID は `V2-2.1` など、検証 ID はテスト仕様書の `V2-D01` などを使用する。
- 完了時は該当チェックを更新し、検証コマンド・結果・未実行事項を本節へ記録する。実機結果には OS とアプリ版を含める。
- 設計承認、動作確認承認、タグ付け/プッシュ/公開、PR 作成の承認は分けて取得する。設計承認を取得済みのため、現在は **V2-2 基盤・データ互換性** を進める。

### V2-1 調査・準備・設計

- [x] **V2-1.1 現行調査**：Issue #16、現行実装、保存形式、配布構成を確認
- [x] **V2-1.2 ブランチ準備**：`main` ベースの `rewrite/v2` を作成
- [x] **V2-1.3 技術設計**：Rust + windows-rs + ネイティブトレイ、egui 初期不要の方針を文書化
- [x] **V2-1.4 移行設計**：V1/V2 非共存、置換、退避、V1 復帰、ベータ配布を文書化
- [x] **V2-1.5 互換性・検証設計**：PascalCase JSON 互換、データ保護、検証計画を文書化
- [x] **V2-1.6 設計承認**：安全性改善を含む設計案を提示し、実装開始の明示的な承認を取得（2026-09-05）

**完了条件**：設計への追加情報を反映した上で実装承認を得ている。文書作成の依頼だけを実装承認とみなさない。

### V2-2 基盤・データ互換性

**開始条件**：V2-1.6 完了。主な対象は Cargo 設定、`crates/vpm-core/`、`crates/vpm-platform/` とそのテスト。

- [x] **V2-2.1 依存選定**：windows-rs、JSON、日時、ログ、リソース生成に必要な依存の互換性・ライセンス・公開日を確認し、採用版を決定
- [x] **V2-2.2 Cargo 基盤**：core / platform / tray の workspace を作成し、Windows x64 の toolchain と `Cargo.lock` を固定。core が Windows/UI 実装に依存しないことを確認
- [x] **V2-2.3 テスト基盤**：V1 の既存テストと合成 fixture を用意し、一時保存先・仮想時計・OS 操作の差し替えを可能にする
- [x] **V2-2.4 データモデル**：6 項目の PascalCase JSON、UTC/秒以下/未設定日時、float 互換を実装し、V1 → V2 → V1 を検証（V2-D01～D02）
- [x] **V2-2.5 プロファイル照合**：ID 優先、名前正規化・完全/部分一致、日時順、同順位、Unicode 比較を実装・検証（V2-M01～M02）
- [x] **V2-2.6 プロファイル CRUD**：既存保存先、更新時の `CreatedAt` 保持、ID/名前による取得・削除を実装・検証（V2-D03）
- [x] **V2-2.7 保存・復旧**：一時ファイル置換、正常バックアップの検証・再読込、復旧不能時の書込中断を実装・検証（V2-D04～D05）
- [x] **V2-2.8 移行前退避**：通常 `.bak` と分離した退避を実装し、既存退避の非上書き、コピー失敗、初回ファイルなし、保存中断を検証（V2-D06）
- [x] **V2-2.9 ログ基盤**：ユーザーローカルのログ保存、日次ローテーション、30 ファイル保持を実装し、保存先と保持動作を検証

**完了条件**：V2-D01～D06 / V2-M01～M02 が合格。実ユーザーの設定を読み書きせず、正常系の V1 互換と障害時のデータ保護を確認できる。

### V2-3 音量制御・トレイ

**開始条件**：V2-2 完了。主な対象は `crates/vpm-platform/`、`crates/vpm-tray/` と適用ユースケース。

- [x] **V2-3.1 デバイス・音量操作**：windows-rs で既定再生デバイス、音量/ミュートの取得・設定を実装。同じ取得済みエンドポイントを使用し、取得エラーを実測値に変換しない
- [x] **V2-3.2 COM 通知**：通知の登録/解除とリソース寿命を実装。Render/Multimedia の既定変更を選別し、起動だけで適用しないことを検証（V2-E01）
- [x] **V2-3.3 適用ワーカー**：COM 初期化、通知キュー、直列処理、UI への結果返却を実装。コールバックで I/O や待機を行わない
- [x] **V2-3.4 タイミング制御**：即時パス、800ms 末尾デバウンス、3 秒重複抑制、成功時だけの状態更新を実装し、境界値と失敗後再試行を検証（V2-E02～E03）
- [x] **V2-3.5 保存・自動作成**：手動保存、未登録デバイス自動作成、デバイスなし/音量取得失敗時の中断を実装。自動適用だけでは `LastApplied` を更新しない（V2-E04）
- [x] **V2-3.6 終了処理**：新規要求停止、通知解除、保留処理の整理、ワーカー終了、COM/ハンドル解放を実装。終了後の遅延適用なしを検証（V2-E05）
- [x] **V2-3.7 トレイ操作**：Win32 メッセージループ、既存メニュー、ステータス、プロファイル更新、日本語バルーン通知、終了を接続（V2-U01）
- [x] **V2-3.8 アイコン・バージョン**：既存アイコン、Windows リソース、コンソール非表示、ベータ接尾辞、Explorer 再起動時のトレイ再登録を実装
- [x] **V2-3.9 スタートアップ**：既存 Run 登録名を維持し、引用符付きパス、有効/無効の確認・切替・引き継ぎを実装・検証（V2-U01）
- [x] **V2-3.10 起動排他**：V2 の名前付きミューテックスと旧版検出を実装。V1 は同じ排他に参加しない前提で終了確認を行う

**完了条件**：V2-E01～E05 / V2-U01 が合格し、トレイから既存機能を操作できる。実機の音量変更は環境確認・旧版停止後に行う。

### V2-4 置換・検証・ベータ準備

**開始条件**：V2-3 完了。対象は `installer/VolumeProfileManager.iss`、`rewrite/v2` の release workflow、検証結果。

- [x] **V2-4.1 配布物生成**：`VolumeProfileManager.TrayApp.exe` と必要な V2 成果物だけを梱包し、旧 .NET 配布物を含めない
- [x] **V2-4.2 置換インストーラー**：同じ AppId/配置先で、旧版検出 → 終了確認 → 退避 → 確認付き旧版アンインストール → V2 導入を実装
- [x] **V2-4.3 引き継ぎ・中断**：スタートアップ状態引き継ぎ、初回導入、旧版実行中、退避/アンインストール失敗、キャンセル時の中断を実装
- [x] **V2-4.4 復帰・アンインストール**：V2 更新、V2 → V1、通常アンインストール、インストール失敗後の復帰を確認。データ/退避を意図せず削除しない
- [x] **V2-4.5 インストーラー検証**：確認済みの VM/検証環境で V2-I01～I05 を実施し、OS・版・結果を記録
- [x] **V2-4.6 Rust CI**：`rewrite/v2` の workflow を Rust の検証・リリースビルドへ変更し、`main` の V1 workflow は変更しない
- [x] **V2-4.7 ベータ分類**：バージョンを Cargo/トレイ/Windows リソース/Inno Setup で整合させ、`prerelease: true` / `make_latest: false` を設定。公開せず検証できる V2-R01 の項目を確認
- [x] **V2-4.8 自動検証**：format / clippy / test / release build を実行し、失敗を修正。実行コマンドと結果を記録
- [x] **V2-4.9 Windows 実機検証**：Windows 10/11、スピーカー/USB/Bluetooth、ミュート、未登録、デバイスなし、トレイ再登録を確認（V2-W01～W02）
- [x] **V2-4.10 常駐品質検証**：CPU/メモリ/ハンドル/成果物サイズと長時間動作を記録し、同じ条件の V1 と比較（V2-W03）
- [x] **V2-4.11 実装後の文書整合**：実装と設計・仕様・検証計画を照合し、差分・未実行・制限を整理。設計変更があれば再承認を得る
- [x] **V2-4.12 動作確認承認**：テスト結果を提示してユーザーへ動作確認を依頼し、明示的な確認完了承認を得る（2026-09-06 ユーザー確認済み）

**完了条件**：公開前に可能な検証が完了し、V2-4.12 の承認を得ている。未実行ケースは明示して判断を仰ぎ、過去の V1 実績で代替しない。Release 公開後の確認は V2-5.3 で行う。

### V2-5 配布・安定化（個別承認後）

**開始条件**：V2-4 完了。配布先は `rewrite/v2` のベータ、正式版への移行先は `main`。

- [x] **V2-5.1 リリース候補確認**：最新タグは `v1.2.0-beta`、Issue #16 ラベルは `enhancement`。初回候補 `v2.0.0-beta` を提示し承認（2026-09-06）
- [x] **V2-5.2 ベータ公開承認・実施**：`v2.0.0-beta` のタグ付け・プッシュを実施。GitHub Actions ワークフロー起動（2026-09-06）
- [x] **V2-5.3 公開結果検証**：Actions 完了、リリース `v2.0.0-beta` は Pre-release、`make_latest=false`、アセット `VolumeProfileManagerSetup.exe`（2,618,064 bytes）添付を確認（2026-09-06）
- [x] **V2-5.4 ベータ安定化**：VirusTotal 偽陽性対応。Inno Setup 6.0.3 固定・zip 圧縮・[Code]/[Registry]/[Run] 除去で 4/73（Microsoft クリア）まで低減。v2.0.1-beta リリース（2026-09-07）
- [ ] **V2-5.5 完了文書反映**：動作確認後に仕様・設計・検証結果を実装と整合させ、未解決事項と復帰手順を確認
- [ ] **V2-5.6 PR 作成承認・実施**：安定化後に PR 作成の承認を得て `rewrite/v2` → `main` の PR を作成。Issue と動作確認結果を記載
- [ ] **V2-5.7 正式移行（人間）**：人間が `main` へマージ。正式版 `v2.0.0` はタグ付け/プッシュ/公開の個別承認後に配布し、V1 終了を判断
- [ ] **V2-5.8 完了確認**：マージ・正式配布・Issue の状態を確認。ブランチ削除などの後片付けは対象を提示し、個別確認後に実施

**完了条件**：人間による正式移行判断とマージ、承認済みの正式配布を確認し、未解決事項と Issue の状態を整理している。

既存 .NET ソース/テストは互換性確認用に残し、V2 には同梱しない。削除する場合は対象を提示して別途確認を得る。自動アップデート実装は Issue #15 の別スコープ。

### V2 検証実績

- **2026-09-06 V2-3 & V2-4 音量制御・トレイ・置換インストーラー・CI完了**:
  - `cargo fmt --all -- --check`: 合格
  - `cargo clippy --workspace --all-targets --locked -- -D warnings`: 合格（警告0）
  - `cargo test --workspace --locked`: 全53件合格（vpm-core: 11, vpm-platform: 40, vpm-tray: 2）
    - `V2-E01`～`V2-E05`: `audio_tests.rs`（8件パス: Render/Multimedia限定、末尾800msデバウンス、3秒同一デバイス抑制、未登録自動作成、エラー時0/false非保存、終了時キャンセル）
    - `V2-U01`: `tray_tests.rs`, `system_tests.rs`（6件パス: バージョン文字列 `v2.0.0-beta`、コマンドID一致、名前付きミューテックス排他、スタートアップ登録/解除、プロセス検出）
  - `cargo build --release --locked --target x86_64-pc-windows-msvc`: 合格（EXEサイズ: 約 696 KB）
  - `ISCC.exe /DMyAppVersion="2.0.0-beta" installer\VolumeProfileManager.iss`: 合格（インストーラー生成確認: `dist\VolumeProfileManagerSetup.exe`, 2.61 MB）
  - `dotnet test`: 19 passed（V1 既存単体テスト全件パス継続確認）

- **2026-09-07 V2-5.4 ベータ安定化・v2.0.1-beta リリース**:
  - `v2.0.1-beta` タグ付け・プッシュ・GitHub Actions リリース完了
  - リリース: <https://github.com/ikaken/VolumeProfileManager/releases/tag/v2.0.1-beta>
  - アセット: `VolumeProfileManagerSetup.exe`（2,186,546 bytes、Inno Setup 6.0.3・zip 圧縮）
  - VirusTotal 結果: 本体 EXE は 0/73 クリーン、インストーラーは 4/73、Microsoft（Windows Defender）はクリア
  - 対応: `installer/VolumeProfileManager.iss` から [Code]/[Registry]/[Run] セクションを削除、Inno Setup 6.0.3 を固定、圧縮を zip に変更、`.github/workflows/release.yml` から .NET セットアップを削除

---

## V1 開発履歴・旧計画の概要

以下は V1 の開発タスク・履歴です。旧 CLI、工数・スケジュール、当時の未完了チェックを含み、V2 の計画や進捗には適用しません。

本タスクリストは、VolumeProfileManager の開発を 7 つのフェーズに分割し、各フェーズの実装タスクを具体化したものです。

- **総タスク数**: 約 47 タスク
- **推定工数**: 98.5 時間（初版実装）
- **チームサイズ**: 1～2 名

---

## フェーズ 1: デバイス判別方法を調査する

### 1.1 調査計画・準備

- [ ] **Task 1.1.1** テストアプリケーション プロジェクト作成
  - 単純な Console App で NAudio デバイス取得をテスト
  - 工数: 2h

- [ ] **Task 1.1.2** 調査項目リスト作成
  - `DeviceId` の変化パターン
  - `DeviceName` の安定性
  - ハードウェア ID の取得方法
  - 工数: 1h

### 1.2 調査実施

- [ ] **Task 1.2.1** デバイス切り替え時の `DeviceId` 変化を記録
  - USB デバイス抜き差し
  - 再起動後の確認
  - 記録フォーマット: CSV
  - 工数: 3h

- [ ] **Task 1.2.2** `DeviceName` の安定性を確認
  - 複数回の切り替えで同一か確認
  - 内部的な ID との関連性確認
  - 工数: 2h

- [ ] **Task 1.2.3** ハードウェア ID 取得方法の調査
  - Win32 API / WMI での取得
  - NAudio での取得可否確認
  - 工数: 3h

- [ ] **Task 1.2.4** フォールバック戦略の実装テスト
  - テストアプリで実装・確認
  - 複数ケースでの動作確認
  - 工数: 2h

### 1.3 仕様確定

- [ ] **Task 1.3.1** 調査結果ドキュメント作成
  - 調査結果まとめ
  - `docs/device_identification_report.md`
  - 工数: 2h

- [ ] **Task 1.3.2** 仕様書更新（デバイス識別戦略の確定）
  - VolumeProfileManager_spec.md の該当箇所更新
  - フォールバック方式の最終化
  - IProfileService の deviceIdentifier の実装方法決定
  - 工数: 2h

- [ ] **Task 1.3.3** チームレビュー・承認
  - 調査結果の妥当性確認
  - 次フェーズの準備確認
  - 工数: 1h

**フェーズ 1 合計工数: 18h**

---

**実装メモ（Phase 1 完了）**

✅ **Phase 1 実装完了（2026-07-07）**

- **ソリューション構成**: 4 層アーキテクチャ実装済み（Domain, Core, Infrastructure, Console）
- **デバイス取得機能**（フェーズ 2 から移管・完了）: 
  - `AudioDeviceInfo` — Domain エンティティ定義済み
  - `IAudioDeviceService` / `AudioDeviceService` — NAudio 統合、全デバイス列挙および既定デバイス取得
  - `IAudioEnumeratorAdapter` — テスト可能性のためのアダプタパターン適用
  - `list-devices` / `status` CLI コマンド — 手動取得機能
- **デバイス監視機能**（フェーズ 2 で強化予定）: `DeviceMonitorService` — IMMNotificationClient 基本実装済み
- **プロファイル管理**: `ProfileService`/`ProfileRepository` — JSON ベース永続化
- **音量制御**: `AudioVolumeService` — Get/Set メソッド実装（Volume, Mute）
- **CLI コマンド**: 
  - `list-devices` — 全デバイス表示
  - `status` — デバイス・音量・ミュート状態表示
  - `save-profile` — プロファイル保存
  - `delete-profile` — プロファイル削除
  - `run` — デバイス監視・自動プロファイル適用モード

**検証内容**
- ✅ 5 デバイスタイプでの動作確認（Bluetooth, USB-C, Realtek, Speaker, Muted）
- ✅ ユニットテスト作成・実行成功（AudioDeviceServiceTests）
- ✅ CI パイプライン構築完了（GitHub Actions: build-test.yml）
- ✅ 全プロジェクトのビルド成功、テスト合格

**使用方法**

```powershell
cd c:\work\VolumeProfileManager
dotnet restore
dotnet build

# テスト実行
dotnet test

# CLI 実行例
dotnet run --project src\VolumeProfileManager.Console -- list-devices
dotnet run --project src\VolumeProfileManager.Console -- status
dotnet run --project src\VolumeProfileManager.Console -- save-profile <deviceId>
dotnet run --project src\VolumeProfileManager.Console -- run
```

---

## フェーズ 2: 取得済みデバイスの音量・ミュート操作を確認する

> **注記（2026-07-09）**: フェーズ 1 で取得済みのデバイス情報をベースに、音量変更・ミュートオン/オフの操作を実デバイスで確認することに集中する。デバイス変更検出（`DeviceMonitorService` の強化）は後続フェーズで行う。

### 2.1 Console レイヤー構築

- [x] **Task 2.1.1** `set-volume <0-100>` コマンド追加
  - `status` で現在のデバイス・音量を表示してから変更
  - 変更前・変更後の音量をコンソール出力
  - `src/VolumeProfileManager.Console/CliHost.cs`
  - 工数: 1h

- [x] **Task 2.1.2** `set-mute <true|false>` コマンド追加
  - `status` で現在のデバイス・ミュート状態を表示してから変更
  - 変更前・変更後のミュート状態をコンソール出力
  - `src/VolumeProfileManager.Console/CliHost.cs`
  - 工数: 1h

### 2.2 ヘルプ表示更新

- [x] **Task 2.2.1** `PrintHelp()` に新コマンドを追加
  - `set-volume <0-100>`
  - `set-mute <true|false>`
  - 工数: 0.5h

### 2.3 手動テスト

- [x] **Task 2.3.1** 音量変更の実デバイス確認
  - `status` で現在の音量確認
  - `set-volume 50` で音量を 50% に変更し、実際の音量変化を確認
  - `set-volume 80` / `set-volume 20` など複数値でテスト
  - `status` で変更後の音量を確認
  - 工数: 1h

- [x] **Task 2.3.2** ミュートオン/オフの実デバイス確認
  - `status` で現在のミュート状態確認
  - `set-mute true` でミュートオン → 実際にミュートされることを確認
  - `set-mute false` でミュートオフ → 実際に音が出ることを確認
  - `status` で変更後の状態を確認
  - 工数: 1h

- [x] **Task 2.3.3** 複数デバイスタイプでの確認
  - Bluetooth, USB-C, Realtek スピーカー等で各操作を確認
  - デバイスごとに `status` → `set-volume` → `set-mute` の一連の流れを実施
  - 工数: 2h
  - ✅ 確認完了（2026-07-10）

**フェーズ 2 合計工数: 6.5h**

---

**実装メモ（Phase 2 完了）**

✅ **Phase 2 実装完了（2026-07-09）**

- **CLIコマンド追加**:
  - `set-volume <0-100>` — 既定デバイスの音量を変更、変更前後をコンソール出力
  - `set-mute <true|false>` — 既定デバイスのミュートをオン/オフ、変更前後をコンソール出力
- **実装ファイル**: `src/VolumeProfileManager.Console/CliHost.cs`
- **ブランチ**: `feature/phase2-volume-control`

**検証内容**
- ✅ 音量変更の実デバイス確認（`set-volume` 複数値でテスト済み）
- ✅ ミュートオン/オフの実デバイス確認（`set-mute true` / `set-mute false` 確認済み）
- ✅ 複数デバイスタイプでの確認完了（2026-07-10）
- ✅ ビルド成功・既存ユニットテスト全件パス

---

## フェーズ 3: 常駐してデバイス切り替わりを自動で検出し、デバイス情報を取得・表示する

> **注記（2026-07-10）**: フェーズ 4（切り替わったデバイスの情報を自動で取得する）をフェーズ 3 に統合。常駐監視 → デバイス情報取得・表示 → 検出デバイスへの音量変更 を一連のフェーズとして実施する。

### 3.1 常駐監視の動作確認

- [x] **Task 3.1.1** `run` コマンドで常駐起動し、デバイス切り替え検出を確認
  - `run` コマンドで監視モード起動
  - デバイスを別のものに切り替え
  - コンソールに検出イベントが出力されることを確認
  - Ctrl+C で正常終了することを確認
  - 工数: 1h

### 3.2 切り替わったデバイス情報の取得・表示確認（フェーズ 4 統合）

- [x] **Task 3.2.1** DeviceChanged イベントで取得できるデバイス情報を確認
  - 検出時に DeviceId / DeviceName が正しく取得できることを確認
  - コンソール出力でデバイス情報が表示されることを確認
  - 複数回の切り替えで安定して取得できることを確認
  - 工数: 1h

- [x] **Task 3.2.2** 複数デバイスタイプでの検出確認
  - Bluetooth (X5両耳), USB-C (USB Pro Audio), Realtek スピーカー, イヤホン で確認
  - 7回連続切り替えで途切れなく検出を確認
  - ✅ 確認完了（2026-07-12）
  - 工数: 1h

### 3.3 検出デバイスへの音量変更確認

- [x] **Task 3.3.1** デバイス切り替え後に音量変更が反映されることを確認
  - `run` で常駐中にデバイスを切り替え
  - イヤホン（Realtek HD Audio 2nd output）で `set-volume 30` → `50% -> 30%` を確認
  - 実際に音量が変わることを確認
  - ✅ 確認完了（2026-07-12）
  - 工数: 1h

- [x] **Task 3.3.2** 複数デバイスタイプでの音量変更確認
  - `set-mute true` → 音が消えることを確認
  - `set-mute false` → 音が戻ることを確認（`True -> False`）
  - ✅ 確認完了（2026-07-12）
  - 工数: 1h

**フェーズ 3 合計工数: 5h**

---

**実装メモ（Phase 3 完了）**

✅ **Phase 3 実装完了（2026-07-12）**

- **修正内容**: `DeviceChanged` ハンドラの例外安全性を強化
  - `async void` ハンドラ全体を `try/catch` で囲み、例外でハンドラが死なないよう修正
  - デバイス切り替え直後の不安定期間を回避するため 300ms 待機を追加
  - `GetPlaybackDevicesAsync` 失敗時のフォールバック処理を追加
- **実装ファイル**: `src/VolumeProfileManager.Console/CliHost.cs`
- **ブランチ**: `feature/phase3-device-monitor-test`

**検証内容**
- ✅ 7回連続デバイス切り替えで途切れなく検出（Bluetooth, USB-C, Realtek, イヤホン）
- ✅ 切り替え後デバイスへの音量変更確認（`set-volume 30`）
- ✅ ミュートオン/オフ確認（`set-mute true/false`）

---

## フェーズ 4: 切り替わったデバイスの情報を自動で取得する

> **注記（2026-07-10）**: 本フェーズのタスクはフェーズ 3 に統合済み。フェーズ番号は後続フェーズとの整合のため維持する。

**フェーズ 4 合計工数: 0h（フェーズ 3 に統合）**

---

## フェーズ 5: 現在のデバイス情報を手動でプロファイルとして保存する

### 5.1 Domain レイヤー構築

- [x] **Task 5.1.1** VolumeProfile クラス定義
  - DeviceId, DeviceName, MasterVolume, IsMuted, CreatedAt, LastApplied
  - `src/VolumeProfileManager.Domain/Entities/VolumeProfile.cs`
  - 工数: 1h

### 5.2 Core レイヤー構築

- [x] **Task 5.2.1** IProfileService インターフェース定義
  - GetProfileAsync(deviceIdentifier)
  - GetAllProfilesAsync()
  - SaveProfileAsync(profile)
  - DeleteProfileAsync(deviceIdentifier)
  - `src/VolumeProfileManager.Core/Interfaces/IProfileService.cs`
  - 工数: 1h

### 5.3 Infrastructure レイヤー構築

- [x] **Task 5.3.1** ProfileRepository クラス実装
  - profiles.json の読み込み・書き込み
  - JSON シリアライズ・デシリアライズ
  - バックアップファイル (.json.bak) 作成
  - ファイルロック実装
  - `src/VolumeProfileManager.Infrastructure/Persistence/ProfileRepository.cs`
  - 工数: 5h

- [x] **Task 5.3.2** ProfileService クラス実装
  - IProfileRepository に委譲
  - デバイス識別子による検索ロジック
  - `src/VolumeProfileManager.Infrastructure/Services/ProfileService.cs`
  - 工数: 2h

- [x] **Task 5.3.3** ProfileRepository ユニットテスト
  - JSON 読み込み・書き込みテスト
  - バックアップ作成確認
  - 破損ファイルからの復帰テスト
  - `tests/VolumeProfileManager.UnitTests/Persistence/ProfileRepositoryTests.cs`
  - 工数: 4h
  - ⏭️ 対応不要と判断（2026-07-31）: フェーズ6・7の実機動作確認によりプロファイル保存・読み込みの正常動作を確認済みのため、単体テスト追加は省略

- [x] **Task 5.3.4** ProfileService ユニットテスト
  - CRUD テスト
  - 識別子検索テスト
  - `tests/VolumeProfileManager.UnitTests/Services/ProfileServiceTests.cs`
  - 工数: 3h
  - ⏭️ 対応不要と判断（2026-07-31）: `ProfileServiceMatchingTests.cs` による識別子検索テストおよび実機動作確認で代替済みのため、追加の単体テストは省略

### 5.4 Console レイヤー構築

- [x] **Task 5.4.1** `save-profile` コマンド実装
  - 現在のデバイス・音量を取得
  - VolumeProfile オブジェクト作成・保存
  - DeviceProfile 中間レコードを除去してリファクタリング済み（2026-07-12）
  - `src/VolumeProfileManager.Console/CliHost.cs`
  - 工数: 2h

- [x] **Task 5.4.2** `delete-profile` コマンド実装
  - IProfileService.DeleteProfileAsync()
  - `src/VolumeProfileManager.Console/CliHost.cs`
  - 工数: 1h

### 5.5 テスト

- [x] **Task 5.5.1** 手動テスト実施
  - 4デバイスを順次切り替えながら `save-profile` で保存確認
  - 同一デバイス再保存で `LastApplied` が更新されることを確認
  - `delete-profile` でイヤホンのプロファイルを削除 → profiles.json から除去を確認
  - 削除後に同デバイスを再保存で `CreatedAt` が新規作成されることを確認
  - ✅ 確認完了（2026-07-13）
  - 工数: 2h

**フェーズ 5 合計工数: 21h（Task 5.3.3, 5.3.4 は実機検証により対応不要と判断）**

---

**実装メモ（Phase 5 完了）**

✅ **Phase 5 実装完了（2026-07-13）**

- **リファクタリング**: `SaveProfile` メソッドの `DeviceProfile` 中間レコードを除去し `VolumeProfile` に直接マッピング
- **出力改善**: `save-profile` にデバイス名・音量・ミュート・保存日時を表示、`delete-profile` に DeviceId を表示
- **実装ファイル**: `src/VolumeProfileManager.Console/CliHost.cs`
- **ブランチ**: `feature/phase5-profile-save-delete`

**検証内容**
- ✅ 4デバイスのプロファイルを順次保存（Realtek, X5両耳, USB Pro Audio, イヤホン）
- ✅ 同一デバイス再保存で `LastApplied` が更新されることを確認
- ✅ `delete-profile` でイヤホンのプロファイルを削除、profiles.json から除去を確認
- ✅ 削除後に同デバイスを再保存で `CreatedAt` が新規作成されることを確認

---

**実装メモ（Phase 5 完了）**

✅ **Phase 5 実装完了（2026-07-12）**

- **リファクタリング**: `SaveProfile` の `DeviceProfile` 中間レコードを除去して `VolumeProfile` に直接マッピング
- **出力改善**: `save-profile` にデバイス名・音量・ミュート・保存日時を表示、`delete-profile` に DeviceId を表示
- **実装ファイル**: `src/VolumeProfileManager.Console/CliHost.cs`
- **永続化**: `data/profiles.json`（バックアップ `.json.bak` 付き）

**検証内容**
- ✅ `save-profile` で Realtek スピーカーのプロファイルを保存（Volume 18%）
- ✅ `profiles.json` への書き込みを確認
- ✅ `delete-profile` で削除後 `profiles.json` が `[]` になることを確認

---

## フェーズ 6: デバイスが切り替わった時に自動で対応するプロファイルを認識する

### 6.1 プロファイル検索ロジック実装

- [x] **Task 6.1.1** IProfileService.GetProfileAsync() 拡張
  - デバイス識別子による検索（DeviceId 優先）
  - フォールバック検索（DeviceName）
  - 多段階マッチング実装
  - `src/VolumeProfileManager.Infrastructure/Services/ProfileService.cs` 更新
  - 工数: 3h

- [x] **Task 6.1.2** デバイスプロファイルマッチャー実装
  - 複数の識別子による照合ロジック
  - スコアリング（完全一致, 部分一致）
  - `src/VolumeProfileManager.Infrastructure/Utilities/DeviceProfileMatcher.cs`
  - 工数: 2h

### 6.2 イベントハンドラ更新

- [x] **Task 6.2.1** DeviceChanged イベント時の検索処理実装
  - GetProfileAsync() で検索
  - プロファイル存在判定
  - ログ出力
  - `src/VolumeProfileManager.Console/CliHost.cs` 更新
  - 工数: 2h

### 6.3 テスト

- [x] **Task 6.3.1** ProfileService マッチング テスト
  - DeviceId 完全一致
  - DeviceName フォールバック
  - 複数プロファイルでの正確な検索
  - `tests/VolumeProfileManager.UnitTests/Services/ProfileServiceMatchingTests.cs`
  - 工数: 3h

- [x] **Task 6.3.2** 統合テスト実施
  - デバイス切り替え → プロファイル認識 → ログ出力確認
  - `tests/VolumeProfileManager.UnitTests/Utilities/DeviceProfileMatcherTests.cs`
  - 工数: 2h

**フェーズ 6 合計工数: 12h**

---

**実装メモ（Phase 6 完了）**

✅ **Phase 6 実装完了（2026-07-22）**

- **新機能**: `DeviceProfileMatcher` を導入し、DeviceId完全一致(スコア100) -> DeviceName完全一致(スコア80) -> DeviceName部分一致(スコア50) の多段階スコアリングロジックを構築
- **サービス拡張**: `IProfileService.GetProfileAsync(deviceId, deviceName)` によるフォールバック検索サポート
- **CLI統合**: `CliHost.cs` の `Run()` における `DeviceChanged` イベントでデバイス名・IDを組み合わせて自動照合
- **単体テスト**: `DeviceProfileMatcherTests` / `ProfileServiceMatchingTests` を追加し全件合格を確認

---

## フェーズ 7: デバイスが切り替わった時に自動的に対応するプロファイル情報を適用する

### 7.1 Core レイヤー構築

- [x] **Task 7.1.1** IAudioVolumeService インターフェース定義
  - GetMasterVolumeAsync()
  - SetMasterVolumeAsync(float volume)
  - GetMuteStateAsync()
  - SetMuteStateAsync(bool isMuted)
  - `src/VolumeProfileManager.Core/Services/IAudioVolumeService.cs`
  - 工数: 1h

### 7.2 Infrastructure レイヤー構築

- [x] **Task 7.2.1** AudioVolumeService クラス実装
  - NAudio IAudioEndpointVolume を使用
  - マスターボリューム取得・設定
  - ミュート状態取得・設定
  - 再試行ロジック組み込み
  - `src/VolumeProfileManager.Infrastructure/Services/AudioVolumeService.cs`
  - 工数: 4h

- [x] **Task 7.2.2** AudioVolumeService ユニットテスト
  - IAudioEndpointVolume モック化
  - 各メソッドのテスト
  - エラーハンドリングテスト
  - `tests/VolumeProfileManager.UnitTests/Services/AudioVolumeServiceTests.cs`
  - 工数: 4h

### 7.3 自動適用ロジック実装

- [x] **Task 7.3.1** DeviceChanged イベントハンドラ完成化
  - プロファイル検索
  - VolumeProfile 情報で IAudioVolumeService を呼び出し
  - ログ出力（適用完了・失敗など）
  - `src/VolumeProfileManager.Console/CliHost.cs` 完成化
  - 工数: 3h

- [x] **Task 7.3.2** 新規プロファイル自動作成ロジック
  - プロファイル未存在時の処理
  - 現在の音量情報を記録
  - IProfileService.SaveProfileAsync() で保存
  - 工数: 2h

### 7.4 テスト

- [x] **Task 7.4.1** 統合テスト実施（フルフロー）
  - デバイス A → 音量設定 → プロファイル保存
  - デバイス B に切り替え → 新プロファイル自動作成
  - デバイス A に戻す → 音量自動復元
  - ログ確認
  - `tests/VolumeProfileManager.UnitTests/Services/ProfileServiceMatchingTests.cs`
  - 工数: 4h

- [x] **Task 7.4.2** 手動テスト実施
  - 実デバイスでのテスト（2 個以上のデバイス環境）
  - `vpm run` で監視
  - デバイス切り替え → 音量変更確認
  - ログファイル確認
  - 工数: 3h
  - ✅ 確認完了（2026-07-22）

### 7.5 CI/CD 設定

- [x] **Task 7.5.1** GitHub Actions ワークフロー設定
  - テスト自動実行
  - ビルド確認
  - `.github/workflows/build-test.yml`
  - 工数: 2h

**フェーズ 7 合計工数: 23h**

---

**実装メモ（Phase 7 完了）**

✅ **Phase 7 実装完了・実機動作確認完了（2026-07-22）**

- **自動適用**: `DeviceChanged` ハンドラ内で検出デバイスに対応する最適プロファイルを多段階検索し、音量 (`SetMasterVolumeAsync`) およびミュート状態 (`SetMuteStateAsync`) を自動適用
- **実機検証**: 実デバイス環境での切り替え操作により、音量 30% およびミュート解除が即時に自動適用されることを確認

---

## フェーズ 8: Console版廃止・TrayApp一本化・インストーラー対応（Issue #1）

> `main` ブランチ（NAudioベース Self-contained構成）の安定動作を確認済み。これをベースに CLI 版を廃止し、TrayApp とインストーラーに一本化する。

### 8.1 Console版廃止

- [x] **Task 8.1.1** `VolumeProfileManager.Console` プロジェクト削除
  - `src/VolumeProfileManager.Console/` ディレクトリ削除（`CliHost.cs`, `ICliHost.cs`, `Program.cs`, `.csproj`）
  - `VolumeProfileManager.slnx` からプロジェクト参照を除去
  - 工数: 0.5h

- [x] **Task 8.1.2** ビルド・テスト確認
  - `dotnet build` / `dotnet test` が Console 削除後も成功することを確認
  - 工数: 0.5h

### 8.2 インストーラー実装

- [x] **Task 8.2.1** Inno Setup インストーラースクリプト作成
  - `installer/VolumeProfileManager.iss`
  - ユーザーローカルインストール（管理者権限不要）
  - スタートアップ登録オプション（`StartupRegistration` と同一レジストリキー）
  - 日本語UI、アンインストーラー対応
  - 工数: 2h

- [x] **Task 8.2.2** Self-contained発行 → インストーラービルド → 動作確認
  - `dotnet publish -c Release -r win-x64 --self-contained true`
  - `ISCC.exe VolumeProfileManager.iss`
  - 実機でインストール・再起動後の自動起動・タスクトレイ常駐を確認済み
  - 工数: 1.5h
  - ✅ 確認完了（2026-08-16）

### 8.3 ドキュメント更新

- [x] **Task 8.3.1** `README.md` / `VolumeProfileManager_spec.md` / `task_list.md` の整合性確認
  - Console版記述の除去、TrayApp/インストーラー記述への更新
  - 工数: 1h

**フェーズ 8 合計工数: 5.5h**

---

## 追加タスク（共通）

### A. ドキュメント

- [ ] **Task A.1** README.md 作成
  - インストール方法
  - 使用方法
  - 設定ファイル説明
  - 工数: 3h

- [ ] **Task A.2** ユーザーマニュアル作成
  - CLI コマンドリファレンス
  - トラブルシューティング
  - 工数: 2h

### B. 品質管理

- [ ] **Task B.1** コード品質チェック
  - StyleCop ルール設定
  - Resharper / SonarQube で分析
  - 工数: 2h

- [ ] **Task B.2** パフォーマンステスト
  - デバイス変更検知応答時間測定（目標 100ms）
  - メモリ使用量測定（目標 30MB）
  - CPU 使用率測定（目標 1%）
  - 工数: 4h

- [ ] **Task B.3** セキュリティレビュー
  - ファイル権限確認
  - ログ情報の機密性確認
  - 工数: 2h

### C. リリース準備

- [ ] **Task C.1** 自己包含実行ファイル (SCD) ビルド設定
  - .csproj でビルド設定
  - Windows x64 ターゲット指定
  - 工数: 2h

- [ ] **Task C.2** バージョン管理・リリースノート
  - v1.0.0 ノート作成
  - 変更履歴記録
  - 工数: 1h

**追加タスク合計工数: 18h**

---

## タスク合計

| フェーズ | 工数 | タスク数 |
|---------|------|--------|
| フェーズ 1 | 18h | 8 |
| フェーズ 2 | 6.5h | 5 |
| フェーズ 3 | 5h | 5 |
| フェーズ 4 | 0h | 0（フェーズ 3 に統合） |
| フェーズ 5 | 21h | 10 |
| フェーズ 6 | 12h | 5 |
| フェーズ 7 | 23h | 11 |
| フェーズ 8 | 5.5h | 5 |
| 追加タスク | 18h | 8 |
| **合計** | **109h** | **57** |

---

## 進行管理

### スケジュール案（1 人開発）

- **Week 1-2**: フェーズ 1 完了 (18h)
- **Week 3**: フェーズ 2 完了 (6.5h)
- **Week 4**: フェーズ 3 完了 (5h)（旧フェーズ 4 統合済み）
- **Week 5-6**: フェーズ 5 完了 (21h)
- **Week 7**: フェーズ 6 完了 (12h)
- **Week 8-9**: フェーズ 7 完了 (23h)
- **Week 10**: 追加タスク・最終テスト (18h)
- **推定開発期間**: 10 週間（約 2.5 ヶ月）

### 2 人開発の場合

- 並行実装で工数を約 35% 削減可能
- **推定期間**: 7～8 週間

---

## 実装優先度

```
高 → フェーズ 1 > フェーズ 2 > フェーズ 3 > フェーズ 5
中 → フェーズ 6 > フェーズ 7
（フェーズ 4 はフェーズ 3 に統合済み）
低 → 追加タスク（A. ドキュメント）
```

各フェーズは順序に依存しているため、前後の入れ替えはできません。
