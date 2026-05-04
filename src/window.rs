// Main window: registration, control creation, message loop, command routing.
//
// AppState lives on the heap and is owned by the window through GWLP_USERDATA.
// It is allocated before CreateWindowExW and dropped in WM_NCDESTROY.

use std::cell::Cell;
use std::ptr::null_mut;

use windows::core::{w, PCWSTR, Result};
use windows::Win32::Foundation::{HMODULE, HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    GetStockObject, UpdateWindow, COLOR_BTNFACE, DEFAULT_GUI_FONT, HBRUSH,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::SystemServices::SS_LEFT;
use windows::Win32::UI::Controls::{BST_CHECKED, BST_UNCHECKED};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::Win32::UI::Input::KeyboardAndMouse::SetFocus;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetMessageW,
    GetWindowLongPtrW, GetWindowTextLengthW, GetWindowTextW, KillTimer, LoadCursorW,
    LoadIconW, PostQuitMessage, RegisterClassExW, SendMessageW, SetForegroundWindow,
    SetTimer, SetWindowLongPtrW, SetWindowTextW, ShowWindow, TranslateMessage,
    BM_GETCHECK, BM_SETCHECK, BN_CLICKED, BS_AUTOCHECKBOX, BS_DEFPUSHBUTTON,
    CBN_SELCHANGE, CB_ADDSTRING, CB_GETCURSEL, CB_SETCURSEL, CBS_DROPDOWNLIST,
    CREATESTRUCTW, CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, EN_KILLFOCUS, ES_NUMBER,
    GWLP_USERDATA, HMENU, IDC_ARROW, IDI_APPLICATION, MSG, SC_MINIMIZE, SW_HIDE,
    SW_SHOW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WM_CLOSE, WM_COMMAND, WM_CREATE,
    WM_DESTROY, WM_NCCREATE, WM_NCDESTROY, WM_SETFONT, WM_SYSCOMMAND, WM_TIMER,
    WNDCLASSEXW, WS_BORDER, WS_CAPTION, WS_CHILD, WS_EX_CLIENTEDGE, WS_MINIMIZEBOX,
    WS_OVERLAPPED, WS_SYSMENU, WS_TABSTOP, WS_VISIBLE, WS_VSCROLL,
};

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

pub const ID_TRAY_SHOW: u16 = 2001;
pub const ID_TRAY_TOGGLE: u16 = 2002;
pub const ID_TRAY_EXIT: u16 = 2003;

pub const TRAY_ICON_ID: u32 = 1;
pub const WM_TRAY_CALLBACK: u32 = WM_APP + 1;

/// Resource id for embedded icon (see `app.rc`: `IDI_APP ICON ...`).
pub const IDI_APP: u16 = 1;

pub const CLASS_NAME: PCWSTR = w!("MouseWigglerMain");
const SAVE_DEBOUNCE_MS: u32 = 250;

// ---- AppState --------------------------------------------------------------

pub struct AppState {
    pub hwnd: HWND,
    pub combo: HWND,
    pub edit_interval: HWND,
    pub btn_start: HWND,
    pub chk_autostart: HWND,
    pub chk_startmin: HWND,
    pub jiggler: Jiggler,
    pub settings: Settings,
    pub running: bool,
    /// Suppresses save during initial control population.
    suppress_save: Cell<bool>,
}

impl AppState {
    fn new(cfg: Settings) -> Self {
        let mode = Mode::from_str_lossy(&cfg.mode);
        Self {
            hwnd: HWND(null_mut()),
            combo: HWND(null_mut()),
            edit_interval: HWND(null_mut()),
            btn_start: HWND(null_mut()),
            chk_autostart: HWND(null_mut()),
            chk_startmin: HWND(null_mut()),
            jiggler: Jiggler::new(mode),
            settings: cfg,
            running: false,
            suppress_save: Cell::new(true),
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
            hIcon: LoadIconW(instance, PCWSTR(IDI_APP as *const u16))
                .or_else(|_| LoadIconW(HMODULE(null_mut()), IDI_APPLICATION))
                .unwrap_or_default(),
            ..Default::default()
        };
        let atom = RegisterClassExW(&wc);
        if atom == 0 {
            return Err(windows::core::Error::from_win32());
        }

        let state = Box::new(AppState::new(cfg));
        let start_minimized = state.settings.start_minimized;
        let state_ptr = Box::into_raw(state);

        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            CLASS_NAME,
            w!("Mouse Wiggler"),
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            260,
            220,
            HWND(null_mut()),
            HMENU(null_mut()),
            instance,
            Some(state_ptr as *const _),
        )?;

        if start_minimized {
            let _ = ShowWindow(hwnd, SW_HIDE);
        } else {
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = UpdateWindow(hwnd);
        }

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, HWND(null_mut()), 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
    Ok(())
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
                    create_controls(st);
                    apply_settings_to_controls(st);
                    tray::add(hwnd, st.running);
                    if st.settings.autostart {
                        let _ = crate::autostart::reconcile(&st.settings);
                    }
                    st.suppress_save.set(false);
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

// ---- Control creation -----------------------------------------------------

fn hmenu_id(id: u16) -> HMENU {
    HMENU(id as usize as *mut _)
}

fn style_with_class(base: WINDOW_STYLE, class_style: i32) -> WINDOW_STYLE {
    WINDOW_STYLE(base.0 | class_style as u32)
}

unsafe fn create_controls(st: &mut AppState) {
    let hwnd = st.hwnd;
    let dpi = GetDpiForWindow(hwnd);
    let scale = if dpi == 0 { 1.0 } else { dpi as f32 / 96.0 };
    let s = |v: i32| -> i32 { (v as f32 * scale).round() as i32 };

    let inst = GetModuleHandleW(PCWSTR::null()).unwrap_or_default();

    create_static(hwnd, w!("Mode:"), s(12), s(12), s(60), s(20), inst);
    create_static(hwnd, w!("Every:"), s(12), s(42), s(60), s(20), inst);
    create_static(hwnd, w!("seconds"), s(178), s(42), s(70), s(20), inst);

    // Mode combo
    let combo = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("COMBOBOX"),
        PCWSTR::null(),
        style_with_class(WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_VSCROLL, CBS_DROPDOWNLIST),
        s(78),
        s(10),
        s(160),
        s(120),
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
    let edit = CreateWindowExW(
        WS_EX_CLIENTEDGE,
        w!("EDIT"),
        PCWSTR::null(),
        style_with_class(WS_CHILD | WS_VISIBLE | WS_TABSTOP | WS_BORDER, ES_NUMBER),
        s(78),
        s(40),
        s(90),
        s(22),
        hwnd,
        hmenu_id(ID_INTERVAL_EDIT),
        inst,
        None,
    )
    .unwrap_or_default();
    st.edit_interval = edit;

    // Start/Stop button
    let btn = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("BUTTON"),
        w!("Start"),
        style_with_class(WS_CHILD | WS_VISIBLE | WS_TABSTOP, BS_DEFPUSHBUTTON),
        s(12),
        s(74),
        s(232),
        s(28),
        hwnd,
        hmenu_id(ID_START_BTN),
        inst,
        None,
    )
    .unwrap_or_default();
    st.btn_start = btn;

    // Checkboxes
    let chk_autostart = create_checkbox(
        hwnd,
        w!("Start with Windows"),
        ID_AUTOSTART_CHK,
        s(12),
        s(112),
        s(232),
        s(20),
        inst,
    );
    st.chk_autostart = chk_autostart;

    let chk_startmin = create_checkbox(
        hwnd,
        w!("Start minimized to tray"),
        ID_STARTMIN_CHK,
        s(12),
        s(136),
        s(232),
        s(20),
        inst,
    );
    st.chk_startmin = chk_startmin;

    // Apply default GUI font to all children.
    let font_obj = GetStockObject(DEFAULT_GUI_FONT);
    for h in [combo, edit, btn, chk_autostart, chk_startmin] {
        let _ = SendMessageW(
            h,
            WM_SETFONT,
            WPARAM(font_obj.0 as usize),
            LPARAM(1),
        );
    }
}

unsafe fn create_static(
    parent: HWND,
    text: PCWSTR,
    x: i32,
    y: i32,
    cx: i32,
    cy: i32,
    inst: HMODULE,
) -> HWND {
    let h = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("STATIC"),
        text,
        style_with_class(WS_CHILD | WS_VISIBLE, SS_LEFT.0 as i32),
        x,
        y,
        cx,
        cy,
        parent,
        HMENU(null_mut()),
        inst,
        None,
    )
    .unwrap_or_default();
    let font_obj = GetStockObject(DEFAULT_GUI_FONT);
    let _ = SendMessageW(h, WM_SETFONT, WPARAM(font_obj.0 as usize), LPARAM(1));
    h
}

unsafe fn create_checkbox(
    parent: HWND,
    text: PCWSTR,
    id: u16,
    x: i32,
    y: i32,
    cx: i32,
    cy: i32,
    inst: HMODULE,
) -> HWND {
    CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("BUTTON"),
        text,
        style_with_class(WS_CHILD | WS_VISIBLE | WS_TABSTOP, BS_AUTOCHECKBOX),
        x,
        y,
        cx,
        cy,
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
