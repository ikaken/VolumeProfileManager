use chrono::{TimeZone, Utc};
use std::{fs, sync::Arc, thread};
use tempfile::tempdir;
use vpm_platform::logging::{DailyFileLogger, Level};

#[test]
fn creates_daily_logs_and_escapes_multiline_messages() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("logs");
    let logger = DailyFileLogger::new(&path);
    let first = Utc.with_ymd_and_hms(2026, 9, 5, 12, 0, 0).unwrap();
    logger
        .write_at(first, Level::Info, "音量を保存\n次の行\r")
        .unwrap();
    logger
        .write_at(first, Level::Warning, "device unavailable")
        .unwrap();
    logger
        .write_at(first + chrono::Duration::days(1), Level::Error, "failed")
        .unwrap();
    let text = fs::read_to_string(path.join("vpm-tray-20260905.log")).unwrap();
    assert_eq!(text.lines().count(), 2);
    assert!(text.contains("[INFO] 音量を保存\\n次の行\\r"));
    assert!(text.contains("[WARNING] device unavailable"));
    assert!(path.join("vpm-tray-20260906.log").exists());
}

#[test]
fn retains_thirty_daily_files_without_removing_other_files_or_directories() {
    let temp = tempdir().unwrap();
    let logger = DailyFileLogger::new(temp.path());
    let first = Utc.with_ymd_and_hms(2026, 7, 1, 0, 0, 0).unwrap();
    fs::write(temp.path().join("other.log"), "keep").unwrap();
    fs::write(temp.path().join("vpm-tray-20260230.log"), "keep").unwrap();
    fs::create_dir(temp.path().join("vpm-tray-20000101.log")).unwrap();
    for day in 0..32 {
        logger
            .write_at(first + chrono::Duration::days(day), Level::Info, "tick")
            .unwrap();
    }
    assert!(!temp.path().join("vpm-tray-20260701.log").exists());
    assert!(!temp.path().join("vpm-tray-20260702.log").exists());
    assert!(temp.path().join("vpm-tray-20260703.log").exists());
    assert!(temp.path().join("vpm-tray-20260801.log").exists());
    assert!(temp.path().join("other.log").exists());
    assert!(temp.path().join("vpm-tray-20260230.log").exists());
    assert!(temp.path().join("vpm-tray-20000101.log").is_dir());
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 33);
}

#[test]
fn concurrent_writes_preserve_complete_lines() {
    let temp = tempdir().unwrap();
    let logger = Arc::new(DailyFileLogger::new(temp.path()));
    let timestamp = Utc.with_ymd_and_hms(2026, 9, 5, 0, 0, 0).unwrap();
    let workers: Vec<_> = (0..4)
        .map(|worker| {
            let logger = logger.clone();
            thread::spawn(move || {
                for item in 0..25 {
                    logger
                        .write_at(timestamp, Level::Info, &format!("{worker}:{item}"))
                        .unwrap();
                }
            })
        })
        .collect();
    for worker in workers {
        worker.join().unwrap();
    }
    let text = fs::read_to_string(temp.path().join("vpm-tray-20260905.log")).unwrap();
    assert_eq!(text.lines().count(), 100);
    assert!(
        text.lines()
            .all(|line| line.starts_with("2026-09-05T00:00:00.000Z [INFO] "))
    );
}

#[test]
fn invalid_directory_reports_error_without_changing_existing_data() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("occupied");
    fs::write(&path, "keep").unwrap();
    let logger = DailyFileLogger::new(&path);
    assert!(logger.write(Level::Error, "failed").is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), "keep");
}
