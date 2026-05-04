// Autostart via HKCU\Software\Microsoft\Windows\CurrentVersion\Run.
// User-scope; no admin required. Stores the quoted current-exe path.

use windows::core::{PCWSTR, Result};
use windows::Win32::Foundation::ERROR_FILE_NOT_FOUND;
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW,
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_SZ,
};

use crate::settings::Settings;

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const VALUE_NAME: &str = "MouseWiggler";

fn to_wide_z(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Returns the current executable path, quoted (so paths with spaces work
/// when Windows parses the Run value as a command line).
pub fn current_exe_command() -> String {
    let mut buf = [0u16; 1024];
    let n = unsafe { GetModuleFileNameW(None, &mut buf) } as usize;
    let path = String::from_utf16_lossy(&buf[..n]);
    format!("\"{}\"", path)
}

pub fn is_enabled() -> bool {
    unsafe {
        let mut hkey = HKEY::default();
        let key_w = to_wide_z(RUN_KEY);
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(key_w.as_ptr()),
            0,
            KEY_READ,
            &mut hkey,
        )
        .is_err()
        {
            return false;
        }
        let val_w = to_wide_z(VALUE_NAME);
        let mut size = 0u32;
        let exists = RegQueryValueExW(
            hkey,
            PCWSTR(val_w.as_ptr()),
            None,
            None,
            None,
            Some(&mut size),
        )
        .is_ok();
        let _ = RegCloseKey(hkey);
        exists
    }
}

pub fn set(enable: bool) -> Result<()> {
    unsafe {
        let mut hkey = HKEY::default();
        let key_w = to_wide_z(RUN_KEY);
        RegOpenKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(key_w.as_ptr()),
            0,
            KEY_SET_VALUE,
            &mut hkey,
        )
        .ok()?;

        let val_w = to_wide_z(VALUE_NAME);
        let result = if enable {
            let cmd = current_exe_command();
            let cmd_w = to_wide_z(&cmd);
            // SAFETY: cmd_w is a valid Vec<u16>; we view its bytes as REG_SZ payload.
            let bytes = std::slice::from_raw_parts(
                cmd_w.as_ptr() as *const u8,
                cmd_w.len() * std::mem::size_of::<u16>(),
            );
            RegSetValueExW(
                hkey,
                PCWSTR(val_w.as_ptr()),
                0,
                REG_SZ,
                Some(bytes),
            )
            .ok()
        } else {
            let r = RegDeleteValueW(hkey, PCWSTR(val_w.as_ptr()));
            if r == ERROR_FILE_NOT_FOUND {
                Ok(())
            } else {
                r.ok()
            }
        };

        let _ = RegCloseKey(hkey);
        result
    }
}

/// At startup, ensure the registry value matches the persisted preference.
/// This also self-heals when the .exe was moved.
pub fn reconcile(cfg: &Settings) -> Result<()> {
    if is_enabled() == cfg.autostart {
        return Ok(());
    }
    set(cfg.autostart)
}
