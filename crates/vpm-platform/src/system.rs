use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use windows::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_WRITE, REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey,
    RegCreateKeyExW, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
};
use windows::Win32::System::Threading::CreateMutexW;
use windows::core::PCWSTR;

pub const DEFAULT_SINGLE_INSTANCE_MUTEX_NAME: &str =
    "Global\\VolumeProfileManager_SingleInstance_Mutex";
pub const FALLBACK_SINGLE_INSTANCE_MUTEX_NAME: &str = "VolumeProfileManager_SingleInstance_Mutex";

#[derive(Debug)]
pub struct SingleInstanceGuard {
    handle: HANDLE,
    name: String,
}

impl SingleInstanceGuard {
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Drop for SingleInstanceGuard {
    fn drop(&mut self) {
        if !self.handle.is_invalid() {
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }
}

pub struct SingleInstanceMutex;

impl SingleInstanceMutex {
    pub fn try_acquire(name: &str) -> std::io::Result<Option<SingleInstanceGuard>> {
        let wide_name: Vec<u16> = OsStr::new(name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let handle = unsafe { CreateMutexW(None, true, PCWSTR(wide_name.as_ptr())) }
            .map_err(|e: windows::core::Error| std::io::Error::other(e.to_string()))?;

        if handle.is_invalid() {
            return Err(std::io::Error::last_os_error());
        }

        let last_err = unsafe { GetLastError() };
        if last_err == ERROR_ALREADY_EXISTS {
            unsafe {
                let _ = CloseHandle(handle);
            }
            return Ok(None);
        }

        Ok(Some(SingleInstanceGuard {
            handle,
            name: name.to_string(),
        }))
    }

    pub fn acquire_default() -> std::io::Result<Option<SingleInstanceGuard>> {
        match Self::try_acquire(DEFAULT_SINGLE_INSTANCE_MUTEX_NAME) {
            Ok(Some(guard)) => Ok(Some(guard)),
            Ok(None) => Ok(None),
            Err(_) => Self::try_acquire(FALLBACK_SINGLE_INSTANCE_MUTEX_NAME),
        }
    }
}

pub const DEFAULT_RUN_SUBKEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
pub const DEFAULT_RUN_VALUE_NAME: &str = "VolumeProfileManager";

pub struct StartupRegistration;

impl StartupRegistration {
    pub fn is_registered() -> std::io::Result<bool> {
        Self::is_registered_in(
            HKEY_CURRENT_USER,
            DEFAULT_RUN_SUBKEY,
            DEFAULT_RUN_VALUE_NAME,
        )
    }

    pub fn register(executable_path: &Path) -> std::io::Result<()> {
        Self::register_in(
            HKEY_CURRENT_USER,
            DEFAULT_RUN_SUBKEY,
            DEFAULT_RUN_VALUE_NAME,
            executable_path,
        )
    }

    pub fn unregister() -> std::io::Result<()> {
        Self::unregister_in(
            HKEY_CURRENT_USER,
            DEFAULT_RUN_SUBKEY,
            DEFAULT_RUN_VALUE_NAME,
        )
    }

    pub fn toggle(executable_path: &Path) -> std::io::Result<bool> {
        Self::toggle_in(
            HKEY_CURRENT_USER,
            DEFAULT_RUN_SUBKEY,
            DEFAULT_RUN_VALUE_NAME,
            executable_path,
        )
    }

    pub fn is_registered_in(root: HKEY, subkey: &str, value_name: &str) -> std::io::Result<bool> {
        let wide_subkey: Vec<u16> = OsStr::new(subkey)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let wide_value: Vec<u16> = OsStr::new(value_name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut hkey = HKEY::default();
        let status = unsafe {
            RegOpenKeyExW(
                root,
                PCWSTR(wide_subkey.as_ptr()),
                Some(0),
                KEY_READ,
                &mut hkey,
            )
        };
        if status.is_err() {
            return Ok(false);
        }

        let mut data_type = REG_SZ;
        let mut data_len = 0u32;
        let query_status = unsafe {
            RegQueryValueExW(
                hkey,
                PCWSTR(wide_value.as_ptr()),
                None,
                Some(&mut data_type),
                None,
                Some(&mut data_len),
            )
        };
        unsafe {
            let _ = RegCloseKey(hkey);
        }

        Ok(query_status.is_ok())
    }

    pub fn register_in(
        root: HKEY,
        subkey: &str,
        value_name: &str,
        executable_path: &Path,
    ) -> std::io::Result<()> {
        let wide_subkey: Vec<u16> = OsStr::new(subkey)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let wide_value: Vec<u16> = OsStr::new(value_name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut hkey = HKEY::default();
        let status = unsafe {
            RegCreateKeyExW(
                root,
                PCWSTR(wide_subkey.as_ptr()),
                Some(0),
                None,
                REG_OPTION_NON_VOLATILE,
                KEY_WRITE,
                None,
                &mut hkey,
                None,
            )
        };
        if status.is_err() {
            return Err(std::io::Error::other(format!(
                "Failed to open/create registry key: {status:?}"
            )));
        }

        let path_str = executable_path.to_string_lossy();
        let formatted_value = format!("\"{path_str}\"");
        let wide_val: Vec<u16> = OsStr::new(&formatted_value)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let byte_slice = unsafe {
            std::slice::from_raw_parts(
                wide_val.as_ptr() as *const u8,
                wide_val.len() * std::mem::size_of::<u16>(),
            )
        };

        let set_status = unsafe {
            RegSetValueExW(
                hkey,
                PCWSTR(wide_value.as_ptr()),
                Some(0),
                REG_SZ,
                Some(byte_slice),
            )
        };
        unsafe {
            let _ = RegCloseKey(hkey);
        }

        if set_status.is_err() {
            return Err(std::io::Error::other(format!(
                "Failed to set registry value: {set_status:?}"
            )));
        }

        Ok(())
    }

    pub fn unregister_in(root: HKEY, subkey: &str, value_name: &str) -> std::io::Result<()> {
        let wide_subkey: Vec<u16> = OsStr::new(subkey)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let wide_value: Vec<u16> = OsStr::new(value_name)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();

        let mut hkey = HKEY::default();
        let status = unsafe {
            RegOpenKeyExW(
                root,
                PCWSTR(wide_subkey.as_ptr()),
                Some(0),
                KEY_WRITE,
                &mut hkey,
            )
        };
        if status.is_err() {
            return Ok(());
        }

        let del_status = unsafe { RegDeleteValueW(hkey, PCWSTR(wide_value.as_ptr())) };
        unsafe {
            let _ = RegCloseKey(hkey);
        }

        let _ = del_status;
        Ok(())
    }

    pub fn toggle_in(
        root: HKEY,
        subkey: &str,
        value_name: &str,
        executable_path: &Path,
    ) -> std::io::Result<bool> {
        if Self::is_registered_in(root, subkey, value_name)? {
            Self::unregister_in(root, subkey, value_name)?;
            Ok(false)
        } else {
            Self::register_in(root, subkey, value_name, executable_path)?;
            Ok(true)
        }
    }
}

pub struct ProcessChecker;

impl ProcessChecker {
    pub fn find_process_ids(process_name: &str) -> std::io::Result<Vec<u32>> {
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }
            .map_err(|e| std::io::Error::other(e.to_string()))?;

        if snapshot.is_invalid() {
            return Err(std::io::Error::last_os_error());
        }

        let mut pids = Vec::new();
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        let first = unsafe { Process32FirstW(snapshot, &mut entry) };
        if first.is_ok() {
            loop {
                let null_pos = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let exe_name = String::from_utf16_lossy(&entry.szExeFile[..null_pos]);
                if vpm_core::ordinal_ignore_case_eq(&exe_name, process_name) {
                    pids.push(entry.th32ProcessID);
                }

                if unsafe { Process32NextW(snapshot, &mut entry) }.is_err() {
                    break;
                }
            }
        }

        unsafe {
            let _ = CloseHandle(snapshot);
        }

        Ok(pids)
    }

    pub fn is_process_running(process_name: &str) -> std::io::Result<bool> {
        let pids = Self::find_process_ids(process_name)?;
        Ok(!pids.is_empty())
    }
}
