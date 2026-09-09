use std::time::Duration;
use vpm_core::{
    Clock, ProfileStore, ProfileTimestamp, SystemClock, VolumeProfile, match_profile,
    ordinal_ignore_case_eq,
};

fn profile(id: &str, name: &str, applied: &str, created: &str) -> VolumeProfile {
    VolumeProfile {
        device_id: id.into(),
        device_name: name.into(),
        last_applied: applied.parse().unwrap(),
        created_at: created.parse().unwrap(),
        ..Default::default()
    }
}

#[test]
fn default_is_v1_minimum_and_pascal_case() {
    let value = serde_json::to_value(VolumeProfile::default()).unwrap();
    assert_eq!(
        value,
        serde_json::json!({"DeviceId":"","DeviceName":"","MasterVolume":0.0,"IsMuted":false,"CreatedAt":"0001-01-01T00:00:00","LastApplied":"0001-01-01T00:00:00"})
    );
    assert_eq!(
        serde_json::from_str::<VolumeProfile>("{}").unwrap(),
        VolumeProfile::default()
    );
}

#[test]
fn timestamps_accept_v1_precision_and_compare_wall_ticks() {
    let min: ProfileTimestamp = "0001-01-01T00:00:00".parse().unwrap();
    assert_eq!(min, ProfileTimestamp::default());
    assert_eq!(min, "0001-01-01T00:00:00Z".parse().unwrap());
    let base: ProfileTimestamp = "2025-01-02T03:04:05Z".parse().unwrap();
    for fraction in ["1", "12", "123", "1234", "12345", "123456", "1234567"] {
        let text = format!("2025-01-02T03:04:05.{fraction}Z");
        let value: ProfileTimestamp = text.parse().unwrap();
        assert!(value > base);
        assert_eq!(
            serde_json::to_string(&value).unwrap(),
            format!("\"{text}\"")
        );
    }
    assert!(
        "9999-12-31T23:59:59.9999999Z"
            .parse::<ProfileTimestamp>()
            .is_ok()
    );
    assert!(ProfileTimestamp::now() > base);
}

#[test]
fn invalid_timestamps_and_nonfinite_volumes_fail_safely() {
    for value in [
        "",
        "0000-01-01T00:00:00",
        "10000-01-01T00:00:00Z",
        "2025-02-29T00:00:00Z",
        "2025-01-01T24:00:00Z",
        "2025-01-01T00:00:60Z",
        "2025-01-01",
        "2025-01-01T00:00:00.12345678Z",
        "2025-01-01T00:00:00junk",
    ] {
        assert!(value.parse::<ProfileTimestamp>().is_err(), "{value}");
    }
    for volume in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        assert!(
            serde_json::to_string(&VolumeProfile {
                master_volume: volume,
                ..Default::default()
            })
            .is_err()
        );
    }
    for json in [
        "{\"MasterVolume\":1e100}",
        "{\"MasterVolume\":null}",
        "{\"MasterVolume\":\"NaN\"}",
        "{\"CreatedAt\":\"bad\"}",
    ] {
        assert!(
            serde_json::from_str::<VolumeProfile>(json).is_err(),
            "{json}"
        );
    }
}

#[test]
fn id_first_then_exact_name_then_partial_with_stable_date_order() {
    let old = "2024-01-01T00:00:00Z";
    let new = "2025-01-01T00:00:00Z";
    let profiles = [
        profile("ID", "name", old, old),
        profile("id", "name", new, new),
        profile("3", "other name", new, new),
    ];
    assert!(std::ptr::eq(
        match_profile(&profiles, "id", Some("other name")).unwrap(),
        &profiles[0]
    ));
    assert!(std::ptr::eq(
        match_profile(&profiles, "new", Some("name")).unwrap(),
        &profiles[1]
    ));
    let profiles = [
        profile("1", "name", new, old),
        profile("2", "name", new, new),
        profile("3", "name", new, new),
    ];
    assert!(std::ptr::eq(
        match_profile(&profiles, "new", Some("name")).unwrap(),
        &profiles[1]
    ));
    assert!(std::ptr::eq(
        match_profile(&profiles, "new", Some("na")).unwrap(),
        &profiles[1]
    ));
    assert!(std::ptr::eq(
        match_profile(&profiles, "new", Some("long name")).unwrap(),
        &profiles[1]
    ));
}

#[test]
fn whitespace_guards_and_normalization_are_not_general_whitespace_collapsing() {
    let profiles = [VolumeProfile {
        device_id: " ".into(),
        device_name: "\u{a0} A   B \u{85}".into(),
        ..Default::default()
    }];
    assert!(match_profile(&profiles, " ", None).is_none());
    assert!(match_profile(&profiles, "new", Some(" \t\r\n\u{85}\u{a0}\u{3000}")).is_none());
    assert!(match_profile(&profiles, "new", Some("a b")).is_some());
    assert!(match_profile(&profiles, "new", Some("a\tb")).is_none());
    assert!(match_profile(&[], "id", Some("name")).is_none());
}

#[test]
fn ordinal_case_preserves_dotnet_exceptions_without_full_expansion() {
    for (left, right) in [
        ("ı", "I"),
        ("ſ", "S"),
        ("ß", "SS"),
        ("ﬃ", "FFI"),
        ("ƛ", "Ƛ"),
    ] {
        assert!(!ordinal_ignore_case_eq(left, right));
    }
    for (left, right) in [("ᾀ", "ᾈ"), ("ᾳ", "ᾼ"), ("ς", "Σ"), ("é", "É"), ("𐐨", "𐐀")]
    {
        assert!(ordinal_ignore_case_eq(left, right));
    }
}

#[test]
fn traits_allow_fake_clock_and_memory_store() {
    struct FakeClock;
    impl Clock for FakeClock {
        fn now(&self) -> ProfileTimestamp {
            ProfileTimestamp::default()
        }
        fn elapsed(&self) -> Duration {
            Duration::from_secs(42)
        }
    }
    struct Store;
    impl ProfileStore for Store {
        fn load(&self) -> std::io::Result<Vec<VolumeProfile>> {
            Ok(Vec::new())
        }
        fn save(&self, _: &VolumeProfile) -> std::io::Result<()> {
            Ok(())
        }
        fn delete(&self, _: &str) -> std::io::Result<()> {
            Ok(())
        }
    }
    let clock: &dyn Clock = &FakeClock;
    assert_eq!(clock.elapsed(), Duration::from_secs(42));
    assert_eq!(clock.now(), ProfileTimestamp::default());
    let store: &dyn ProfileStore = &Store;
    assert!(store.load().unwrap().is_empty());
    store.save(&VolumeProfile::default()).unwrap();
    store.delete("synthetic").unwrap();
    let system = SystemClock::default();
    assert!(system.now() > ProfileTimestamp::default());
    assert!(system.elapsed() <= system.elapsed());
}
