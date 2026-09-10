# Issue #17: ポータブル版の追加とインストーラー版との同時リリース

## 背景 / 目的
VolumeProfileManager V2 において、Inno Setup で作成した未署名インストーラーが一部環境（Windows Defender 等）で誤検出（`Trojan:Win32/Wacatac.C!ml`）される問題が発生した。
アプリ本体の Rust EXE および ZIP 圧縮ファイルはセキュリティソフト等で問題が検出されていないため、以下の要件を満たすためにインストーラー版とポータブル版を併用し、同一の GitHub Release で同時配布する。

- セキュリティソフトによりインストーラーがブロックされた場合の代替手段の提供
- インストール不要でのポータブル利用
- USBメモリー等による設定を含めた持ち運び
- OSやユーザープロファイル（%LOCALAPPDATA%）に設定を残さない利用

## 変更内容

### 1. ポータブルモード判定とストレージレイアウト
- 実行ファイル（`VolumeProfileManager.TrayApp.exe`）と同じディレクトリに `portable.flag` が存在するか判定する。
  - `portable.flag` が存在する場合: **ポータブルモード**
  - `portable.flag` が存在しない場合: **通常モード**
- データ保存先:
  - **通常モード**: `%LOCALAPPDATA%\VolumeProfileManager\` 配下（`profiles.json`, `logs/`, バックアップファイル等）
  - **ポータブルモード**: 実行ファイル同一ディレクトリの `data\` 配下（`data\profiles.json`, `data\logs\`, バックアップファイル等）
- ポータブルモード起動時、`data` ディレクトリが存在しない場合は自動作成する。
- ポータブルモードでは `%LOCALAPPDATA%` にいかなる永続データも作成しない。

### 2. 書き込み権限エラーハンドリング
- ポータブル版配置先（`data\`）に書き込み権限がない場合:
  - `%LOCALAPPDATA%` へ暗黙的にフォールバックしない。
  - 他の場所への保存も行わない。
  - 既存の設定ファイルを破損させない。
  - ユーザーへ書き込み権限エラーをダイアログ（MessageBox）等で明示的に通知して安全に終了する。

### 3. 単一インスタンス制御・スタートアップ・互換性
- **単一インスタンス**: 既存の Global 名前付き Mutex（`Global\VolumeProfileManager_SingleInstance_Mutex`）を共有し、通常版とポータブル版の同時起動を防止。
- **スタートアップ**: ポータブル版でもトレイメニューからの手動登録/解除は可能（ただし配置フォルダ移動時は自動追従しない仕様）。
- **設定ファイル互換性**: PascalCase 6 項目の JSON 構造は完全維持され、通常版とポータブル版の間で `profiles.json` を相互利用可能。

### 4. GitHub Actions リリース自動化
- `.github/workflows/release.yml` にポータブル版 ZIP 作成ステップを追加。
- 以下のディレクトリ構成を ZIP 化し、インストーラー（`VolumeProfileManagerSetup.exe`）とともに GitHub Release / Pre-release へ添付:
  ```text
  VolumeProfileManager-{version}-portable/
  ├── VolumeProfileManager.TrayApp.exe
  ├── app.ico
  ├── portable.flag
  └── data/
  ```

## 影響範囲
- **修正コンポーネント**:
  - `crates/vpm-platform` または `crates/vpm-tray`: ストレージパス解決、ポータブル判定、書き込み権限チェック
  - `.github/workflows/release.yml`: ポータブル ZIP 生成・リリース添付
  - `docs/VolumeProfileManager_spec.md`, `docs/design.md`, `docs/task_list.md`: 仕様・設計ドキュメント更新
- **ユーザーへの影響**:
  - 通常版の動作・既存設定ファイルパス（`%LOCALAPPDATA%`）は一切変更なし。
  - ZIP 版をダウンロードして展開するだけで、レジストリや `%LOCALAPPDATA%` を汚さずに利用可能となる。

## 検証結果
- 2026-09-09: `cargo fmt --all -- --check` パス。
- 2026-09-09: `cargo clippy --workspace --all-targets --locked -- -D warnings` パス（警告ゼロ）。
- 2026-09-09: `cargo test --workspace --locked` 全件合格（新設のポータブルモード・書き込み権限テスト 6 件を含む全 59 件合格）。
- 2026-09-09: `cargo build --release --locked` 成功確認。
- 2026-09-09: `.github/workflows/release.yml` のポータブル ZIP 生成・リリース添付定義の更新確認。

## 備考
- `portable.flag` は空ファイルで問題ない。
- コード署名証明書（OV/EV）導入や自動アップデートは対象外。
