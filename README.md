# VolumeProfileManager

軽量な Windows 向けオーディオデバイス別ボリュームプロファイル管理ツール。

## 目的
- デバイス切替時に保存済みプロファイルを自動適用する
- CLI でプロファイルの保存／削除／確認が可能

## 開発環境
- .NET 10.0 SDK
- Visual Studio / VS Code

## リポジトリ構成
- `src/` - ソースコード
- `tests/` - 単体テスト
- `docs/` - 仕様・設計・テスト計画・タスクリスト

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

## 実行例
- デバイス一覧表示
```powershell
dotnet run --project src\VolumeProfileManager.Console -- list-devices
```

- 監視モードで実行
```powershell
dotnet run --project src\VolumeProfileManager.Console -- run
```

## ステータス
- **Phase 1**: ✅ 完了 - デバイス取得・プロファイル管理・自動適用の実装完了
- **CI/CD**: ✅ 構成済み - GitHub Actions で自動テスト実行
- **テスト**: ✅ 複数デバイスタイプで検証済み

## 備考
- 実機でのデバイス切り替えテストを推奨します。
- 詳細な実装内容については [docs/task_list.md](docs/task_list.md) を参照してください。
