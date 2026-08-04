// Main window: registration, control creation, message loop, command routing.
//
// AppState lives on the heap and is owned by the window through GWLP_USERDATA.
// It is allocated before CreateWindowExW and dropped in WM_NCDESTROY.
//
// DPI: the process is PerMonitorV2. All geometry below is in 96-DPI design
// units and converted at placement time (`dpi::scale`), so the same table
// drives the initial layout and every WM_DPICHANGED re-layout.

use std::cell::Cell;
use std::ptr::null_mut;

use windows::core::{w, PCWSTR, Result};
use windows::Win32::Foundation::{HMODULE, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    DeleteObject, InvalidateRect, COLOR_BTNFACE, HBRUSH, HFONT, HGDIOBJ,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemServices::SS_LEFT;
use windows::Win32::UI::Controls::{LoadIconWithScaleDown, BST_CHECKED, BST_UNCHECKED};
use windows::Win32::UI::HiDpi::AdjustWindowRectExForDpi;
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetSystemMetrics, GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, KillTimer,
    LoadCursorW, LoadIconW, LoadImageW, PostQuitMessage, RegisterClassExW, SendMessageW,
    SetForegroundWindow, SetTimer, SetWindowLongPtrW, SetWindowPos, SetWindowTextW,
    ShowWindow, TranslateMessage, BM_GETCHECK, BM_SETCHECK, BN_CLICKED, BS_AUTOCHECKBOX,
    BS_DEFPUSHBUTTON, CBN_SELCHANGE, CB_ADDSTRING, CB_GETCURSEL, CB_SETCURSEL,
    CBS_DROPDOWNLIST, CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, EN_KILLFOCUS,
    ES_NUMBER, GWLP_USERDATA, GWL_EXSTYLE, GWL_STYLE, HICON, HMENU, IDC_ARROW,
    IDI_APPLICATION, IMAGE_ICON, LR_DEFAULTCOLOR, MSG, SC_MINIMIZE, SM_CXICON, SM_CXSMICON,
    SM_CYICON, SM_CYSMICON, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOZORDER, SW_HIDE, SW_SHOW,
    WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WM_CLOSE, WM_COMMAND, WM_CREATE, WM_DESTROY,
    WM_DPICHANGED, WM_NCCREATE, WM_NCDESTROY, WM_SETFONT, WM_SYSCOMMAND, WM_TIMER,
    WNDCLASSEXW, WS_BORDER, WS_CAPTION, WS_CHILD, WS_EX_CLIENTEDGE, WS_MINIMIZEBOX,
    WS_OVERLAPPED, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
};

use crate::dpi;
use crate::jiggler::{Jiggler, Mode};
use crate::settings::{self, Settings};
use crate::tray;

// ---- Public IDs and constants (used across modules) ------------------------

pub const TIMER_JIGGLE: usize = 1;
pub const TIMER_SAVE: usize = 2;

const ID_MODE_COMBO: u16 = 1001;
const ID_INTERVAL_EDIT: u16 = 1002;
const ID_START_BTN: u16 = 1003;
const ID_AUTOSTART_CHK: u16 = 1004;
const ID_STARTMIN_CHK: u16 = 1005;
const ID_VISIBLE_DX_EDIT: u16 = 1006;

pub const ID_TRAY_SHOW: u16 = 2001;
pub const ID_TRAY_TOGGLE: u16 = 2002;
pub const ID_TRAY_EXIT: u16 = 2003;

pub const TRAY_ICON_ID: u32 = 1;
pub const WM_TRAY_CALLBACK: u32 = WM_APP + 1;

/// Resource id for embedded icon (see `app.rc`: `IDI_APP ICON ...`).
pub const IDI_APP: u16 = 1;

pub const CLASS_NAME: PCWSTR = w!("MouseWigglerMain");
const SAVE_DEBOUNCE_MS: u32 = 250;

// ---- Layout table (96-DPI design units) ------------------------------------

/// Rect as (x, y, cx, cy).
type Rect96 = (i32, i32, i32, i32);

/// Client area the table below is authored against.
const CLIENT_W: i32 = 256;
const CLIENT_H: i32 = 200;

const LABEL_COUNT: usize = 5;
const CONTROL_COUNT: usize = LABEL_COUNT + 6;

const LABEL_TEXT: [PCWSTR; LABEL_COUNT] = [
    w!("Mode:"),
    w!("Every:"),
    w!("seconds"),
    w!("Pixels:"),
    w!("px (Visible)"),
];
const LABEL_RECT: [Rect96; LABEL_COUNT] = [
    (12, 12, 60, 18),
    (12, 42, 60, 18),
    (178, 42, 70, 18),
    (12, 72, 60, 18),
    (178, 72, 70, 18),
];

/// Combo cy is the dropped-down list height; the closed height follows the font.
const R_COMBO: Rect96 = (78, 10, 160, 120);
const R_INTERVAL: Rect96 = (78, 40, 90, 22);
const R_DX: Rect96 = (78, 70, 90, 22);
const R_START: Rect96 = (12, 104, 232, 28);
const R_AUTOSTART: Rect96 = (12, 142, 232, 20);
const R_STARTMIN: Rect96 = (12, 166, 232, 20);

// ---- AppState --------------------------------------------------------------

pub struct AppState {
    pub hwnd: HWND,
    pub combo: HWND,
    pub edit_interval: HWND,
    pub edit_dx: HWND,
    pub btn_start: HWND,
    pub chk_autostart: HWND,
    pub chk_startmin: HWND,
    labels: [HWND; LABEL_COUNT],
    /// UI font owned by this state; recreated on every DPI change.
    font: HFONT,
    dpi: u32,
    pub jiggler: Jiggler,
    pub settings: Settings,
    pub running: bool,
    /// Suppresses save during initial control population.
    suppress_save: Cell<bool>,
}

impl AppState {
    fn new(cfg: Settings) -> Self {
        let mode = Mode::from_str_lossy(&cfg.mode);
        let dx = cfg.visible_dx;
        Self {
            hwnd: HWND(null_mut()),
            combo: HWND(null_mut()),
            edit_interval: HWND(null_mut()),
            edit_dx: HWND(null_mut()),
            btn_start: HWND(null_mut()),
            chk_autostart: HWND(null_mut()),
            chk_startmin: HWND(null_mut()),
            labels: [HWND(null_mut()); LABEL_COUNT],
            font: HFONT(null_mut()),
            dpi: dpi::BASE_DPI,
            jiggler: Jiggler::new(mode, dx),
            settings: cfg,
            running: false,
            suppress_save: Cell::new(true),
        }
    }

    /// Every child window, in creation order.
    fn controls(&self) -> [HWND; CONTROL_COUNT] {
        let mut all = [HWND(null_mut()); CONTROL_COUNT];
        all[..LABEL_COUNT].copy_from_slice(&self.labels);
        all[LABEL_COUNT..].copy_from_slice(&[
            self.combo,
            self.edit_interval,
            self.edit_dx,
            self.btn_start,
            self.chk_autostart,
            self.chk_startmin,
        ]);
        all
    }
}

impl Drop for AppState {
    fn drop(&mut self) {
        if !self.font.0.is_null() {
            unsafe {
                let _ = DeleteObject(HGDIOBJ(self.font.0));
            }
        }
    }
}

// ---- Public entry ---------------------------------------------------------

pub fn run(cfg: Settings) -> Result<()> {
    unsafe {
        let instance: HMODULE = GetModuleHandleW(PCWSTR::null())?;

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: instance.into(),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            hbrBackground: HBRUSH((COLOR_BTNFACE.0 as isize + 1) as *mut _),
            lpszClassName: CLASS_NAME,
            hIcon: load_app_icon(GetSystemMetrics(SM_CXICON), GetSystemMetrics(SM_CYICON)),
            hIconSm: load_app_icon(
                GetSystemMetrics(SM_CXSMICON),
                GetSystemMetrics(SM_CYSMICON),
            ),
            ..Default::default()
        };
        let atom = RegisterClassExW(&wc);
        if atom == 0 {
            return Err(windows::core::Error::from_win32());
        }

        let state = Box::new(AppState::new(cfg));
        let start_minimized = state.settings.start_minimized;
        let state_ptr = Box::into_raw(state);

        // Size is a placeholder: WM_CREATE resizes to the DPI-correct client
        // area once the window's monitor (and therefore its DPI) is known.
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            CLASS_NAME,
            w!("Mouse Wiggler"),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            CLIENT_W,
            CLIENT_H,
            HWND(null_mut()),
            HMENU(null_mut()),
            instance,
            Some(state_ptr as *const _),
        )?;

        if start_minimized {
            let _ = ShowWindow(hwnd, SW_HIDE);
        } else {
            let _ = ShowWindow(hwnd, SW_SHOW);
        }

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND(null_mut()), 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    Ok(())
}

/// Load the embedded app icon at an explicit pixel size.
///
/// `LoadIconW` always yields the SM_CXICON image and lets the shell stretch it,
/// which is visibly soft in the tray and title bar on high-DPI displays.
/// `LoadIconWithScaleDown` picks the best frame in the .ico and smooth-scales
/// it; `LoadImageW` is the fallback if comctl32 is unavailable.
pub unsafe fn load_app_icon(cx: i32, cy: i32) -> HICON {
    let inst = GetModuleHandleW(PCWSTR::null()).unwrap_or_default();
    let name = PCWSTR(IDI_APP as *const u16);

    if let Ok(icon) = LoadIconWithScaleDown(inst, name, cx, cy) {
        if !icon.0.is_null() {
            return icon;
        }
    }

    if let Ok(h) = LoadImageW(inst, name, IMAGE_ICON, cx, cy, LR_DEFAULTCOLOR) {
        if !h.0.is_null() {
            return HICON(h.0);
        }
    }

    LoadIconW(inst, name)
        .or_else(|_| LoadIconW(HMODULE(null_mut()), IDI_APPLICATION))
        .unwrap_or_default()
}

// ---- WndProc --------------------------------------------------------------

unsafe fn state_ptr(hwnd: HWND) -> *mut AppState {
    GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState
}

extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    unsafe {
        match msg {
            WM_NCCREATE => {
                let cs = lparam.0 as *const CREATESTRUCTW;
                if !cs.is_null() {
                    let ptr = (*cs).lpCreateParams as *mut AppState;
                    if !ptr.is_null() {
                        (*ptr).hwnd = hwnd;
                    }
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, ptr as isize);
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }

            WM_CREATE => {
                let p = state_ptr(hwnd);
                if !p.is_null() {
                    let st = &mut *p;
                    st.dpi = dpi::dpi_for(hwnd);
                    st.font = dpi::ui_font(st.dpi);
                    create_controls(st);
                    apply_font(st);
                    fit_window(hwnd, st.dpi, None);
                    relayout(st);
                    apply_settings_to_controls(st);
                    tray::add(hwnd, st.running);
                    if st.settings.autostart {
                        let _ = crate::autostart::reconcile(&st.settings);
                    }
                    st.suppress_save.set(false);
                }
                LRESULT(0)
            }

            // PerMonitorV2: the window moved to a monitor with a different DPI.
            // wParam LOWORD is the new DPI; lParam is the suggested window rect.
            WM_DPICHANGED => {
                let p = state_ptr(hwnd);
                if !p.is_null() {
                    let new_dpi = (wparam.0 & 0xFFFF) as u32;
                    let suggested = lparam.0 as *const RECT;
                    let pos = if suggested.is_null() {
                        None
                    } else {
                        Some(((*suggested).left, (*suggested).top))
                    };
                    rescale(&mut *p, new_dpi, pos);
                }
                LRESULT(0)
            }

            WM_COMMAND => {
                let p = state_ptr(hwnd);
                if !p.is_null() {
                    let id = (wparam.0 & 0xFFFF) as u16;
                    if id == ID_TRAY_EXIT {
                        // Handle Exit *outside* any &mut AppState borrow:
                        // DestroyWindow synchronously fires WM_NCDESTROY which
                        // drops the Box<AppState>. Holding `&mut *p` across it
                        // is aliasing UB.
                        let _ = DestroyWindow(hwnd);
                    } else {
                        handle_command(&mut *p, wparam, lparam);
                    }
                }
                LRESULT(0)
            }

            WM_TIMER => match wparam.0 {
                v if v == TIMER_JIGGLE => {
                    let p = state_ptr(hwnd);
                    if !p.is_null() {
                        (*p).jiggler.tick();
                    }
                    LRESULT(0)
                }
                v if v == TIMER_SAVE => {
                    let _ = KillTimer(hwnd, TIMER_SAVE);
                    let p = state_ptr(hwnd);
                    if !p.is_null() {
                        settings::save(&(*p).settings);
                    }
                    LRESULT(0)
                }
                _ => DefWindowProcW(hwnd, msg, wparam, lparam),
            },

            WM_CLOSE => {
                let _ = ShowWindow(hwnd, SW_HIDE);
                LRESULT(0)
            }

            WM_SYSCOMMAND => {
                if (wparam.0 as u32) & 0xFFF0 == SC_MINIMIZE {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                    return LRESULT(0);
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }

            WM_TRAY_CALLBACK => {
                let p = state_ptr(hwnd);
                if !p.is_null() {
                    // Pass raw pointer — TrackPopupMenu pumps messages and
                    // re-enters WndProc, which would alias any live borrow.
                    tray::handle_callback(p, lparam);
                }
                LRESULT(0)
            }

            WM_DESTROY => {
                let p = state_ptr(hwnd);
                if !p.is_null() && (*p).running {
                    let _ = KillTimer(hwnd, TIMER_JIGGLE);
                }
                let _ = KillTimer(hwnd, TIMER_SAVE);
                tray::remove(hwnd);
                PostQuitMessage(0);
                LRESULT(0)
            }

            WM_NCDESTROY => {
                let prev = SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0) as *mut AppState;
                if !prev.is_null() {
                    drop(Box::from_raw(prev));
                }
                DefWindowProcW(hwnd, msg, wparam, lparam)
            }

            _ => DefWindowProcW(hwnd, msg, wparam, lparam),
        }
    }
}

// ---- DPI-aware sizing and layout ------------------------------------------

/// Resize the window so its *client* area is CLIENT_W x CLIENT_H at `dpi`.
/// `pos` moves the window as well (used by WM_DPICHANGED).
unsafe fn fit_window(hwnd: HWND, dpi: u32, pos: Option<(i32, i32)>) {
    let mut rc = RECT {
        left: 0,
        top: 0,
        right: dpi::scale(CLIENT_W, dpi),
        bottom: dpi::scale(CLIENT_H, dpi),
    };
    let style = WINDOW_STYLE(GetWindowLongPtrW(hwnd, GWL_STYLE) as u32);
    let ex_style = WINDOW_EX_STYLE(GetWindowLongPtrW(hwnd, GWL_EXSTYLE) as u32);
    let _ = AdjustWindowRectExForDpi(&mut rc, style, false, ex_style, dpi);

    let (x, y, flags) = match pos {
        Some((x, y)) => (x, y, SWP_NOZORDER | SWP_NOACTIVATE),
        None => (0, 0, SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOMOVE),
    };
    let _ = SetWindowPos(
        hwnd,
        HWND(null_mut()),
        x,
        y,
        rc.right - rc.left,
        rc.bottom - rc.top,
        flags,
    );
}

/// Position every child from the design-unit table at the current DPI.
unsafe fn relayout(st: &AppState) {
    for (i, h) in st.labels.iter().enumerate() {
        place(*h, LABEL_RECT[i], st.dpi);
    }
    place(st.combo, R_COMBO, st.dpi);
    place(st.edit_interval, R_INTERVAL, st.dpi);
    place(st.edit_dx, R_DX, st.dpi);
    place(st.btn_start, R_START, st.dpi);
    place(st.chk_autostart, R_AUTOSTART, st.dpi);
    place(st.chk_startmin, R_STARTMIN, st.dpi);
}

unsafe fn place(h: HWND, r: Rect96, dpi: u32) {
    if h.0.is_null() {
        return;
    }
    let _ = SetWindowPos(
        h,
        HWND(null_mut()),
        dpi::scale(r.0, dpi),
        dpi::scale(r.1, dpi),
        dpi::scale(r.2, dpi),
        dpi::scale(r.3, dpi),
        SWP_NOZORDER | SWP_NOACTIVATE,
    );
}

unsafe fn apply_font(st: &AppState) {
    for h in st.controls() {
        if h.0.is_null() {
            continue;
        }
        let _ = SendMessageW(h, WM_SETFONT, WPARAM(st.font.0 as usize), LPARAM(1));
    }
}

/// Re-fit window, font and controls for `new_dpi` (WM_DPICHANGED).
unsafe fn rescale(st: &mut AppState, new_dpi: u32, pos: Option<(i32, i32)>) {
    st.dpi = if new_dpi == 0 { dpi::BASE_DPI } else { new_dpi };

    let new_font = dpi::ui_font(st.dpi);
    if !new_font.0.is_null() {
        // Swap first, then delete: the old font must not be in use by any
        // control when DeleteObject runs.
        let old = std::mem::replace(&mut st.font, new_font);
        apply_font(st);
        if !old.0.is_null() {
            let _ = DeleteObject(HGDIOBJ(old.0));
        }
    }

    fit_window(st.hwnd, st.dpi, pos);
    relayout(st);
    let _ = InvalidateRect(st.hwnd, None, true);
}

// ---- Control creation -----------------------------------------------------

fn hmenu_id(id: u16) -> HMENU {
    HMENU(id as usize as *mut _)
}

fn style_with_class(base: WINDOW_STYLE, class_style: i32) -> WINDOW_STYLE {
    WINDOW_STYLE(base.0 | class_style as u32)
}

/// Controls are created at the origin with zero size; `relayout` places them so
/// the layout table has exactly one consumer.
unsafe fn create_controls(st: &mut AppState) {
    let hwnd = st.hwnd;
    let inst = GetModuleHandleW(PCWSTR::null()).unwrap_or_default();

    for (i, text) in LABEL_TEXT.iter().enumerate() {
        st.labels[i] = create_static(hwnd, *text, inst);
    }

    // Mode combo
    let combo = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("COMBOBOX"),
        PCWSTR::null(),
        style_with_class(
            WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL,
            CBS_DROPDOWNLIST,
        ),
        0,
        0,
        0,
        0,
        hwnd,
        hmenu_id(ID_MODE_COMBO),
        inst,
        None,
    )
    .unwrap_or_default();
    st.combo = combo;
    let _ = SendMessageW(combo, CB_ADDSTRING, WPARAM(0), LPARAM(w!("Zen").0 as isize));
    let _ = SendMessageW(
        combo,
        CB_ADDSTRING,
        WPARAM(0),
        LPARAM(w!("Visible").0 as isize),
    );

    // Interval edit
    st.edit_interval = CreateWindowExW(
        WS_EX_CLIENTEDGE,
        w!("EDIT"),
        PCWSTR::null(),
        style_with_class(WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_BORDER, ES_NUMBER),
        0,
        0,
        0,
        0,
        hwnd,
        hmenu_id(ID_INTERVAL_EDIT),
        inst,
        None,
    )
    .unwrap_or_default();

    // Visible-mode dx edit
    st.edit_dx = CreateWindowExW(
        WS_EX_CLIENTEDGE,
        w!("EDIT"),
        PCWSTR::null(),
        style_with_class(WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_BORDER, ES_NUMBER),
        0,
        0,
        0,
        0,
        hwnd,
        hmenu_id(ID_VISIBLE_DX_EDIT),
        inst,
        None,
    )
    .unwrap_or_default();

    // Start/Stop button
    st.btn_start = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("BUTTON"),
        w!("Start"),
        style_with_class(WS_CHILD | WS_VISIBLE | WS_TABSTOP, BS_DEFPUSHBUTTON),
        0,
        0,
        0,
        0,
        hwnd,
        hmenu_id(ID_START_BTN),
        inst,
        None,
    )
    .unwrap_or_default();

    st.chk_autostart =
        create_checkbox(hwnd, w!("Start with Windows"), ID_AUTOSTART_CHK, inst);
    st.chk_startmin =
        create_checkbox(hwnd, w!("Start minimized to tray"), ID_STARTMIN_CHK, inst);
}

unsafe fn create_static(parent: HWND, text: PCWSTR, inst: HMODULE) -> HWND {
    CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("STATIC"),
        text,
        style_with_class(WS_CHILD | WS_VISIBLE, SS_LEFT.0 as i32),
        0,
        0,
        0,
        0,
        parent,
        HMENU(null_mut()),
        inst,
        None,
    )
    .unwrap_or_default()
}

unsafe fn create_checkbox(parent: HWND, text: PCWSTR, id: u16, inst: HMODULE) -> HWND {
    CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("BUTTON"),
        text,
        style_with_class(WS_CHILD | WS_VISIBLE | WS_TABSTOP, BS_AUTOCHECKBOX),
        0,
        0,
        0,
        0,
        parent,
        hmenu_id(id),
        inst,
        None,
    )
    .unwrap_or_default()
}

// ---- Apply persisted settings to controls ---------------------------------

unsafe fn apply_settings_to_controls(st: &AppState) {
    let mode_idx: usize = match Mode::from_str_lossy(&st.settings.mode) {
        Mode::Zen => 0,
        Mode::Visible => 1,
    };
    let _ = SendMessageW(st.combo, CB_SETCURSEL, WPARAM(mode_idx), LPARAM(0));

    let s = format!("{}", st.settings.interval_sec);
    let s_w: Vec<u16> = s.encode_utf16().chain(Some(0)).collect();
    let _ = SetWindowTextW(st.edit_interval, PCWSTR(s_w.as_ptr()));

    let dx = format!("{}", st.settings.visible_dx);
    let dx_w: Vec<u16> = dx.encode_utf16().chain(Some(0)).collect();
    let _ = SetWindowTextW(st.edit_dx, PCWSTR(dx_w.as_ptr()));

    let _ = SendMessageW(
        st.chk_autostart,
        BM_SETCHECK,
        WPARAM(if st.settings.autostart {
            BST_CHECKED.0 as usize
        } else {
            BST_UNCHECKED.0 as usize
        }),
        LPARAM(0),
    );
    let _ = SendMessageW(
        st.chk_startmin,
        BM_SETCHECK,
        WPARAM(if st.settings.start_minimized {
            BST_CHECKED.0 as usize
        } else {
            BST_UNCHECKED.0 as usize
        }),
        LPARAM(0),
    );

    let _ = SetFocus(st.btn_start);
}

// ---- Command handling -----------------------------------------------------

unsafe fn handle_command(st: &mut AppState, wparam: WPARAM, _lparam: LPARAM) {
    let id = (wparam.0 & 0xFFFF) as u16;
    let notif = ((wparam.0 >> 16) & 0xFFFF) as u32;

    match id {
        ID_START_BTN => {
            if notif == BN_CLICKED {
                toggle_running(st);
            }
        }
        ID_MODE_COMBO => {
            if notif == CBN_SELCHANGE {
                let sel = SendMessageW(st.combo, CB_GETCURSEL, WPARAM(0), LPARAM(0));
                let mode = if sel.0 == 1 { Mode::Visible } else { Mode::Zen };
                st.jiggler.set_mode(mode);
                st.settings.mode = mode.as_str().into();
                schedule_save(st);
            }
        }
        ID_INTERVAL_EDIT => {
            if notif == EN_KILLFOCUS {
                let val = read_edit_u32(st.edit_interval).unwrap_or(60);
                let val = val.clamp(1, 86400);
                if val != st.settings.interval_sec {
                    st.settings.interval_sec = val;
                    let s = format!("{}", val);
                    let s_w: Vec<u16> = s.encode_utf16().chain(Some(0)).collect();
                    let _ = SetWindowTextW(st.edit_interval, PCWSTR(s_w.as_ptr()));
                    if st.running {
                        let _ = KillTimer(st.hwnd, TIMER_JIGGLE);
                        let _ = SetTimer(
                            st.hwnd,
                            TIMER_JIGGLE,
                            val.saturating_mul(1000),
                            None,
                        );
                    }
                    schedule_save(st);
                }
            }
        }
        ID_VISIBLE_DX_EDIT => {
            if notif == EN_KILLFOCUS {
                let val = read_edit_u32(st.edit_dx).unwrap_or(1);
                let val = val.clamp(1, 100);
                // Always rewrite the field to canonicalize (reject 0/garbage).
                let s = format!("{}", val);
                let s_w: Vec<u16> = s.encode_utf16().chain(Some(0)).collect();
                let _ = SetWindowTextW(st.edit_dx, PCWSTR(s_w.as_ptr()));
                if val != st.settings.visible_dx {
                    st.settings.visible_dx = val;
                    st.jiggler.set_visible_dx(val);
                    schedule_save(st);
                }
            }
        }
        ID_AUTOSTART_CHK => {
            let checked = SendMessageW(st.chk_autostart, BM_GETCHECK, WPARAM(0), LPARAM(0))
                .0
                == BST_CHECKED.0 as isize;
            match crate::autostart::set(checked) {
                Ok(()) => {
                    st.settings.autostart = checked;
                    schedule_save(st);
                }
                Err(_) => {
                    let _ = SendMessageW(
                        st.chk_autostart,
                        BM_SETCHECK,
                        WPARAM(if st.settings.autostart {
                            BST_CHECKED.0 as usize
                        } else {
                            BST_UNCHECKED.0 as usize
                        }),
                        LPARAM(0),
                    );
                }
            }
        }
        ID_STARTMIN_CHK => {
            let checked = SendMessageW(st.chk_startmin, BM_GETCHECK, WPARAM(0), LPARAM(0))
                .0
                == BST_CHECKED.0 as isize;
            st.settings.start_minimized = checked;
            schedule_save(st);
        }
        ID_TRAY_SHOW => {
            let _ = ShowWindow(st.hwnd, SW_SHOW);
            let _ = SetForegroundWindow(st.hwnd);
        }
        ID_TRAY_TOGGLE => toggle_running(st),
        // ID_TRAY_EXIT is handled in WndProc itself — see comment in WM_COMMAND arm.
        _ => {}
    }
}

unsafe fn toggle_running(st: &mut AppState) {
    if st.running {
        let _ = KillTimer(st.hwnd, TIMER_JIGGLE);
        st.running = false;
        let _ = SetWindowTextW(st.btn_start, w!("Start"));
    } else {
        let interval = st.settings.interval_sec.max(1);
        let id = SetTimer(
            st.hwnd,
            TIMER_JIGGLE,
            interval.saturating_mul(1000),
            None,
        );
        if id != 0 {
            st.running = true;
            let _ = SetWindowTextW(st.btn_start, w!("Stop"));
        }
    }
    tray::update_tooltip(st.hwnd, st.running);
}

unsafe fn read_edit_u32(h: HWND) -> Option<u32> {
    let len = GetWindowTextLengthW(h);
    // Defensive bounds: ES_NUMBER input fits trivially in 16 chars.
    if len <= 0 || len > 16 {
        return None;
    }
    let mut buf = vec![0u16; (len as usize) + 1];
    let n = GetWindowTextW(h, &mut buf) as usize;
    let n = n.min(buf.len());
    let s = String::from_utf16_lossy(&buf[..n]);
    s.trim().parse::<u32>().ok()
}

unsafe fn schedule_save(st: &AppState) {
    if st.suppress_save.get() {
        return;
    }
    let _ = SetTimer(st.hwnd, TIMER_SAVE, SAVE_DEBOUNCE_MS, None);
}
