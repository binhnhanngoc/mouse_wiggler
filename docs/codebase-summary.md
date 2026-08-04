# Codebase Summary

Generated from repomix analysis of Mouse Wiggler source tree.

## Module Overview

| Module | File | LOC | Purpose | Public API |
|--------|------|-----|---------|-----------|
| **main** | `src/main.rs` | 64 | Entry point; DPI init, settings load, autostart reconcile | `fn main()` |
| **jiggler** | `src/jiggler.rs` | 87 | Input injection core (Zen/Visible modes) | `enum Mode`, `struct Jiggler`, `fn tick()` |
| **autostart** | `src/autostart.rs` | 112 | HKCU Run registry helpers | `fn is_enabled()`, `fn set()`, `fn reconcile()` |
| **settings** | `src/settings.rs` | 97 | JSON load/save, validation | `struct Settings`, `fn load()`, `fn save()` |
| **window** | `src/window.rs` | 811 | Main window, controls, message loop | `fn run()`, `struct AppState` |
| **tray** | `src/tray.rs` | 143 | Tray icon + popup menu | `fn add()`, `fn update_tooltip()`, `fn handle_callback()` |
| **dpi** | `src/dpi.rs` | 73 | DPI scaling + per-DPI UI font | `fn dpi_for()`, `fn scale()`, `fn ui_font()` |

**Total:** 1,387 LOC (excluding build script, manifests, resources).

## Dependency Graph

```
main()
├── settings::load()
├── autostart::reconcile(&Settings)
│   └── autostart::is_enabled() / set()
│       └── [windows::Registry APIs]
└── window::run(Settings)
    ├── jiggler::Jiggler
    │   └── [windows::SendInput]
    ├── tray::add() / update_tooltip()
    │   └── [windows::Shell, UI APIs]
    └── [windows::timer, window, message-loop APIs]
```

## Module Details

### main.rs (32 LOC)
**Entry point for the application.**

- Sets `DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2` via Win32 (best-effort; manifest is primary).
- Calls `settings::load()` to read persisted config (defaults if missing/corrupt).
- Calls `autostart::reconcile(&cfg)` to sync registry with saved preference.
- Hands off to `window::run(cfg)` which owns GUI lifecycle.

**Win32 APIs:** `SetProcessDpiAwarenessContext`.

### jiggler.rs (78 LOC)
**Pure input-injection engine; no windowing dependencies.**

**Public Types:**
- `enum Mode { Zen, Visible }` — Input mode selector.
  - `from_str_lossy(s: &str)` — Serde-friendly parsing.
  - `as_str()` — Serialization (lowercase "zen", "visible").
- `struct Jiggler { mode: Mode, direction: i32 }` — State holder.

**Public Methods:**
- `new(mode: Mode)` — Construct with initial mode.
- `set_mode(&mut self, m: Mode)` — Change mode at runtime.
- `tick(&mut self)` — Emit one synthetic mouse-move via `SendInput`.
  - **Zen:** dx=0, dy=0 (invisible).
  - **Visible:** dx = ±1 (alternates each tick so net drift = 0); dy=0.
  - Errors swallowed silently (UIPI, locked desktop = no-op).

**Win32 APIs:** `SendInput`, `INPUT_MOUSE`, `MOUSEEVENTF_MOVE`.

### autostart.rs (112 LOC)
**Registry helper for HKCU\Software\Microsoft\Windows\CurrentVersion\Run.**

**Registry Key:** `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`  
**Value Name:** `"MouseWiggler"` (REG_SZ, quoted exe path).

**Public Functions:**
- `is_enabled() -> bool` — Check if registry value exists.
- `set(enable: bool) -> Result<()>` — Add/remove from Run key.
  - On disable, treats `ERROR_FILE_NOT_FOUND` as success (idempotent).
- `reconcile(cfg: &Settings) -> Result<()>` — At startup, ensure registry matches persisted flag.
  - No-op if already synced; self-heals when exe moved.
- `current_exe_command() -> String` — Get quoted exe path (handles spaces).

**Helper:**
- `to_wide_z(s: &str) -> Vec<u16>` — UTF-16 null-terminated encoding.

**Win32 APIs:** `RegOpenKeyExW`, `RegQueryValueExW`, `RegSetValueExW`, `RegDeleteValueW`, `RegCloseKey`, `GetModuleFileNameW`.

### settings.rs (91 LOC)
**Persisted user config (JSON file next to .exe).**

**Location:** `<exe-dir>/settings.json`  
**Format:** JSON, pretty-printed.

**Public Types:**
- `struct Settings { mode: String, interval_sec: u32, autostart: bool, start_minimized: bool }`
  - Derives `Serialize`, `Deserialize` (serde).
  - Implements `Default` (Zen, 60 sec, no autostart, no minimize).

**Public Functions:**
- `load() -> Settings` — Read/deserialize from disk.
  - Fault-tolerant: missing/corrupt file → default.
  - Orphaned `.json.tmp` from prior crash is cleaned up.
  - Validates loaded values (interval clamped 1–86400, mode forced to "zen" or "visible").
- `save(cfg: &Settings)` — Serialize to disk atomically.
  - Writes to `.json.tmp` first, then renames via `MoveFileExW(REPLACE_EXISTING)`.
  - Silent failure if I/O fails (non-fatal).
- `settings_path() -> PathBuf` — Resolve exe-relative path.

**Internal:**
- `Settings::validated(self) -> Self` — Clamp/normalize loaded values.

**Win32 APIs:** `GetModuleFileNameW`, `MoveFileExW`.

### window.rs (614 LOC)
**Main GUI window, control management, message loop.**

**Core Structures:**
- `struct AppState { hwnd, combo, edit_interval, btn_start, chk_autostart, chk_startmin, jiggler, settings, running, suppress_save }`
  - Heap-allocated, owned via `GWLP_USERDATA`; dropped in `WM_NCDESTROY`.
- Window class registration, `CreateWindowExW`, message loop dispatch.

**Message Handlers:**
- `WM_CREATE` — Register class, create child controls, populate combo/edit boxes, add tray icon.
- `WM_COMMAND` — Button clicks (Start/Stop), combobox selection, checkbox toggles.
- `WM_TIMER` — Two timers:
  - `TIMER_JIGGLE` (configured interval) — Calls `jiggler.tick()`.
  - `TIMER_SAVE` (250 ms debounce) — Calls `settings::save()`.
- `WM_SYSCOMMAND(SC_MINIMIZE)` — Hide to tray instead of taskbar.
- `WM_CLOSE` → minimize (not exit).
- `WM_NCDESTROY` — Clean up tray, destroy AppState.
- Tray callback routed via `tray::handle_callback()`.

**Control IDs:**
- 1001: Mode combobox
- 1002: Interval edit box
- 1003: Start button
- 1004: Autostart checkbox
- 1005: Start minimized checkbox

**Public Constants:**
- `TIMER_JIGGLE`, `TIMER_SAVE` (timer IDs)
- `ID_TRAY_SHOW`, `ID_TRAY_TOGGLE`, `ID_TRAY_EXIT` (tray menu IDs)
- `WM_TRAY_CALLBACK` (custom message for tray)
- `IDI_APP` (resource ID for embedded icon)

**Public Functions:**
- `run(cfg: Settings) -> Result<()>` — Create window, run message loop until exit.

**Win32 APIs:** `CreateWindowExW`, `RegisterClassExW`, `SetTimer`, `KillTimer`, `SendMessageW`, `GetWindowTextW`, `UpdateWindow`, `ShowWindow`, `DefWindowProcW`, `PostQuitMessage`, `GetMessageW`, `DispatchMessageW`, `TranslateMessage`, `SendInput` (via jiggler).

### tray.rs (148 LOC)
**System tray icon and popup context menu.**

**Tray Icon Behavior:**
- Single icon (ID `TRAY_ICON_ID`), custom callback message `WM_TRAY_CALLBACK`.
- Tooltip updates reflect running state ("Running" vs "Stopped").
- Left-click / double-click → restore window.
- Right-click → popup menu.

**Popup Menu Options:**
- Show (restore window)
- Start / Stop (toggle based on state)
- Exit (post `WM_CLOSE`)

**Public Functions:**
- `add(hwnd: HWND, running: bool)` — Register tray icon on window creation.
- `update_tooltip(hwnd: HWND, running: bool)` — Change tooltip text when state changes.
- `remove(hwnd: HWND)` — Unregister icon on exit.
- `handle_callback(state: *mut AppState, lparam: LPARAM)` — Route tray click/double-click/right-click to appropriate handlers.

**Internal Helpers:**
- `tooltip(running: bool) -> [u16; 128]` — Generate UTF-16 tooltip text.
- `load_app_icon() -> HICON` — Load embedded icon or fallback to stock.
- `restore_window(hwnd: HWND)` — Show window and bring to foreground.
- `show_menu(hwnd: HWND, running: bool)` — Create and track popup menu.

**Win32 APIs:** `Shell_NotifyIconW`, `CreatePopupMenu`, `AppendMenuW`, `TrackPopupMenu`, `GetCursorPos`, `SetForegroundWindow`, `LoadIconW`, `DestroyMenu`, `PostMessageW`.

## Build Configuration

**File:** `Cargo.toml`

**Release Profile:**
```toml
[profile.release]
opt-level = "z"        # Size optimization
lto = true             # Link-time optimization
codegen-units = 1      # Monolithic codegen
strip = true           # Remove debug symbols
panic = "abort"        # No unwinding
```

**Build Script:** `build.rs` — Single line: `embed_resource::compile("app.rc", ...)`.

## Resource Files

**app.manifest** (42 LOC)
- Declares PerMonitorV2 DPI awareness.
- Requests Common-Controls v6 dependency.
- Declares asInvoker privilege (no elevation).
- Lists Win10/11 `supportedOS` GUIDs.

**app.rc** (8 LOC)
- Embeds manifest as RT_MANIFEST (24).
- Embeds icon as IDI_APP (1).

**assets/icon.ico**
- Compiled from `assets/icon.svg`; embedded in .exe.

## Data Flow

```
startup
  ├─> settings::load() → Settings
  ├─> autostart::reconcile() → sync registry
  └─> window::run(Settings)
        ├─> AppState { jiggler, settings, hwnd, controls }
        ├─> message loop
        │    ├─ WM_TIMER (jiggle) → jiggler.tick() → SendInput
        │    ├─ WM_TIMER (save) → settings::save()
        │    ├─ WM_COMMAND (button/checkbox/combobox) → update AppState
        │    └─ WM_TRAY_CALLBACK → tray::handle_callback()
        └─> on exit: settings::save(), tray::remove(), AppState dropped
```

## Win32 Feature Flags

**windows 0.58 features enabled:**
- `Win32_Foundation`
- `Win32_System_LibraryLoader`
- `Win32_System_Registry`
- `Win32_System_SystemServices`
- `Win32_UI_WindowsAndMessaging`
- `Win32_UI_Controls`
- `Win32_UI_Input_KeyboardAndMouse`
- `Win32_UI_Shell`
- `Win32_UI_HiDpi`
- `Win32_Graphics_Gdi`
- `Win32_Storage_FileSystem`

## Build Artifacts

**Output:** `target/release/mouse_wiggler.exe`

**Size Target:** ≤ 1.5 MB (after stripping, LTO).

## Error Handling

**Philosophy:** Non-actionable errors are silently swallowed.
- Registry queries: missing key/value → default behavior.
- `SendInput` failures (UIPI, locked desktop): silent no-op.
- Settings I/O failures: in-memory state preserved; save retried on next debounce.
- Win32 API `Err()` → `.ok()` (drop error, continue).

**Actionable Errors:** None explicitly propagated; all `Result<()>` returns logged implicitly by caller or swallowed at entry point.

## Testing

No unit tests in codebase. Manual testing checklist in `plans/260504-1552-mouse-wiggler/phase-07-packaging-and-test.md`.

## Concurrency

**Single-threaded message loop only.** No background threads, no async/await. All operations (UI, input injection, registry, I/O) serialized through Win32 message queue.
