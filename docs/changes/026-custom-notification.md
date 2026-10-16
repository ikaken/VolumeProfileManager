# Issue #26: 通知領域の変更（独自通知ポップアップ）

## 背景 / 目的
プロファイル適用・ステータス表示等の通知に `Shell_NotifyIconW` のバルーン通知（`NIF_INFO`）を使用しているが、OS 管理の通知は前の通知が消えるまで表示を待機するため応答が遅い。独自の通知ポップアップを実装し、通知要求と同時に即座に表示されるようにする。

## 変更内容

### 1. 独自通知ポップアップの新設（`crates/vpm-tray/src/notification.rs`）
- `NotificationWindow` を新設。`WS_POPUP` スタイルかつ `WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE` の非アクティブ化されたトップレベルウィンドウを、アプリ起動時に非表示のまま 1 つだけ生成する。
- 表示位置は `SystemParametersInfoW(SPI_GETWORKAREA)` で取得したワークエリア右下から `12 * scale` 内側。サイズは幅 `360 * scale`・高さ `96 * scale`（`scale` は `GetDeviceCaps(GetDC(None), LOGPIXELSX) / 96` で算出）。配置計算は純粋関数 `compute_popup_rect` として切り出し、ユニットテストで検証する。
- 描画は GDI で自前実装（白背景・灰枠・濃灰文字）。フォントは `SystemParametersInfoW(SPI_GETNONCLIENTMETRICS)` の `lfMessageFont` から `CreateFontIndirectW` で生成し、タイトル用は `lfWeight = FW_BOLD` のコピー。取得失敗時は `DEFAULT_GUI_FONT` にフォールバック。
- 表示開始から 4 秒後に `SetTimer`/`WM_TIMER` で自動的に非表示化。左・右クリックでも即座に非表示にする。`WM_MOUSEACTIVATE` で `MA_NOACTIVATE` を返すため、クリックでフォーカスを奪わない。
- `show` は任意スレッドから呼び出し可能で、`PostMessageW` で UI スレッドにペイロード（`Box<(String, String)>`）を送るだけ（待機・GDI 呼び出し・ロックなし）。UI スレッド側は既存の同一ウィンドウの内容を差し替えて再表示するため、連続した通知はキューせず最新のものに上書きされ、タイマーもリセットされる。

### 2. バルーン通知経路の削除（`crates/vpm-tray/src/tray.rs`）
- `TrayIconWindow` に `notification: Option<NotificationWindow>` を追加し、生成失敗時は `None` のまま通知なしで継続動作する。
- `show_balloon`（`NIF_INFO`/`NIIF_INFO` + `NIM_MODIFY`）を廃止し、`show_notification(title, message)` に置き換え。

### 3. 呼び出し側のリネーム（`crates/vpm-tray/src/main.rs`）
- `tray.show_balloon(...)` を `tray.show_notification(...)` に変更。通知文言・タイミング・ログ出力は変更なし。

## 影響範囲
- **修正コンポーネント**:
  - `crates/vpm-tray/src/notification.rs`: 新規（独自ポップアップ実装）
  - `crates/vpm-tray/src/tray.rs`: 通知フィールド追加、`show_notification` への置き換え、バルーン経路削除
  - `crates/vpm-tray/src/main.rs`: 呼び出しリネーム
  - `crates/vpm-tray/src/lib.rs`: `pub mod notification` 追加
  - `crates/vpm-tray/tests/tray_tests.rs`: `compute_popup_rect` のテスト追加
  - `crates/vpm-platform`（`audio.rs` 等）、`Cargo.toml`/`Cargo.lock`: 変更なし
- **ユーザーへの影響**:
  - 通知が OS キュー待ちせず即時表示される。表示トリガー・文言は従来どおり。
  - 見た目が OS 標準バルーンから簡素な白背景ポップアップに変わる。
  - 通知センターの履歴に残らなくなる。

## 検証結果
- 2026-10-17: `cargo fmt --all -- --check` パス。
- 2026-10-17: `cargo clippy --workspace --all-targets --locked -- -D warnings` パス（警告ゼロ）。
- 2026-10-17: `cargo test --workspace --locked` で vpm-platform 44 件・vpm-tray 5 件（新規 `compute_popup_rect` テスト 3 件を含む）・vpm-core 7 件合格。vpm-core `compatibility` テスト 4 件は検証環境に .NET SDK がないため未実行（本変更とは無関係）。
- 2026-10-17: `cargo build --release --locked` 成功（`VolumeProfileManager_TrayApp.exe` 約 708KB）。
- 実機確認: （ユーザー確認後に追記）

## 備考
- **採用理由**: 遅延の原因は OS 側の通知キューであるため、アプリ側からの設定変更では解消できず、自前描画以外に手段がない。WinRT トースト通知は AppUserModelID の登録が必要で、通知センターのキューと集中モードの影響を同様に受けるため不採用。外部クレートによる実装も依存追加に加え内部経路は同じため不採用。
- **対象外**: ダークモード対応、マルチモニタ追従、フェード演出などの視覚的改善。
- `audio.rs` のロック保持中にコールバックを呼ぶ構造は本 Issue では変更しない。構造の見直しは Issue #28 に切り出す。
