# トレイメニューにアプリ名とバージョンを表示する

## 背景 / 目的
現在、タスクトレイアイコンを右クリックして表示されるコンテキストメニューには、アプリ名およびバージョン情報が表示されておらず、どのアプリのどのバージョンが起動中であるかをトレイメニューから確認することができない。
Issue #9 の要望に基づき、タスクトレイメニューの最上部に「VolumeProfileManager vX.Y.Z」のような形式でアプリ名とバージョン情報を非活性（Disabled / Grayed）表示し、直下に区切り線（Separator）を追加する。

## 変更内容
1. **Win32 定数の追加 (`NativeMethods.cs`)**:
   - メニュー項目の非活性表示およびセパレータ作成に必要な Win32 定数 `MF_GRAYED`, `MF_DISABLED`, `MF_SEPARATOR` を定義する。
2. **バージョン情報の取得とトレイメニューヘッダーの追加 (`TrayIconWindow.cs`)**:
   - `Assembly` からアセンブリバージョン情報を取得するヘルパーメソッドまたはプロパティを追加。
   - `ShowContextMenu()` 内でメニュー作成時に、最上部へ「VolumeProfileManager v1.0.0」等の項目を `MF_STRING | MF_GRAYED | MF_DISABLED` で追加し、続いて `MF_SEPARATOR` で区切り線を追加する。
3. **ユニットテストの追加 / 更新 (`TrayIconWindowTests.cs` 等)**:
   - バージョン情報取得およびメニュー生成ロジックの動作検証を行う。

## 影響範囲
- 変更: `src/VolumeProfileManager.TrayApp/NativeMethods.cs`
- 変更: `src/VolumeProfileManager.TrayApp/TrayIconWindow.cs`
- 追加: `docs/changes/009-tray-appname-version.md`（本ファイル）
- ユーザーへの影響:
  - タスクトレイアイコンを右クリックした際に、メニューの最上部にアプリ名と現在のバージョンが表示されるようになり、確認が容易になる。

## 備考
- `MF_GRAYED` / `MF_DISABLED` フラグを付与することにより、アプリ名・バージョン表示行はクリック不可とし、IDの選択処理 (`MenuCommandSelected`) には影響を与えない。
