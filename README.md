# VolumeProfileManager

軽量な Windows 向けオーディオデバイス別ボリュームプロファイル管理ツール。

## 目的
- デバイス切替時に保存済みプロファイルを自動適用する
- タスクトレイ常駐アプリからプロファイルの保存／確認が可能

## 主な機能
- オーディオデバイスごとに音量・ミュート状態をプロファイルとして保存し、デバイス切替時に自動適用
- 未登録デバイスへの切替時は現在の音量で自動的に新規プロファイルを作成
- タスクトレイ常駐アプリ（`VolumeProfileManager.TrayApp`）: Rust + windows-rs による軽量 Win32 API 実装（約 700KB）
  - ステータス表示 / プロファイルを更新（現在の音量を保存） / スタートアップ登録・解除 / 終了
- プロファイルの保存先: `%LOCALAPPDATA%\VolumeProfileManager\profiles.json`（ポータブル版は `data\` 配下）

## ダウンロード
最新版（v2.0.0）は [Releases](https://github.com/ikaken/VolumeProfileManager/releases/tag/v2.0.0) からダウンロードできます。

- `VolumeProfileManagerSetup.exe` - インストーラー版（推奨）
- `VolumeProfileManager-2.0.0-portable.zip` - ポータブル版

## 使い方

1. **インストール**
   [Releases](https://github.com/ikaken/VolumeProfileManager/releases/tag/v2.0.0) から `VolumeProfileManagerSetup.exe` をダウンロードして実行し、ウィザードに従ってインストールします。
   ポータブル版を使う場合は ZIP を展開し、`VolumeProfileManager.TrayApp.exe` を実行します。

2. **現在のデバイスの音量を設定**
   Windowsの音量ミキサーなどで、現在使用しているオーディオデバイスの音量・ミュート状態をお好みに調整します。

3. **タスクトレイアイコンのメニューからプロファイルを保存**
   タスクトレイアイコンを右クリック →「プロファイルを更新」を選択すると、現在のデバイスの音量・ミュート状態がプロファイルとして保存されます。

4. **利用しているデバイスそれぞれでプロファイルを保存**
   ヘッドホン・スピーカーなど、普段切り替えて使うオーディオデバイスに切り替えるたびに手順2～3を繰り返し、デバイスごとにプロファイルを保存しておきます。

5. **デバイスを切り替えると自動で反映**
   一度プロファイルを保存すれば、以降はデバイスを切り替えるだけで、保存済みの音量・ミュート状態が自動的に適用されます（バルーン通知で結果が表示されます）。未登録のデバイスに切り替えた場合は、現在の音量で新規プロファイルが自動作成されます。

## 開発環境
- Rust 1.93.1（`rust-toolchain.toml` で固定 / `x86_64-pc-windows-msvc` / Windows専用）
- Inno Setup 6（インストーラービルド時のみ）

## リポジトリ構成
- `crates/` - V2 ソースコード（Rust ワークスペース）
  - `vpm-core` - ドメインモデル・プロファイル照合・永続化抽象
  - `vpm-platform` - Core Audio API 連携・永続化・ログ・システム機能
  - `vpm-tray` - Win32 タスクトレイアプリ
- `src/`, `tests/` - V1（.NET 版）レガシーコード（参照用に残存）
- `installer/` - Inno Setup インストーラースクリプト
- `assets/` - アプリケーションアイコン
- `docs/` - 仕様・設計・テスト計画・タスクリスト・変更履歴（`docs/changes/`）

## ビルド方法
```powershell
cargo build --release --locked --target x86_64-pc-windows-msvc
```
実行ファイルは `target\x86_64-pc-windows-msvc\release\VolumeProfileManager_TrayApp.exe` に生成されます。

## テスト実行
```powershell
cargo test --workspace --locked
```

## タスクトレイ常駐アプリの実行（開発用）
```powershell
cargo run -p vpm-tray
```
タスクトレイアイコンを右クリックすると「ステータス表示」「プロファイルを更新」「スタートアップ登録/解除」「終了」のメニューが表示されます。デバイス切替を検知すると保存済みプロファイルを自動適用し、バルーン通知で結果を表示します。

## インストーラーのビルド（配布用）
事前に [Inno Setup](https://jrsoftware.org/isinfo.php) をインストールしておく必要があります。

1. リリースビルドと publish 配置
```powershell
cargo build --release --locked --target x86_64-pc-windows-msvc
New-Item -ItemType Directory -Force -Path publish\TrayApp
Copy-Item target\x86_64-pc-windows-msvc\release\VolumeProfileManager_TrayApp.exe publish\TrayApp\VolumeProfileManager.TrayApp.exe
Copy-Item assets\app.ico publish\TrayApp\app.ico
```

2. インストーラービルド（ISCC.exe に PATH が通っていること）
```powershell
ISCC.exe /DMyAppVersion=2.0.0 installer\VolumeProfileManager.iss
```

3. `dist\VolumeProfileManagerSetup.exe` が生成されます。このインストーラーを実行すると、管理者権限不要でユーザーローカルにインストールされ、インストール時にスタートアップ登録の有無を選択できます。

## ポータブル版
`VolumeProfileManager-*-portable.zip` を展開したディレクトリには `portable.flag` が含まれており、実行するとプロファイル・ログを `<展開先>\data\` 配下に保存します（`%LOCALAPPDATA%` には書き込みません）。

## プロファイルの保存先
```
%LOCALAPPDATA%\VolumeProfileManager\profiles.json
```
ポータブル版: `<展開先>\data\profiles.json`

## ステータス
- **v2.0.0**: ✅ リリース済み（[Releases](https://github.com/ikaken/VolumeProfileManager/releases/tag/v2.0.0)）- Rust + windows-rs による全面再構築版（Issue #16）
- **V1（.NET 版）**: ✅ 終了 - `src/` はレガシー参照用
- **タスクトレイ常駐アプリ**: ✅ 実装済み・実機動作確認済み
- **インストーラー / ポータブル版**: ✅ GitHub Actions による自動ビルド・公開
- **テスト**: ✅ 全63件合格（V1 とのデータ互換性検証を含む）

## 備考
- 実機でのデバイス切り替えテストを推奨します。
- 詳細な実装内容については [docs/task_list.md](docs/task_list.md)、変更履歴は [docs/changes/](docs/changes/) を参照してください。
