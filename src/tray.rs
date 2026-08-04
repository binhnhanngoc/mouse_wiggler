// System tray icon + popup menu.
// Single icon registered with WM_APP+1 callback. Right-click → popup menu;
// left-click / double-click → restore main window.

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, GetSystemMetrics, PostMessageW,
    SetForegroundWindow, ShowWindow, TrackPopupMenu, HICON, MF_SEPARATOR, MF_STRING,
    SM_CXSMICON, SM_CYSMICON, SW_SHOW, TPM_BOTTOMALIGN, TPM_RIGHTBUTTON, WM_LBUTTONDBLCLK,
    WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP,
};

use crate::window::{
    self, AppState, ID_TRAY_EXIT, ID_TRAY_SHOW, ID_TRAY_TOGGLE, TRAY_ICON_ID,
    WM_TRAY_CALLBACK,
};

fn tooltip(running: bool) -> [u16; 128] {
    let s = if running {
        "Mouse Wiggler — Running"
    } else {
        "Mouse Wiggler — Stopped"
    };
    let mut buf = [0u16; 128];
    for (i, ch) in s.encode_utf16().enumerate() {
        if i >= buf.len() - 1 {
            break;
        }
        buf[i] = ch;
    }
    buf
}

/// Tray icons are drawn at the shell's small-icon size, which scales with DPI.
fn tray_icon() -> HICON {
    unsafe {
        window::load_app_icon(GetSystemMetrics(SM_CXSMICON), GetSystemMetrics(SM_CYSMICON))
    }
}

pub fn add(hwnd: HWND, running: bool) {
    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ICON_ID,
        uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
        uCallbackMessage: WM_TRAY_CALLBACK,
        hIcon: tray_icon(),
        ..Default::default()
    };
    nid.szTip = tooltip(running);
    unsafe {
        let _ = Shell_NotifyIconW(NIM_ADD, &nid);
    }
}

pub fn update_tooltip(hwnd: HWND, running: bool) {
    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ICON_ID,
        uFlags: NIF_TIP,
        ..Default::default()
    };
    nid.szTip = tooltip(running);
    unsafe {
        let _ = Shell_NotifyIconW(NIM_MODIFY, &nid);
    }
}

pub fn remove(hwnd: HWND) {
    let nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: TRAY_ICON_ID,
        ..Default::default()
    };
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
    }
}

/// Handle WM_TRAY_CALLBACK: lParam LOWORD encodes the mouse event.
///
/// SAFETY: caller passes a non-null `*mut AppState`. We deliberately do not
/// take a `&mut AppState` borrow across `TrackPopupMenu` (which pumps messages
/// and re-enters WndProc) — that would alias any nested `&mut *p`.
pub fn handle_callback(state: *mut AppState, lparam: LPARAM) {
    let event = (lparam.0 as u32) & 0xFFFF;
    let (hwnd, running) = unsafe { ((*state).hwnd, (*state).running) };
    match event {
        WM_LBUTTONUP | WM_LBUTTONDBLCLK => restore_window(hwnd),
        WM_RBUTTONUP => show_menu(hwnd, running),
        _ => {}
    }
}

fn restore_window(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
        let _ = SetForegroundWindow(hwnd);
    }
}

fn show_menu(hwnd: HWND, running: bool) {
    unsafe {
        let menu = match CreatePopupMenu() {
            Ok(m) => m,
            Err(_) => return,
        };
        let _ = AppendMenuW(menu, MF_STRING, ID_TRAY_SHOW as usize, w!("Show"));
        let toggle_label = if running { w!("Stop") } else { w!("Start") };
        let _ = AppendMenuW(menu, MF_STRING, ID_TRAY_TOGGLE as usize, toggle_label);
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let _ = AppendMenuW(menu, MF_STRING, ID_TRAY_EXIT as usize, w!("Exit"));

        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);

        // Required so the popup dismisses on outside-click (KB 135788).
        let _ = SetForegroundWindow(hwnd);

        let _ = TrackPopupMenu(
            menu,
            TPM_RIGHTBUTTON | TPM_BOTTOMALIGN,
            pt.x,
            pt.y,
            0,
            hwnd,
            None,
        );

        // Workaround posts a benign message to flush the message queue.
        let _ = PostMessageW(hwnd, WM_NULL, WPARAM(0), LPARAM(0));

        let _ = DestroyMenu(menu);
    }
}
