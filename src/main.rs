// Mouse Wiggler — entry point.
// Phase 01 scaffolding: register a class, show an empty centered window,
// run a standard message loop. Phases 02-07 hook into modules below.

#![windows_subsystem = "windows"]

mod autostart;
mod jiggler;
mod settings;
mod tray;
mod window;

use windows::Win32::UI::HiDpi::{
    SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};

fn main() -> windows::core::Result<()> {
    // Best-effort: process is already PerMonitorV2 via manifest, but this is harmless
    // and helps when the manifest is stripped by some packagers.
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    // Load persisted settings (defaults if file is missing or corrupt).
    let cfg = settings::load();

    // Reconcile autostart registry with persisted flag (self-heals moved .exe).
    let _ = autostart::reconcile(&cfg);

    // Hand off to the window module which owns the GUI lifecycle.
    window::run(cfg)
}
