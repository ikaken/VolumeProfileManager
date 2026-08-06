# VolumeProfileManager

軽量な Windows 向けオーディオデバイス別ボリュームプロファイル管理ツール。

## 目的
- デバイス切替時に保存済みプロファイルを自動適用する
- CLI およびタスクトレイ常駐アプリからプロファイルの保存／削除／確認が可能

## 主な機能
- オーディオデバイスごとの音量・ミュート状態をプロファイルとして保存し、デバイス切替時に自動適用
- 未登録デバイスへの切替時は現在の音量で自動的に新規プロファイルを作成
- タスクトレイ常駐アプリ（`VolumeProfileManager.TrayApp`）: WinForms 不使用の軽量 Win32 API 実装
  - ステータス表示 / プロファイルを更新（現在の音量を保存） / スタートアップ登録・解除 / 終了
- プロファイルの保存先は CLI・TrayApp で共通化: `%LOCALAPPDATA%\VolumeProfileManager\profiles.json`

## 開発環境
- .NET 10.0 SDK（`net10.0-windows` / Windows専用）
- Visual Studio / VS Code

## リポジトリ構成
- `src/` - ソースコード（`Domain` / `Core` / `Infrastructure` / `Console` / `TrayApp`）
- `tests/` - 単体テスト
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

## 実行例（CLI）
- デバイス一覧表示
```powershell
dotnet run --project src\VolumeProfileManager.Console -- list-devices
```

- 現在の状態を確認
```powershell
dotnet run --project src\VolumeProfileManager.Console -- status
```

- 音量・ミュートを操作
```powershell
dotnet run --project src\VolumeProfileManager.Console -- set-volume 30
dotnet run --project src\VolumeProfileManager.Console -- set-mute true
```

- プロファイルの保存・削除
```powershell
dotnet run --project src\VolumeProfileManager.Console -- save-profile
dotnet run --project src\VolumeProfileManager.Console -- delete-profile <deviceId>
```

- 監視モードで実行（コンソール常駐、デバイス切替時にプロファイル自動適用）
```powershell
dotnet run --project src\VolumeProfileManager.Console -- run
```

## タスクトレイ常駐アプリの実行
```powershell
dotnet run --project src\VolumeProfileManager.TrayApp
```
タスクトレイアイコンを右クリックすると「ステータス表示」「プロファイルを更新」「スタートアップ登録/解除」「終了」のメニューが表示されます。デバイス切替を検知すると保存済みプロファイルを自動適用し、バルーン通知で結果を表示します。

## プロファイルの保存先
```
%LOCALAPPDATA%\VolumeProfileManager\profiles.json
```
CLI・TrayAppともに同一ファイルを参照するため、どちらから更新しても内容が共有されます。

## ステータス
- **Phase 1〜7**: ✅ 完了 - デバイス取得・音量操作・プロファイル自動照合/自動適用の実装完了
- **タスクトレイ常駐アプリ**: ✅ 実装済み・実機動作確認済み
- **CI/CD**: ✅ 構成済み - GitHub Actions で自動テスト実行
- **テスト**: ✅ 複数デバイスタイプで検証済み

## 備考
- 実機でのデバイス切り替えテストを推奨します。
- 詳細な実装内容については [docs/task_list.md](docs/task_list.md)、変更履歴は [docs/changes/](docs/changes/) を参照してください。
