---
phase: 6
name: Autostart registry
status: completed
priority: medium
effort: S
---

# Phase 06 — Autostart (HKCU Run)

## Context
- [plan.md](plan.md)
- Depends on: phase-05

## Overview
Toggle a user-scope `Run` registry value pointing at the current .exe. No admin required (HKCU). Reconcile on startup with the persisted `autostart` flag.

## Files to Create
- `src/autostart.rs`

## Files to Modify
- `src/main.rs` — call `autostart::reconcile()` after settings load.
- `src/window.rs` — call `autostart::set(checked)` from autostart checkbox handler; on failure, revert checkbox + persisted flag.

## Registry Path
```
HKCU\Software\Microsoft\Windows\CurrentVersion\Run
  Value name: "MouseWiggler"
  Value type: REG_SZ
  Value data: "<exe path>"   (quoted if path contains spaces)
```

## API
```rust
// src/autostart.rs
pub fn is_enabled() -> bool;
pub fn set(enable: bool) -> windows::core::Result<()>;
pub fn current_exe_command() -> String; // returns quoted path
```

## Implementation Steps

### 1. Get current exe path (quoted)
```rust
fn current_exe_command() -> String {
    let mut buf = [0u16; 260];
    let n = unsafe { GetModuleFileNameW(None, &mut buf) } as usize;
    let path = String::from_utf16_lossy(&buf[..n]);
    format!("\"{}\"", path)
}
```

### 2. is_enabled
- `RegOpenKeyExW(HKEY_CURRENT_USER, "Software\\Microsoft\\Windows\\CurrentVersion\\Run", 0, KEY_READ)`.
- `RegQueryValueExW(key, "MouseWiggler", ...)`. Exists with our path → true. Missing → false.
- (Optional sanity) Compare retrieved string to `current_exe_command()`; if mismatch, treat as "disabled" so toggling re-writes correct path.

### 3. set(true)
- `RegOpenKeyExW(... KEY_SET_VALUE)`.
- `RegSetValueExW(key, "MouseWiggler", 0, REG_SZ, <utf16 bytes incl null>, byte_len)`.

### 4. set(false)
- `RegDeleteValueW(key, "MouseWiggler")`. Ignore "value not found" error.

### 5. Reconcile on startup
- After loading settings:
  ```
  let registry = is_enabled();
  if settings.autostart != registry {
      // Trust settings; rewrite registry. Cheap and idempotent.
      let _ = set(settings.autostart);
  }
  ```
- This also self-heals if the .exe was moved (path changed).

### 6. Checkbox UX
- On click: `state.settings.autostart = checked;`
- Call `autostart::set(checked)`. On error: revert checkbox visually, revert state, do NOT persist.
- Trigger debounced settings save.

## Todo
- [x] Create `src/autostart.rs` with `is_enabled`, `set`, `current_exe_command`
- [x] Wire startup reconcile after settings load
- [x] Wire checkbox handler with error-revert

## Success Criteria
- Checking the box adds an HKCU Run entry pointing at the .exe.
- Unchecking removes it.
- After enabling autostart and reboot, the app launches automatically.
- Moving the .exe and relaunching with autostart on → registry value updated to new path.
- HKLM is **never** written (no admin needed, no UAC).

## Risks
- **AV / IT policies** may flag changes to `Run` keys. Acceptable for personal use.
- **Path with quotes**: if exe path itself contains a literal `"` (extremely rare on Windows), our naive quoting breaks. Skip — Windows doesn't allow `"` in filenames anyway.
- **Roaming profiles**: HKCU values roam. Path may be invalid on another machine. Self-heal on next launch handles it.

## Next
Phase 07 — packaging and manual test.
