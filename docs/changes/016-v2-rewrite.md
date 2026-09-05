# Issue #16: V2開発（Rust / windows-rs / Native Win32 による全面再構築）

## 背景 / 目的
VolumeProfileManager を新技術スタック（Rust + windows-rs + Win32 ネイティブタスクトレイ）により全面再構築し、機能・挙動・設定ファイル互換性を 100% 維持したまま、軽量性・安全性・起動性能を向上させる。
V2 開発ラインは `rewrite/v2` ブランチで並行開発し、ベータ版リリースおよび安定化を経て `main` へマージする。

## 変更内容
- **Cargo Workspace 構成**:
  - `crates/vpm-core`: ドメインモデル（`VolumeProfile`, `ProfileTimestamp`）、多段階照合マッチャー（`match_profile`, `ordinal_ignore_case_eq/contains`）、抽象化トレイト（`Clock`, `ProfileStore`）。
  - `crates/vpm-platform`: Core Audio API バインディング（windows-rs）、オーケストレーター（800ms末尾デバウンス、3秒同一デバイス抑制、即時適用）、JSON永続化（アトミック置換、破損時自動復元、移行用スナップショット）、日次ローテーションロガー、ミューテックス排他制御、スタートアップ登録。
  - `crates/vpm-tray`: Win32 メッセージループ・トレイアイコン・バルーン通知・コンテキストメニュー・アプリケーションアイコン埋め込み。
- **データ互換性と安全性改善**:
  - V1 の実際の保存形式である PascalCase 6 項目 JSON を完全維持。
  - .NET 10 実機プローブ連携テスト（43件）により、V1 とのシリアライズ往復・Unicode 大小文字照合の一致を検証。
  - JSON 破損時に有効なバックアップのみから復旧し、両方破損時は空リストでの上書きを安全に中断。
  - 音量取得エラー時に 0/false に置換して保存せず、既存データを保護。
- **インストーラー・リリース基盤**:
  - Inno Setup（`installer/VolumeProfileManager.iss`）によるユーザーローカル置換インストール（旧 .NET 版残留ファイルの自動クリーンアップ、多重起動防止）。
  - GitHub Actions ワークフロー（`.github/workflows/release.yml`）を Rust ビルド・テスト・Pre-release 自動判定に対応。

## 影響範囲
- **新規コンポーネント**:
  - `crates/vpm-core/`
  - `crates/vpm-platform/`
  - `crates/vpm-tray/`
- **ドキュメント**:
  - `docs/design.md`
  - `docs/VolumeProfileManager_spec.md`
  - `docs/test_specification.md`
  - `docs/task_list.md`
- **ユーザーへの影響**:
  - 機能変更なし。以前と同じトレイメニュー操作（ステータス表示、プロファイル更新、スタートアップ登録/解除、終了）、バルーン通知、デバイス切り替え時の自動音量適用を利用可能。
  - 実行ファイルサイズが約 50MB（Self-contained .NET）から約 700KB へ大幅に軽量化。

## 備考
- V1 と V2 は同一 PC 上で共存させず置換運用。
- ベータ版は手動ダウンロード（Pre-release）とし、自動アップデート対象外。
