use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Barrier};
use std::thread;
use std::time::Duration;
use tempfile::tempdir;
use vpm_core::{Clock, ProfileStore, ProfileTimestamp, VolumeProfile};
use vpm_platform::persistence::JsonProfileStore;

const V1: &str = r#"[
  {
    "DeviceId": "DEVICE-A",
    "DeviceName": "スピーカー (USB)",
    "MasterVolume": 0.33333334,
    "IsMuted": false,
    "CreatedAt": "2024-01-02T03:04:05.1234567Z",
    "LastApplied": "0001-01-01T00:00:00"
  }
]"#;

struct FixedClock;

impl Clock for FixedClock {
    fn now(&self) -> ProfileTimestamp {
        "2025-06-07T08:09:10.1234567Z".parse().unwrap()
    }

    fn elapsed(&self) -> Duration {
        Duration::ZERO
    }
}

fn profile(id: &str, name: &str) -> VolumeProfile {
    VolumeProfile {
        device_id: id.into(),
        device_name: name.into(),
        master_volume: 0.5,
        is_muted: false,
        created_at: ProfileTimestamp::default(),
        last_applied: "2025-01-01T00:00:00Z".parse().unwrap(),
    }
}

fn backup(path: &Path) -> PathBuf {
    let mut value = path.as_os_str().to_os_string();
    value.push(".bak");
    value.into()
}

#[test]
fn missing_store_loads_empty_and_creates_on_first_save() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("nested/profiles.json");
    let store = JsonProfileStore::with_clock(&path, FixedClock);
    assert_eq!(store.path(), path);
    assert!(store.load().unwrap().is_empty());
    assert!(!path.exists());
    store.save(&profile("id", "name")).unwrap();
    assert_eq!(store.load().unwrap()[0].created_at, FixedClock.now());
    assert!(!backup(&path).exists());
}

#[test]
fn reads_v1_pascal_case_bom_and_preserves_created_at_on_update() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("profiles.json");
    let original = [b"\xef\xbb\xbf".as_slice(), V1.as_bytes()].concat();
    fs::write(&path, &original).unwrap();
    let store = JsonProfileStore::with_clock(&path, FixedClock);
    let before = store.load().unwrap().remove(0);
    let mut update = profile("device-a", "Updated");
    update.master_volume = 0.75;
    update.is_muted = true;
    store.save(&update).unwrap();
    let loaded = store.load().unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].device_id, "DEVICE-A");
    assert_eq!(loaded[0].created_at, before.created_at);
    assert_eq!(loaded[0].last_applied, update.last_applied);
    assert_eq!(loaded[0].device_name, "Updated");
    assert_eq!(loaded[0].master_volume, 0.75);
    assert!(loaded[0].is_muted);
    assert_eq!(fs::read(backup(&path)).unwrap(), original);
    let json: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    assert!(json.is_array());
    let object = json[0].as_object().unwrap();
    assert_eq!(object.len(), 6);
    for key in [
        "DeviceId",
        "DeviceName",
        "MasterVolume",
        "IsMuted",
        "CreatedAt",
        "LastApplied",
    ] {
        assert!(object.contains_key(key));
    }
}

#[test]
fn save_matches_id_only_and_delete_removes_all_exact_id_or_name_matches() {
    let temp = tempdir().unwrap();
    let store = JsonProfileStore::new(temp.path().join("profiles.json"));
    for (id, name) in [
        ("first", "Shared"),
        ("second", "shared"),
        ("SHARED", "third"),
        ("keep", "Shared suffix"),
        ("space", " Shared "),
    ] {
        store.save(&profile(id, name)).unwrap();
    }
    assert_eq!(store.load().unwrap().len(), 5);
    assert_eq!(
        store.get_by_identifier("FIRST").unwrap().unwrap().device_id,
        "first"
    );
    assert!(store.get_by_identifier("missing").unwrap().is_none());
    assert_eq!(
        store
            .get_by_identifier("SHARED")
            .unwrap()
            .unwrap()
            .device_id,
        "first"
    );
    store.delete("sHaReD").unwrap();
    let remaining = store.load().unwrap();
    assert_eq!(
        remaining
            .iter()
            .map(|p| p.device_id.as_str())
            .collect::<Vec<_>>(),
        ["keep", "space"]
    );
}

#[test]
fn comparisons_use_ordinal_ignore_case_without_unicode_expansion() {
    let temp = tempdir().unwrap();
    let store = JsonProfileStore::new(temp.path().join("profiles.json"));
    store.save(&profile("Ä", "日本語")).unwrap();
    store.save(&profile("ä", "変更済み")).unwrap();
    store.save(&profile("ß", "sharp")).unwrap();
    store.save(&profile("SS", "double")).unwrap();
    assert_eq!(store.load().unwrap().len(), 3);
    store.delete("ä").unwrap();
    assert_eq!(store.load().unwrap().len(), 2);
}

#[test]
fn corrupt_primary_recovers_valid_backup_in_same_load_and_next_save() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("profiles.json");
    fs::write(&path, b"broken").unwrap();
    fs::write(backup(&path), V1).unwrap();
    let store = JsonProfileStore::new(&path);
    assert_eq!(store.load().unwrap().len(), 1);
    assert_eq!(fs::read(&path).unwrap(), V1.as_bytes());
    assert_eq!(fs::read(backup(&path)).unwrap(), V1.as_bytes());
    store.save(&profile("new", "New")).unwrap();
    assert_eq!(store.load().unwrap().len(), 2);
    assert_eq!(fs::read(backup(&path)).unwrap(), V1.as_bytes());
}

#[test]
fn save_directly_after_corruption_keeps_recovered_profiles() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("profiles.json");
    fs::write(&path, b"broken").unwrap();
    fs::write(backup(&path), V1).unwrap();
    let store = JsonProfileStore::new(&path);
    store.save(&profile("new", "New")).unwrap();
    assert_eq!(store.load().unwrap().len(), 2);
    assert_eq!(fs::read(backup(&path)).unwrap(), V1.as_bytes());
}

#[test]
fn missing_primary_recovers_backup_including_bom() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("profiles.json");
    let bytes = [b"\xef\xbb\xbf".as_slice(), V1.as_bytes()].concat();
    fs::write(backup(&path), &bytes).unwrap();
    let store = JsonProfileStore::new(&path);
    assert_eq!(store.load().unwrap().len(), 1);
    assert_eq!(fs::read(&path).unwrap(), bytes);
}

#[test]
fn irrecoverable_json_never_becomes_empty_or_gets_overwritten() {
    for bad in [
        "broken",
        "",
        "null",
        "{}",
        "[null]",
        "[{\"MasterVolume\":null}]",
        "[{\"MasterVolume\":1e100}]",
    ] {
        for backup_bytes in [None, Some(b"broken backup".as_slice())] {
            let temp = tempdir().unwrap();
            let path = temp.path().join("profiles.json");
            fs::write(&path, bad).unwrap();
            if let Some(bytes) = backup_bytes {
                fs::write(backup(&path), bytes).unwrap();
            }
            let store = JsonProfileStore::new(&path);
            assert!(store.load().is_err(), "accepted {bad:?}");
            assert!(store.save(&profile("new", "new")).is_err());
            assert!(store.delete("old").is_err());
            assert_eq!(fs::read(&path).unwrap(), bad.as_bytes());
            assert_eq!(fs::read(backup(&path)).ok().as_deref(), backup_bytes);
        }
    }
}

#[test]
fn missing_primary_and_bad_backup_returns_error_without_creating_primary() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("profiles.json");
    fs::write(backup(&path), b"broken").unwrap();
    let store = JsonProfileStore::new(&path);
    assert!(store.load().is_err());
    assert!(store.save(&profile("new", "new")).is_err());
    assert!(store.delete("old").is_err());
    assert!(!path.exists());
    assert_eq!(fs::read(backup(&path)).unwrap(), b"broken");
}

#[test]
fn primary_io_error_does_not_recover_or_overwrite_backup() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("profiles.json");
    fs::create_dir(&path).unwrap();
    fs::write(backup(&path), V1).unwrap();
    let store = JsonProfileStore::new(&path);
    assert!(store.load().is_err());
    assert!(store.save(&profile("new", "new")).is_err());
    assert!(store.delete("old").is_err());
    assert!(path.is_dir());
    assert_eq!(fs::read(backup(&path)).unwrap(), V1.as_bytes());
}

#[test]
fn backup_io_error_aborts_recovery_and_update_without_touching_primary() {
    for original in ["broken", V1] {
        let temp = tempdir().unwrap();
        let path = temp.path().join("profiles.json");
        fs::write(&path, original).unwrap();
        fs::create_dir(backup(&path)).unwrap();
        let store = JsonProfileStore::new(&path);
        assert!(store.save(&profile("new", "new")).is_err());
        assert!(store.delete("DEVICE-A").is_err());
        assert_eq!(fs::read(&path).unwrap(), original.as_bytes());
        assert!(backup(&path).is_dir());
    }
}

#[test]
fn nonfinite_volume_is_rejected_before_any_file_changes() {
    for volume in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let temp = tempdir().unwrap();
        let path = temp.path().join("profiles.json");
        fs::write(&path, V1).unwrap();
        fs::write(backup(&path), b"[]").unwrap();
        let mut invalid = profile("DEVICE-A", "bad");
        invalid.master_volume = volume;
        let store = JsonProfileStore::new(&path);
        assert_eq!(
            store.save(&invalid).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(fs::read(&path).unwrap(), V1.as_bytes());
        assert_eq!(fs::read(backup(&path)).unwrap(), b"[]");
    }
}

#[test]
fn backup_always_contains_the_immediately_previous_valid_bytes() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("profiles.json");
    let store = JsonProfileStore::new(&path);
    store.save(&profile("one", "One")).unwrap();
    let first = fs::read(&path).unwrap();
    store.save(&profile("two", "Two")).unwrap();
    assert_eq!(fs::read(backup(&path)).unwrap(), first);
    let second = fs::read(&path).unwrap();
    store.delete("one").unwrap();
    assert_eq!(fs::read(backup(&path)).unwrap(), second);
    assert_eq!(store.load().unwrap().len(), 1);
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 3);
}

#[test]
fn independent_store_instances_do_not_lose_concurrent_updates() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("profiles.json");
    let barrier = Arc::new(Barrier::new(8));
    let threads: Vec<_> = (0..8)
        .map(|worker| {
            let path = path.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                let store = JsonProfileStore::new(path);
                barrier.wait();
                for index in 0..8 {
                    store
                        .save(&profile(&format!("{worker}-{index}"), "name"))
                        .unwrap();
                    assert!(!store.load().unwrap().is_empty());
                }
            })
        })
        .collect();
    for worker in threads {
        worker.join().unwrap();
    }
    let store = JsonProfileStore::new(&path);
    assert_eq!(store.load().unwrap().len(), 64);
    let previous: Vec<VolumeProfile> =
        serde_json::from_slice(&fs::read(backup(&path)).unwrap()).unwrap();
    assert_eq!(previous.len(), 63);
}

#[test]
fn migration_snapshot_copies_both_exact_bytes_and_survives_normal_rotation() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("profiles.json");
    let destination = temp.path().join("migration");
    let original = [b"\xef\xbb\xbf".as_slice(), V1.as_bytes()].concat();
    fs::write(&path, &original).unwrap();
    fs::write(backup(&path), b"[]").unwrap();
    let store = JsonProfileStore::new(&path);
    store.backup_for_migration(&destination).unwrap();
    assert_eq!(
        fs::read(destination.join("profiles.json")).unwrap(),
        original
    );
    assert_eq!(
        fs::read(destination.join("profiles.json.bak")).unwrap(),
        b"[]"
    );
    store.save(&profile("new", "new")).unwrap();
    assert_eq!(
        store.backup_for_migration(&destination).unwrap_err().kind(),
        io::ErrorKind::AlreadyExists
    );
    assert_eq!(
        fs::read(destination.join("profiles.json")).unwrap(),
        original
    );
    assert_eq!(
        fs::read(destination.join("profiles.json.bak")).unwrap(),
        b"[]"
    );
}

#[test]
fn migration_snapshot_preserves_corrupt_sources_without_attempting_recovery() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("profiles.json");
    let destination = temp.path().join("migration");
    fs::write(&path, b"broken primary").unwrap();
    fs::write(backup(&path), b"broken backup").unwrap();
    JsonProfileStore::new(&path)
        .backup_for_migration(&destination)
        .unwrap();
    assert_eq!(
        fs::read(destination.join("profiles.json")).unwrap(),
        b"broken primary"
    );
    assert_eq!(
        fs::read(destination.join("profiles.json.bak")).unwrap(),
        b"broken backup"
    );
    assert_eq!(fs::read(&path).unwrap(), b"broken primary");
}

#[test]
fn migration_handles_each_missing_source_without_manufacturing_profiles() {
    for (primary_exists, backup_exists) in [(false, false), (true, false), (false, true)] {
        let temp = tempdir().unwrap();
        let path = temp.path().join("profiles.json");
        let destination = temp.path().join("migration");
        if primary_exists {
            fs::write(&path, V1).unwrap();
        }
        if backup_exists {
            fs::write(backup(&path), b"[]").unwrap();
        }
        JsonProfileStore::new(&path)
            .backup_for_migration(&destination)
            .unwrap();
        assert!(destination.is_dir());
        assert_eq!(destination.join("profiles.json").exists(), primary_exists);
        assert_eq!(
            destination.join("profiles.json.bak").exists(),
            backup_exists
        );
        assert_eq!(path.exists(), primary_exists);
        assert_eq!(backup(&path).exists(), backup_exists);
    }
}

#[test]
fn migration_copy_failure_preserves_sources_and_existing_destination() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("profiles.json");
    fs::write(&path, V1).unwrap();
    fs::write(backup(&path), b"[]").unwrap();
    let blocked_parent = temp.path().join("blocked");
    fs::write(&blocked_parent, b"keep").unwrap();
    let store = JsonProfileStore::new(&path);
    assert!(
        store
            .backup_for_migration(&blocked_parent.join("snapshot"))
            .is_err()
    );
    assert!(store.backup_for_migration(&blocked_parent).is_err());
    assert_eq!(fs::read(&path).unwrap(), V1.as_bytes());
    assert_eq!(fs::read(backup(&path)).unwrap(), b"[]");
    assert_eq!(fs::read(blocked_parent).unwrap(), b"keep");
}

#[test]
fn migration_source_io_error_aborts_without_recovery_or_empty_copy() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("profiles.json");
    let destination = temp.path().join("migration");
    fs::write(&path, V1).unwrap();
    fs::create_dir(backup(&path)).unwrap();
    assert!(
        JsonProfileStore::new(&path)
            .backup_for_migration(&destination)
            .is_err()
    );
    assert_eq!(fs::read(&path).unwrap(), V1.as_bytes());
    assert!(!destination.exists());
}

#[test]
fn migration_rejects_normal_data_paths_even_when_missing() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("profiles.json");
    let store = JsonProfileStore::new(&path);
    assert!(store.backup_for_migration(&path).is_err());
    assert!(store.backup_for_migration(&backup(&path)).is_err());
    assert!(!path.exists());
    assert!(!backup(&path).exists());
}

#[test]
fn invalid_new_profile_does_not_create_or_recover_any_files() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("profiles.json");
    let store = JsonProfileStore::new(&path);
    let mut invalid = profile("bad", "bad");
    invalid.master_volume = f32::NAN;
    assert!(store.save(&invalid).is_err());
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
    fs::write(&path, b"broken").unwrap();
    fs::write(backup(&path), V1).unwrap();
    assert!(store.save(&invalid).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"broken");
    assert_eq!(fs::read(backup(&path)).unwrap(), V1.as_bytes());
}

#[test]
fn concurrent_deletes_and_saves_keep_all_unrelated_profiles() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("profiles.json");
    let store = Arc::new(JsonProfileStore::new(&path));
    for index in 0..16 {
        store
            .save(&profile(&format!("old-{index}"), "old"))
            .unwrap();
    }
    let barrier = Arc::new(Barrier::new(4));
    let workers: Vec<_> = (0..4)
        .map(|worker| {
            let store = store.clone();
            let barrier = barrier.clone();
            thread::spawn(move || {
                barrier.wait();
                for index in 0..4 {
                    store
                        .delete(&format!("old-{}", worker * 4 + index))
                        .unwrap();
                    JsonProfileStore::new(store.path())
                        .save(&profile(&format!("new-{worker}-{index}"), "new"))
                        .unwrap();
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    let profiles = store.load().unwrap();
    assert_eq!(profiles.len(), 16);
    assert!(
        profiles
            .iter()
            .all(|profile| profile.device_id.starts_with("new-"))
    );
}

#[test]
fn migration_never_reuses_an_existing_empty_directory() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("profiles.json");
    fs::write(&path, V1).unwrap();
    let destination = temp.path().join("snapshot");
    fs::create_dir(&destination).unwrap();
    let error = JsonProfileStore::new(&path)
        .backup_for_migration(&destination)
        .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::AlreadyExists);
    assert_eq!(fs::read_dir(&destination).unwrap().count(), 0);
    assert_eq!(fs::read(&path).unwrap(), V1.as_bytes());
}

#[cfg(windows)]
#[test]
fn unreadable_primary_is_an_error_even_with_valid_backup() {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;

    let temp = tempdir().unwrap();
    let path = temp.path().join("profiles.json");
    fs::write(&path, V1).unwrap();
    fs::write(backup(&path), b"[]").unwrap();
    let held = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(&path)
        .unwrap();
    let store = JsonProfileStore::new(&path);
    assert!(store.load().is_err());
    assert!(store.save(&profile("new", "new")).is_err());
    assert!(store.delete("DEVICE-A").is_err());
    assert!(
        store
            .backup_for_migration(&temp.path().join("snapshot"))
            .is_err()
    );
    assert_eq!(fs::read(backup(&path)).unwrap(), b"[]");
    drop(held);
    assert_eq!(fs::read(&path).unwrap(), V1.as_bytes());
}

#[cfg(windows)]
#[test]
fn unreadable_backup_aborts_recovery_without_writing_primary() {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;

    let temp = tempdir().unwrap();
    let path = temp.path().join("profiles.json");
    fs::write(&path, b"broken").unwrap();
    fs::write(backup(&path), V1).unwrap();
    let held = OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(backup(&path))
        .unwrap();
    let store = JsonProfileStore::new(&path);
    assert!(store.load().is_err());
    assert!(store.save(&profile("new", "new")).is_err());
    assert!(store.delete("DEVICE-A").is_err());
    assert_eq!(fs::read(&path).unwrap(), b"broken");
    drop(held);
    assert_eq!(fs::read(backup(&path)).unwrap(), V1.as_bytes());
}

#[cfg(windows)]
#[test]
fn failed_backup_replace_retains_both_original_files_and_cleans_temporary_files() {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;

    let temp = tempdir().unwrap();
    let path = temp.path().join("profiles.json");
    fs::write(&path, V1).unwrap();
    fs::write(backup(&path), b"[]").unwrap();
    let held = OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(backup(&path))
        .unwrap();
    let store = JsonProfileStore::new(&path);
    assert!(store.save(&profile("new", "new")).is_err());
    assert!(store.delete("DEVICE-A").is_err());
    assert_eq!(fs::read(&path).unwrap(), V1.as_bytes());
    assert_eq!(fs::read(backup(&path)).unwrap(), b"[]");
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 3);
    drop(held);
    store.save(&profile("new", "new")).unwrap();
    assert_eq!(store.load().unwrap().len(), 2);
}

#[cfg(windows)]
#[test]
fn failed_primary_replace_preserves_primary_and_valid_backup() {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;

    let temp = tempdir().unwrap();
    let path = temp.path().join("profiles.json");
    fs::write(&path, V1).unwrap();
    let held = OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&path)
        .unwrap();
    let store = JsonProfileStore::new(&path);
    assert!(store.save(&profile("new", "new")).is_err());
    assert_eq!(fs::read(&path).unwrap(), V1.as_bytes());
    assert_eq!(fs::read(backup(&path)).unwrap(), V1.as_bytes());
    drop(held);
    store.save(&profile("new", "new")).unwrap();
    assert_eq!(store.load().unwrap().len(), 2);
}

#[cfg(windows)]
#[test]
fn failed_recovery_replace_returns_error_and_preserves_backup() {
    use std::fs::OpenOptions;
    use std::os::windows::fs::OpenOptionsExt;

    let temp = tempdir().unwrap();
    let path = temp.path().join("profiles.json");
    fs::write(&path, b"broken").unwrap();
    fs::write(backup(&path), V1).unwrap();
    let held = OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&path)
        .unwrap();
    let store = JsonProfileStore::new(&path);
    assert!(store.load().is_err());
    assert!(store.save(&profile("new", "new")).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"broken");
    assert_eq!(fs::read(backup(&path)).unwrap(), V1.as_bytes());
    drop(held);
    assert_eq!(store.load().unwrap().len(), 1);
}
