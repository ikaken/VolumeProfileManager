# タスクトレイへの常駐

## 背景 / 目的
現状の `VolumeProfileManager.Console` は `run` コマンド実行中、コンソールウィンドウを開いたままにする必要があり、Windows起動時の自動常駐や、コンソールを閉じずに使い続ける運用に向いていない。
Issue #5 の要望は「Windows起動時にタスクトレイに常駐して動作してほしい」というもの。デバイス監視・プロファイル自動適用機能はバックグラウンドで継続させつつ、タスクトレイアイコンから状態確認・終了操作ができるようにする。

## 変更内容
- 新規プロジェクト `src/VolumeProfileManager.TrayApp` を追加（`net10.0-windows`、**WinForms不使用**）
  - **リソース節約のため `System.Windows.Forms` / `System.Drawing.Common` は利用せず、Win32 API（`Shell_NotifyIcon`, `CreateWindowEx`, `TrackPopupMenuEx` 等）を P/Invoke で直接呼び出してトレイアイコンを実装する**
  - メッセージ専用ウィンドウ（画面非表示）を `RegisterClassEx` + `CreateWindowEx` で作成し、`Shell_NotifyIcon`（`NIM_ADD`）でアイコンを登録
  - 右クリック時に `CreatePopupMenu` / `TrackPopupMenuEx` でメニューを表示: 「ステータス表示」「スタートアップ登録/解除」「終了」
  - `GetMessage` / `DispatchMessage` の素のメッセージループ（`Application.Run()` は使用しない）
  - アイコンリソースは `.ico` を 1 つ埋め込み、`LoadImage` でロード（`System.Drawing` 不要）
  - 既存 `Core`/`Infrastructure` 層を再利用した DI 構成
- `CliHost.Run()` 内のデバイス切り替え検出〜プロファイル自動適用ロジックを `Infrastructure` 層の `DeviceMonitorOrchestrator` として抽出し、`CliHost` と `TrayApp` で共通利用
- Windows 起動時の自動起動を `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` へのレジストリ登録/削除で実現し、トレイメニューからトグル操作可能にする
- プロファイル自動適用時に `Shell_NotifyIcon`（`NIM_MODIFY` + `NIF_INFO`）によるバルーン通知を表示（既存の Serilog ファイルログは維持）
- トレイメニューに「プロファイルを更新」を追加し、現在のデフォルトデバイスの音量・ミュート状態をプロファイルへ保存できるようにする
  - `Core` 層に `IProfileCaptureService` を定義し、`Infrastructure` 層の `ProfileCaptureService` で実装（デフォルトデバイス取得・音量取得・`IProfileService` 経由の保存を担当）
  - CLI の `save-profile` コマンドも同サービスに委譲し、重複実装を排除
- プロファイル保存先を `AppContext.BaseDirectory\data\profiles.json`（実行ファイルごとに別々）から `%LOCALAPPDATA%\VolumeProfileManager\profiles.json` に統一し、CLI と TrayApp で同一ファイルを共有する
  - 旧ファイルからの自動移行機能は実装しない（本体をシンプルに保つ方針。必要な場合はユーザーが手動でコピーする）

## 影響範囲
- 新規: `src/VolumeProfileManager.TrayApp/`（`Program.cs`, `NativeMethods.cs`（Win32 P/Invoke定義）, `TrayIconWindow.cs` 等）
- 新規: `src/VolumeProfileManager.Infrastructure/Services/DeviceMonitorOrchestrator.cs`（および `Core` 側のインターフェース）
- 変更: `src/VolumeProfileManager.Console/CliHost.cs`（`Run()` を `DeviceMonitorOrchestrator` 呼び出しにリファクタ）
- 変更: `VolumeProfileManager.slnx`（`TrayApp` プロジェクト追加）
- 新規: `src/VolumeProfileManager.Core/Interfaces/IProfileCaptureService.cs`, `src/VolumeProfileManager.Infrastructure/Services/ProfileCaptureService.cs`
- 変更: `src/VolumeProfileManager.Infrastructure/Persistence/ProfileRepository.cs`（保存先パスを `%LOCALAPPDATA%\VolumeProfileManager\profiles.json` に変更）
- 変更: `src/VolumeProfileManager.Console/CliHost.cs`（`save-profile` を `IProfileCaptureService` に委譲）
- ユーザーへの影響: `run` コマンドは後方互換で維持。新たに `TrayApp` (`VolumeProfileManager.TrayApp.exe`) を起動することでタスクトレイ常駐が可能になる

## リソース消費の比較
| 方式 | 追加アセンブリ | 想定メモリオーバーヘッド |
|---|---|---|
| WinForms `NotifyIcon`（不採用） | `System.Windows.Forms`, `System.Drawing.Common` | 約25〜40MB増 |
| **Win32 API直接呼び出し（採用）** | なし（.NET標準ライブラリのみ） | 約5〜10MB増 |

## 備考
- インストーラー化（SCD自己完結ビルド、Task C.1）は別タスク・別Issueとして今回の対象外とする。ただし将来インストーラーからも呼び出せるよう、スタートアップ登録処理は `StartupRegistration` として独立したクラスに切り出し、`TrayApp` からも将来のインストーラーからも同じロジックを再利用できる構成にする
- 既存の `run` コマンド（コンソール常駐）は後方互換のため維持する
- Native AOTパブリッシュは将来的な検討事項（`NAudio` の COM 相互運用が AOT 互換かの検証が必要）とし、今回はフレームワーク依存ビルドで対応する
