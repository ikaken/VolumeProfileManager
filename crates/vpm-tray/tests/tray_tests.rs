use vpm_tray::tray::{CMD_EXIT, CMD_STATUS, CMD_TOGGLE_STARTUP, CMD_UPDATE_PROFILE};
use vpm_tray::{APP_NAME, APP_VERSION, version_string};

#[test]
fn test_version_and_metadata() {
    assert_eq!(APP_NAME, "VolumeProfileManager");
    assert!(!APP_VERSION.is_empty());
    assert_eq!(version_string(), format!("v{APP_VERSION}"));
}

#[test]
fn test_command_ids_match_v1_constants() {
    assert_eq!(CMD_STATUS, 1001);
    assert_eq!(CMD_TOGGLE_STARTUP, 1002);
    assert_eq!(CMD_EXIT, 1003);
    assert_eq!(CMD_UPDATE_PROFILE, 1004);
}
