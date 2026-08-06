# VolumeProfileManager 抽出コンポーネント

このフォルダは AudioPilot から VolumeProfileManager に流用できるコンポーネントを格納しています。

## ディレクトリ構造

```
extract-for-vpm/
├── Core/Interfaces/
│   ├── IDeviceMonitorService.cs     # デバイス監視インターフェース
│   └── IAudioDeviceService.cs       # デバイス管理インターフェース + AudioDeviceInfo モデル
├── Infrastructure/Services/
│   ├── DeviceMonitorService.cs      # NAudio を使用したデバイス監視実装
│   └── AudioDeviceService.cs        # NAaudio を使用したデバイス管理実装
└── README/
    └── EXTRACTION_GUIDE.md          # このファイル
```

## 使用方法

### 手順 1：ファイルをコピー

VolumeProfileManager プロジェクトの対応するレイヤーにファイルをコピーします：

- `Core/Interfaces/*.cs` → `VolumeProfileManager.Core/Interfaces/`
- `Infrastructure/Services/*.cs` → `VolumeProfileManager.Infrastructure/Services/`

### 手順 2：名前空間を修正

ファイルをコピー後、名前空間を以下に変更してください：

```csharp
// Before
namespace AudioPilot.Core.Interfaces;
namespace AudioPilot.Infrastructure.Services;

// After
namespace VolumeProfileManager.Core.Interfaces;
namespace VolumeProfileManager.Infrastructure.Services;
```

### 手順 3：プロジェクト参照を更新

VolumeProfileManager.csproj に以下のパッケージ参照を追加：

```xml
<PackageReference Include="NAudio" Version="2.2.*" />
<PackageReference Include="Serilog" Version="4.0.*" />
```

## 抽出コンポーネント詳細

### IDeviceMonitorService
- **用途**: オーディオデバイスの変更を監視
- **イベント**: `DeviceChanged`

### IAudioDeviceService + AudioDeviceInfo
- **用途**: 利用可能なオーディオデバイスの取得・管理
- **メソッド**:
  - `GetPlaybackDevices()` - 再生デバイス一覧
  - `GetDefaultPlaybackDevice()` - 既定デバイス取得
  - `SetDefaultPlaybackDevice(deviceId)` - 既定デバイス設定

### DeviceMonitorService
- **用途**: Core Audio API 経由のリアルタイムデバイス監視
- **依存**: NAudio.CoreAudioApi, Serilog

### AudioDeviceService
- **用途**: デバイス一覧の取得と既定デバイスの管理
- **依存**: NAudio.CoreAudioApi, Serilog

## 注意事項

1. **PolicyConfig 実装**: `SetDefaultPlaybackDevice()` の実装は TODO です。必要に応じて Windows PolicyConfig API を統合してください。

2. **名前空間修正**: コピー後、すべての名前空間を VolumeProfileManager に統一してください。

3. **テスト**: 抽出後は統合テストで動作確認を実施してください。

4. **バージョン管理**: 将来 AudioPilot で同じコンポーネントが更新された場合、VolumeProfileManager にも反映する必要があります。

## ライセンス

これらのコンポーネントは AudioPilot から抽出されたものです。AudioPilot のライセンスに準じます。
