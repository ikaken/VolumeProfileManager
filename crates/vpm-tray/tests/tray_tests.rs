use vpm_tray::notification::compute_popup_rect;
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

#[test]
fn test_popup_rect_bottom_right_at_96dpi() {
    assert_eq!(
        compute_popup_rect((0, 0, 1920, 1040), 96),
        (1548, 932, 360, 96)
    );
}

#[test]
fn test_popup_rect_scales_with_dpi() {
    assert_eq!(
        compute_popup_rect((0, 0, 2880, 1560), 144),
        (2880 - 18 - 540, 1560 - 18 - 144, 540, 144)
    );
}

#[test]
fn test_popup_rect_respects_work_area_origin() {
    // 左上が (100, 50) の場合でも右下基準で計算されること
    assert_eq!(
        compute_popup_rect((100, 50, 1000, 800), 96),
        (1000 - 12 - 360, 800 - 12 - 96, 360, 96)
    );
}
