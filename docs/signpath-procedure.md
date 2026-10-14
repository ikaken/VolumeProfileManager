# コード署名対応 手順書（Issue #30 / SignPath Foundation）

GitHub Releases で配布するインストーラー版・ポータブル版に SignPath Foundation の無料コード署名を付与し、Smart App Control (SAC) 有効環境で実行できるようにするための手順書。

- 凡例: **【あなた】** = リポジトリオーナーの手動作業（アカウント・申請・リポジトリ設定など、エージェントが代行できないもの） / **【エージェント】** = コード・ワークフロー・ドキュメントの変更作業
- 進行は 2 フェーズ。フェーズ A はこの PR で完了。フェーズ B は SignPath の審査承認後に別 PR で実施する。

---

## フェーズ A: 申請の前提条件を整える（本 PR）

### 【エージェント】
- [x] `LICENSE`（MIT）を追加
- [x] `Cargo.toml`（ワークスペース）に `license = "MIT"`、各クレートに `license.workspace = true`
- [x] README に SAC 有効環境での制限事項の暫定注記と「ライセンス」節を追加
- [x] `docs/changes/030-signpath-code-signing.md`（設計書）を作成
- [x] 本手順書を作成
- [ ] PR を作成する（ユーザーの指示後）

### 【あなた】
- [ ] PR の内容を確認し、`main` へマージする
- [ ] GitHub アカウントで 2 要素認証（MFA）を有効にする（SignPath Foundation の必須条件）
  - GitHub → Settings → Password and authentication → Two-factor authentication

---

## フェーズ A-2: SignPath Foundation への申請

### 【あなた】
- [ ] https://signpath.org/apply を開き、申請フォームを送信する。入力内容の目安:
  - プロジェクト名: `VolumeProfileManager`
  - リポジトリ URL: `https://github.com/ikaken/VolumeProfileManager`
  - ダウンロードページ URL: `https://github.com/ikaken/VolumeProfileManager/releases`
  - ライセンス: MIT
  - 説明（英語）: 例 `Lightweight Windows tray app that saves per-audio-device volume/mute profiles and applies them automatically when the default device changes. Built with Rust + windows-rs, distributed as Inno Setup installer and portable ZIP via GitHub Releases.`
  - ビルドシステム: GitHub Actions（`.github/workflows/release.yml`）
  - 署名したい成果物: `VolumeProfileManager.TrayApp.exe`、`VolumeProfileManagerSetup.exe`（Inno Setup）、アンインストーラー `unins000.exe`
- [ ] SignPath からの返信メールを待つ（数日〜数週間）。追加質問があれば回答する
- [ ] 承認されたら、案内に従って SignPath.io（https://app.signpath.io）のアカウントを作成し、MFA を有効にする
- [ ] 承認結果（Organization 名・Organization ID・証明書の種類）をエージェントに共有する

---

## フェーズ B-1: SignPath 側の設定（承認後）

> SignPath.io の Web UI での操作。エージェントはアカウントにアクセスできないため、すべて【あなた】の作業になる。具体的な設定値はエージェントが事前に提示する。

### 【エージェント】
- [ ] アーティファクト構成 XML（PE 単体の Authenticode 署名用）を用意して提示する
- [ ] 署名ポリシーの推奨設定（ブランチ / タグ制限）を提示する

### 【あなた】
- [ ] SignPath.io で **プロジェクト** を作成する
  - Repository URL: `https://github.com/ikaken/VolumeProfileManager`
  - Trusted Build System: `GitHub.com` をリンクする
- [ ] **アーティファクト構成**を追加する（エージェント提示の XML を貼り付け）
- [ ] **署名ポリシー**を追加する
  - `release-signing`: Origin Verification を有効、タグ `v*` からのビルドに制限
  - （任意）`test-signing`: 動作確認用
- [ ] **API トークン**を発行する（署名ポリシーに対して Submitter 権限を持つユーザー）
- [ ] GitHub リポジトリに値を登録する（Settings → Secrets and variables → Actions）
  - Secrets: `SIGNPATH_API_TOKEN`（API トークン。**絶対にコードやチャットに貼らない**）
  - Variables: `SIGNPATH_ORGANIZATION_ID`、`SIGNPATH_PROJECT_SLUG`、`SIGNPATH_SIGNING_POLICY_SLUG`
- [ ] 登録した Variables の値（Secrets 以外）をエージェントに共有する

---

## フェーズ B-2: ワークフロー・インストーラーの改修（承認後・別 PR）

### 【エージェント】
- [ ] `feature/030-signpath-code-signing-b`（仮）ブランチを作成し、設計方針を提示 → 承認を得る
- [ ] `.github/workflows/release.yml` を改修する
  - ビルド直後の `VolumeProfileManager.TrayApp.exe` を `actions/upload-artifact` → `signpath/github-action-submit-signing-request` で署名し、署名済み exe を `publish\TrayApp` へ配置
  - Inno Setup ビルド後の `VolumeProfileManagerSetup.exe` を同様に署名
  - `signtool verify /pa` で署名を検証し、未署名の成果物がリリースされないようにする
- [ ] `installer/VolumeProfileManager.iss` に `SignTool=` ディレクティブを追加し、アンインストーラー stub を SignPath で署名する
- [ ] README の暫定注記を削除し、SignPath Foundation の謝辞と「署名者は SignPath Foundation と表示される」旨を追記する
- [ ] `docs/changes/030-signpath-code-signing.md` を更新する
- [ ] PR を作成する（ユーザーの指示後）

### 【あなた】
- [ ] 設計方針を確認・承認する
- [ ] PR の内容を確認し、`main` へマージする

---

## フェーズ B-3: リリースと実機確認

### 【あなた】
- [ ] ベータタグ（例 `v2.1.0-beta.1`）を push してリリースワークフローを実行する（手順は `release-launcher` スキル参照）
- [ ] SignPath 側で署名リクエストの承認が必要な設定の場合、SignPath.io の Web UI で承認する（リリースごとに最大 3 件: 本体 exe / アンインストーラー / Setup.exe）
- [ ] GitHub Release に添付された成果物をダウンロードし、以下を確認する
  - [ ] ファイルのプロパティ → デジタル署名 タブに署名者「SignPath Foundation」が表示される（Setup.exe、ZIP 内の exe）
  - [ ] タイムスタンプが付いている
  - [ ] Windows Defender で誤検出されない
- [ ] SAC 有効（強制モード）の Windows 11 環境で動作確認する
  - [ ] インストーラーが実行できる
  - [ ] アプリが起動し、デバイス切り替え時の音量制御が動作する
  - [ ] アンインストールが実行できる（`unins000.exe` がブロックされない）
  - [ ] ポータブル版が起動する
- [ ] 問題なければ正式タグで再リリースする

### 【エージェント】
- [ ] 実機確認で問題があれば原因を調査し修正する
- [ ] 完了後、Issue #30 をクローズする（`main` マージ後）

---

## 補足

- **SAC の確認方法**: Windows セキュリティ → アプリとブラウザー コントロール → スマート アプリ コントロールの設定。「オン」= 強制モード、「評価」= 評価モード（ブロックしない）。評価モードから一度「オフ」にすると Windows 再インストールまで再有効化できないため注意
- **署名後も SmartScreen の警告は残り得る**: SmartScreen の評判はダウンロード実績で蓄積されるため、署名直後は「WindowsによってPCが保護されました」が表示されることがある。SAC のブロックとは別物で、ユーザーが「詳細情報 → 実行」で回避できる
- **Microsoft Store 配布（#29）との関係**: 本手順は GitHub Releases 版の対応。Store 版は MSIX 化が必要で別 Issue として進める

## 参考リンク
- [SignPath Foundation](https://signpath.org/) / [利用条件](https://signpath.org/terms.html) / [申請フォーム](https://signpath.org/apply)
- [SignPath Docs: GitHub Actions 連携](https://docs.signpath.io/trusted-build-systems/github)
- [SignPath Docs: アーティファクト構成リファレンス](https://docs.signpath.io/artifact-configuration/reference)
- [Inno Setup: SignTool ディレクティブ](https://jrsoftware.org/ishelp/index.php?topic=setup_signtool)
- [Smart App Control 概要（Microsoft Learn）](https://learn.microsoft.com/windows/apps/develop/smart-app-control/overview)
