use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// アプリケーションの動作モード。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StorageMode {
    /// 通常モード: `%LOCALAPPDATA%\VolumeProfileManager` をデータ保存先とする。
    Normal,
    /// ポータブルモード: 実行ファイルと同じディレクトリの `data` 配下をデータ保存先とする。
    Portable,
}

/// ストレージレイアウト。プロファイル・ログ等の保存先パスを提供する。
#[derive(Clone, Debug)]
pub struct StorageLayout {
    mode: StorageMode,
    base_dir: PathBuf,
}

const PORTABLE_FLAG_NAME: &str = "portable.flag";
const PORTABLE_DATA_DIR: &str = "data";
const APP_DATA_DIR_NAME: &str = "VolumeProfileManager";

impl StorageLayout {
    /// 実行ファイルのディレクトリを基に `portable.flag` の有無でモードを判定し、
    /// ストレージレイアウトを構築する。
    pub fn detect(exe_dir: &Path) -> Self {
        if exe_dir.join(PORTABLE_FLAG_NAME).exists() {
            Self {
                mode: StorageMode::Portable,
                base_dir: exe_dir.join(PORTABLE_DATA_DIR),
            }
        } else {
            let base = std::env::var("LOCALAPPDATA")
                .map(|local| PathBuf::from(local).join(APP_DATA_DIR_NAME))
                .unwrap_or_else(|_| PathBuf::from(".").join(APP_DATA_DIR_NAME));
            Self {
                mode: StorageMode::Normal,
                base_dir: base,
            }
        }
    }

    /// テスト用: 明示的にモードとベースディレクトリを指定して構築する。
    #[cfg(test)]
    pub fn with_mode(mode: StorageMode, base_dir: impl Into<PathBuf>) -> Self {
        Self {
            mode,
            base_dir: base_dir.into(),
        }
    }

    /// 現在のストレージモードを返す。
    pub fn mode(&self) -> StorageMode {
        self.mode
    }

    /// データ保存先のベースディレクトリを返す。
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// プロファイル保存ファイル (`profiles.json`) のパスを返す。
    pub fn profile_path(&self) -> PathBuf {
        self.base_dir.join("profiles.json")
    }

    /// ログ保存ディレクトリのパスを返す。
    pub fn log_dir(&self) -> PathBuf {
        self.base_dir.join("logs")
    }

    /// ベースディレクトリへの書き込みが可能であることを検証する。
    ///
    /// ディレクトリが存在しない場合は作成を試み、一時ファイルの書き込みと削除で
    /// 書き込み権限を確認する。検証に失敗した場合は `io::Error` を返す。
    pub fn ensure_writable(&self) -> io::Result<()> {
        fs::create_dir_all(&self.base_dir)?;
        let probe = self.base_dir.join(".vpm_write_probe");
        fs::write(&probe, b"probe")?;
        fs::remove_file(&probe)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn detect_normal_mode_without_portable_flag() {
        let dir = TempDir::new().unwrap();
        let layout = StorageLayout::detect(dir.path());
        assert_eq!(layout.mode(), StorageMode::Normal);
    }

    #[test]
    fn detect_portable_mode_with_portable_flag() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("portable.flag"), b"").unwrap();
        let layout = StorageLayout::detect(dir.path());
        assert_eq!(layout.mode(), StorageMode::Portable);
        assert_eq!(layout.base_dir(), dir.path().join("data"));
        assert_eq!(
            layout.profile_path(),
            dir.path().join("data").join("profiles.json")
        );
        assert_eq!(layout.log_dir(), dir.path().join("data").join("logs"));
    }

    #[test]
    fn detect_portable_mode_with_nonempty_flag_file() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("portable.flag"), b"anything").unwrap();
        let layout = StorageLayout::detect(dir.path());
        assert_eq!(layout.mode(), StorageMode::Portable);
    }

    #[test]
    fn ensure_writable_succeeds_in_writable_directory() {
        let dir = TempDir::new().unwrap();
        let layout = StorageLayout::with_mode(StorageMode::Portable, dir.path().join("data"));
        assert!(layout.ensure_writable().is_ok());
        assert!(dir.path().join("data").is_dir());
        // probe file should be cleaned up
        assert!(!dir.path().join("data").join(".vpm_write_probe").exists());
    }

    #[test]
    fn ensure_writable_creates_nested_data_directory() {
        let dir = TempDir::new().unwrap();
        let nested = dir.path().join("deep").join("nested").join("data");
        let layout = StorageLayout::with_mode(StorageMode::Portable, &nested);
        assert!(layout.ensure_writable().is_ok());
        assert!(nested.is_dir());
    }

    #[test]
    fn normal_mode_paths_do_not_include_data_subdir() {
        let dir = TempDir::new().unwrap();
        let base = dir.path().join("AppData").join("VolumeProfileManager");
        let layout = StorageLayout::with_mode(StorageMode::Normal, &base);
        assert_eq!(layout.profile_path(), base.join("profiles.json"));
        assert_eq!(layout.log_dir(), base.join("logs"));
    }
}
