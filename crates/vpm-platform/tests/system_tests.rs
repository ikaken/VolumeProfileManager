use std::path::Path;
use vpm_platform::system::{ProcessChecker, SingleInstanceMutex, StartupRegistration};
use windows::Win32::System::Registry::HKEY_CURRENT_USER;

#[test]
fn test_single_instance_mutex_exclusion() {
    let mutex_name = format!("Local\\VpmTest_SingleInstance_{}", std::process::id());

    // 1. Acquire mutex first time -> Success
    let guard1 = SingleInstanceMutex::try_acquire(&mutex_name)
        .expect("Failed to call try_acquire")
        .expect("First mutex acquire should succeed");
    assert_eq!(guard1.name(), &mutex_name);

    // 2. Second attempt while guard1 is held -> Returns None (already running)
    let guard2 = SingleInstanceMutex::try_acquire(&mutex_name).expect("Failed to call try_acquire");
    assert!(
        guard2.is_none(),
        "Second acquire should return None while held"
    );

    // 3. Drop guard1 -> Mutex handle released
    drop(guard1);

    // 4. Third attempt after dropping -> Success
    let guard3 = SingleInstanceMutex::try_acquire(&mutex_name)
        .expect("Failed to call try_acquire")
        .expect("Mutex acquire after drop should succeed");
    assert_eq!(guard3.name(), &mutex_name);
}

#[test]
fn test_single_instance_mutex_different_names() {
    let name1 = format!("Local\\VpmTest_Different1_{}", std::process::id());
    let name2 = format!("Local\\VpmTest_Different2_{}", std::process::id());

    let guard1 = SingleInstanceMutex::try_acquire(&name1).unwrap().unwrap();
    let guard2 = SingleInstanceMutex::try_acquire(&name2).unwrap().unwrap();

    assert_eq!(guard1.name(), &name1);
    assert_eq!(guard2.name(), &name2);
}

#[test]
fn test_startup_registration_isolated_subkey() {
    let test_subkey = format!(
        r"Software\VolumeProfileManagerTest_{}\Run",
        std::process::id()
    );
    let test_value = "TestApp";
    let exe_path =
        Path::new(r"C:\Program Files\VolumeProfileManager\VolumeProfileManager.TrayApp.exe");

    // Clean initial state
    let _ = StartupRegistration::unregister_in(HKEY_CURRENT_USER, &test_subkey, test_value);

    // 1. Check is_registered -> false
    assert!(
        !StartupRegistration::is_registered_in(HKEY_CURRENT_USER, &test_subkey, test_value)
            .unwrap()
    );

    // 2. Register
    StartupRegistration::register_in(HKEY_CURRENT_USER, &test_subkey, test_value, exe_path)
        .unwrap();
    assert!(
        StartupRegistration::is_registered_in(HKEY_CURRENT_USER, &test_subkey, test_value).unwrap()
    );

    // 3. Toggle -> unregisters, returns false
    let toggled_off =
        StartupRegistration::toggle_in(HKEY_CURRENT_USER, &test_subkey, test_value, exe_path)
            .unwrap();
    assert!(!toggled_off);
    assert!(
        !StartupRegistration::is_registered_in(HKEY_CURRENT_USER, &test_subkey, test_value)
            .unwrap()
    );

    // 4. Toggle -> registers, returns true
    let toggled_on =
        StartupRegistration::toggle_in(HKEY_CURRENT_USER, &test_subkey, test_value, exe_path)
            .unwrap();
    assert!(toggled_on);
    assert!(
        StartupRegistration::is_registered_in(HKEY_CURRENT_USER, &test_subkey, test_value).unwrap()
    );

    // 5. Unregister
    StartupRegistration::unregister_in(HKEY_CURRENT_USER, &test_subkey, test_value).unwrap();
    assert!(
        !StartupRegistration::is_registered_in(HKEY_CURRENT_USER, &test_subkey, test_value)
            .unwrap()
    );
}

#[test]
fn test_process_checker_finds_system_process() {
    // Current process or standard Windows processes (like explorer.exe or svchost.exe) should be found
    let explorer_pids = ProcessChecker::find_process_ids("explorer.exe").unwrap();
    let svchost_pids = ProcessChecker::find_process_ids("svchost.exe").unwrap();

    // At least one of explorer or svchost is always running on Windows
    assert!(!explorer_pids.is_empty() || !svchost_pids.is_empty());

    let non_existent =
        ProcessChecker::find_process_ids("non_existent_process_123456789.exe").unwrap();
    assert!(non_existent.is_empty());
    assert!(!ProcessChecker::is_process_running("non_existent_process_123456789.exe").unwrap());
}
