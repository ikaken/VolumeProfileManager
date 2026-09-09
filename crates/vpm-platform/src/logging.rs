use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::Mutex,
};

#[derive(Clone, Copy, Debug)]
pub enum Level {
    Info,
    Warning,
    Error,
}

impl Level {
    fn label(self) -> &'static str {
        match self {
            Self::Info => "INFO",
            Self::Warning => "WARNING",
            Self::Error => "ERROR",
        }
    }
}

pub struct DailyFileLogger {
    directory: PathBuf,
    gate: Mutex<()>,
}

impl DailyFileLogger {
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
            gate: Mutex::new(()),
        }
    }

    pub fn directory(&self) -> &Path {
        &self.directory
    }

    pub fn write(&self, level: Level, message: &str) -> io::Result<()> {
        self.write_at(Utc::now(), level, message)
    }

    pub fn write_at(
        &self,
        timestamp: DateTime<Utc>,
        level: Level,
        message: &str,
    ) -> io::Result<()> {
        let _guard = self
            .gate
            .lock()
            .map_err(|_| io::Error::other("log lock poisoned"))?;
        fs::create_dir_all(&self.directory)?;
        let path = self
            .directory
            .join(format!("vpm-tray-{}.log", timestamp.format("%Y%m%d")));
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(path)?;
        file.lock()?;
        writeln!(
            file,
            "{} [{}] {}",
            timestamp.to_rfc3339_opts(SecondsFormat::Millis, true),
            level.label(),
            message.replace('\r', "\\r").replace('\n', "\\n")
        )?;
        file.flush()?;
        drop(file);
        self.prune()
    }

    fn prune(&self) -> io::Result<()> {
        let mut files = Vec::new();
        for entry in fs::read_dir(&self.directory)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let name = entry.file_name();
            let Some(date) = name
                .to_str()
                .and_then(|name| name.strip_prefix("vpm-tray-"))
                .and_then(|name| name.strip_suffix(".log"))
                .filter(|date| date.len() == 8 && date.bytes().all(|b| b.is_ascii_digit()))
                .and_then(|date| NaiveDate::parse_from_str(date, "%Y%m%d").ok())
            else {
                continue;
            };
            files.push((date, entry.path()));
        }
        files.sort_by(|a, b| b.0.cmp(&a.0));
        for (_, path) in files.into_iter().skip(30) {
            match fs::remove_file(path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }
}
