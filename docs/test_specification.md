# VolumeProfileManager テスト仕様書

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
