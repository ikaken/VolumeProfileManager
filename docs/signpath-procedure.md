# コード署名対応 手順書（Issue #30 / SignPath Foundation）

GitHub Releases で配布するインストーラー版・ポータブル版に SignPath Foundation の無料コード署名を付与し、Smart App Control (SAC) 有効環境で実行できるようにするための手順書。

- 凡例: **【あなた】** = リポジトリオーナーの手動作業（アカウント・申請・リポジトリ設定など、エージェントが代行できないもの） / **【エージェント】** = コード・ワークフロー・ドキュメントの変更作業
- 進行は 2 フェーズ。フェーズ A は PR #31 で完了。フェーズ B は SignPath の審査承認後に別 PR で実施する。
- 本手順書は進捗に応じて随時更新する（チェック済み項目は `[x]`）。

---

## フェーズ A: 申請の前提条件を整える（PR #31・完了）

### 【エージェント】
- [x] `LICENSE`（MIT）を追加
- [x] `Cargo.toml`（ワークスペース）に `license = "MIT"`、各クレートに `license.workspace = true`
- [x] README に SAC 有効環境での制限事項の暫定注記と「ライセンス」節を追加
- [x] `docs/changes/030-signpath-code-signing.md`（設計書）を作成
- [x] 本手順書を作成
- [x] PR #31 を作成する

### 【あなた】
- [x] PR #31 の内容を確認し、`main` へマージする
- [x] GitHub アカウントで 2 要素認証（MFA）を有効にする（SignPath Foundation の必須条件）
  - GitHub → Settings → Password and authentication → Two-factor authentication
  - 確認済み: Enabled（認証アプリ + GitHub Mobile 構成済み）

---

## フェーズ A-2: SignPath Foundation への申請

### 【あなた】
- [x] https://signpath.org/apply を開き、申請フォームを送信する（**2026-10-14 送信済み**。Privacy Policy URL は空欄で送信可能だった）。各項目の入力例は以下のとおり（フォームの項目名・構成はサイト側で変わることがあるため、画面に合わせて読み替える）

  | フォーム項目 | 入力例 |
  |---|---|
  | Project Name | `VolumeProfileManager` |
  | Repository URL | `https://github.com/ikaken/VolumeProfileManager` |
  | Homepage URL | `https://github.com/ikaken/VolumeProfileManager`（専用サイトがないため README を指す） |
  | Download URL | `https://github.com/ikaken/VolumeProfileManager/releases` |
  | Privacy Policy URL | 必須なら `https://github.com/ikaken/VolumeProfileManager/blob/main/PRIVACY.md`（下記「プライバシーポリシーについて」参照）。任意なら空欄 |
  | Wikipedia URL（任意） | 空欄 |
  | Tagline | `Per-device volume profiles for Windows, applied automatically on audio device switch` |
  | Category | `Utilities` / `System tools` / `Audio` など選択肢の中で最も近いもの |
  | Description | `Lightweight Windows tray application that saves volume and mute state per audio output device and automatically restores the saved profile when the default device changes (e.g. switching between headphones and speakers). Built with Rust and windows-rs using the Win32 Core Audio API. Distributed as an Inno Setup installer and a portable ZIP via GitHub Releases, built entirely by GitHub Actions.` |
  | Expectations（署名が必要な理由を問う項目がある場合） | `Our unsigned installer and executable are blocked by Windows 11 Smart App Control and flagged by SmartScreen. We want to sign the main executable, the Inno Setup installer and its uninstaller so users can install and run the app on protected devices.` |
  | Reputation（必須） | 下記「Reputation 項目について」参照。**事実のみを記載する** |
  | Maintainer Type | `Individual` |
  | Build System | `GitHub Actions` |
  | First Name / Last Name | 本人の氏名 |
  | Email | 連絡用メールアドレス（承認通知が届く） |
  | Company Name | 空欄（個人のため） |
  | Primary Discovery Channel | SignPath を知った経路（`Search engine` / `Microsoft documentation` など） |
  | Please specify the exact source（任意） | 空欄可 |
  | チェックボックス | SignPath 側プライバシーポリシーへの同意は必須。ニュースレター等は任意 |

  **プライバシーポリシーについて**: 本アプリは外部通信・テレメトリ・個人情報収集を一切行わないため、現状ポリシーページがない。Privacy Policy URL が必須項目の場合は、エージェントに依頼して `PRIVACY.md`（データ収集なし・ネットワーク通信なし・プロファイルはローカル保存のみ）を `main` に追加してから URL を指定する。

  **Reputation 項目について**: 「広く使われている / 信頼されている」ことを示す情報（メディア掲載・ブログ・ダウンロード統計・GitHub Insights・コミュニティ議論）を求められる。2026-10-14 時点の実績は以下のとおりで、審査基準に対して弱い。**虚偽・誇張は規約違反**のため事実のみ記載し、却下された場合は実績を積んで再申請する（方針 C）。

  | 指標 | 値（2026-10-14） |
  |---|---|
  | リポジトリ作成 | 2026-08-06 |
  | Star / Fork / Watcher | 1 / 0 / 0 |
  | リリース数 | 10（正式 2・ベータ 8） |
  | 累計ダウンロード | 約 23 件 |
  | メディア掲載・ブログ・コミュニティ議論 | なし |

  記入例（英語・申請時点の数値に更新すること）:
  ```
  This is a young project (repository created August 2026) with a small user base so far:
  - 10 releases on GitHub (2 stable, 8 beta), ~20 cumulative downloads
  - https://github.com/ikaken/VolumeProfileManager/releases
  - Fully automated, reproducible builds via GitHub Actions (.github/workflows/release.yml)
  - 63 automated tests, documented specs and change history in docs/
  We are applying early because the unsigned installer is blocked by Windows 11 Smart App Control,
  which prevents new users from trying the app at all. We understand reputation is limited and are
  happy to re-apply later if the project does not yet meet the Foundation's criteria.
  ```
  最新の数値は次のコマンドで取得できる: `gh api repos/ikaken/VolumeProfileManager --jq '{stars:.stargazers_count,forks:.forks_count}'` / `gh api repos/ikaken/VolumeProfileManager/releases --jq '[.[].assets[].download_count] | add'`
- [ ] SignPath からの返信メールを待つ（数日〜数週間）。追加質問があれば回答する
- [ ] **却下された場合**: 却下理由を記録し、下記「フェーズ A-3: 実績づくり」を進めたうえで再申請する
- [ ] 承認されたら、案内に従って SignPath.io（https://app.signpath.io）のアカウントを作成し、MFA を有効にする
- [ ] 承認結果（Organization 名・Organization ID・証明書の種類）をエージェントに共有する

---

## フェーズ A-3: 実績づくり（申請と並行して進める・方針 C）

SignPath の審査結果に関わらず、プロジェクトの露出と利用実績を増やす。SAC ブロックの根本解決としては実績に依存しない Microsoft Store 配布（#29）の優先度も高い。

### 【エージェント】
- [ ] README の英語版（`README.en.md`）を作成し、英語圏からの到達性を上げる
- [ ] Microsoft Store 配布（#29）の設計に着手する（Store 署名により SAC 問題を実績に依存せず解決できる）
- [ ] winget（`microsoft/winget-pkgs`）へのマニフェスト登録手順を調査する（署名済み成果物が前提になる可能性があるため、SignPath または Store 対応後）
- [ ] 技術記事の下書き支援（Rust + windows-rs でのトレイアプリ実装、Core Audio API 連携など）

### 【あなた】
- [ ] GitHub リポジトリの About に Description / Topics（`windows`, `rust`, `audio`, `volume`, `tray`, `win32` など）を設定する
- [ ] 技術記事（Zenn / Qiita / dev.to 等）を公開し、リポジトリへリンクする
- [ ] Microsoft Store 配布（#29）の設計レビュー・開発者アカウント登録（無料）
- [ ] 一定期間後（目安: 1〜3 か月）に Star / ダウンロード数を再確認し、再申請の判断をする

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
