---
phase: 5
name: Settings persistence
status: completed
priority: medium
effort: S
---

# Phase 05 — Settings Persistence

## Context
- [plan.md](plan.md)
- Depends on: phase-03, phase-04

## Overview
Persist user prefs to `settings.json` next to the .exe. Load on startup → apply to controls + state. Save (debounced) when any setting changes.

## Files to Create
- `src/settings.rs`

## Files to Modify
- `src/main.rs` — load settings before window creation, pass into AppState.
- `src/window.rs` — call `settings::save_debounced()` from control change handlers.

## Schema
```rust
// src/settings.rs
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub mode: String,          // "zen" | "visible"
    pub interval_sec: u32,     // clamp 1..=86400 on load
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
```
JSON example:
```json
{
  "mode": "zen",
  "interval_sec": 60,
  "autostart": false,
  "start_minimized": false
}
```

## Implementation Steps

### 1. Path resolution
```rust
fn settings_path() -> PathBuf {
    let mut buf = [0u16; 260];
    let n = unsafe { GetModuleFileNameW(None, &mut buf) } as usize;
    let exe = PathBuf::from(OsString::from_wide(&buf[..n]));
    exe.with_file_name("settings.json")
}
```

### 2. Load
- Read file → `serde_json::from_str` → on any error, return `Settings::default()` (no panic).
- Validate: clamp `interval_sec` to `1..=86400`. Coerce unknown `mode` → `"zen"`.

### 3. Save
- Serialize with `serde_json::to_vec_pretty`.
- Atomic write: write to `settings.json.tmp` → `MoveFileExW(MOVEFILE_REPLACE_EXISTING)` to final.
- Swallow errors (log via `OutputDebugStringW`); don't crash UI.

### 4. Debounce
- 250ms after last change. Implement with a one-shot Win32 timer:
  - On change: `SetTimer(hWnd, SAVE_TIMER_ID, 250, None)` — re-arming overwrites.
  - On `WM_TIMER` matching `SAVE_TIMER_ID`: `KillTimer`, then `settings::save(&state.settings)`.

### 5. Apply on startup
- Load before creating window.
- After controls are created, set initial values:
  - `ComboBox_SetCurSel` → 0 (Zen) or 1 (Visible).
  - `SetWindowTextW(interval_edit, "60")`.
  - `SendMessageW(chk, BM_SETCHECK, BST_CHECKED/BST_UNCHECKED, 0)`.
- If `start_minimized` → call `ShowWindow(SW_HIDE)` instead of `SW_SHOW` after create.
- If `autostart` is true at startup, also reconcile with registry (phase 06).

## Todo
- [x] Create `src/settings.rs` with `Settings`, `load`, `save`, `settings_path`
- [x] Atomic save (tmp + rename)
- [x] Debounce timer (250ms) wired to control changes
- [x] Apply on window creation (mode, interval, checkboxes, start_minimized)
- [x] Fallback to default on read/parse error

## Success Criteria
- First run with no file → uses defaults (Zen, 60s, autostart off, start_minimized off).
- Change mode/interval/checkbox → file updates within 1s.
- Restart → previous values restored.
- Corrupt JSON → app still launches with defaults; bad file overwritten on next save.

## Risks
- **Unwritable directory** (e.g. `Program Files` without admin) — save silently fails. For private use this is OK; document it. Mitigation later: fall back to `%APPDATA%\MouseWiggler\settings.json`.
- **Concurrent instances** writing same file — last writer wins; not a real concern for single-user single-instance.

## Next
Phase 06 — autostart registry.
