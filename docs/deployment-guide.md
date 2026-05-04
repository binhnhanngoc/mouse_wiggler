# Deployment Guide

Build, test, and deploy Mouse Wiggler.

## Build from Source

### Prerequisites

- **OS:** Windows 10/11 x64 with current updates.
- **Toolchain:** Rust 1.70+ (MSVC or GNU).
  ```powershell
  # Check Rust version
  rustc --version
  ```
- **Cargo:** Included with Rust.

### Build Steps

1. **Clone or navigate to project root:**
   ```powershell
   cd D:\Projects\mouse_wiggler
   ```

2. **Build release binary:**
   ```powershell
   cargo build --release
   ```
   - First build: ~30–60 sec (downloads/compiles windows-rs).
   - Subsequent builds: ~5–10 sec (incremental).

3. **Verify executable:**
   ```powershell
   # Check file exists
   Get-Item .\target\release\mouse_wiggler.exe

   # Check size
   (Get-Item .\target\release\mouse_wiggler.exe).Length / 1MB
   ```
   - **Target:** ≤ 1.5 MB.
   - **Typical:** 1.1–1.3 MB (varies by Rust version, optimization level).

4. **Verify no runtime dependencies:**
   ```powershell
   # List DLLs (should show only system Win32 DLLs)
   & (Get-Command dumpbin).Path /imports .\target\release\mouse_wiggler.exe
   ```
   - Expected: `kernel32.dll`, `user32.dll`, `shell32.dll`, `advapi32.dll` (all built-in).
   - NOT expected: `.NET redist`, `Visual C++ redist`, third-party DLLs.

### Troubleshooting Build

| Issue | Cause | Fix |
|-------|-------|-----|
| `error: linker 'cc' not found` | MSVC/GNU toolchain missing | Install via `rustup` |
| `error[E0512]: cannot transmute` | Unsafe code mismatch | Verify windows-rs version (0.58 required) |
| `warning: Windows SDK not found` | Optional; SDK-based optimizations skipped | Proceed; no impact on functionality |
| `target/release/` missing | Build failed silently | Run `cargo build --release` again; check `cargo.lock` |

## Manual Testing Checklist

**Run after each build or code change.**

### Startup & Initialization

- [ ] Double-click `mouse_wiggler.exe`.
  - [ ] Window opens within 200 ms.
  - [ ] Window centers on screen.
  - [ ] Title bar shows "Mouse Wiggler".
  - [ ] All 5 controls visible (combobox, edit, button, 2 checkboxes).

- [ ] Check `settings.json` created next to .exe.
  - [ ] Contains default values: `"mode": "zen"`, `"interval_sec": 60`, etc.

### Mode & Interval Configuration

- [ ] Click Mode combobox, select "Visible".
  - [ ] Combobox shows "Visible".
  - [ ] Mode persists on close/reopen.

- [ ] Click Mode combobox, select "Zen".
  - [ ] Combobox shows "Zen".

- [ ] Click Interval edit box, clear and type `120`.
  - [ ] Edit box shows "120".
  - [ ] Click outside (lose focus) or press Enter.
  - [ ] Settings saved within 250 ms (check `settings.json` modified time).

- [ ] Click Interval edit, type invalid value (e.g., `999999`).
  - [ ] Value clamped to 86400 on save.
  - [ ] Check `settings.json`: `"interval_sec": 86400`.

### Start/Stop Button

- [ ] Click "Start" button.
  - [ ] Button text changes to "Stop".
  - [ ] If Mode is "Visible", cursor briefly moves ±1px (verify with slow-motion or spy).
  - [ ] Cursor returns to original position (net drift = 0).

- [ ] Move cursor to corner, click "Start".
  - [ ] Wait 3 seconds (if interval_sec = 60, use shorter for test; edit to `3`).
  - [ ] Cursor should not drift over time.
  - [ ] If Mode is "Zen", cursor is invisible (no visible movement).

- [ ] Click "Stop" button.
  - [ ] Button text changes to "Start".
  - [ ] Cursor stops moving.

### Autostart Checkbox

- [ ] Check "Enable Autostart" checkbox.
  - [ ] Checkbox shows checked state.
  - [ ] Registry key created: `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\MouseWiggler`.
  - [ ] Value points to current .exe path (quoted).

- [ ] Uncheck "Enable Autostart" checkbox.
  - [ ] Registry key removed.
  - [ ] Verify via Registry Editor (`regedit`).

- [ ] Move .exe to a different folder (e.g., `C:\Users\<username>\Desktop\`).
  - [ ] Run .exe from new location.
  - [ ] On startup, if autostart was enabled, registry key is auto-updated to new path.
  - [ ] Verify in Registry Editor.

### Start Minimized Checkbox

- [ ] Check "Start Minimized" checkbox.
  - [ ] Close and reopen .exe.
  - [ ] Window does NOT appear; only tray icon visible.
  - [ ] Click tray icon to restore.

- [ ] Uncheck "Start Minimized" checkbox.
  - [ ] Close and reopen .exe.
  - [ ] Window appears normally (not hidden).

### Minimize-to-Tray

- [ ] Click minimize button (−) on window.
  - [ ] Window hides.
  - [ ] Tray icon remains visible.
  - [ ] Taskbar entry disappears.

- [ ] Right-click tray icon.
  - [ ] Popup menu appears: "Show", "Stop" (or "Start"), "Exit".

- [ ] Click "Show" in menu.
  - [ ] Window restores and comes to foreground.

### Tray Tooltip

- [ ] Hover over tray icon.
  - [ ] Tooltip shows "Mouse Wiggler — Stopped" (if jiggling off) or "Mouse Wiggler — Running" (if on).
  - [ ] Tooltip updates when state changes.

### Tray Menu Commands

- [ ] Click "Start" in tray menu.
  - [ ] Running state toggles.
  - [ ] Button in window updates.
  - [ ] Tooltip updates.

- [ ] Click "Stop" in tray menu.
  - [ ] Running state toggles off.

- [ ] Click "Exit" in tray menu.
  - [ ] Application closes cleanly.
  - [ ] Tray icon disappears.
  - [ ] `settings.json` is saved one final time.

### Tray Restore

- [ ] Left-click tray icon.
  - [ ] Window restores (same as "Show" menu).

- [ ] Double-click tray icon.
  - [ ] Window restores.

### Settings Persistence

- [ ] Configure: Mode = "Visible", Interval = 90, Autostart = ON, Start Minimized = OFF.
- [ ] Close application.
- [ ] Reopen application.
  - [ ] All settings restored (combobox, edit, checkboxes show saved values).

- [ ] Delete `settings.json`.
- [ ] Reopen application.
  - [ ] Defaults applied: Mode = "zen", Interval = 60, Autostart = OFF, Start Minimized = OFF.

### Zen Mode (Invisible Idle Reset)

**Test in Microsoft Teams, Slack, or similar:**

- [ ] Open Teams, set status to "Available".
- [ ] Idle timeout: 5 minutes.
- [ ] Configure Mouse Wiggler: Mode = "Zen", Interval = 60 sec.
- [ ] Click "Start".
- [ ] Lock hands (don't touch keyboard/mouse), watch Teams status.
  - [ ] After ~2 minutes (without Wiggler): Status changes to "Away".
  - [ ] With Wiggler running: Status stays "Available".
  - [ ] **Cursor doesn't move visually; idle timer resets invisibly.**

### Visible Mode (±1px Alternating)

- [ ] Configure: Mode = "Visible", Interval = 3 sec (for testing).
- [ ] Click "Start".
- [ ] Watch cursor position.
  - [ ] Every 3 seconds, cursor moves slightly right/left (±1px).
  - [ ] Cursor returns to original position on next tick.
  - [ ] Net displacement over time = 0 (no drift).

### Keyboard Navigation

- [ ] Tab through controls (combobox → edit → button → checkbox → checkbox).
  - [ ] Focus moves in expected order.

- [ ] Press Space on button when focused.
  - [ ] Button click is triggered (Start/Stop toggles).

- [ ] Press Enter in edit box.
  - [ ] Save is triggered (settings debounce starts).

### Window Behavior

- [ ] Press Alt+F4 (close).
  - [ ] Application exits cleanly (not minimize).
  - [ ] Settings saved.

- [ ] Click X button (close).
  - [ ] Application exits cleanly.
  - [ ] Settings saved.

- [ ] Drag window to different monitor (if multi-monitor).
  - [ ] Window redraws correctly.
  - [ ] DPI scaling respects monitor (PerMonitorV2).

### Performance

- [ ] Measure cold start time:
  ```powershell
  $start = [DateTime]::UtcNow
  & .\target\release\mouse_wiggler.exe
  # Wait until window appears and is responsive
  $elapsed = ([DateTime]::UtcNow - $start).TotalMilliseconds
  Write-Host "Cold start: $elapsed ms"
  ```
  - **Target:** < 200 ms.

- [ ] Monitor CPU usage while running:
  - [ ] With jiggling stopped: < 1%.
  - [ ] With jiggling running (60 sec interval): < 1% (brief spike on tick, negligible).

- [ ] Monitor memory usage (Task Manager):
  - [ ] Typical: 2–5 MB.

### Edge Cases

- [ ] Run two instances simultaneously.
  - [ ] Both share same `settings.json`.
  - [ ] Changes in one instance may not immediately reflect in other (race condition, but benign).
  - [ ] Application handles gracefully (no crash).

- [ ] Kill process (Task Manager → End Task).
  - [ ] No orphaned processes.
  - [ ] Tray icon cleans up on next startup (or Windows auto-removes).

- [ ] Change `.exe` permissions to read-only.
  - [ ] Settings changes fail silently (in-memory state preserved).
  - [ ] Restore write permission to save.

- [ ] Run with restricted account (non-admin).
  - [ ] Should work (all operations are user-scope).

## Distribution

### Single-File Distribution

1. **Build release:**
   ```powershell
   cargo build --release
   ```

2. **Copy executable:**
   ```powershell
   Copy-Item .\target\release\mouse_wiggler.exe -Destination C:\Users\<username>\Desktop\
   ```

3. **Ship single file:**
   - No installer needed.
   - No MSI, no MSIX.
   - Just `mouse_wiggler.exe` on disk.

4. **User runs:**
   - Double-click `mouse_wiggler.exe`.
   - `settings.json` auto-created next to .exe on first run.

### SmartScreen Bypass

On first run, Windows SmartScreen may warn: "Unknown Publisher".

**User instructions:**
1. Click "More info".
2. Click "Run anyway".
3. Application runs.

**Note:** No code signing; unsigned .exe is expected.

## Troubleshooting Deployment

| Issue | Cause | Fix |
|-------|-------|-----|
| .exe crashes on startup | Corrupted settings.json | Delete settings.json; rerun |
| Autostart doesn't work | Registry not writable | Run as admin once; check permissions |
| Tray icon missing | Shell_NotifyIconW failed | Restart Explorer; rerun app |
| Interval changes don't save | Disk full or read-only | Check disk space; restore write permission |
| Visible mode too fast/slow | Interval too small/large | Edit settings.json manually (1–86400 range) |

## Uninstall

Mouse Wiggler has no installer, so uninstall is manual:

1. Close the application.
2. Delete `mouse_wiggler.exe`.
3. Delete `settings.json` (optional; won't persist).
4. Optionally, remove registry entry:
   - Open Registry Editor (`regedit`).
   - Navigate to `HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Run`.
   - Delete `MouseWiggler` value.

## Rollback

If a new build causes issues:
1. Keep previous `mouse_wiggler.exe` as backup.
2. Restore from backup.
3. Report issue to developer.

---

## Build Artifacts

- **Source:** `src/`, `Cargo.toml`, `build.rs`, `app.manifest`, `app.rc`
- **Build output:** `target/release/mouse_wiggler.exe` (~1.1–1.3 MB)
- **Settings:** `settings.json` (user-side, next to .exe)
- **Registry:** `HKCU\Software\Microsoft\Windows\CurrentVersion\Run\MouseWiggler` (if autostart enabled)

---

## Next Steps After Deployment

1. **Feedback:** Collect user reports on performance, behavior, or edge cases.
2. **Monitoring:** Watch for common issues (tray icon glitches, settings not saving, etc.).
3. **Improvements:** Address bugs in subsequent builds.
4. **Optimization:** Reduce binary size further if needed (currently good at ~1.2 MB).
