#![windows_subsystem = "windows"]

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::sync::{Arc, Mutex};
use vpm_core::SystemClock;
use vpm_platform::audio::{
    AudioDeviceAdapter, DeviceMonitorOrchestrator, ProfileCaptureService, WindowsAudioDeviceAdapter,
};
use vpm_platform::logging::{DailyFileLogger, Level};
use vpm_platform::persistence::JsonProfileStore;
use vpm_platform::storage::StorageLayout;
use vpm_platform::system::{SingleInstanceMutex, StartupRegistration};
use vpm_tray::tray::{
    CMD_EXIT, CMD_STATUS, CMD_TOGGLE_STARTUP, CMD_UPDATE_PROFILE, TrayIconWindow,
};

fn show_error_dialog(title: &str, message: &str) {
    let wide_title: Vec<u16> = OsStr::new(title)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let wide_message: Vec<u16> = OsStr::new(message)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::MessageBoxW(
            None,
            windows::core::PCWSTR(wide_message.as_ptr()),
            windows::core::PCWSTR(wide_title.as_ptr()),
            windows::Win32::UI::WindowsAndMessaging::MB_OK
                | windows::Win32::UI::WindowsAndMessaging::MB_ICONERROR,
        );
    }
}

fn main() {
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| std::path::PathBuf::from("."));

    let storage = StorageLayout::detect(&exe_dir);

    if let Err(e) = storage.ensure_writable() {
        let msg = format!(
            "データ保存先ディレクトリへの書き込みに失敗しました。\n\n保存先: {}\nエラー: {}",
            storage.base_dir().display(),
            e
        );
        show_error_dialog("VolumeProfileManager - 起動エラー", &msg);
        return;
    }

    let log_dir = storage.log_dir();
    let logger = Arc::new(DailyFileLogger::new(log_dir));
    logger
        .write(Level::Info, "VolumeProfileManager TrayApp starting...")
        .ok();

    let _instance_guard = match SingleInstanceMutex::acquire_default() {
        Ok(Some(guard)) => guard,
        Ok(None) => {
            logger
                .write(
                    Level::Warning,
                    "Another instance is already running. Exiting.",
                )
                .ok();
            return;
        }
        Err(e) => {
            logger
                .write(
                    Level::Error,
                    &format!("Failed to acquire single instance mutex: {e}"),
                )
                .ok();
            return;
        }
    };

    let profile_path = storage.profile_path();
    let store = Arc::new(JsonProfileStore::new(profile_path));
    let adapter = Arc::new(WindowsAudioDeviceAdapter::new());
    let clock = Arc::new(SystemClock::default());

    let mut orchestrator = DeviceMonitorOrchestrator::new(
        adapter.clone(),
        store.clone(),
        clock.clone(),
        Some(std::time::Duration::from_millis(800)),
        Some(std::time::Duration::from_secs(3)),
    );

    let tray_icon_cell: Arc<Mutex<Option<Arc<TrayIconWindow>>>> = Arc::new(Mutex::new(None));

    let tray_cell_for_events = tray_icon_cell.clone();
    let logger_for_events = logger.clone();
    orchestrator.set_on_profile_applied(move |event| {
        let mute_suffix = if event.is_muted {
            " (ミュート)"
        } else {
            ""
        };
        let message = if event.is_new_profile {
            format!(
                "新しいプロファイルを作成しました: {:.0}%{}",
                event.master_volume * 100.0,
                mute_suffix
            )
        } else {
            format!(
                "音量 {:.0}%{} を適用しました",
                event.master_volume * 100.0,
                mute_suffix
            )
        };

        logger_for_events
            .write(
                Level::Info,
                &format!(
                    "Profile applied notification: {} -> {}",
                    event.device_name, message
                ),
            )
            .ok();

        if let Ok(guard) = tray_cell_for_events.lock()
            && let Some(tray) = guard.as_ref()
        {
            tray.show_balloon(&event.device_name, &message);
        }
    });

    if let Err(e) = orchestrator.start() {
        logger
            .write(Level::Error, &format!("Failed to start orchestrator: {e}"))
            .ok();
        return;
    }

    let orchestrator_cell = Arc::new(Mutex::new(Some(orchestrator)));

    let adapter_for_cmd = adapter.clone();
    let store_for_cmd = store.clone();
    let clock_for_cmd = clock.clone();
    let logger_for_cmd = logger.clone();
    let orchestrator_for_cmd = orchestrator_cell.clone();
    let tray_cell_for_cmd = tray_icon_cell.clone();

    let tray_window = match TrayIconWindow::new(move |cmd| match cmd {
        CMD_STATUS => {
            let device_name = match adapter_for_cmd.get_default_playback_device() {
                Ok(Some(dev)) => dev.device_name,
                _ => "(unknown device)".to_string(),
            };
            let vol = adapter_for_cmd.get_master_volume(None).unwrap_or(0.0);
            let mute = adapter_for_cmd.get_mute(None).unwrap_or(false);
            let mute_str = if mute { "ON" } else { "OFF" };
            let msg = format!("音量: {:.0}% / ミュート: {}", vol * 100.0, mute_str);

            if let Ok(guard) = tray_cell_for_cmd.lock()
                && let Some(tray) = guard.as_ref()
            {
                tray.show_balloon(&device_name, &msg);
            }
        }
        CMD_UPDATE_PROFILE => {
            let capture = ProfileCaptureService::new(
                adapter_for_cmd.clone(),
                store_for_cmd.clone(),
                clock_for_cmd.clone(),
            );
            match capture.capture_current_profile(None) {
                Ok(Some(profile)) => {
                    let mute_str = if profile.is_muted { "ON" } else { "OFF" };
                    let msg = format!(
                        "プロファイルを更新しました。音量: {:.0}% / ミュート: {}",
                        profile.master_volume * 100.0,
                        mute_str
                    );
                    if let Ok(guard) = tray_cell_for_cmd.lock()
                        && let Some(tray) = guard.as_ref()
                    {
                        tray.show_balloon(&profile.device_name, &msg);
                    }
                }
                _ => {
                    if let Ok(guard) = tray_cell_for_cmd.lock()
                        && let Some(tray) = guard.as_ref()
                    {
                        tray.show_balloon(
                                "VolumeProfileManager",
                                "現在の再生デバイスを特定できなかったため、プロファイルを更新できませんでした",
                            );
                    }
                }
            }
        }
        CMD_TOGGLE_STARTUP => {
            if let Ok(exe) = std::env::current_exe() {
                match StartupRegistration::toggle(&exe) {
                    Ok(registered) => {
                        let msg = if registered {
                            "スタートアップに登録しました"
                        } else {
                            "スタートアップ登録を解除しました"
                        };
                        logger_for_cmd
                            .write(
                                Level::Info,
                                &format!("Startup registration toggled: {registered}"),
                            )
                            .ok();
                        if let Ok(guard) = tray_cell_for_cmd.lock()
                            && let Some(tray) = guard.as_ref()
                        {
                            tray.show_balloon("VolumeProfileManager", msg);
                        }
                    }
                    Err(e) => {
                        logger_for_cmd
                            .write(
                                Level::Error,
                                &format!("Failed to toggle startup registration: {e}"),
                            )
                            .ok();
                    }
                }
            }
        }
        CMD_EXIT => {
            if let Ok(mut guard) = orchestrator_for_cmd.lock()
                && let Some(mut orch) = guard.take()
            {
                orch.stop();
            }
            if let Ok(guard) = tray_cell_for_cmd.lock()
                && let Some(tray) = guard.as_ref()
            {
                tray.request_exit();
            }
        }
        _ => {}
    }) {
        Ok(window) => Arc::new(window),
        Err(e) => {
            logger
                .write(Level::Error, &format!("Failed to create tray window: {e}"))
                .ok();
            if let Ok(mut guard) = orchestrator_cell.lock()
                && let Some(mut orch) = guard.take()
            {
                orch.stop();
            }
            return;
        }
    };

    if let Ok(mut guard) = tray_icon_cell.lock() {
        *guard = Some(tray_window.clone());
    }

    TrayIconWindow::run_message_loop();

    if let Ok(mut guard) = orchestrator_cell.lock()
        && let Some(mut orch) = guard.take()
    {
        orch.stop();
    }

    logger
        .write(Level::Info, "VolumeProfileManager TrayApp exiting.")
        .ok();
}
