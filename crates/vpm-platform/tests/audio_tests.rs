use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use vpm_core::{Clock, ProfileStore, ProfileTimestamp, VolumeProfile};
use vpm_platform::audio::{
    AudioDeviceAdapter, AudioDeviceInfo, AudioError, ComNotificationClient, DeviceChangeType,
    DeviceChangedEventArgs, OrchestratorEngine, ProfileCaptureService,
};
use windows::Win32::Media::Audio::{
    IMMNotificationClient, eCapture, eCommunications, eMultimedia, eRender,
};
use windows::core::PCWSTR;

#[derive(Clone)]
struct MockClock {
    now_ts: Arc<Mutex<ProfileTimestamp>>,
    elapsed: Arc<Mutex<Duration>>,
}

impl MockClock {
    fn new() -> Self {
        Self {
            now_ts: Arc::new(Mutex::new(ProfileTimestamp::now())),
            elapsed: Arc::new(Mutex::new(Duration::ZERO)),
        }
    }

    fn advance(&self, duration: Duration) {
        let mut el = self.elapsed.lock().unwrap();
        *el += duration;
    }

    fn set_timestamp(&self, ts: ProfileTimestamp) {
        let mut n = self.now_ts.lock().unwrap();
        *n = ts;
    }
}

impl Clock for MockClock {
    fn now(&self) -> ProfileTimestamp {
        *self.now_ts.lock().unwrap()
    }

    fn elapsed(&self) -> Duration {
        *self.elapsed.lock().unwrap()
    }
}

#[derive(Clone, Default)]
struct MockAudioDeviceAdapter {
    default_device: Arc<Mutex<Option<AudioDeviceInfo>>>,
    playback_devices: Arc<Mutex<Vec<AudioDeviceInfo>>>,
    master_volume: Arc<Mutex<f32>>,
    mute: Arc<Mutex<bool>>,
    fail_get_volume: Arc<Mutex<bool>>,
    set_volume_calls: Arc<Mutex<Vec<f32>>>,
    set_mute_calls: Arc<Mutex<Vec<bool>>>,
}

impl AudioDeviceAdapter for MockAudioDeviceAdapter {
    fn get_default_playback_device(&self) -> Result<Option<AudioDeviceInfo>, AudioError> {
        Ok(self.default_device.lock().unwrap().clone())
    }

    fn get_playback_devices(&self) -> Result<Vec<AudioDeviceInfo>, AudioError> {
        Ok(self.playback_devices.lock().unwrap().clone())
    }

    fn get_master_volume(&self, _device_id: Option<&str>) -> Result<f32, AudioError> {
        if *self.fail_get_volume.lock().unwrap() {
            return Err(AudioError::Other("Failed to query volume".to_string()));
        }
        Ok(*self.master_volume.lock().unwrap())
    }

    fn set_master_volume(&self, _device_id: Option<&str>, volume: f32) -> Result<(), AudioError> {
        if !volume.is_finite() {
            return Err(AudioError::InvalidVolume(volume));
        }
        let clamped = volume.clamp(0.0, 1.0);
        *self.master_volume.lock().unwrap() = clamped;
        self.set_volume_calls.lock().unwrap().push(clamped);
        Ok(())
    }

    fn get_mute(&self, _device_id: Option<&str>) -> Result<bool, AudioError> {
        Ok(*self.mute.lock().unwrap())
    }

    fn set_mute(&self, _device_id: Option<&str>, mute: bool) -> Result<(), AudioError> {
        *self.mute.lock().unwrap() = mute;
        self.set_mute_calls.lock().unwrap().push(mute);
        Ok(())
    }
}

#[derive(Clone, Default)]
struct InMemoryProfileStore {
    profiles: Arc<Mutex<Vec<VolumeProfile>>>,
}

impl InMemoryProfileStore {
    fn new(profiles: Vec<VolumeProfile>) -> Self {
        Self {
            profiles: Arc::new(Mutex::new(profiles)),
        }
    }
}

impl ProfileStore for InMemoryProfileStore {
    fn load(&self) -> std::io::Result<Vec<VolumeProfile>> {
        Ok(self.profiles.lock().unwrap().clone())
    }

    fn save(&self, profile: &VolumeProfile) -> std::io::Result<()> {
        let mut list = self.profiles.lock().unwrap();
        if let Some(pos) = list
            .iter()
            .position(|p| vpm_core::ordinal_ignore_case_eq(&p.device_id, &profile.device_id))
        {
            list[pos] = profile.clone();
        } else {
            list.push(profile.clone());
        }
        Ok(())
    }

    fn delete(&self, identifier: &str) -> std::io::Result<()> {
        let mut list = self.profiles.lock().unwrap();
        list.retain(|p| {
            !vpm_core::ordinal_ignore_case_eq(&p.device_id, identifier)
                && !vpm_core::ordinal_ignore_case_eq(&p.device_name, identifier)
        });
        Ok(())
    }
}

#[test]
fn test_v2_e01_com_notification_flow_role_filtering() {
    let (tx, rx) = mpsc::channel();
    let client: IMMNotificationClient = ComNotificationClient::new(tx).into();

    let dev_id: Vec<u16> = "device-1\0".encode_utf16().collect();
    let pcwstr = PCWSTR(dev_id.as_ptr());

    // 1. Render + Multimedia -> Should send event
    let res = unsafe { client.OnDefaultDeviceChanged(eRender, eMultimedia, pcwstr) };
    assert!(res.is_ok());
    let ev = rx.try_recv().expect("Expected event for Render+Multimedia");
    assert_eq!(ev.change_type, DeviceChangeType::DefaultDeviceChanged);
    assert_eq!(ev.new_device_id, "device-1");

    // 2. Capture + Multimedia -> Ignored
    let res = unsafe { client.OnDefaultDeviceChanged(eCapture, eMultimedia, pcwstr) };
    assert!(res.is_ok());
    assert!(rx.try_recv().is_err());

    // 3. Render + Communications -> Ignored
    let res = unsafe { client.OnDefaultDeviceChanged(eRender, eCommunications, pcwstr) };
    assert!(res.is_ok());
    assert!(rx.try_recv().is_err());
}

#[test]
fn test_v2_e01_engine_startup_does_not_apply_without_events() {
    let adapter = Arc::new(MockAudioDeviceAdapter::default());
    *adapter.default_device.lock().unwrap() = Some(AudioDeviceInfo {
        device_id: "dev-1".to_string(),
        device_name: "Speaker".to_string(),
        is_default: true,
    });
    let store = Arc::new(InMemoryProfileStore::default());
    let clock = Arc::new(MockClock::new());

    let mut engine = OrchestratorEngine::new(
        Arc::clone(&adapter),
        Arc::clone(&store),
        Arc::clone(&clock),
        None,
        None,
    );

    // Initial state: nothing applied
    assert_eq!(engine.drain_applied_events().len(), 0);
    assert!(adapter.set_volume_calls.lock().unwrap().is_empty());

    // Advance time without any event -> still nothing
    clock.advance(Duration::from_secs(5));
    let events = engine.advance_time().unwrap();
    assert!(events.is_empty());
    assert!(adapter.set_volume_calls.lock().unwrap().is_empty());
}

#[test]
fn test_v2_e02_rapid_events_and_trailing_debounce() {
    let adapter = Arc::new(MockAudioDeviceAdapter::default());
    *adapter.default_device.lock().unwrap() = Some(AudioDeviceInfo {
        device_id: "dev-1".to_string(),
        device_name: "Headphones".to_string(),
        is_default: true,
    });
    *adapter.master_volume.lock().unwrap() = 0.4;
    *adapter.mute.lock().unwrap() = false;

    let profile = VolumeProfile {
        device_id: "dev-1".to_string(),
        device_name: "Headphones".to_string(),
        master_volume: 0.85,
        is_muted: false,
        created_at: ProfileTimestamp::now(),
        last_applied: ProfileTimestamp::now(),
    };
    let store = Arc::new(InMemoryProfileStore::new(vec![profile]));
    let clock = Arc::new(MockClock::new());

    let mut engine = OrchestratorEngine::new(
        Arc::clone(&adapter),
        Arc::clone(&store),
        Arc::clone(&clock),
        Some(Duration::from_millis(800)),
        None,
    );

    // Event 1 at t=0ms (DeviceStateChanged -> debounced only)
    let ev1 = engine
        .on_device_changed(DeviceChangedEventArgs {
            previous_device_id: String::new(),
            new_device_id: "dev-1".to_string(),
            change_type: DeviceChangeType::DeviceStateChanged,
        })
        .unwrap();
    assert!(ev1.is_none());

    // Advance 300ms (t=300ms)
    clock.advance(Duration::from_millis(300));
    assert!(engine.advance_time().unwrap().is_empty());

    // Event 2 at t=300ms (pushes deadline to 300 + 800 = 1100ms)
    let ev2 = engine
        .on_device_changed(DeviceChangedEventArgs {
            previous_device_id: String::new(),
            new_device_id: "dev-1".to_string(),
            change_type: DeviceChangeType::DeviceAdded,
        })
        .unwrap();
    assert!(ev2.is_none());

    // Advance to t=900ms (past original 800ms, but before 1100ms) -> should NOT trigger
    clock.advance(Duration::from_millis(600)); // now at 900ms
    assert!(engine.advance_time().unwrap().is_empty());
    assert!(adapter.set_volume_calls.lock().unwrap().is_empty());

    // Advance to t=1150ms (past 1100ms) -> should trigger settled apply!
    clock.advance(Duration::from_millis(250)); // now at 1150ms
    let triggered = engine.advance_time().unwrap();
    assert_eq!(triggered.len(), 1);
    assert_eq!(triggered[0].device_id, "dev-1");
    assert_eq!(triggered[0].master_volume, 0.85);
    assert_eq!(*adapter.master_volume.lock().unwrap(), 0.85);
}

#[test]
fn test_v2_e02_immediate_apply_failure_retries_on_settle() {
    let adapter = Arc::new(MockAudioDeviceAdapter::default());
    // Initially device is absent/not ready
    *adapter.default_device.lock().unwrap() = None;

    let profile = VolumeProfile {
        device_id: "dev-1".to_string(),
        device_name: "Speaker".to_string(),
        master_volume: 0.70,
        is_muted: false,
        created_at: ProfileTimestamp::now(),
        last_applied: ProfileTimestamp::now(),
    };
    let store = Arc::new(InMemoryProfileStore::new(vec![profile]));
    let clock = Arc::new(MockClock::new());

    let mut engine = OrchestratorEngine::new(
        Arc::clone(&adapter),
        Arc::clone(&store),
        Arc::clone(&clock),
        Some(Duration::from_millis(800)),
        None,
    );

    // DefaultDeviceChanged at t=0ms, but device lookup returns None
    let res = engine.on_device_changed(DeviceChangedEventArgs {
        previous_device_id: String::new(),
        new_device_id: "dev-1".to_string(),
        change_type: DeviceChangeType::DefaultDeviceChanged,
    });
    assert!(res.unwrap().is_none());

    // Now default device becomes available at t=400ms
    clock.advance(Duration::from_millis(400));
    *adapter.default_device.lock().unwrap() = Some(AudioDeviceInfo {
        device_id: "dev-1".to_string(),
        device_name: "Speaker".to_string(),
        is_default: true,
    });

    // Settle at t=800ms
    clock.advance(Duration::from_millis(400));
    let settled = engine.advance_time().unwrap();
    assert_eq!(settled.len(), 1);
    assert_eq!(settled[0].master_volume, 0.70);
    assert_eq!(*adapter.master_volume.lock().unwrap(), 0.70);
}

#[test]
fn test_v2_e03_same_device_3_second_suppression_and_different_devices() {
    let adapter = Arc::new(MockAudioDeviceAdapter::default());
    *adapter.default_device.lock().unwrap() = Some(AudioDeviceInfo {
        device_id: "dev-A".to_string(),
        device_name: "Headset A".to_string(),
        is_default: true,
    });

    let profile_a = VolumeProfile {
        device_id: "dev-A".to_string(),
        device_name: "Headset A".to_string(),
        master_volume: 0.50,
        is_muted: false,
        created_at: ProfileTimestamp::now(),
        last_applied: ProfileTimestamp::now(),
    };
    let profile_b = VolumeProfile {
        device_id: "dev-B".to_string(),
        device_name: "Headset B".to_string(),
        master_volume: 0.75,
        is_muted: false,
        created_at: ProfileTimestamp::now(),
        last_applied: ProfileTimestamp::now(),
    };
    let store = Arc::new(InMemoryProfileStore::new(vec![profile_a, profile_b]));
    let clock = Arc::new(MockClock::new());

    let mut engine = OrchestratorEngine::new(
        Arc::clone(&adapter),
        Arc::clone(&store),
        Arc::clone(&clock),
        Some(Duration::from_millis(800)),
        Some(Duration::from_secs(3)),
    );

    // 1. Initial DefaultDeviceChanged to dev-A at t=0s -> Applied
    let ev1 = engine
        .on_device_changed(DeviceChangedEventArgs {
            previous_device_id: String::new(),
            new_device_id: "dev-A".to_string(),
            change_type: DeviceChangeType::DefaultDeviceChanged,
        })
        .unwrap();
    assert!(ev1.is_some());
    assert_eq!(ev1.unwrap().master_volume, 0.50);

    // 2. Duplicate event for dev-A at t=1.5s (< 3.0s) -> Suppressed
    clock.advance(Duration::from_millis(1500));
    let ev2 = engine
        .on_device_changed(DeviceChangedEventArgs {
            previous_device_id: String::new(),
            new_device_id: "dev-A".to_string(),
            change_type: DeviceChangeType::DefaultDeviceChanged,
        })
        .unwrap();
    assert!(ev2.is_none());

    // 3. Switch to dev-B at t=2.0s (< 3.0s from dev-A, but different device) -> Applied immediately!
    clock.advance(Duration::from_millis(500));
    *adapter.default_device.lock().unwrap() = Some(AudioDeviceInfo {
        device_id: "dev-B".to_string(),
        device_name: "Headset B".to_string(),
        is_default: true,
    });
    let ev3 = engine
        .on_device_changed(DeviceChangedEventArgs {
            previous_device_id: "dev-A".to_string(),
            new_device_id: "dev-B".to_string(),
            change_type: DeviceChangeType::DefaultDeviceChanged,
        })
        .unwrap();
    assert!(ev3.is_some());
    assert_eq!(ev3.unwrap().master_volume, 0.75);

    // 4. Switch back to dev-A at t=4.0s (> 3s since dev-A was last applied at t=0s) -> Applied!
    clock.advance(Duration::from_secs(2));
    *adapter.default_device.lock().unwrap() = Some(AudioDeviceInfo {
        device_id: "dev-A".to_string(),
        device_name: "Headset A".to_string(),
        is_default: true,
    });
    let ev4 = engine
        .on_device_changed(DeviceChangedEventArgs {
            previous_device_id: "dev-B".to_string(),
            new_device_id: "dev-A".to_string(),
            change_type: DeviceChangeType::DefaultDeviceChanged,
        })
        .unwrap();
    assert!(ev4.is_some());
    assert_eq!(ev4.unwrap().master_volume, 0.50);
}

#[test]
fn test_v2_e04_registered_vs_unregistered_and_error_protection() {
    let adapter = Arc::new(MockAudioDeviceAdapter::default());
    *adapter.default_device.lock().unwrap() = Some(AudioDeviceInfo {
        device_id: "dev-known".to_string(),
        device_name: "Known Speaker".to_string(),
        is_default: true,
    });

    let known_profile = VolumeProfile {
        device_id: "dev-known".to_string(),
        device_name: "Known Speaker".to_string(),
        master_volume: 0.62,
        is_muted: true,
        created_at: "2024-01-01T00:00:00".parse().unwrap(),
        last_applied: "2024-01-01T00:00:00".parse().unwrap(),
    };
    let store = Arc::new(InMemoryProfileStore::new(vec![known_profile.clone()]));
    let clock = Arc::new(MockClock::new());
    let current_ts = "2025-06-01T12:00:00".parse().unwrap();
    clock.set_timestamp(current_ts);

    let mut engine = OrchestratorEngine::new(
        Arc::clone(&adapter),
        Arc::clone(&store),
        Arc::clone(&clock),
        None,
        None,
    );

    // 1. Registered device apply: LastApplied in store must NOT be changed!
    let ev = engine
        .on_device_changed(DeviceChangedEventArgs {
            previous_device_id: String::new(),
            new_device_id: "dev-known".to_string(),
            change_type: DeviceChangeType::DefaultDeviceChanged,
        })
        .unwrap()
        .unwrap();
    assert!(!ev.is_new_profile);
    assert_eq!(ev.master_volume, 0.62);
    assert!(ev.is_muted);

    let loaded = store.load().unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].last_applied, known_profile.last_applied);

    // 2. Unregistered device: auto-creates new profile with current volume & mute
    clock.advance(Duration::from_secs(5));
    *adapter.default_device.lock().unwrap() = Some(AudioDeviceInfo {
        device_id: "dev-new".to_string(),
        device_name: "USB DAC".to_string(),
        is_default: true,
    });
    *adapter.master_volume.lock().unwrap() = 0.33;
    *adapter.mute.lock().unwrap() = false;

    let ev_new = engine
        .on_device_changed(DeviceChangedEventArgs {
            previous_device_id: "dev-known".to_string(),
            new_device_id: "dev-new".to_string(),
            change_type: DeviceChangeType::DefaultDeviceChanged,
        })
        .unwrap()
        .unwrap();
    assert!(ev_new.is_new_profile);
    assert_eq!(ev_new.device_id, "dev-new");
    assert_eq!(ev_new.master_volume, 0.33);
    assert!(!ev_new.is_muted);

    let loaded2 = store.load().unwrap();
    assert_eq!(loaded2.len(), 2);
    let new_saved = loaded2.iter().find(|p| p.device_id == "dev-new").unwrap();
    assert_eq!(new_saved.device_name, "USB DAC");
    assert_eq!(new_saved.master_volume, 0.33);
    assert!(!new_saved.is_muted);
    assert_eq!(new_saved.created_at, current_ts);
    assert_eq!(new_saved.last_applied, current_ts);

    // 3. Error protection: when volume query fails on an unregistered device,
    // do NOT save 0/false and do NOT emit event!
    clock.advance(Duration::from_secs(5));
    *adapter.default_device.lock().unwrap() = Some(AudioDeviceInfo {
        device_id: "dev-err".to_string(),
        device_name: "Faulty Device".to_string(),
        is_default: true,
    });
    *adapter.fail_get_volume.lock().unwrap() = true;

    let res_err = engine.on_device_changed(DeviceChangedEventArgs {
        previous_device_id: "dev-new".to_string(),
        new_device_id: "dev-err".to_string(),
        change_type: DeviceChangeType::DefaultDeviceChanged,
    });
    assert!(res_err.is_err());

    let loaded3 = store.load().unwrap();
    assert_eq!(loaded3.len(), 2);
    assert!(loaded3.iter().all(|p| p.device_id != "dev-err"));
}

#[test]
fn test_v2_e05_stop_and_cancellation() {
    let adapter = Arc::new(MockAudioDeviceAdapter::default());
    *adapter.default_device.lock().unwrap() = Some(AudioDeviceInfo {
        device_id: "dev-1".to_string(),
        device_name: "Speaker".to_string(),
        is_default: true,
    });

    let store = Arc::new(InMemoryProfileStore::default());
    let clock = Arc::new(MockClock::new());

    let mut engine = OrchestratorEngine::new(
        Arc::clone(&adapter),
        Arc::clone(&store),
        Arc::clone(&clock),
        Some(Duration::from_millis(800)),
        None,
    );

    // Schedule debounce event at t=0
    engine
        .on_device_changed(DeviceChangedEventArgs {
            previous_device_id: String::new(),
            new_device_id: "dev-1".to_string(),
            change_type: DeviceChangeType::DeviceStateChanged,
        })
        .unwrap();

    // Stop engine at t=200ms
    clock.advance(Duration::from_millis(200));
    engine.stop();
    assert!(engine.is_stopped());

    // Advance time past debounce window -> nothing should be applied!
    clock.advance(Duration::from_secs(2));
    let events = engine.advance_time().unwrap();
    assert!(events.is_empty());
    assert!(store.load().unwrap().is_empty());
}

#[test]
fn test_profile_capture_service() {
    let adapter = Arc::new(MockAudioDeviceAdapter::default());
    *adapter.default_device.lock().unwrap() = Some(AudioDeviceInfo {
        device_id: "dev-capture".to_string(),
        device_name: "Studio Monitors".to_string(),
        is_default: true,
    });
    *adapter.master_volume.lock().unwrap() = 0.45;
    *adapter.mute.lock().unwrap() = false;

    let store = Arc::new(InMemoryProfileStore::default());
    let clock = Arc::new(MockClock::new());
    let ts: ProfileTimestamp = "2025-07-07T07:07:07".parse().unwrap();
    clock.set_timestamp(ts);

    let capture_svc =
        ProfileCaptureService::new(Arc::clone(&adapter), Arc::clone(&store), Arc::clone(&clock));

    let captured = capture_svc.capture_current_profile(None).unwrap().unwrap();
    assert_eq!(captured.device_id, "dev-capture");
    assert_eq!(captured.device_name, "Studio Monitors");
    assert_eq!(captured.master_volume, 0.45);
    assert!(!captured.is_muted);
    assert_eq!(captured.created_at, ts);
    assert_eq!(captured.last_applied, ts);

    let stored = store.load().unwrap();
    assert_eq!(stored.len(), 1);
    assert_eq!(stored[0].device_id, "dev-capture");
}
