# VolumeProfileManager テスト仕様書

## 文書の対象

既存の第 1～5 章とテスト実行計画は V1 開発時の計画・例を含む。廃止済み CLI の例も履歴として残しており、V2 の検証手順には使用しない。V2 は [第 6 章の検証計画](#v2-tests) を参照する。V2 は設計承認済み・実装中。各ケースの実行状況は [V2 検証実績](task_list.md#v2-tasks) に記録し、未実行のケースを合格扱いにしない。

---

## テスト実行手順

- ローカルでのテスト実行 (全テスト):

```powershell
cd c:\work\VolumeProfileManager
dotnet test
```

- 単体テストのみを実行する場合:

```powershell
dotnet test tests\VolumeProfileManager.UnitTests\VolumeProfileManager.UnitTests.csproj
```

- カバレッジレポートを生成する場合 (Coverlet):

```powershell
dotnet test /p:CollectCoverage=true /p:CoverletOutputFormat=opencover
```


## 目次

1. [テスト戦略](#1-テスト戦略)
2. [ユニットテスト](#2-ユニットテスト)
3. [統合テスト](#3-統合テスト)
4. [E2E テスト](#4-e2e-テスト)
5. [非機能テスト](#5-非機能テスト)
6. [V2 再構築検証計画（Issue #16）](#v2-tests)

---

## 1. テスト戦略

### 1.1 テストピラミッド

```
            E2E
       統合テスト
      ユニットテスト
```

- **ユニットテスト**: 60%
- **統合テスト**: 30%
- **E2E テスト**: 10%
- **カバレッジ目標**: 80% 以上

### 1.2 テスト環境

- **フレームワーク**: xUnit + Moq
- **実行**: `dotnet test`
- **CI/CD**: GitHub Actions で自動実行
- **アーティファクト**: カバレッジレポート (Coverlet)

---

## 2. ユニットテスト

### 2.1 DeviceMonitorService

| テストケース | 入力 | 期待値 | 備考 |
|-------------|------|--------|------|
| デバイス変更イベント発火 | `OnDefaultDeviceChanged()` | `DeviceChanged` イベント発火 | MockNotificationClient 使用 |
| 複数デバイス取得 | `GetAvailableDevicesAsync()` | 2 個以上のデバイス | MockMMDeviceEnumerator |
| 既定デバイス取得 | `GetCurrentDefaultDeviceAsync()` | デバイス情報 | null でない |
| イベント登録・解除 | イベント登録 → 解除 | イベント未発火 | 登録後解除のテスト |

テストコード例:

```csharp
[Fact]
public async Task GetCurrentDefaultDeviceAsync_ReturnsDefault()
{
    // Arrange
    var mockEnumerator = new Mock<IMMDeviceEnumerator>();
    var mockDevice = new Mock<IMMDevice>();
    
    mockEnumerator
        .Setup(x => x.GetDefaultAudioEndpoint(DataFlow.Render, Role.Multimedia))
        .Returns(mockDevice.Object);
    
    var service = new DeviceMonitorService(mockEnumerator.Object, _logger);
    
    // Act
    var result = await service.GetCurrentDefaultDeviceAsync();
    
    // Assert
    Assert.NotNull(result);
    Assert.Equal("expected-device-id", result.DeviceId);
}
```

### 2.2 ProfileService

| テストケース | 入力 | 期待値 | 備考 |
|-------------|------|--------|------|
| プロファイル保存 | `VolumeProfile` オブジェクト | `profiles.json` に追記 | リポジトリ Mock |
| プロファイル取得 | `deviceIdentifier` | 対応プロファイル | null でない |
| プロファイル取得（未存在） | 存在しない `deviceId` | `null` | 例外なし |
| プロファイル削除 | `deviceId` | ファイルから削除 | リポジトリ Mock |
| 全プロファイル取得 | なし | プロファイル配列 | 複数件 |

テストコード例:

```csharp
[Fact]
public async Task SaveProfileAsync_SavesProfile()
{
    // Arrange
    var mockRepository = new Mock<IProfileRepository>();
    var profile = new VolumeProfile 
    { 
        DeviceId = "test-id", 
        DeviceName = "Test Device",
        MasterVolume = 0.5f
    };
    
    var service = new ProfileService(mockRepository.Object, _logger);
    
    // Act
    await service.SaveProfileAsync(profile);
    
    // Assert
    mockRepository.Verify(x => x.SaveAsync(profile), Times.Once);
}
```

### 2.3 AudioVolumeService

| テストケース | 入力 | 期待値 | 備考 |
|-------------|------|--------|------|
| マスターボリューム取得 | なし | 0.0 ~ 1.0 の値 | NAudio Mock |
| マスターボリューム設定 | `0.75` | NAudio に適用 | Mock で確認 |
| ミュート状態取得 | なし | true/false | NAudio Mock |
| ミュート状態設定 | `true` | NAudio に適用 | Mock で確認 |
| 無効な値設定 | `1.5` | 例外または値の正規化 | クリッピング処理 |

### 2.4 ProfileRepository

| テストケース | 入力 | 期待値 | 備考 |
|-------------|------|--------|------|
| ファイルなし時の初期化 | なし | `profiles.json` 作成 | アプリ起動時 |
| JSON 破損時の復帰 | 破損した `profiles.json` | バックアップから復帰 | `.json.bak` 使用 |
| 再試行ロジック | API エラー | 最大 5 回までリトライ | 遅延確認 |
| 並行アクセス制御 | 複数スレッド | ファイルロック | 排他制御 |

---

## 3. 統合テスト

### 3.1 デバイス切り替え統合テスト

```csharp
[Fact]
public async Task DeviceSwitch_RestoresProfile()
{
    // Arrange
    var monitorService = _serviceProvider.GetRequiredService<IDeviceMonitorService>();
    var profileService = _serviceProvider.GetRequiredService<IProfileService>();
    var volumeService = _serviceProvider.GetRequiredService<IAudioVolumeService>();
    
    // デバイスとプロファイルを準備
    var deviceId = "device-1";
    var profile = new VolumeProfile { DeviceId = deviceId, MasterVolume = 0.75f };
    await profileService.SaveProfileAsync(profile);
    
    // Act
    // デバイス変更イベントをシミュレート
    // monitorService.RaiseDeviceChanged(...);
    
    // Assert
    var appliedVolume = await volumeService.GetMasterVolumeAsync();
    Assert.Equal(0.75f, appliedVolume, 0.01f);
}
```

### 3.2 プロファイル CRUD 統合テスト

```csharp
[Fact]
public async Task ProfileCRUD_Success()
{
    var profileService = _serviceProvider.GetRequiredService<IProfileService>();
    
    // Create
    var profile = new VolumeProfile { DeviceId = "test", DeviceName = "Test" };
    await profileService.SaveProfileAsync(profile);
    
    // Read
    var retrieved = await profileService.GetProfileAsync("test");
    Assert.NotNull(retrieved);
    Assert.Equal("Test", retrieved.DeviceName);
    
    // Update
    profile.MasterVolume = 0.8f;
    await profileService.SaveProfileAsync(profile);
    
    var updated = await profileService.GetProfileAsync("test");
    Assert.Equal(0.8f, updated.MasterVolume);
    
    // Delete
    await profileService.DeleteProfileAsync("test");
    var deleted = await profileService.GetProfileAsync("test");
    Assert.Null(deleted);
}
```

### 3.3 エラーハンドリング統合テスト

| テストケース | シナリオ | 期待値 |
|-------------|---------|--------|
| Core Audio API 一時的エラー | `COMException` 発生 → 復帰 | 再試行後、正常終了 |
| ファイル I/O エラー | `IOException` 発生 | 再試行、バックアップから復帰 |
| JSON 破損 | `JsonException` 発生 | バックアップファイルから復帰 |
| 複数エラー | 連続エラー | ログ記録、アプリ継続 |

---

## 4. E2E テスト

### 4.1 CLI コマンドテスト

| テストケース | コマンド | 期待値 |
|-------------|---------|--------|
| デバイス一覧表示 | `vpm list-devices` | 接続デバイスを表示 |
| ステータス確認 | `vpm status` | 現在のデバイス・音量を表示 |
| プロファイル保存 | `vpm save-profile device-0` | プロファイル保存、確認 |
| プロファイル削除 | `vpm delete-profile device-0` | プロファイル削除、確認 |
| 常駐実行 | `vpm run` | バックグラウンド実行開始 |
| 実行停止 | `vpm stop` | バックグラウンド実行停止 |

テスト実行例:

```bash
# デバイス一覧表示テスト
$ vpm list-devices
# Output: Device 0: Speaker [DEFAULT], Device 1: Headphones

# プロファイル保存テスト
$ vpm save-profile 0
$ vpm status
# Output: Profile Saved: Yes

# プロファイル削除テスト
$ vpm delete-profile 0
$ vpm status
# Output: Profile Saved: No
```

### 4.2 デバイス切り替えシナリオテスト

1. デバイス A をプロファイル作成（音量 70%）
2. デバイス B に切り替え
3. デバイス B のプロファイル作成（音量 50%）
4. デバイス A に戻す
5. デバイス A の音量が 70% に復元されることを確認

---

## 5. 非機能テスト

### 5.1 パフォーマンステスト

| 項目 | 基準 | 測定方法 |
|-----|------|--------|
| デバイス変更検知応答時間 | 100ms 以内 | Stopwatch で測定 |
| 音量調整応答時間 | 200ms 以内 | Stopwatch で測定 |
| メモリ使用量 | 30MB 以下 | Process.WorkingSet64 |
| CPU 使用率 | 1% 以下（アイドル時） | パフォーマンスカウンタ |

テストコード例:

```csharp
[Fact]
public async Task DeviceChange_RespondWithin100ms()
{
    var stopwatch = Stopwatch.StartNew();
    
    await monitorService.RaiseDeviceChangedAsync();
    
    stopwatch.Stop();
    Assert.True(stopwatch.ElapsedMilliseconds <= 100);
}
```

### 5.2 負荷テスト

| シナリオ | 負荷 | 期待値 |
|---------|------|--------|
| 連続デバイス切り替え | 100 回/分 | エラーなし、応答遅延なし |
| 大量プロファイル | 1,000 件 | 読み込み/検索時間 < 500ms |
| ファイル破損復帰 | 10 回連続破損 | 毎回バックアップから復帰 |

### 5.3 ストレステスト

- **メモリリーク**: 24 時間連続実行でメモリ増加なし
- **ハンドルリーク**: 24 時間連続実行でハンドル増加なし
- **スレッドセーフ**: 複数スレッドでの並行アクセスでデータ破損なし

---

## テスト実行計画

### Phase 1: ユニットテスト
- 開発フェーズ 1～3 完了時点で実施
- 各サービスクラスのテストケース 30+ 件

### Phase 2: 統合テスト
- 開発フェーズ 5 完了時点で実施
- エンドツーエンドのデータフロー確認

### Phase 3: E2E テスト
- 開発フェーズ 7 完了時点で実施
- CLI コマンド、実運用シナリオ確認

### Phase 4: 非機能テスト
- リリース前に実施
- パフォーマンス、負荷、ストレステスト

---

<a id="v2-tests"></a>

## 6. V2 再構築検証計画（Issue #16・未実行）

要件は [V2 仕様](VolumeProfileManager_spec.md#v2-spec)、実装方式は [V2 設計](design.md#v2-design) に従う。設計の文書反映はテスト合格や実装承認を意味しない。

### 6.1 自動テスト

Windows API・保存先・時計を差し替え可能にする。合成データと一時ディレクトリを使用し、実ユーザーのプロファイルやスタートアップ登録を単体テストで変更しない。

| ID | ケース | 期待結果 |
|---|---|---|
| V2-D01 | V1 生成 JSON → V2 読込/保存 → V1 読込 | PascalCase の 6 項目・配列形式・値が互換。追加の必須キーなし |
| V2-D02 | UTC/秒以下/未設定日時、日本語デバイス名、音量境界値 | 日時と文字列を読み戻せ、float と互換な値を保持 |
| V2-D03 | 既存 ID の保存、名前/ID による削除 | `CreatedAt` を保持し、V1 と同じ更新・削除対象になる |
| V2-D04 | 本体 JSON 破損、正常なバックアップあり | 検証済みバックアップを再読込し、復旧データを空リストで上書きしない |
| V2-D05 | 本体/バックアップ両方破損、読込・書込失敗 | 自動作成/上書きを中断し、エラーを報告。保存成功通知なし |
| V2-D06 | 移行退避作成・通常保存・保存中断 | 退避を上書きせず、正常な元データを保持。初回でファイルなしの場合も扱える |
| V2-M01 | ID 一致、正規化名一致、双方向包含、同順位候補 | ID → 名前完全一致 → 部分一致。日時降順、同順位は元順序、ID は先頭 |
| V2-M02 | 大小文字、Unicode、連続半角スペース、前後空白、空名 | .NET `OrdinalIgnoreCase` と V1 の正規化・非一致条件を再現 |
| V2-E01 | 既定変更と Role/Flow 違い、起動のみ | Render/Multimedia の既定変更だけ即時処理。起動のみでは適用しない |
| V2-E02 | イベント連発、800ms 静定、即時適用失敗 | 最新イベントから静定を計測し、失敗は検証パスで再試行 |
| V2-E03 | 同一デバイスで 3 秒未満/境界、異なるデバイス | 成功済み同一デバイスのみ抑制し、切替を取りこぼさない |
| V2-E04 | 未登録/登録済み、デバイスなし、音量取得失敗 | 正常時だけ保存/通知。取得失敗を 0/false として保存しない。自動適用で `LastApplied` を変更しない |
| V2-E05 | 処理中の連続切替と終了要求 | 適用は直列化し、同じ取得済みエンドポイントで音量/ミュートを扱う。終了後の遅延適用なし |
| V2-U01 | 日本語メニュー、通知、バージョン、スタートアップ状態 | 既存操作と表示を維持。ベータ接尾辞を表示し、有効/無効を引き継ぐ |

経過時間のテストには仮想時計を用い、実時間 sleep に依存しない。V1/V2 の比較はアプリを同時起動せず、純粋な照合・シリアライズ処理で行う。

### 6.2 実機・インストーラー検証

専用 VM またはユーザー確認済みの検証環境を使用する。インストール/アンインストール/レジストリ変更は実施前に確認を得る。V1 と V2 は同時に動かさない。

| ID | シナリオ | 合格条件 |
|---|---|---|
| V2-I01 | 新規導入、V1 → V2 | 管理者権限不要。同じ AppId/配置先で置換され、旧版終了・設定退避・旧配布物除去を確認 |
| V2-I02 | 旧版実行中、退避/アンインストール失敗、途中キャンセル | 条件未達で先へ進まず、正常な設定・退避を保持。無断強制終了なし |
| V2-I03 | V2 → 次のベータ、V2 → V1 | アプリ登録・スタートアップが重複せず、設定を引き継げる。必要時は退避復元で復帰 |
| V2-I04 | V2 二重起動、旧版検出、再ログオン | V2 の重複起動を抑止し、旧版との競合を避ける。有効/無効に応じて起動 |
| V2-I05 | 通常アンインストール、置換失敗後の復帰 | アプリ登録を整理し、データ/移行退避を意図せず削除しない。V1 再インストールで復帰可能 |
| V2-W01 | Windows 10/11、USB/Bluetooth/スピーカー切替 | 既存音量・ミュートの復元、未登録自動作成、デバイスなしを確認 |
| V2-W02 | トレイ操作、Explorer 再起動、終了 | メニュー・通知・アイコンが機能し、トレイを再登録できる。終了後に処理を残さない |
| V2-W03 | 常駐/連続切替/長時間運転 | CPU/メモリ/ハンドル/成果物サイズを記録し、V1 と比較。未計測の改善は合格実績にしない |
| V2-R01 | ベータのパッケージ・Release 設定 | V2 の成果物だけを同梱。表示バージョン整合、Pre-release=true、Latest=false、自動更新対象外 |

Release の公開確認は、タグ付け・プッシュ・公開の個別承認を得た後に実施する。未公開の設定レビューを公開済みの検証結果として扱わない。

### 6.3 ビルド確認と完了条件

以下は Cargo workspace 実装後の検証コマンド案で、現時点では実行できる V2 プロジェクトは未作成。

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --release --locked --target x86_64-pc-windows-msvc
```

- 自動テストの結果と実機ケースの OS・アプリ版・結果を記録する。未実行/実施不能は明示し、V1 の過去実績で V2 を合格にしない。
- ユーザーへ動作確認を依頼し、明示的な確認完了承認を得る。
- ベータ公開・PR 作成の承認は動作確認承認とは分離する。V2 の安定化と `main` マージは人間が判断する。
