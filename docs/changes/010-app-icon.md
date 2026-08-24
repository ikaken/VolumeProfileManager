# アプリのアイコンを作成

## 背景 / 目的
現在、`VolumeProfileManager.TrayApp` には固有のアイコンが設定されておらず、Windowsの標準アプリアイコン（`IDI_APPLICATION`）がタスクトレイおよび実行ファイルに表示されている。
Issue #10 の要望に基づき、VolumeProfileManager に適した専用アイコン（音量調整・プロファイル管理をモチーフとしたデザイン）を作成し、アプリケーション本体（EXE）、タスクトレイ、インストーラーに適用する。

## 変更内容
1. **アセットディレクトリ (`assets/`) の作成**:
   - 外部で作成したアイコンファイル（`app.ico`, `app.png` 等）を配置・管理するための [assets](file:///c:/work/VolumeProfileManager/assets/) フォルダを作成。
2. **実行ファイルへのアイコン埋め込み (`VolumeProfileManager.TrayApp.csproj`)**:
   - `<ApplicationIcon>..\..\assets\app.ico</ApplicationIcon>` を追加し、EXEファイル自体のリソースとして埋め込み、ビルド出力にもコピーする設定を追加。
3. **タスクトレイアイコンのロード処理改修 (`TrayIconWindow.cs` / `NativeMethods.cs`)**:
   - Win32 API の `LoadImage` / `ExtractIconEx` を使用し、自身のEXEに埋め込まれたリソースアイコンおよび配置された `app.ico` から小アイコンを取得してタスクトレイに設定。
   - 取得に失敗した場合は、既存の `IDI_APPLICATION` へ安全にフォールバック。
4. **インストーラーのアイコン設定 (`installer/VolumeProfileManager.iss`)**:
   - `[Setup]` セクションに `SetupIconFile=..\assets\app.ico` を指定し、インストーラーのアイコンも統一。

## 影響範囲
- 新規: `assets/`（[README.md](file:///c:/work/VolumeProfileManager/assets/README.md)）
- 変更: `src/VolumeProfileManager.TrayApp/VolumeProfileManager.TrayApp.csproj`
- 変更: `src/VolumeProfileManager.TrayApp/TrayIconWindow.cs`
- 変更: `src/VolumeProfileManager.TrayApp/NativeMethods.cs`
- 変更: `installer/VolumeProfileManager.iss`
- 変更: `docs/changes/010-app-icon.md`（本ファイル）
- ユーザーへの影響:
  - EXEファイルおよびタスクトレイに専用のアイコンが表示されるようになり、視認性と操作性が向上する。

## 備考
- `System.Drawing` や `System.Windows.Forms` に依存せず、軽量な Win32 API P/Invoke 経由でのアイコンロードを維持する。
