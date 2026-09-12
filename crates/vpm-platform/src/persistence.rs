use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use vpm_core::{Clock, ProfileStore, SystemClock, VolumeProfile, ordinal_ignore_case_eq};

pub struct JsonProfileStore {
    path: PathBuf,
    clock: Box<dyn Clock + Send + Sync>,
}

struct LoadedProfiles {
    profiles: Vec<VolumeProfile>,
    bytes: Option<Vec<u8>>,
}

impl JsonProfileStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self::with_clock(path, SystemClock::default())
    }

    pub fn with_clock(path: impl Into<PathBuf>, clock: impl Clock + Send + Sync + 'static) -> Self {
        Self {
            path: path.into(),
            clock: Box::new(clock),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn get_by_identifier(&self, identifier: &str) -> io::Result<Option<VolumeProfile>> {
        Ok(self.load()?.into_iter().find(|profile| {
            ordinal_ignore_case_eq(&profile.device_id, identifier)
                || ordinal_ignore_case_eq(&profile.device_name, identifier)
        }))
    }

    pub fn backup_for_migration(&self, destination: &Path) -> io::Result<()> {
        let _lock = self.lock()?;
        let destination_name = destination.file_name().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "snapshot requires a new directory",
            )
        })?;
        let destination_path = fs::canonicalize(parent(destination))?.join(destination_name);
        let source_directory = fs::canonicalize(parent(&self.path))?;
        for source in [&self.path, &self.backup_path(), &self.lock_path()] {
            let source_name = source.file_name().ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "profile path requires a file name",
                )
            })?;
            if paths_equal(&destination_path, &source_directory.join(source_name)) {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "snapshot must be independent of profile, backup and lock files",
                ));
            }
        }
        let sources = [self.path.clone(), self.backup_path()];
        let bytes = sources
            .iter()
            .map(|path| read_optional(path))
            .collect::<io::Result<Vec<_>>>()?;
        fs::create_dir(destination)?;
        for (source, bytes) in sources.iter().zip(bytes.iter()) {
            if let Some(bytes) = bytes {
                let target = destination.join(source.file_name().ok_or_else(|| {
                    io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "profile path requires a file name",
                    )
                })?);
                let temporary = stage(&target, bytes)?;
                temporary
                    .persist_noclobber(&target)
                    .map_err(|error| error.error)?;
                verify_bytes(&target, bytes)?;
            }
        }
        Ok(())
    }

    fn backup_path(&self) -> PathBuf {
        append_suffix(&self.path, ".bak")
    }

    fn lock_path(&self) -> PathBuf {
        append_suffix(&self.path, ".lock")
    }

    fn lock(&self) -> io::Result<File> {
        if self.path.file_name().is_none() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "profile path requires a file name",
            ));
        }
        fs::create_dir_all(parent(&self.path))?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.lock_path())?;
        file.lock()?;
        Ok(file)
    }

    fn load_locked(&self) -> io::Result<LoadedProfiles> {
        let primary = read_optional(&self.path)?;
        let primary_error = if let Some(bytes) = primary {
            match decode(&bytes) {
                Ok(profiles) => {
                    return Ok(LoadedProfiles {
                        profiles,
                        bytes: Some(bytes),
                    });
                }
                Err(error) => Some(error),
            }
        } else {
            None
        };
        match read_optional(&self.backup_path())? {
            Some(bytes) => {
                decode(&bytes)?;
                replace(&self.path, &bytes)?;
                let recovered = fs::read(&self.path)?;
                if recovered != bytes {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        "recovered profiles failed byte verification",
                    ));
                }
                let profiles = decode(&recovered)?;
                Ok(LoadedProfiles {
                    profiles,
                    bytes: Some(recovered),
                })
            }
            None => match primary_error {
                Some(error) => Err(error),
                None => Ok(LoadedProfiles {
                    profiles: Vec::new(),
                    bytes: None,
                }),
            },
        }
    }

    fn write_locked(&self, loaded: LoadedProfiles) -> io::Result<()> {
        if loaded
            .profiles
            .iter()
            .any(|profile| !profile.master_volume.is_finite())
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "MasterVolume must be finite",
            ));
        }
        let bytes = serde_json::to_vec_pretty(&loaded.profiles)
            .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
        if decode(&bytes)? != loaded.profiles {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "serialized profiles failed round-trip verification",
            ));
        }
        let temporary = stage(&self.path, &bytes)?;
        if let Some(previous) = loaded.bytes {
            replace(&self.backup_path(), &previous)?;
        }
        temporary.persist(&self.path).map_err(|error| error.error)?;
        Ok(())
    }
}

impl ProfileStore for JsonProfileStore {
    fn load(&self) -> io::Result<Vec<VolumeProfile>> {
        let _lock = self.lock()?;
        Ok(self.load_locked()?.profiles)
    }

    fn save(&self, profile: &VolumeProfile) -> io::Result<()> {
        if !profile.master_volume.is_finite() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "MasterVolume must be finite",
            ));
        }
        let _lock = self.lock()?;
        let mut loaded = self.load_locked()?;
        if let Some(existing) = loaded
            .profiles
            .iter_mut()
            .find(|existing| ordinal_ignore_case_eq(&existing.device_id, &profile.device_id))
        {
            existing.device_name = profile.device_name.clone();
            existing.master_volume = profile.master_volume;
            existing.is_muted = profile.is_muted;
            existing.last_applied = profile.last_applied;
        } else {
            let mut profile = profile.clone();
            profile.created_at = self.clock.now();
            loaded.profiles.push(profile);
        }
        self.write_locked(loaded)
    }

    fn delete(&self, identifier: &str) -> io::Result<()> {
        let _lock = self.lock()?;
        let mut loaded = self.load_locked()?;
        loaded.profiles.retain(|profile| {
            !ordinal_ignore_case_eq(&profile.device_id, identifier)
                && !ordinal_ignore_case_eq(&profile.device_name, identifier)
        });
        self.write_locked(loaded)
    }
}

fn parent(path: &Path) -> &Path {
    path.parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut result = path.as_os_str().to_os_string();
    result.push(suffix);
    result.into()
}

fn read_optional(path: &Path) -> io::Result<Option<Vec<u8>>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn decode(bytes: &[u8]) -> io::Result<Vec<VolumeProfile>> {
    let bytes = bytes.strip_prefix(b"\xef\xbb\xbf").unwrap_or(bytes);
    let profiles: Vec<VolumeProfile> = serde_json::from_slice(bytes)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    if profiles
        .iter()
        .any(|profile| !profile.master_volume.is_finite())
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "MasterVolume must be finite",
        ));
    }
    Ok(profiles)
}

fn verify_bytes(path: &Path, expected: &[u8]) -> io::Result<()> {
    if fs::read(path)? != expected {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "file failed byte verification",
        ));
    }
    Ok(())
}

fn stage(path: &Path, bytes: &[u8]) -> io::Result<NamedTempFile> {
    let mut temporary = NamedTempFile::new_in(parent(path))?;
    temporary.write_all(bytes)?;
    temporary.as_file().sync_all()?;
    verify_bytes(temporary.path(), bytes)?;
    Ok(temporary)
}

fn replace(path: &Path, bytes: &[u8]) -> io::Result<()> {
    stage(path, bytes)?
        .persist(path)
        .map_err(|error| error.error)?;
    Ok(())
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    if cfg!(windows) {
        ordinal_ignore_case_eq(&left.to_string_lossy(), &right.to_string_lossy())
    } else {
        left == right
    }
}
