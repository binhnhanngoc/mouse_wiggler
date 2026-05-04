---
phase: 4
name: System tray
status: completed
priority: high
effort: M
---

# Phase 04 — System Tray

## Context
- [plan.md](plan.md)
- Depends on: phase-03

## Overview
Add tray icon via `Shell_NotifyIconW`. Minimize-to-tray behavior. Right-click popup menu (Show, Start/Stop, Exit). Left-click restores window. Close (X) hides to tray; only tray-Exit quits.

## Files to Create
- `src/tray.rs`

## Files to Modify
- `src/main.rs` — handle `WM_APP+1` callback, route to tray module.
- `src/window.rs` — change `WM_CLOSE` to hide window instead of destroy. Handle `WM_SYSCOMMAND/SC_MINIMIZE` to hide.

## Constants
```rust
const TRAY_ICON_ID: u32 = 1;
const WM_TRAY_CALLBACK: u32 = WM_APP + 1;
const ID_TRAY_SHOW:     u32 = 2001;
const ID_TRAY_TOGGLE:   u32 = 2002; // Start or Stop
const ID_TRAY_EXIT:     u32 = 2003;
```

## Implementation Steps

### 1. Add tray icon
On `WM_CREATE` (after window created):
```rust
NOTIFYICONDATAW {
    cbSize: size_of::<NOTIFYICONDATAW>() as u32,
    hWnd: hWnd,
    uID: TRAY_ICON_ID,
    uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
    uCallbackMessage: WM_TRAY_CALLBACK,
    hIcon: LoadIconW(instance, IDI_APP) // embedded in phase 07
        .unwrap_or_else(|_| LoadIconW(None, IDI_APPLICATION).unwrap()),
    szTip: utf16_zero_padded("Mouse Wiggler"),
    ..zeroed()
}
Shell_NotifyIconW(NIM_ADD, &nid);
```
On `WM_DESTROY` → `NIM_DELETE`.

### 2. Tray callback (`WM_TRAY_CALLBACK`)
- `lParam` LOWORD = event:
  - `WM_LBUTTONUP` → restore window: `ShowWindow(hWnd, SW_SHOW)`, `SetForegroundWindow(hWnd)`.
  - `WM_RBUTTONUP` → popup menu at cursor.
  - `WM_LBUTTONDBLCLK` → same as left-click (restore).

### 3. Popup menu
```rust
let menu = CreatePopupMenu();
AppendMenuW(menu, MF_STRING, ID_TRAY_SHOW as usize, "Show");
AppendMenuW(menu, MF_STRING, ID_TRAY_TOGGLE as usize, if running { "Stop" } else { "Start" });
AppendMenuW(menu, MF_SEPARATOR, 0, None);
AppendMenuW(menu, MF_STRING, ID_TRAY_EXIT as usize, "Exit");
GetCursorPos(&mut pt);
SetForegroundWindow(hWnd); // required for popup to dismiss correctly
TrackPopupMenu(menu, TPM_RIGHTBUTTON, pt.x, pt.y, 0, hWnd, None);
DestroyMenu(menu);
```
- Tray menu choices route through `WM_COMMAND` like normal buttons (LOWORD = ID_TRAY_*).

### 4. Hide-on-close / hide-on-minimize
- `WM_CLOSE` → `ShowWindow(hWnd, SW_HIDE)`. **Do not call DefWindowProc.**
- `WM_SYSCOMMAND` with `wParam == SC_MINIMIZE` → `ShowWindow(hWnd, SW_HIDE)` and return 0.
- Document: tray-Exit is the only quit path → calls `DestroyWindow`.

### 5. Tray tooltip updates
When running state changes, optionally update tooltip via `NIM_MODIFY` with `"Mouse Wiggler — Running"` / `"Mouse Wiggler — Stopped"`. Optional polish.

## Todo
- [x] Create `src/tray.rs` with `add`, `remove`, `show_menu`, `set_tooltip`
- [x] Define `WM_TRAY_CALLBACK` and tray menu IDs
- [x] Override `WM_CLOSE` and `SC_MINIMIZE` to hide
- [x] Implement popup menu handlers (Show, Start/Stop, Exit)
- [x] Update tooltip on state change (optional polish)

## Success Criteria
- Closing the X button hides window; tray icon remains.
- Minimize button hides to tray (no taskbar entry).
- Right-click tray → menu appears with current state ("Start" or "Stop").
- Left-click / double-click tray restores window.
- Tray-Exit terminates the process cleanly (icon disappears, no leak).

## Risks
- **Orphaned tray icons** if process crashes — Windows shows them until mouseover. No fix beyond clean shutdown handling.
- **Foreground rules**: `TrackPopupMenu` won't dismiss on outside-click without `SetForegroundWindow` first.
- **Multiple instances**: not handled — running twice creates two icons. Acceptable for v1.

## Next
Phase 05 — settings persistence.
