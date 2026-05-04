---
type: brainstorm
date: 2026-05-04
slug: mouse-wiggler
status: approved
---

# Brainstorm — Mouse Wiggler (Windows, tiny portable)

## Problem Statement
Build a small, portable Windows app that simulates mouse activity at a configurable interval to prevent idle/lock states (e.g. Teams "Away", screensaver). Reference: Mouse Jiggler on Microsoft Store (`9nblggh2sq2n`). Private use, single-user, no Store/MSIX distribution.

## Requirements

### Functional
- Two jiggle modes:
  - **Zen** — `SendInput` w/ `MOUSEEVENTF_MOVE dx=0 dy=0`, no visible cursor motion.
  - **Visible** — alternating ±1px nudge.
- Configurable interval in seconds (default 60).
- Toggle Start / Stop from UI.
- Auto-start with Windows (HKCU Run key).
- Persist settings between runs (JSON beside .exe).
- Minimize to system tray (taskbar hidden, tray icon shown). Restore on tray click.

### Non-Functional
- Single portable `.exe`, no installer, no runtime dependency (Win10/11 x64).
- Final size ≤ 1.5MB stripped.
- DPI-aware UI (no blur on scaled displays).
- Cold start < 200ms.

## Approaches Evaluated

| Approach | Size | Dev cost | Verdict |
|---|---|---|---|
| **Rust + windows-rs (chosen)** | ~600KB–1.2MB | Medium (verbose Win32) | Best size/maintainability balance |
| Rust + native-windows-gui | ~1–2MB | Low–Medium | Slightly bigger, smaller community |
| C# .NET 9 + WinForms (Native AOT) | 8–15MB | Low | Likely overshoots 5MB; AOT+WinForms still rough |
| C++ / Win32 | <200KB | High | Smallest but excessive given 1–5MB budget |

## Final Solution

### Stack
- **Rust** edition 2021
- Crates: `windows` (Win32), `serde`, `serde_json`
- Release profile: `opt-level="z"`, `lto=true`, `strip=true`, `panic="abort"`, `codegen-units=1`

### File Layout
```
mouse_wiggler/
├── Cargo.toml
├── build.rs            # embed icon + manifest via embed-resource or winres
├── app.manifest        # CommCtrl v6, perMonitorV2 DPI
├── assets/icon.ico
└── src/
    ├── main.rs         # entry, message loop
    ├── window.rs       # main window + child controls + WM_* handlers
    ├── tray.rs         # Shell_NotifyIcon + popup menu
    ├── jiggler.rs      # SendInput zen/visible, tick state
    ├── settings.rs     # JSON load/save next to .exe path
    └── autostart.rs    # HKCU\...\Run toggle
```

### UI (≈240×170 px window)
```
Mode:    [ Zen        ▾ ]
Every:   [   60  ] seconds
[   Start   ]
☐ Start with Windows
☐ Start minimized to tray
```

### Mechanics
- **Timer**: `SetTimer(hWnd, ID, ms, NULL)` → `WM_TIMER` calls `SendInput`. Single-threaded; no race.
- **Zen tick**: `INPUT { type:INPUT_MOUSE, mi: { dx:0, dy:0, dwFlags:MOUSEEVENTF_MOVE } }`.
- **Visible tick**: alternate `dx = +1 / -1` per tick (relative move).
- **Tray**: `Shell_NotifyIconW` w/ `NIM_ADD`. Custom msg `WM_APP+1`. Left-click → restore. Right-click → popup (Show / Start-Stop / Exit).
- **Autostart**: `RegSetValueExW` / `RegDeleteValueW` on `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\MouseWiggler` = `GetModuleFileNameW` path quoted.
- **Settings**: `<exe_dir>\settings.json` — `{ mode, interval_sec, autostart, start_minimized }`. Load on boot, debounced save on change.

### Key Win32 API
`SendInput`, `SetTimer`/`KillTimer`, `CreateWindowExW`, `Shell_NotifyIconW`, `TrackPopupMenu`, `RegSetValueExW`/`RegDeleteValueW`, `GetModuleFileNameW`, `SetProcessDpiAwarenessContext`.

## Risks & Mitigations
- **AV heuristics** flagging jigglers — keep code simple, no hidden window / process injection. Document that unsigned .exe may need user allow-list once.
- **Enterprise GPO lock** — `SendInput` won't bypass enforced lock policies. Note as limitation in README.
- **DPI scaling blur** — declare `perMonitorV2` in manifest; query DPI at WM_CREATE.
- **Visible mode cursor drift** on long sessions — invert dx each tick so net motion = 0.
- **Settings file write race** on rapid config changes — debounce ~250ms.

## Success Criteria
- [ ] Single `.exe` ≤ 1.5MB, runs on clean Win10/11 x64 without prereqs.
- [ ] Both modes verifiably keep system "active" (Teams stays Available beyond default idle with 60s interval).
- [ ] Toggle, minimize-to-tray, autostart, settings persistence all functional across reboots.
- [ ] DPI-aware (no blur at 125%/150%/200%).
- [ ] Cold start < 200ms.

## Out of Scope
Global hotkeys, scheduled active-hours, multi-language UI, MSIX/Store packaging, code signing, telemetry.

## Next Steps
1. `/ck:plan` → produce phased implementation plan in `plans/260504-1552-mouse-wiggler/`.
2. Phases (anticipated): scaffold → jiggler core → window UI → tray → settings → autostart → packaging/test.

## Unresolved Questions
- Icon source: do you have an `.ico` already, or should plan include "design/source a 16/32/48px icon"?
- Window close (X) behavior: minimize-to-tray, or actually exit? (Reference Store app minimizes; a tray-app convention is X=hide, tray-Exit=quit.)
- Default `mode` on first run: Zen or Visible?
