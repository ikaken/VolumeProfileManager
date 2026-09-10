# Issue #19: トレイアイコンメニューにバージョンが正常に表示されない

## 背景 / 目的

`v2.1.0` のタグでリリースしても、アプリ起動中のトレイメニューが `VolumeProfileManager v2.0.0-beta` のまま表示される。

原因は、タグ作成時に `Cargo.toml` のワークスペースバージョンが更新されず、`env!("CARGO_PKG_VERSION")` が古い値を表示していたためである。

## 採用する方針

`Cargo.toml` のワークスペース `version` を、アプリケーションバージョンの唯一の情報源とする。アプリ側は標準の `env!("CARGO_PKG_VERSION")` を引き続き使用する。

リリース時には、タグの `v` を除いた値と `Cargo.toml` のバージョンが一致していることを GitHub Actions で検証する。一致しない場合は、ビルド・リリースを停止する。

例:

```text
Cargo.toml: 2.1.1-beta
タグ:       v2.1.1-beta
トレイ表示: VolumeProfileManager v2.1.1-beta
```

## 変更内容

### 1. バージョン表示のテストを汎用化

`crates/vpm-tray/tests/tray_tests.rs` の固定値 `2.0.0-beta` 比較を廃止し、以下を検証する。

- `APP_VERSION` が空でないこと
- `version_string()` が `v` + `APP_VERSION` であること

これにより、バージョン更新のたびにテストコードを変更する必要をなくす。

### 2. GitHub Actionsにリリース前バージョン検証を追加

`.github/workflows/release.yml` のRustセットアップ後に、次を検証する。

- `github.ref_name` の先頭 `v` を除いた値を取得
- `cargo metadata` からワークスペースパッケージのバージョンを取得
- すべてのワークスペースパッケージが同じバージョンであることを確認
- タグのバージョンとCargoのバージョンが一致することを確認
- 不一致時はエラーで停止

### 3. リリース運用

今後のリリースでは、タグを作成する前に `Cargo.toml` のバージョンを対象バージョンへ更新してコミットする。タグ作成後はCIの一致検証を通過した成果物だけを公開する。

## 影響範囲

- `crates/vpm-tray/tests/tray_tests.rs`
- `.github/workflows/release.yml`
- `Cargo.toml` は各リリース準備時に対象バージョンへ更新する

## 検証項目

- [ ] `cargo test --workspace --locked` が全件通る
- [ ] `APP_VERSION` が空でないことをテストできる
- [ ] `version_string()` が `v{APP_VERSION}` を返す
- [ ] タグとCargoバージョンが一致する場合、CI検証が成功する
- [ ] タグとCargoバージョンが不一致の場合、CI検証が失敗する
- [ ] リリース成果物のトレイメニューにCargoのバージョンが表示される

## 対象外

- タグ名をビルド時に環境変数で注入する方式
- `Cargo.toml` のバージョンをタグから自動書き換えする処理
- Windowsのファイルバージョンリソース表示の変更
- トレイメニューUIデザインの変更
