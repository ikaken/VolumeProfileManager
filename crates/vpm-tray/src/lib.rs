pub mod tray;

pub const APP_NAME: &str = "VolumeProfileManager";
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn version_string() -> String {
    format!("v{}", APP_VERSION)
}
