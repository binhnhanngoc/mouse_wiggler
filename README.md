# Mouse Wiggler

Tiny portable Windows mouse mover that prevents idle/lock states.

## What It Does

Mouse Wiggler injects synthetic mouse-move events at configurable intervals to reset idle timers, keeping your system from locking or marking you as "Away" in Teams/Slack. Two modes:
- **Zen**: Move dx=0, dy=0 (invisible to user; resets idle timer).
- **Visible**: Alternate ±1px cursor movement (cursor stays in place; net drift = 0).

## Why It Exists

Personal productivity tool — prevents unwanted lock/away states during legitimate activity that doesn't generate mouse/keyboard input (e.g., reading, watching, thinking). No telemetry, no admin required, single-file distribution.

## Current Status

**CODE COMPLETE — VERIFICATION PENDING.** All 6 implementation phases are written (`src/` = 1,075 LOC across 6 modules). Phase 07 (release-build size check + manual smoke test) is the only outstanding work.

| Phase | Status | Details |
|-------|--------|---------|
| 01 | ✅ | Scaffold: class registration, window creation, message loop |
| 02 | ✅ | Jiggler core: input injection (Zen/Visible modes) |
| 03 | ✅ | Window UI: controls, WM_COMMAND routing, debounced save |
| 04 | ✅ | Tray: icon, popup menu, callback dispatch |
| 05 | ✅ | Settings: JSON load/save, validation, defaults |
| 06 | ✅ | Autostart: HKCU Run registry, self-heal on move |
| 07 | ⚠️ | Packaging: manifest/icon embedded; size + smoke test pending |

See [docs/project-roadmap.md](docs/project-roadmap.md) for detailed phase breakdown.

## Quick Start

### Build
```powershell
cargo build --release
```

Output: `target/release/mouse_wiggler.exe` (~1–2 MB; final size pending optimization).

### Run
```powershell
.\target\release\mouse_wiggler.exe
```

On first launch, `settings.json` is created next to the .exe with defaults.

### Settings

Modify `settings.json` (next to `.exe`):
```json
{
  "mode": "zen",
  "interval_sec": 60,
  "autostart": false,
  "start_minimized": false
}
```

| Field | Type | Range | Default |
|-------|------|-------|---------|
| `mode` | string | "zen" \| "visible" | "zen" |
| `interval_sec` | u32 | 1–86400 | 60 |
| `autostart` | bool | true \| false | false |
| `start_minimized` | bool | true \| false | false |

Changes saved automatically when UI controls change (debounced 250 ms).

## File Layout

```
mouse_wiggler/
├── README.md                    # This file
├── Cargo.toml                   # Package metadata + release optimizations
├── Cargo.lock
├── build.rs                     # Embeds manifest + icon
├── app.manifest                 # DPI awareness, Win10/11 support
├── app.rc                       # Resource script (icon + manifest)
│
├── src/
│   ├── main.rs                  # Entry: DPI awareness, load settings, reconcile autostart
│   ├── jiggler.rs               # Input injection (Zen/Visible modes)
│   ├── autostart.rs             # Registry HKCU Run helper
│   ├── settings.rs              # Load/save settings.json
│   ├── window.rs                # Main window: controls, message loop, timer
│   └── tray.rs                  # Tray icon + popup menu
│
├── assets/
│   ├── icon.svg                 # Source vector icon
│   └── icon.ico                 # Compiled Windows icon (embedded in .exe)
│
├── docs/                        # Documentation
│   ├── project-overview-pdr.md
│   ├── code-standards.md
│   ├── codebase-summary.md
│   ├── system-architecture.md
│   ├── project-roadmap.md
│   ├── deployment-guide.md
│   └── design-guidelines.md
│
└── target/                      # Build artifacts (gitignored)
```

## Dependencies

All vendored in `windows` 0.58 crate (Win32 bindings):
- Input: `SendInput`, `INPUT_MOUSE`, `MOUSEEVENTF_MOVE`
- Window: `CreateWindowExW`, `SetTimer`, `PostMessageW`, `SendMessageW`
- Tray: `Shell_NotifyIconW`, `TrackPopupMenu`
- Registry: `RegOpenKeyExW`, `RegSetValueExW`, `RegDeleteValueW`
- Settings: `serde`, `serde_json`
- DPI: `SetProcessDpiAwarenessContext` (PerMonitorV2)

## Compile & Size Goals

Release profile (`Cargo.toml`):
- `opt-level = "z"` (size optimization)
- `lto = true` (link-time optimization)
- `codegen-units = 1` (monolithic codegen)
- `strip = true` (remove debug symbols)
- `panic = "abort"` (no unwinding bloat)

**Target:** ≤ 1.5 MB; actual size TBD after full build + link.

## Platform

- **OS:** Windows 10, Windows 11 (x64 only; ARM untested).
- **Admin:** Not required (user-scope registry operations only).
- **Runtime deps:** None (uses Win32 APIs directly).

## Out of Scope

- Global hotkeys (user-initiated commands via UI only)
- Scheduling (always-on once running)
- MSIX packaging or Windows Store
- Code signing (unsigned .exe; SmartScreen bypass manual)
- Telemetry or crash reporting
- Multi-language UI
- Auto-update mechanism

## License

MIT

## Development

See [docs/code-standards.md](docs/code-standards.md) for Rust idioms, unsafe boundaries, and commit conventions.

For architecture details, see [docs/system-architecture.md](docs/system-architecture.md).
