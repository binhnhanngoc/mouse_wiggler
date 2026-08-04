# System Architecture

High-level design, data flow, and component interactions for Mouse Wiggler.

## Design Philosophy

**Single-threaded message loop.** All application state, UI updates, input injection, and persistence are serialized through the Win32 message queue. No background threads, async/await, or cross-thread synchronization.

**Rationale:** Win32 windowing is inherently single-threaded. Attempting background threads adds complexity (UIPI, marshaling, synchronization) with minimal benefit for a simple timer-based application. Message loop remains responsive because input injection is instantaneous and settings I/O is debounced.

## Process Model

```
┌─────────────────────────────────────────────────────────────────┐
│                   Mouse Wiggler Process (main)                  │
│                                                                 │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Entry (main.rs)                                        │   │
│  │  1. SetProcessDpiAwarenessContext(PER_MONITOR_V2)       │   │
│  │  2. load_settings()                                     │   │
│  │  3. reconcile_autostart()                               │   │
│  │  4. window::run()                                       │   │
│  └─────────────────────────────────────────────────────────┘   │
│                          │                                       │
│                          ↓                                       │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │  Win32 Message Loop (window.rs)                         │   │
│  │                                                         │   │
│  │  while GetMessageW() {                                 │   │
│  │    DispatchMessageW()  ──→ WndProc()                   │   │
│  │  }                                                      │   │
│  │                                                         │   │
│  │  Handles:                                              │   │
│  │  • WM_CREATE      → init controls, add tray icon      │   │
│  │  • WM_TIMER       → jiggle input or save settings      │   │
│  │  • WM_COMMAND     → button/checkbox/combobox clicks   │   │
│  │  • WM_TRAY_CALLBACK → tray icon interactions          │   │
│  │  • WM_SYSCOMMAND  → minimize to tray                  │   │
│  │  • WM_CLOSE       → hide window (exit = tray Exit)    │   │
│  │  • WM_NCDESTROY   → cleanup, drop AppState            │   │
│  └─────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ┌──────────────────┐  ┌──────────────────┐  ┌────────────┐   │
│  │  AppState        │  │  Jiggler         │  │  Registry  │   │
│  │  (on heap)       │  │  (inside State)  │  │  (HKCU)    │   │
│  │                  │  │                  │  │            │   │
│  │ • hwnd           │  │ • mode           │  │ • Run key  │   │
│  │ • controls       │  │ • direction      │  │ • value    │   │
│  │ • jiggler        │  │                  │  │            │   │
│  │ • settings       │  │ pub fn tick()    │  │ reconcile()│   │
│  │ • running flag   │  │  └─ SendInput()  │  │            │   │
│  │ • timers         │  │                  │  │            │   │
│  └──────────────────┘  └──────────────────┘  └────────────┘   │
│                                                                 │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │  Settings Persistence (settings.rs)                      │  │
│  │                                                          │  │
│  │  load()  ─ <exe-dir>/settings.json ──────→ Settings    │  │
│  │  save()  ─ Settings ─────────────────→  json.tmp       │  │
│  │           (atomic MoveFileEx)                json (final) │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

## Component Interactions

### 1. Startup Flow

```
main()
  │
  ├─ SetProcessDpiAwarenessContext()
  │   └─ [Windows honors PerMonitorV2 manifest; this is backup]
  │
  ├─ settings::load()
  │   ├─ Read <exe>/settings.json
  │   ├─ Deserialize (serde_json)
  │   └─ Return Settings or Default
  │
  ├─ autostart::reconcile(&Settings)
  │   ├─ Check HKCU\...\Run\MouseWiggler
  │   ├─ If mismatch with cfg.autostart, call set()
  │   └─ [self-heals if exe was moved]
  │
  └─ window::run(Settings)
      └─ CreateWindowExW() ──→ WndProc(WM_CREATE)
         ├─ Register window class
         ├─ Create child controls (combobox, edit, buttons, checkboxes)
         ├─ Populate with current settings
         ├─ tray::add()  [register system tray icon]
         ├─ SetTimer(TIMER_JIGGLE, interval_ms)
         ├─ SetTimer(TIMER_SAVE, 250ms)
         └─ message loop
```

### 2. User Interaction Flow

#### Start/Stop Button Clicked

```
User clicks "Start" button
  │
  ├─ WM_COMMAND(ID_START_BTN)
  │
  ├─ Toggle AppState.running flag
  ├─ Update button text ("Start" ↔ "Stop")
  ├─ tray::update_tooltip()  [change "Running" ↔ "Stopped"]
  │
  ├─ If running == true:
  │   └─ SetTimer(TIMER_JIGGLE, interval_ms)
  │
  └─ If running == false:
      └─ KillTimer(TIMER_JIGGLE)
```

#### Mode Changed (Combobox)

```
User selects "Visible" from Mode dropdown
  │
  ├─ WM_COMMAND(ID_MODE_COMBO, CBN_SELCHANGE)
  ├─ Get selected index from combobox
  ├─ Update Jiggler::mode
  ├─ Update AppState.settings.mode
  │
  └─ Cancel & reschedule TIMER_SAVE
     └─ [debounce 250ms before writing JSON]
```

#### Interval Changed (Edit Box)

```
User modifies interval edit box
  │
  ├─ WM_COMMAND(ID_INTERVAL_EDIT, EN_KILLFOCUS)
  ├─ Read text from edit box
  ├─ Parse u32, clamp to [1, 86400]
  ├─ Update AppState.settings.interval_sec
  ├─ If running, reschedule TIMER_JIGGLE with new interval
  │
  └─ Cancel & reschedule TIMER_SAVE
     └─ [debounce 250ms before writing JSON]
```

#### Tray Interaction

```
User right-clicks tray icon
  │
  ├─ WM_TRAY_CALLBACK(WM_RBUTTONUP)
  ├─ tray::handle_callback()
  ├─ tray::show_menu()
  │   ├─ CreatePopupMenu()
  │   ├─ AppendMenuW("Show", "Start"/"Stop", "Exit")
  │   ├─ TrackPopupMenu()  [blocks until user selects]
  │   └─ DestroyMenu()
  │
  └─ Menu selection routed back to WM_COMMAND
     ├─ ID_TRAY_SHOW   → ShowWindow(SW_SHOW)
     ├─ ID_TRAY_TOGGLE → toggle running
     └─ ID_TRAY_EXIT   → PostQuitMessage() / WM_CLOSE
```

### 3. Timer-Driven Flow

#### TIMER_JIGGLE (configurable interval, default 60 sec)

```
SetTimer(TIMER_JIGGLE, interval_ms)
  │
  └─ WM_TIMER(TIMER_JIGGLE) fires every interval
     │
     ├─ Check AppState.running flag
     │
     ├─ If true:
     │   ├─ jiggler.tick()
     │   │   ├─ Build INPUT struct
     │   │   ├─ Set dwFlags = MOUSEEVENTF_MOVE
     │   │   ├─ Set dx, dy based on mode
     │   │   │   ├─ Zen: (0, 0)
     │   │   │   └─ Visible: (±1, 0), toggle direction each tick
     │   │   └─ SendInput()  [silent if fails; UIPI/locked desktop OK]
     │   │
     │   └─ [message loop continues, user sees no stall]
     │
     └─ If false: no-op
```

#### TIMER_SAVE (250 ms debounce)

```
WM_COMMAND changes settings (mode, interval, etc.)
  │
  ├─ Cancel existing TIMER_SAVE (KillTimer)
  └─ SetTimer(TIMER_SAVE, 250)
     │
     └─ WM_TIMER(TIMER_SAVE) fires after 250ms with no further changes
        │
        ├─ Call settings::save()
        │   ├─ Serialize AppState.settings to JSON
        │   ├─ Write to <exe>/settings.json.tmp
        │   ├─ MoveFileEx(tmp, final, REPLACE_EXISTING)  [atomic]
        │   └─ [silent if fails; in-memory state preserved]
        │
        └─ KillTimer(TIMER_SAVE)
```

### 4. Shutdown Flow

```
User clicks "Exit" (tray menu or close button)
  │
  ├─ PostQuitMessage()  or  WM_CLOSE
  │
  ├─ Message loop exits (GetMessageW returns false)
  │
  ├─ WM_NCDESTROY is dispatched
  │   ├─ KillTimer(TIMER_JIGGLE)
  │   ├─ KillTimer(TIMER_SAVE)
  │   ├─ tray::remove()  [unregister icon]
  │   ├─ One final settings::save() [in window::run cleanup]
  │   └─ Drop AppState [allocated on heap, Box destructed]
  │
  └─ Process exits
     └─ Windows auto-cleanup: window destroyed, resources released
```

## Data Structures

### AppState (window.rs, stack-allocated then moved to heap)

```rust
pub struct AppState {
    pub hwnd: HWND,                    // Main window handle
    pub combo: HWND,                   // Mode combobox
    pub edit_interval: HWND,           // Interval edit box
    pub btn_start: HWND,               // Start/Stop button
    pub chk_autostart: HWND,           // Autostart checkbox
    pub chk_startmin: HWND,            // Start minimized checkbox
    
    pub jiggler: Jiggler,              // Input injection state
    pub settings: Settings,            // Persisted config
    pub running: bool,                 // Is jiggling active?
    
    suppress_save: Cell<bool>,         // Suppress save during init
}
```

**Lifetime:** Allocated in `WM_CREATE`, stored in window's `GWLP_USERDATA` (opaque pointer). Dropped in `WM_NCDESTROY`. Accessed via pointer in message handlers to avoid borrow-checker issues with re-entrance.

### Settings (settings.rs)

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub mode: String,          // "zen" or "visible"
    pub interval_sec: u32,     // 1–86400 (clamped on load)
    pub autostart: bool,       // Enable HKCU Run entry?
    pub start_minimized: bool, // Hide window on startup?
}
```

**Storage:** Serialized to JSON, stored next to .exe as `settings.json`.

### Jiggler (jiggler.rs)

```rust
pub struct Jiggler {
    pub mode: Mode,    // Zen or Visible
    direction: i32,    // +1 or -1, flipped each Visible tick
}
```

**Contained in:** `AppState.jiggler`. Modified when user changes mode in UI.

## Registry Interaction

### HKCU Run Key

**Path:** `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`

**Value:**
- **Name:** `MouseWiggler`
- **Type:** REG_SZ
- **Data:** Quoted exe path (e.g., `"C:\Users\alice\AppData\Local\MouseWiggler.exe"`)

**Operations:**
- **Check:** `RegQueryValueExW` → presence indicates enabled.
- **Enable:** `RegSetValueExW(REG_SZ)` → add/update value.
- **Disable:** `RegDeleteValueW` → remove value (no-op if missing).
- **Reconcile:** At startup, ensure registry matches `cfg.autostart` (self-heals on exe move).

**Privilege:** User-scope only; no admin required. Write access to HKCU is standard for all users.

## Win32 API Boundaries

### Input Injection (jiggler.rs → SendInput)

```
Jiggler::tick()
  └─ unsafe { SendInput(&[input], sizeof(INPUT)) }
     │
     ├─ Builds synthetic mouse-move event
     ├─ dx, dy based on mode
     ├─ dwFlags = MOUSEEVENTF_MOVE
     ├─ No user-visible cursor movement on Zen
     └─ UIPI/locked desktop: silent failure (OK)
```

### Window Management (window.rs → User32)

```
window::run()
  ├─ RegisterClassExW()  → register window class
  ├─ CreateWindowExW()   → create main window
  │   └─ WndProc dispatches all messages
  ├─ GetMessageW() loop  → fetch messages
  ├─ DispatchMessageW()  → route to WndProc
  ├─ TranslateMessage()  → convert keypresses to chars
  └─ ShowWindow() / SetForegroundWindow()  → tray restore
```

### Registry (autostart.rs → Advapi32)

```
autostart::set()
  ├─ RegOpenKeyExW()      → open HKCU\...\Run
  ├─ RegSetValueExW()     → write REG_SZ (enable)
  ├─ RegDeleteValueW()    → delete value (disable)
  ├─ RegQueryValueExW()   → check if exists (is_enabled)
  └─ RegCloseKey()        → cleanup handle
```

### I/O (settings.rs → FileSystem)

```
settings::load()
  ├─ GetModuleFileNameW()  → get exe path
  ├─ PathBuf::with_file_name("settings.json")
  └─ fs::read_to_string() → load JSON

settings::save()
  ├─ serde_json::to_vec_pretty()  → serialize
  ├─ fs::write() to .tmp file
  └─ MoveFileExW(REPLACE_EXISTING) → atomic rename
```

## Event Flow Diagram

```
           ┌─────────────────────────────┐
           │  System Events              │
           │  (timer, user input, OS)    │
           └─────────────────────────────┘
                      │
                      ↓
           ┌─────────────────────────────┐
           │  Win32 Message Queue        │
           └─────────────────────────────┘
                      │
                      ↓
           ┌─────────────────────────────┐
           │  GetMessageW()              │
           └─────────────────────────────┘
                      │
                      ↓
           ┌─────────────────────────────┐
           │  DispatchMessageW()         │
           └─────────────────────────────┘
                      │
                      ↓
           ┌─────────────────────────────┐
           │  WndProc(hwnd, msg, wp, lp) │
           └─────────────────────────────┘
                      │
      ┌───────────────┼───────────────────┐
      ↓               ↓                   ↓
  ┌────────────┐  ┌────────────┐  ┌───────────────┐
  │ WM_TIMER   │  │ WM_COMMAND │  │ WM_TRAY_CB    │
  │            │  │            │  │               │
  │ • JIGGLE   │  │ • Button   │  │ • Show        │
  │   SendInput│  │ • Checkbox │  │ • Toggle      │
  │ • SAVE     │  │ • Combobox │  │ • Exit        │
  │   settings │  │   Update   │  │               │
  │   to JSON  │  │   settings │  └───────────────┘
  └────────────┘  └────────────┘
```

## Concurrency Model

**Single-threaded, fully serialized.**

- **No background threads:** All work serialized through message loop.
- **No async/await:** Rust futures not used.
- **No locks/mutexes:** Not needed (single thread).
- **Message queue:** Serializes all events (timers, user input, OS notifications).
- **Debouncing:** TIMER_SAVE prevents rapid JSON writes; only fires 250ms after last change.

**Blocking operations:**
- `RegOpenKeyExW` / `RegSetValueExW` → fast, no blocking risk.
- `fs::write` → I/O bound, but debounced so message loop not starved.
- `TrackPopupMenu()` → blocks until user closes menu; OK (UI idle).
- `SendInput()` → instantaneous.

## DPI Awareness

**Manifest:** `app.manifest` declares `<dpiAware>PerMonitorV2</dpiAware>`.

**Fallback:** `SetProcessDpiAwarenessContext(PER_MONITOR_V2)` in `main()`.

**Custom scaling is required.** A PerMonitorV2 process receives *physical* pixels from
Win32 — nothing is auto-scaled. `src/dpi.rs` + the layout table in `src/window.rs` do it:

| Concern | Handling |
|---------|----------|
| Geometry | One design-unit table (96 DPI) → `dpi::scale(v, dpi)` at placement time. `relayout()` is the single consumer, used for both initial layout and rescale. |
| Window size | `AdjustWindowRectExForDpi()` on the scaled client rect (`CLIENT_W` × `CLIENT_H`) → `SetWindowPos`. Never a hardcoded pixel size. |
| Font | `SystemParametersInfoForDpi(SPI_GETNONCLIENTMETRICS, dpi)` → `lfMessageFont` → `CreateFontIndirectW`. Owned by `AppState`, deleted in `Drop`. (`DEFAULT_GUI_FONT` is a fixed-size bitmap font and does not scale.) |
| Monitor change | `WM_DPICHANGED` → `rescale()`: new font, window moved/sized to the suggested rect, children re-placed, repaint. |
| Icons | `LoadImageW` at `SM_CXICON` / `SM_CXSMICON` instead of `LoadIconW`, so the tray and title bar get a native-size image rather than a stretched 32×32. |

**Benefit:** Window and controls are correct at 100/125/150/175/200% and follow the
monitor when the window is dragged between displays of different DPI.

## Resource Embedding

**Manifest & Icon:** Embedded in .exe via `build.rs` → `embed_resource::compile("app.rc", ...)`.

- `app.rc`: References `app.manifest` and `assets/icon.ico`.
- `build.rs`: Invoked by Cargo; embeds resources at link time.
- **Result:** Single standalone `.exe` with icon + manifest baked in.

## Failure Modes & Recovery

| Failure | Handling | User Impact |
|---------|----------|-------------|
| Registry key missing on autostart check | Treat as "disabled" | Autostart OFF if key doesn't exist; benign |
| Settings.json corrupt | Load defaults | Default settings used; user can reconfigure |
| SendInput fails (UIPI) | Silent no-op | Jiggle doesn't happen; message loop unaffected |
| Exe moved, Run key stale | Reconcile updates path | Runs next time; auto-fixes registry |
| Settings save fails | Retry on next debounce | In-memory state preserved; no data loss |

## Design Trade-offs

| Decision | Upside | Downside |
|----------|--------|----------|
| Single-threaded message loop | Simplicity, no locks, no UIPI issues | Must not block message loop (OK here) |
| Silent error swallowing | Clean UI, no crash dialogs | Hard to debug unusual failures |
| Debounced settings save | Fewer I/O operations | 250ms delay before settings written |
| Heap-allocated AppState | Clean separation (state ≠ window) | Pointer handling in message loop |
| No unit tests | Faster development | Limited regression safety |
| Embedded manifest/icon | Single .exe distribution | Rebuild needed if icon changes |
