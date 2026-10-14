# GitHub Releases 配布物のコード署名対応（SignPath Foundation）

## 背景 / 目的
Issue #30。現在配布している `VolumeProfileManagerSetup.exe` / `VolumeProfileManager.TrayApp.exe` は未署名のため、Windows 11 の Smart App Control (SAC) 有効環境では実行がブロックされる。SAC はユーザー側でアプリ単位の例外設定ができず、「Microsoft 信頼されたルートプログラム参加 CA の証明書で署名されている」ことが実質的な必須条件となる。

OSS プロジェクト向けに無料でコード署名を提供する SignPath Foundation を利用し、GitHub Actions のリリースフローで成果物を自動署名する。Microsoft Store 配布（#29）とは併用する。

本 Issue は SignPath Foundation の審査承認（外部要因・数日〜数週間）を挟むため、2 フェーズに分けて進める。

- **フェーズ A（本 PR）**: SignPath 申請の前提条件となるライセンス整備と、README への暫定注記
- **フェーズ B（承認後・別 PR）**: `release.yml` / `.iss` への署名ステップ組み込みと README 謝辞

## 変更内容

### フェーズ A（本 PR）
- `LICENSE` を新規追加（MIT License 全文、著作権表記は `Copyright (c) 2025 ikaken`）
- ワークスペース `Cargo.toml` の `[workspace.package]` に `license = "MIT"` を追加し、`crates/vpm-core` / `crates/vpm-platform` / `crates/vpm-tray` の各 `Cargo.toml` に `license.workspace = true` を追加
- `README.md` に以下を追加
  - 「ダウンロード」節の直後に、Smart App Control 有効環境では現状実行できない旨の注記（コード署名対応は #30 で進行中であることを明記。フェーズ B 完了時に削除）
  - 末尾に「ライセンス」節（MIT）

### フェーズ B（SignPath 承認後・別 PR で対応）
- `.github/workflows/release.yml`
  - ビルド直後の `VolumeProfileManager.TrayApp.exe` を `actions/upload-artifact` → `signpath/github-action-submit-signing-request` で署名し、署名済み exe を `publish\TrayApp` へ配置
  - Inno Setup ビルド後の `VolumeProfileManagerSetup.exe` を同様に署名
  - `signtool verify /pa` による検証ステップを追加し、未署名成果物のリリースを防止
- `installer/VolumeProfileManager.iss`
  - `SignTool=` ディレクティブからアンインストーラー stub（`unins000.exe`）を SignPath で署名（省略すると SAC 環境でアンインストールがブロックされる）
- `README.md`
  - SignPath Foundation の謝辞（利用条件で必須）と、Publisher 表示が「SignPath Foundation」になる旨を記載
  - フェーズ A の暫定注記を削除
- GitHub Secrets `SIGNPATH_API_TOKEN` / Variables `SIGNPATH_ORGANIZATION_ID` 等の登録（リポジトリ設定・手動）

## 影響範囲
- 新規: `LICENSE`
- 変更: `Cargo.toml`（ワークスペース）、`crates/*/Cargo.toml`（3 ファイル）、`README.md`
- コード（`src/`, `crates/*/src/`）への変更なし。ビルド成果物・動作への影響なし
- `Cargo.lock` への影響なし（メタデータのみの変更）
- ユーザーへの影響: README に SAC 環境での制限事項が明示される。配布物自体はフェーズ B まで変化なし

## 備考
- ライセンスは Issue #30 でユーザー指定の **MIT** を採用。SignPath Foundation の条件「OSI 承認ライセンス・商用デュアルライセンスなし」を満たす
- `src/`, `tests/` の V1（.NET 版）レガシーコードも同一リポジトリに含まれるため、`LICENSE` はリポジトリ全体に適用される
- 依存クレート（windows-rs: MIT/Apache-2.0、serde: MIT/Apache-2.0、chrono: MIT/Apache-2.0、serde_json: MIT/Apache-2.0）はいずれも MIT と互換
- SignPath への申請（signpath.org/apply）と GitHub アカウントの MFA 有効化はリポジトリオーナーの手動作業。本 PR マージ後に申請可能になる
- フェーズ B の署名リクエストは 1 リリースあたり 3 件（本体 exe・アンインストーラー stub・Setup.exe）。SignPath の Inno Setup 深い署名は非対応のため、PE 単体署名を 3 回行う構成とする
- SmartScreen の評判獲得は本 Issue の対象外（配布実績に応じて自然蓄積）
