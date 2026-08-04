// DPI helpers for the window layout.
//
// Every layout constant in `window.rs` is authored in 96-DPI design units and
// converted to physical pixels through `scale`. The UI font is queried per-DPI
// so text matches the shell instead of the fixed-size DEFAULT_GUI_FONT.

use std::ffi::c_void;
use std::mem::size_of;

use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Gdi::{CreateFontIndirectW, HFONT, LOGFONTW};
use windows::Win32::UI::HiDpi::{GetDpiForWindow, SystemParametersInfoForDpi};
use windows::Win32::UI::WindowsAndMessaging::{NONCLIENTMETRICSW, SPI_GETNONCLIENTMETRICS};

/// DPI the layout constants are authored against.
pub const BASE_DPI: u32 = 96;

/// Current DPI of `hwnd`; falls back to 96 when the window is not yet valid.
pub fn dpi_for(hwnd: HWND) -> u32 {
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    if dpi == 0 {
        BASE_DPI
    } else {
        dpi
    }
}

/// Convert a 96-DPI design unit to physical pixels (rounded to nearest).
pub fn scale(v: i32, dpi: u32) -> i32 {
    let base = BASE_DPI as i64;
    ((v as i64 * dpi as i64 + base / 2) / base) as i32
}

/// Create the shell message font sized for `dpi`.
///
/// The caller owns the handle and must `DeleteObject` it. Returns a null HFONT
/// only if GDI itself refuses; callers keep their previous font in that case.
pub fn ui_font(dpi: u32) -> HFONT {
    unsafe {
        let mut ncm = NONCLIENTMETRICSW {
            cbSize: size_of::<NONCLIENTMETRICSW>() as u32,
            ..Default::default()
        };
        // Note: unlike SystemParametersInfoW, the ForDpi variant types uiaction
        // and fwinini as plain u32.
        let queried = SystemParametersInfoForDpi(
            SPI_GETNONCLIENTMETRICS.0,
            size_of::<NONCLIENTMETRICSW>() as u32,
            Some(&mut ncm as *mut NONCLIENTMETRICSW as *mut c_void),
            0,
            dpi,
        )
        .is_ok();

        let lf = if queried {
            ncm.lfMessageFont
        } else {
            fallback_logfont(dpi)
        };
        CreateFontIndirectW(&lf)
    }
}

/// Segoe UI 9pt at `dpi` — used only if the metrics query fails.
fn fallback_logfont(dpi: u32) -> LOGFONTW {
    let mut lf = LOGFONTW::default();
    // Negative height = character height in pixels: -MulDiv(9pt, dpi, 72).
    lf.lfHeight = -((9 * dpi as i32 + 36) / 72);
    for (i, ch) in "Segoe UI".encode_utf16().enumerate() {
        lf.lfFaceName[i] = ch;
    }
    lf
}
