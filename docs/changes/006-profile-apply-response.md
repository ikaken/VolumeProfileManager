# プロファイル適用までのレスポンスアップ

## 背景 / 目的
Issue #6 の要望は、再生デバイス切替後に保存済みプロファイルが適用されるまでの待ち時間を短縮し、適用前の音量で音が再生される時間をできるだけなくすことです。

現状はデバイス変更イベントごとに 800ms の末尾デバウンスを行ってから、確定したデフォルトデバイスの取得とプロファイル適用を開始しています。この待機時間が、プロファイル適用までの主な遅延要因になっています。

## 変更内容
- `DeviceChangedEventArgs` にデバイス変更通知の種別を追加し、`OnDefaultDeviceChanged` と状態変更・追加・削除通知を区別する。
- `OnDefaultDeviceChanged` の場合は、800msのデバウンスを待たずにプロファイル適用を開始する。
- 短時間に複数発火する Windows のデバイス変更通知に対する800msの末尾デバウンスは、検証・救済パスとして維持する。
- 即時適用と検証パスを直列化し、同一デバイスの3秒間の重複抑制によって二重適用・二重通知を防ぐ。
- デバウンス時間をコンストラクタから注入できる構造にし、即時適用・遅延適用・重複抑制・失敗時救済を単体テストで確認する。

## 影響範囲
- `src/VolumeProfileManager.Domain/Entities/DeviceChangedEventArgs.cs`
  - デバイス変更通知種別の定義。
- `src/VolumeProfileManager.Infrastructure/Services/DeviceMonitorService.cs`
  - COMコールバックごとの通知種別設定。
- `src/VolumeProfileManager.Infrastructure/Services/DeviceMonitorOrchestrator.cs`
  - 即時適用パス、検証パス、適用処理の直列化。
- `tests/VolumeProfileManager.UnitTests/Services/DeviceMonitorOrchestratorTests.cs`
  - ハイブリッド方式の単体テスト。
- ユーザーへの影響
  - デフォルトデバイス変更通知を受けた時点でプロファイル適用を開始し、適用前の音量で再生される時間を短縮する。

## 備考
`OnDefaultDeviceChanged` はデフォルトデバイスの切替を示すため即時適用に利用します。一方、デバイス状態変更などの通知は切替完了を保証しないため従来どおり静定後に処理します。即時パスでデバイス解決に失敗した場合や切替途中だった場合も、800ms後の検証パスで確定したデバイスへ収束させます。
