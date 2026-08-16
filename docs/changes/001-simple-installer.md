# インストーラー対応

## 背景 / 目的
Issue #1「インストーラー対応」。現状、`TrayApp` を利用するには `dotnet publish` 済みのフォルダを手動配置する必要があり、一般ユーザーへの配布に向かない。
過去に Native AOT + 自前 COM Interop 層での軽量化を試みたが、特定デバイスで `COMException` / `InvalidProgramException` が発生し不安定だったため断念（`main` は NAudio ベースの Self-contained 構成に戻して安定動作を確認済み）。
この安定版 `main` をベースに、Windows へのインストール・アンインストール・スタートアップ登録をワンクリックで行えるシンプルなインストーラーを実装する。

## 変更内容
- `VolumeProfileManager.Console`（CLI版）プロジェクトを削除し、`TrayApp`（タスクトレイ常駐版）に一本化する
  - `VolumeProfileManager.slnx` から `VolumeProfileManager.Console` プロジェクト参照を除去
  - `src/VolumeProfileManager.Console/` ディレクトリを削除
  - CLI固有のコマンド（`list-devices`, `set-volume`, `set-mute`, `save-profile`, `delete-profile`, `run` 等）は `TrayApp` のメニュー（ステータス表示・プロファイル更新）で代替済みのため、CLI版としての再実装は行わない
  - `Core`/`Infrastructure` 層（`IProfileCaptureService`, `DeviceMonitorOrchestrator` 等）は `TrayApp` が引き続き利用するため変更なし
- [Inno Setup](https://jrsoftware.org/isinfo.php) を用いたインストーラースクリプト `installer/VolumeProfileManager.iss` を新規追加
  - `dotnet publish -c Release -r win-x64 --self-contained true` で作成した Self-contained 発行物（`publish/TrayApp/`）一式をインストール対象とする
  - 管理者権限不要のユーザーローカルインストール（`{userpf}\VolumeProfileManager`、`PrivilegesRequired=lowest`）
  - スタートアップ登録タスク（`Tasks` セクション、`startupicon`）を用意し、インストール時に「Windowsログオン時に自動起動する」をオプション選択可能にする
    - 実体は `TrayApp` 側の `StartupRegistration`（`HKCU\...\Run`）と同じレジストリキーを使うため、インストーラー起点でもトレイメニュー起点でも状態が一致する
  - アンインストーラー（`unins000.exe`）でスタートアップ登録・インストールファイルを削除
  - 日本語UI（`Languages: japanese`）
- `README.md` にインストーラーのビルド手順（`dotnet publish` → `ISCC.exe`）を追記
- `.gitignore` に `dist/`, `publish/` を追加（発行物・インストーラー成果物はリポジトリに含めない）

## 影響範囲
- 削除: `src/VolumeProfileManager.Console/`（`CliHost.cs`, `ICliHost.cs`, `Program.cs`, `.csproj`）
- 変更: `VolumeProfileManager.slnx`（Console プロジェクト参照を除去）
- 新規: `installer/VolumeProfileManager.iss`
- 変更: `README.md`（Console版記述を削除し、インストーラービルド手順を追記）
- 変更: `.gitignore`
- ユーザーへの影響:
  - CLIコマンド（`list-devices`, `set-volume` 等）は利用不可になる。以後は `TrayApp` のトレイメニューのみで操作する
  - `dist/VolumeProfileManagerSetup.exe` を実行するだけで `TrayApp` をインストール・スタートアップ登録できるようになる

## 備考
- Native AOT 化は不採用（`docs/changes/002-native-aot-com-interop.md` 相当の検証により、NAudio の COM 相互運用が不安定だったため）。今回のインストーラーは Self-contained（フレームワーク同梱、約118MB）を前提とする
- 署名（コード署名証明書）は未対応。SmartScreen警告が出る可能性があるが、今回のスコープ外とする
- 自動アップデート機能は実装しない。バージョンアップ時は新しいインストーラーを再配布し、上書きインストールする運用とする
