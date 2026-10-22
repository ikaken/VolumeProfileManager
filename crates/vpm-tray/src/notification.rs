use std::ffi::c_void;
use windows::Win32::Foundation::{COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreateFontIndirectW, CreateSolidBrush, DEFAULT_GUI_FONT, DT_END_ELLIPSIS,
    DT_NOPREFIX, DT_SINGLELINE, DT_WORDBREAK, DeleteObject, DrawTextW, EndPaint, FW_BOLD, FillRect,
    FrameRect, GetDC, GetDeviceCaps, GetStockObject, HFONT, HGDIOBJ, InvalidateRect, LOGFONTW,
    LOGPIXELSX, PAINTSTRUCT, ReleaseDC, SelectObject, SetBkMode, SetTextColor, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GWLP_USERDATA, GetClientRect,
    GetWindowLongPtrW, HWND_TOPMOST, IDC_ARROW, KillTimer, LoadCursorW, MA_NOACTIVATE,
    NONCLIENTMETRICSW, PostMessageW, RegisterClassExW, SPI_GETNONCLIENTMETRICS, SPI_GETWORKAREA,
    SW_HIDE, SWP_NOACTIVATE, SWP_SHOWWINDOW, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, SetTimer,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, SystemParametersInfoW, WM_APP, WM_LBUTTONUP,
    WM_MOUSEACTIVATE, WM_NCDESTROY, WM_PAINT, WM_RBUTTONUP, WM_TIMER, WNDCLASSEXW,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};
use windows::core::{PCWSTR, w};

const WM_APP_NOTIFY: u32 = WM_APP + 1;
const TIMER_ID: usize = 1;
const NOTIFY_TIMEOUT_MS: u32 = 4000;
const WINDOW_CLASS_NAME: PCWSTR = w!("VolumeProfileManagerNotifyClass");

struct State {
    title: String,
    message: String,
    font_title: HFONT,
    font_body: HFONT,
}

pub struct NotificationWindow {
    hwnd: HWND,
}

unsafe impl Send for NotificationWindow {}
unsafe impl Sync for NotificationWindow {}

impl NotificationWindow {
    pub fn new() -> windows::core::Result<Self> {
        unsafe {
            let hmodule = GetModuleHandleW(None)?;
            let hinstance = HINSTANCE(hmodule.0);

            let wnd_class = WNDCLASSEXW {
                cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
                style: Default::default(),
                lpfnWndProc: Some(Self::wnd_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: hinstance,
                hIcon: Default::default(),
                hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
                hbrBackground: Default::default(),
                lpszMenuName: PCWSTR::null(),
                lpszClassName: WINDOW_CLASS_NAME,
                hIconSm: Default::default(),
            };

            RegisterClassExW(&wnd_class);

            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                WINDOW_CLASS_NAME,
                w!("VolumeProfileManager Notification"),
                WS_POPUP,
                0,
                0,
                0,
                0,
                None,
                None,
                Some(hinstance),
                None,
            )?;

            let (font_title, font_body) = Self::create_fonts();
            let state = Box::new(State {
                title: String::new(),
                message: String::new(),
                font_title,
                font_body,
            });
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);

            Ok(Self { hwnd })
        }
    }

    pub fn show(&self, title: &str, message: &str) {
        let payload = Box::new((title.to_string(), message.to_string()));
        let raw = Box::into_raw(payload);
        unsafe {
            if PostMessageW(
                Some(self.hwnd),
                WM_APP_NOTIFY,
                WPARAM(0),
                LPARAM(raw as isize),
            )
            .is_err()
            {
                drop(Box::from_raw(raw));
            }
        }
    }

    fn create_fonts() -> (HFONT, HFONT) {
        unsafe {
            let mut metrics = NONCLIENTMETRICSW {
                cbSize: std::mem::size_of::<NONCLIENTMETRICSW>() as u32,
                ..Default::default()
            };
            let ok = SystemParametersInfoW(
                SPI_GETNONCLIENTMETRICS,
                metrics.cbSize,
                Some(&mut metrics as *mut NONCLIENTMETRICSW as *mut c_void),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )
            .is_ok();

            if ok {
                let font_body = CreateFontIndirectW(&metrics.lfMessageFont);
                let mut title_lf: LOGFONTW = metrics.lfMessageFont;
                title_lf.lfWeight = FW_BOLD.0 as i32;
                let font_title = CreateFontIndirectW(&title_lf);
                if !font_title.is_invalid() && !font_body.is_invalid() {
                    return (font_title, font_body);
                }
                if !font_title.is_invalid() {
                    let _ = DeleteObject(HGDIOBJ(font_title.0));
                }
                if !font_body.is_invalid() {
                    let _ = DeleteObject(HGDIOBJ(font_body.0));
                }
            }

            let fallback = HFONT(GetStockObject(DEFAULT_GUI_FONT).0);
            (fallback, fallback)
        }
    }

    fn current_dpi() -> u32 {
        unsafe {
            let hdc = GetDC(None);
            if hdc.is_invalid() {
                return 96;
            }
            let dpi = GetDeviceCaps(Some(hdc), LOGPIXELSX);
            ReleaseDC(None, hdc);
            if dpi <= 0 { 96 } else { dpi as u32 }
        }
    }

    fn state_ptr(hwnd: HWND) -> *mut State {
        unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut State }
    }

    unsafe extern "system" fn wnd_proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        unsafe {
            match msg {
                WM_APP_NOTIFY => {
                    let payload = Box::from_raw(lparam.0 as *mut (String, String));
                    let (title, message) = *payload;
                    Self::handle_notify(hwnd, title, message);
                    LRESULT(0)
                }
                WM_TIMER | WM_LBUTTONUP | WM_RBUTTONUP => {
                    let _ = KillTimer(Some(hwnd), TIMER_ID);
                    let _ = ShowWindow(hwnd, SW_HIDE);
                    LRESULT(0)
                }
                WM_PAINT => {
                    Self::handle_paint(hwnd);
                    LRESULT(0)
                }
                WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
                WM_NCDESTROY => {
                    let ptr = Self::state_ptr(hwnd);
                    if !ptr.is_null() {
                        let state = Box::from_raw(ptr);
                        if !state.font_title.is_invalid() {
                            let _ = DeleteObject(HGDIOBJ(state.font_title.0));
                        }
                        if !state.font_body.is_invalid() && state.font_body.0 != state.font_title.0
                        {
                            let _ = DeleteObject(HGDIOBJ(state.font_body.0));
                        }
                        SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                    }
                    DefWindowProcW(hwnd, msg, wparam, lparam)
                }
                _ => DefWindowProcW(hwnd, msg, wparam, lparam),
            }
        }
    }

    fn handle_notify(hwnd: HWND, title: String, message: String) {
        unsafe {
            let state = Self::state_ptr(hwnd);
            if state.is_null() {
                return;
            }
            (*state).title = title;
            (*state).message = message;

            let mut work_area = RECT::default();
            if SystemParametersInfoW(
                SPI_GETWORKAREA,
                0,
                Some(&mut work_area as *mut RECT as *mut c_void),
                SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0),
            )
            .is_err()
            {
                return;
            }

            let dpi = Self::current_dpi();
            let (x, y, w, h) = compute_popup_rect(
                (
                    work_area.left,
                    work_area.top,
                    work_area.right,
                    work_area.bottom,
                ),
                dpi,
            );

            let _ = SetWindowPos(
                hwnd,
                Some(HWND_TOPMOST),
                x,
                y,
                w,
                h,
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            let _ = InvalidateRect(Some(hwnd), None, true);
            let _ = SetTimer(Some(hwnd), TIMER_ID, NOTIFY_TIMEOUT_MS, None);
        }
    }

    fn handle_paint(hwnd: HWND) {
        unsafe {
            let state = Self::state_ptr(hwnd);
            let mut ps = PAINTSTRUCT::default();
            let hdc = BeginPaint(hwnd, &mut ps);
            if hdc.is_invalid() {
                return;
            }

            let mut client = RECT::default();
            GetClientRect(hwnd, &mut client).ok();

            let white = CreateSolidBrush(COLORREF(0x00FFFFFF));
            FillRect(hdc, &client, white);
            let _ = DeleteObject(HGDIOBJ(white.0));

            let gray = CreateSolidBrush(COLORREF(0x00A0A0A0));
            FrameRect(hdc, &client, gray);
            let _ = DeleteObject(HGDIOBJ(gray.0));

            if !state.is_null() {
                let state = &*state;
                let dpi = Self::current_dpi();
                let scale = dpi as i32;
                let pad = (12 * scale / 96).max(1);
                let title_h = (20 * scale / 96).max(1);

                SetBkMode(hdc, TRANSPARENT);
                SetTextColor(hdc, COLORREF(0x00202020));

                let mut title_wide = to_wide(&state.title);
                let mut title_rect = RECT {
                    left: client.left + pad,
                    top: client.top + pad,
                    right: client.right - pad,
                    bottom: client.top + pad + title_h,
                };
                SelectObject(hdc, HGDIOBJ(state.font_title.0));
                DrawTextW(
                    hdc,
                    &mut title_wide,
                    &mut title_rect,
                    DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX,
                );

                let mut message_wide = to_wide(&state.message);
                let mut message_rect = RECT {
                    left: client.left + pad,
                    top: title_rect.bottom + 4 * scale / 96,
                    right: client.right - pad,
                    bottom: client.bottom - pad,
                };
                SelectObject(hdc, HGDIOBJ(state.font_body.0));
                DrawTextW(
                    hdc,
                    &mut message_wide,
                    &mut message_rect,
                    DT_WORDBREAK | DT_NOPREFIX | DT_END_ELLIPSIS,
                );
            }

            let _ = EndPaint(hwnd, &ps);
        }
    }
}

impl Drop for NotificationWindow {
    fn drop(&mut self) {
        unsafe {
            if !self.hwnd.is_invalid() {
                DestroyWindow(self.hwnd).ok();
            }
        }
    }
}

pub fn compute_popup_rect(work_area: (i32, i32, i32, i32), dpi: u32) -> (i32, i32, i32, i32) {
    let (_left, _top, right, bottom) = work_area;
    let scale = dpi as f32 / 96.0;
    let w = (360.0 * scale).round() as i32;
    let h = (96.0 * scale).round() as i32;
    let margin = (12.0 * scale).round() as i32;
    (right - margin - w, bottom - margin - h, w, h)
}

fn to_wide(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}
