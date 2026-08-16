# VolumeProfileManager

軽量な Windows 向けオーディオデバイス別ボリュームプロファイル管理ツール。

## 目的
- デバイス切替時に保存済みプロファイルを自動適用する
- タスクトレイ常駐アプリからプロファイルの保存／確認が可能

## 主な機能
- オーディオデバイスごとに音量・ミュート状態をプロファイルとして保存し、デバイス切替時に自動適用
- 未登録デバイスへの切替時は現在の音量で自動的に新規プロファイルを作成
- タスクトレイ常駐アプリ（`VolumeProfileManager.TrayApp`）: WinForms 不使用の軽量 Win32 API 実装
  - ステータス表示 / プロファイルを更新（現在の音量を保存） / スタートアップ登録・解除 / 終了
- プロファイルの保存先: `%LOCALAPPDATA%\VolumeProfileManager\profiles.json`

## ダウンロード
最新版（v1.0.0）のインストーラーは [Releases](https://github.com/ikaken/VolumeProfileManager/releases/tag/v1.0.0) からダウンロードできます。

## 使い方

1. **インストール**
   [Releases](https://github.com/ikaken/VolumeProfileManager/releases/tag/v1.0.0) から `VolumeProfileManagerSetup.exe` をダウンロードして実行し、ウィザードに従ってインストールします。

2. **現在のデバイスの音量を設定**
   Windowsの音量ミキサーなどで、現在使用しているオーディオデバイスの音量・ミュート状態をお好みに調整します。

3. **タスクトレイアイコンのメニューからプロファイルを保存**
   タスクトレイアイコンを右クリック →「プロファイルを更新」を選択すると、現在のデバイスの音量・ミュート状態がプロファイルとして保存されます。

4. **利用しているデバイスそれぞれでプロファイルを保存**
   ヘッドホン・スピーカーなど、普段切り替えて使うオーディオデバイスに切り替えるたびに手順2～3を繰り返し、デバイスごとにプロファイルを保存しておきます。

5. **デバイスを切り替えると自動で反映**
   一度プロファイルを保存すれば、以降はデバイスを切り替えるだけで、保存済みの音量・ミュート状態が自動的に適用されます（バルーン通知で結果が表示されます）。未登録のデバイスに切り替えた場合は、現在の音量で新規プロファイルが自動作成されます。

## 開発環境
- .NET 10.0 SDK（`net10.0-windows` / Windows専用）
- Visual Studio / VS Code

## リポジトリ構成
- `src/` - ソースコード（`Domain` / `Core` / `Infrastructure` / `TrayApp`）
- `tests/` - 単体テスト
- `installer/` - Inno Setup インストーラースクリプト
- `docs/` - 仕様・設計・テスト計画・タスクリスト・変更履歴（`docs/changes/`）

## ビルド方法
```powershell
cd c:\work\VolumeProfileManager
dotnet restore
dotnet build
```

## テスト実行
```powershell
dotnet test
```

## タスクトレイ常駐アプリの実行（開発用）
```powershell
dotnet run --project src\VolumeProfileManager.TrayApp
```
タスクトレイアイコンを右クリックすると「ステータス表示」「プロファイルを更新」「スタートアップ登録/解除」「終了」のメニューが表示されます。デバイス切替を検知すると保存済みプロファイルを自動適用し、バルーン通知で結果を表示します。

## インストーラーのビルド（配布用）
事前に [Inno Setup](https://jrsoftware.org/isinfo.php) をインストールしておく必要があります。

1. Self-contained 発行
```powershell
dotnet publish src\VolumeProfileManager.TrayApp -c Release -r win-x64 --self-contained true -o publish\TrayApp
```

2. インストーラービルド（ISCC.exe に PATH が通っていること）
```powershell
ISCC.exe installer\VolumeProfileManager.iss
```

3. `dist\VolumeProfileManagerSetup.exe` が生成されます。このインストーラーを実行すると、管理者権限不要でユーザーローカルにインストールされ、インストール時にスタートアップ登録の有無を選択できます。

## プロファイルの保存先
```
%LOCALAPPDATA%\VolumeProfileManager\profiles.json
```

## ステータス
- **v1.0.0**: ✅ リリース済み（[Releases](https://github.com/ikaken/VolumeProfileManager/releases/tag/v1.0.0)）
- **Phase 1～7**: ✅ 完了 - デバイス取得・音量操作・プロファイル自動照合/自動適用の実装完了
- **タスクトレイ常駐アプリ**: ✅ 実装済み・実機動作確認済み
- **CLI版（Console）**: ✅ 廃止済み（Issue #1、TrayAppに一本化）
- **インストーラー**: ✅ 実装済み・実機インストール/自動起動確認済み（Issue #1）
- **テスト**: ✅ 複数デバイスタイプで検証済み

## 備考
- 実機でのデバイス切り替えテストを推奨します。
- 詳細な実装内容については [docs/task_list.md](docs/task_list.md)、変更履歴は [docs/changes/](docs/changes/) を参照してください。
