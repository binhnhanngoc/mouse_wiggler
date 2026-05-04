// Persisted user settings stored as `settings.json` next to the .exe.
//
// Loading is fault-tolerant: a missing or corrupt file silently falls back
// to defaults. Saving is atomic via a tmp file + MoveFileExW(REPLACE_EXISTING).

use serde::{Deserialize, Serialize};
use std::ffi::OsString;
use std::fs;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;

use windows::core::PCWSTR;
use windows::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_REPLACE_EXISTING};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub mode: String,        // "zen" | "visible"
    pub interval_sec: u32,   // clamped to 1..=86400 on load
    pub autostart: bool,
    pub start_minimized: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: "zen".into(),
            interval_sec: 60,
            autostart: false,
            start_minimized: false,
        }
    }
}

impl Settings {
    fn validated(mut self) -> Self {
        if self.interval_sec == 0 {
            self.interval_sec = 60;
        }
        self.interval_sec = self.interval_sec.clamp(1, 86400);
        let m = self.mode.to_ascii_lowercase();
        if m != "zen" && m != "visible" {
            self.mode = "zen".into();
        } else {
            self.mode = m;
        }
        self
    }
}

/// Resolve `<dir-of-exe>/settings.json`.
pub fn settings_path() -> PathBuf {
    let mut buf = [0u16; 1024];
    let n = unsafe { GetModuleFileNameW(None, &mut buf) } as usize;
    let exe = PathBuf::from(OsString::from_wide(&buf[..n]));
    exe.with_file_name("settings.json")
}

pub fn load() -> Settings {
    let path = settings_path();
    // Best-effort cleanup of a tmp file orphaned by a prior crash mid-save.
    let _ = fs::remove_file(path.with_extension("json.tmp"));
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
        .map(Settings::validated)
        .unwrap_or_default()
}

pub fn save(cfg: &Settings) {
    let path = settings_path();
    let tmp = path.with_extension("json.tmp");
    let Ok(json) = serde_json::to_vec_pretty(cfg) else {
        return;
    };
    if fs::write(&tmp, &json).is_err() {
        return;
    }

    // Atomic rename: tmp -> final.
    let from: Vec<u16> = tmp.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
    unsafe {
        let _ = MoveFileExW(
            PCWSTR(from.as_ptr()),
            PCWSTR(to.as_ptr()),
            MOVEFILE_REPLACE_EXISTING,
        );
    }
}
