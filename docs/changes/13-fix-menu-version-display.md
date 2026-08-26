# メニューのタイトルに表示されるのバージョン番号が変わらない

## 背景 / 目的

リリースタグ（例: `v1.2.0-beta`）をプッシュしても、GitHub Actions のビルドでアセンブリバージョンが設定されていないため、タスクトレイメニューに表示されるバージョンが常に `v1.0.0` のままになる。タグに応じた正しいバージョンを表示させる。

## 変更内容

- `.github/workflows/release.yml` の `Build` ステップに `/p:Version=<タグ名から v を除去>` を追加
- `.github/workflows/release.yml` の `Publish TrayApp` ステップにも同様の `/p:Version=<タグ名から v を除去>` を追加

## 影響範囲

- `VolumeProfileManager.TrayApp/TrayIconWindow.cs` の `AppVersionString`（`AssemblyVersion` を参照している）
- GitHub Actions によるリリースビルド・パッケージング
- インストーラーに同梱される EXE/DLL のファイルバージョン情報

## 備考

- ローカルでの手動ビルドでは引き続き `v1.0.0` と表示されるが、本プロジェクトのリリースは GitHub Actions 経由で行うため許容する。
- `Version` プロパティに `1.2.0-beta` を渡しても、`AssemblyVersion` は `1.2.0.0` となりメニュー表示は `v1.2.0` となる。`-beta` サフィックスは `AssemblyInformationalVersion` に残る。
