// Mouse Wiggler — entry point.
// Phase 01 scaffolding: register a class, show an empty centered window,
// run a standard message loop. Phases 02-07 hook into modules below.

#![windows_subsystem = "windows"]

mod autostart;
mod jiggler;
mod settings;
mod tray;
mod window;

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS};
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows::Win32::UI::WindowsAndMessaging::{
    FindWindowW, IsIconic, SetForegroundWindow, ShowWindow, SW_RESTORE, SW_SHOW,
};

fn main() -> windows::core::Result<()> {
    // Best-effort: process is already PerMonitorV2 via manifest, but this is harmless
    // and helps when the manifest is stripped by some packagers.
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    // Single-instance guard. The mutex handle is intentionally leaked into the
    // process — kernel auto-releases on exit. ERROR_ALREADY_EXISTS means another
    // instance is running; surface its window and bail out.
    if !acquire_singleton() {
        return Ok(());
    }

    // Load persisted settings (defaults if file is missing or corrupt).
    let cfg = settings::load();

    // Reconcile autostart registry with persisted flag (self-heals moved .exe).
    let _ = autostart::reconcile(&cfg);

    // Hand off to the window module which owns the GUI lifecycle.
    window::run(cfg)
}

fn acquire_singleton() -> bool {
    unsafe {
        let _mutex = CreateMutexW(None, false, w!("MouseWigglerSingleton_v1"));
        if GetLastError() != ERROR_ALREADY_EXISTS {
            return true;
        }

        if let Ok(hwnd) = FindWindowW(window::CLASS_NAME, PCWSTR::null()) {
            if !hwnd.0.is_null() {
                let cmd = if IsIconic(hwnd).as_bool() { SW_RESTORE } else { SW_SHOW };
                let _ = ShowWindow(hwnd, cmd);
                let _ = SetForegroundWindow(hwnd);
            }
        }
        false
    }
}
