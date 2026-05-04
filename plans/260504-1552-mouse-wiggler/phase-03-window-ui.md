---
phase: 3
name: Window UI
status: completed
priority: high
effort: M
---

# Phase 03 — Window UI

## Context
- [plan.md](plan.md)
- Depends on: phase-02

## Overview
Build the small main window with Mode dropdown, interval input, Start/Stop button, two checkboxes. Hook controls to AppState. No tray yet.

## Files to Create
- `src/window.rs`

## Files to Modify
- `src/main.rs` — delegate window registration/creation to `window::create()`.

## Layout (≈240×170 client area, fixed size, non-resizable)
```
┌─ Mouse Wiggler ─────────────┐
│ Mode:    [ Zen        ▾ ]  │  y=10
│ Every:   [   60  ] seconds  │  y=40
│                              │
│ [        Start        ]     │  y=72  full width
│ ☐ Start with Windows         │  y=108
│ ☐ Start minimized to tray    │  y=128
└──────────────────────────────┘
```
- Window style: `WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX` (no maximize, no resize).
- Optional: `WS_EX_TOPMOST` ON by default? **Decision: NO** — annoying. User can pin if they want via system menu.

## Control IDs
```rust
const ID_MODE_COMBO:    i32 = 1001;
const ID_INTERVAL_EDIT: i32 = 1002;
const ID_START_BTN:     i32 = 1003;
const ID_AUTOSTART_CHK: i32 = 1004;
const ID_STARTMIN_CHK:  i32 = 1005;
```

## Implementation Steps

### 1. Register window class
- Class name: `"MouseWigglerMain"`. Background `COLOR_BTNFACE+1`.
- Use default arrow cursor, no menu, default icon (real icon embedded in phase 07).

### 2. Create child controls in `WM_CREATE`
- Static labels: `"Mode:"`, `"Every:"`, `"seconds"`.
- ComboBox (`"COMBOBOX"`, `CBS_DROPDOWNLIST | WS_VSCROLL`) — populate via `CB_ADDSTRING` with "Zen", "Visible".
- Edit (`"EDIT"`, `ES_NUMBER | WS_BORDER`) — interval seconds.
- Button (`"BUTTON"`, `BS_PUSHBUTTON`) — initial caption "Start".
- Two checkboxes (`"BUTTON"`, `BS_AUTOCHECKBOX`).
- Use system font: `SendMessageW(child, WM_SETFONT, default_gui_font, TRUE)`.

### 3. Handle commands in `WM_COMMAND`
- Switch on `LOWORD(wParam)`:
  - `ID_START_BTN` + `BN_CLICKED` → toggle `state.running`. If starting: validate interval (clamp 1..=86400), `SetTimer(hWnd, TIMER_ID, secs*1000, None)`, button caption → "Stop". If stopping: `KillTimer`, caption → "Start".
  - `ID_MODE_COMBO` + `CBN_SELCHANGE` → update `state.jiggler.set_mode(...)`.
  - `ID_INTERVAL_EDIT` + `EN_CHANGE` → parse text; if running, restart timer with new interval (debounce: only on focus loss `EN_KILLFOCUS` to avoid restart per keystroke).
  - `ID_AUTOSTART_CHK` → call into phase-06 autostart toggle (stub for now, real wiring phase 06).
  - `ID_STARTMIN_CHK` → toggles persisted flag only (effect on next launch).

### 4. Sizing helper
Convert dialog units / DPI: query window DPI on `WM_CREATE` via `GetDpiForWindow`, scale all positions by `dpi/96`.

### 5. Close behavior
- `WM_CLOSE`: in this phase just call `DestroyWindow`. Phase 04 changes this to "minimize to tray".

## Todo
- [x] Create `src/window.rs` with `register_class`, `create_window`, `wnd_proc`
- [x] Implement `AppState` extension (running flag, interval, mode)
- [x] Lay out controls with DPI scaling
- [x] Wire Start/Stop button → timer start/stop + caption swap
- [x] Wire Mode combo → `Jiggler::set_mode`
- [x] Wire interval Edit → restart timer on `EN_KILLFOCUS`
- [x] Stub checkbox handlers (logged only)

## Success Criteria
- Window opens correctly at 100/125/150% DPI without blur or layout breakage.
- Start → button shows "Stop", timer fires at configured interval.
- Stop → timer halts, button shows "Start".
- Changing interval while stopped is fine; while running, restart applies new interval after edit defocus.
- Mode change applies immediately on next tick.

## Risks
- DPI quirk: per-monitor DPI changes when dragging across monitors → `WM_DPICHANGED` not handled (acceptable for v1 — single monitor typical).
- Edit `ES_NUMBER` allows pasted non-digits in some Windows versions — additionally validate in handler.

## Next
Phase 04 — system tray.
