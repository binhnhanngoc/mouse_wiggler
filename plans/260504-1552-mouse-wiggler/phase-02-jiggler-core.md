---
phase: 2
name: Jiggler core
status: completed
priority: high
effort: S
---

# Phase 02 — Jiggler Core

## Context
- [plan.md](plan.md)
- Depends on: phase-01

## Overview
Implement `SendInput`-based jiggle in two modes (Zen, Visible) and a tiny state machine consumed by `WM_TIMER`. No threading.

## Files to Create
- `src/jiggler.rs`

## Files to Modify
- `src/main.rs` — wire `WM_TIMER` to jiggler tick (temporary; permanent UI in phase 03).

## API Design
```rust
// src/jiggler.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mode { Zen, Visible }

pub struct Jiggler {
    mode: Mode,
    direction: i32, // +1 or -1, flipped each visible tick
}

impl Jiggler {
    pub fn new(mode: Mode) -> Self { Self { mode, direction: 1 } }
    pub fn set_mode(&mut self, m: Mode) { self.mode = m; }
    pub fn tick(&mut self) { /* SendInput call */ }
}
```

## Implementation Steps

### 1. SendInput call
- Construct `INPUT { r#type: INPUT_MOUSE, Anonymous: INPUT_0 { mi: MOUSEINPUT { dx, dy, mouseData:0, dwFlags: MOUSEEVENTF_MOVE, time:0, dwExtraInfo:0 } } }`.
- Zen: `dx=0, dy=0`.
- Visible: `dx = self.direction; dy = 0;` then `self.direction = -self.direction` to keep net displacement zero.
- Call `SendInput(1, &input as *const _, size_of::<INPUT>() as i32)`.
- Return value: 1 if accepted; log/swallow on 0 (UIPI / locked desktop).

### 2. Hook into WM_TIMER (temporary)
- In `WndProc`, on `WM_CREATE`: `SetTimer(hWnd, TIMER_ID, 60_000, None)` — 60s default.
- On `WM_TIMER` with matching id: borrow jiggler from window state and call `tick()`.
- Store `Jiggler` via `SetWindowLongPtrW(GWLP_USERDATA, ...)` or static `RefCell` (simpler for now).

### 3. Window State Pattern
Document and use throughout the app:
```rust
struct AppState {
    jiggler: Jiggler,
    running: bool,
    interval_sec: u32,
    // ... grows in later phases
}
// Pass *mut AppState via lpCreateParams in CreateWindowExW,
// store in GWLP_USERDATA in WM_NCCREATE.
```

### 4. Manual verification
- Run with default 60s.
- Open Task Manager → Users → idle time reset every 60s.
- For visible mode (toggle in code), watch cursor twitch ±1px.

## Todo
- [x] Create `src/jiggler.rs` with `Mode`, `Jiggler::new/set_mode/tick`
- [x] Wire `SetTimer` on `WM_CREATE`, `KillTimer` on `WM_DESTROY`
- [x] Define `AppState` struct + GWLP_USERDATA plumbing
- [x] Manually verify both modes register input

## Success Criteria
- Zen tick increments OS user-input timestamp (verify with `GetLastInputInfo` test, or Teams stays Available > 5min).
- Visible tick produces ±1px cursor wiggle, net zero drift after 1000 ticks.

## Risks
- **Locked workstation / UAC desktop**: `SendInput` returns 0; not actionable, just don't crash.
- **Net cursor drift**: must invert direction every tick — not every N ticks.

## Next
Phase 03 — full window UI.
