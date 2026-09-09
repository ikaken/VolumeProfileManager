use std::ffi::c_void;
use std::sync::atomic::{AtomicPtr, Ordering};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_TIP, NIIF_INFO, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NIM_SETVERSION, NOTIFYICON_VERSION_4, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    DispatchMessageW, GetCursorPos, GetMessageW, GetSystemMetrics, HICON, HWND_MESSAGE,
    IDI_APPLICATION, IMAGE_ICON, LR_LOADFROMFILE, LoadIconW, LoadImageW, MF_GRAYED, MF_SEPARATOR,
    MF_STRING, MSG, PostMessageW, PostQuitMessage, RegisterClassExW, RegisterWindowMessageW,
    SM_CXSMICON, SM_CYSMICON, SetForegroundWindow, TPM_RIGHTBUTTON, TrackPopupMenu,
    TranslateMessage, WINDOW_EX_STYLE, WM_COMMAND, WM_CONTEXTMENU, WM_DESTROY, WM_LBUTTONUP,
    WM_NULL, WM_RBUTTONUP, WM_USER, WNDCLASSEXW,
};
use windows::core::{PCWSTR, w};

pub const CMD_STATUS: u32 = 1001;
pub const CMD_TOGGLE_STARTUP: u32 = 1002;
pub const CMD_EXIT: u32 = 1003;
pub const CMD_UPDATE_PROFILE: u32 = 1004;
const CMD_HEADER: u32 = 9999;

const WM_TRAYICON: u32 = WM_USER + 1;
const TRAY_ICON_ID: u32 = 1;
const WINDOW_CLASS_NAME: PCWSTR = w!("VolumeProfileManagerTrayWindowClass");

static ACTIVE_HANDLER: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

type HandlerFn = Box<dyn Fn(u32) + Send + Sync>;

pub struct TrayIconWindow {
    hwnd: HWND,
    hicon: HICON,
    _handler: Box<HandlerFn>,
}

unsafe impl Send for TrayIconWindow {}
unsafe impl Sync for TrayIconWindow {}

impl TrayIconWindow {
    pub fn new<F>(on_command: F) -> windows::core::Result<Self>
    where
        F: Fn(u32) + Send + Sync + 'static,
    {
        let handler_box: Box<HandlerFn> = Box::new(Box::new(on_command));
        let handler_ptr = &*handler_box as *const HandlerFn as *mut c_void;
        ACTIVE_HANDLER.store(handler_ptr, Ordering::SeqCst);

        unsafe {
            let hmodule = GetModuleHandleW(None)?;
            let hinstance = HINSTANCE(hmodule.0);
            let hicon = Self::load_app_icon();

            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: Default::default(),
                lpfnWndProc: Some(Self::wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance,
                hIcon: hicon,
                hCursor: Default::default(),
                hbrBackground: Default::default(),
                lpszMenuName: PCWSTR::null(),
                lpszClassName: WINDOW_CLASS_NAME,
                hIconSm: hicon,
            };

            RegisterClassExW(&wnd_class);

            let hwnd = CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                WINDOW_CLASS_NAME,
                w!("VolumeProfileManager"),
                Default::default(),
                0,
                0,
                0,
                0,
                Some(HWND_MESSAGE),
                None,
                Some(hinstance),
                None,
            )?;

            let window = Self {
                hwnd,
                hicon,
                _handler: handler_box,
            };

            window.add_tray_icon();
            Ok(window)
        }
    }

    pub fn show_balloon(&self, title: &str, message: &str) {
        unsafe {
            let mut data = self.create_notify_icon_data();
            data.uFlags = NIF_INFO;
            data.dwInfoFlags = NIIF_INFO;

            copy_str_to_u16_buf(&mut data.szInfoTitle, title);
            copy_str_to_u16_buf(&mut data.szInfo, message);

            let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
        }
    }

    pub fn request_exit(&self) {
        unsafe {
            PostMessageW(Some(self.hwnd), WM_DESTROY, WPARAM(0), LPARAM(0)).ok();
        }
    }

    pub fn run_message_loop() {
        unsafe {
            let mut msg = MSG::default();
            while GetMessageW(&mut msg, None, 0, 0).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    fn create_notify_icon_data(&self) -> NOTIFYICONDATAW {
        let mut data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.hwnd,
            uID: TRAY_ICON_ID,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: WM_TRAYICON,
            hIcon: self.hicon,
            ..Default::default()
        };
        copy_str_to_u16_buf(&mut data.szTip, "VolumeProfileManager");
        data
    }

    fn add_tray_icon(&self) {
        unsafe {
            let mut data = self.create_notify_icon_data();
            let added = Shell_NotifyIconW(NIM_ADD, &data).as_bool();
            if added {
                data.Anonymous.uVersion = NOTIFYICON_VERSION_4;
                let _ = Shell_NotifyIconW(NIM_SETVERSION, &data);
            }
        }
    }

    fn remove_tray_icon(&self) {
        unsafe {
            let data = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: self.hwnd,
                uID: TRAY_ICON_ID,
                ..Default::default()
            };
            let _ = Shell_NotifyIconW(NIM_DELETE, &data);
        }
    }

    fn load_app_icon() -> HICON {
        unsafe {
            let cx = GetSystemMetrics(SM_CXSMICON).max(16);
            let cy = GetSystemMetrics(SM_CYSMICON).max(16);

            if let Ok(hmodule) = GetModuleHandleW(None) {
                let resource_id = std::ptr::without_provenance(1);
                if let Ok(handle) = LoadImageW(
                    Some(HINSTANCE(hmodule.0)),
                    PCWSTR(resource_id),
                    IMAGE_ICON,
                    cx,
                    cy,
                    Default::default(),
                ) {
                    let hicon = HICON(handle.0);
                    if !hicon.is_invalid() {
                        return hicon;
                    }
                }
            }

            if let Ok(exe_path) = std::env::current_exe()
                && let Some(dir) = exe_path.parent()
            {
                let ico_path = dir.join("app.ico");
                if ico_path.exists() {
                    let wide_path: Vec<u16> = ico_path
                        .to_string_lossy()
                        .encode_utf16()
                        .chain(std::iter::once(0))
                        .collect();
                    if let Ok(handle) = LoadImageW(
                        None,
                        PCWSTR(wide_path.as_ptr()),
                        IMAGE_ICON,
                        cx,
                        cy,
                        LR_LOADFROMFILE,
                    ) {
                        let hicon = HICON(handle.0);
                        if !hicon.is_invalid() {
                            return hicon;
                        }
                    }
                }
            }

            LoadIconW(None, IDI_APPLICATION).unwrap_or_default()
        }
    }

    fn show_context_menu(&self) {
        unsafe {
            if let Ok(hmenu) = CreatePopupMenu() {
                let version_label = format!("VolumeProfileManager {}", crate::version_string());
                let version_wide = to_wide_null(&version_label);
                AppendMenuW(
                    hmenu,
                    MF_STRING | MF_GRAYED,
                    CMD_HEADER as usize,
                    PCWSTR(version_wide.as_ptr()),
                )
                .ok();
                AppendMenuW(hmenu, MF_SEPARATOR, 0, PCWSTR::null()).ok();

                let status_wide = to_wide_null("ステータス表示");
                AppendMenuW(
                    hmenu,
                    MF_STRING,
                    CMD_STATUS as usize,
                    PCWSTR(status_wide.as_ptr()),
                )
                .ok();

                let update_wide = to_wide_null("プロファイルを更新（現在の音量を保存）");
                AppendMenuW(
                    hmenu,
                    MF_STRING,
                    CMD_UPDATE_PROFILE as usize,
                    PCWSTR(update_wide.as_ptr()),
                )
                .ok();

                let startup_wide = to_wide_null("スタートアップ登録/解除");
                AppendMenuW(
                    hmenu,
                    MF_STRING,
                    CMD_TOGGLE_STARTUP as usize,
                    PCWSTR(startup_wide.as_ptr()),
                )
                .ok();

                let exit_wide = to_wide_null("終了");
                AppendMenuW(
                    hmenu,
                    MF_STRING,
                    CMD_EXIT as usize,
                    PCWSTR(exit_wide.as_ptr()),
                )
                .ok();

                let mut pt = POINT::default();
                GetCursorPos(&mut pt).ok();

                let _ = SetForegroundWindow(self.hwnd);
                let _ =
                    TrackPopupMenu(hmenu, TPM_RIGHTBUTTON, pt.x, pt.y, Some(0), self.hwnd, None);
                PostMessageW(Some(self.hwnd), WM_NULL, WPARAM(0), LPARAM(0)).ok();
                DestroyMenu(hmenu).ok();
            }
        }
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        unsafe {
            if msg == WM_TRAYICON {
                let mouse_msg = (lparam.0 as u32) & 0xFFFF;
                if mouse_msg == WM_LBUTTONUP
                    || mouse_msg == WM_RBUTTONUP
                    || mouse_msg == WM_CONTEXTMENU
                {
                    let window = TrayIconWindow {
                        hwnd,
                        hicon: HICON::default(),
                        _handler: Box::new(Box::new(|_| {})),
                    };
                    window.show_context_menu();
                    std::mem::forget(window);
                }
                return LRESULT(0);
            }

            if msg == WM_COMMAND {
                let cmd_id = (wparam.0 as u32) & 0xFFFF;
                let handler_ptr = ACTIVE_HANDLER.load(Ordering::SeqCst);
                if !handler_ptr.is_null() {
                    let handler = &*(handler_ptr as *const HandlerFn);
                    handler(cmd_id);
                }
                return LRESULT(0);
            }

            if msg == WM_DESTROY {
                PostQuitMessage(0);
                return LRESULT(0);
            }

            let taskbar_msg = RegisterWindowMessageW(w!("TaskbarCreated"));
            if taskbar_msg != 0 && msg == taskbar_msg {
                let window = TrayIconWindow {
                    hwnd,
                    hicon: Self::load_app_icon(),
                    _handler: Box::new(Box::new(|_| {})),
                };
                window.add_tray_icon();
                std::mem::forget(window);
                return LRESULT(0);
            }

            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
    }
}

impl Drop for TrayIconWindow {
    fn drop(&mut self) {
        self.remove_tray_icon();
        ACTIVE_HANDLER.store(std::ptr::null_mut(), Ordering::SeqCst);
        unsafe {
            if !self.hwnd.is_invalid() {
                DestroyWindow(self.hwnd).ok();
            }
        }
    }
}

fn copy_str_to_u16_buf(buf: &mut [u16], text: &str) {
    let mut index = 0;
    for c in text.encode_utf16() {
        if index < buf.len() - 1 {
            buf[index] = c;
            index += 1;
        } else {
            break;
        }
    }
    buf[index] = 0;
}

fn to_wide_null(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}
