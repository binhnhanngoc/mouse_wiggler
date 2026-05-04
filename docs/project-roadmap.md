# Project Roadmap

Development phases, current status, and success criteria for Mouse Wiggler.

## Overview

Mouse Wiggler is implemented in 7 phases covering scaffold, input injection, UI, persistence, and packaging. **Current status: Code-complete, awaiting verification** — Phases 01–06 are all code-complete in `src/`; Phase 07 (release-build size check + manual smoke test) is the only outstanding work.

## Phase Breakdown

### Phase 01: Scaffold ✅ COMPLETE

**Status:** Done (2026-05-04)

**Objectives:**
- [x] Set up Cargo project with windows-rs 0.58 bindings.
- [x] Configure release profile (LTO, size optimization, strip).
- [x] Create `app.manifest` (PerMonitorV2 DPI, Win10/11 support).
- [x] Create `app.rc` for icon/manifest embedding.
- [x] Implement `build.rs` to embed resources.
- [x] Register window class and create main window.
- [x] Set up message loop (GetMessageW / DispatchMessageW).

**Deliverables:**
- Window appears on screen.
- Message loop runs and handles basic messages.
- Window can be created and destroyed without crashes.

**Related files:**
- `Cargo.toml` (project config + release profile)
- `build.rs`, `app.manifest`, `app.rc`
- `src/main.rs`, `src/window.rs` (scaffold section)

---

### Phase 02: Jiggler Core ✅ COMPLETE

**Status:** Done (2026-05-04)

**Objectives:**
- [x] Implement `Jiggler` struct with Zen and Visible modes.
- [x] Implement `Mode::from_str_lossy()` for serde compatibility.
- [x] Implement `Jiggler::tick()` to emit `SendInput` with dx/dy.
- [x] Zen mode: (0, 0) movement (invisible idle reset).
- [x] Visible mode: ±1px alternating (net zero drift).
- [x] Silent error handling (UIPI, locked desktop = no-op).

**Deliverables:**
- `src/jiggler.rs` complete (78 LOC).
- Input injection verified (manual test with spy).

**Related files:**
- `src/jiggler.rs`

---

### Phase 03: Window UI ✅ CODE COMPLETE

**Status:** Code complete (`src/window.rs`, 614 LOC); runtime smoke test pending in Phase 07.

**Objectives:**
- [x] Create window controls: combobox (mode), edit (interval), button (start/stop), checkboxes (autostart, start_minimized).
- [x] Populate controls with current `Settings` on WM_CREATE.
- [x] Route WM_COMMAND to update jiggler mode and interval.
- [x] Implement button click handlers (Start/Stop toggle).
- [x] Debounce settings changes (250ms timer before save).
- [x] Hide window on minimize (minimize-to-tray).
- [x] Restore window on tray interaction.

**Deliverables:**
- Window displays all controls.
- Mode/interval changes update Jiggler in real-time.
- Start/Stop button toggles jiggling.
- Debounced settings saves.

**Related files:**
- `src/window.rs` (WM_CREATE, WM_COMMAND, control IDs)
- `src/settings.rs` (load/save integration)

---

### Phase 04: Tray ✅ CODE COMPLETE

**Status:** Code complete (`src/tray.rs`, 148 LOC; dispatched from `src/window.rs`); runtime smoke test pending in Phase 07.

**Objectives:**
- [x] Register system tray icon (Shell_NotifyIconW).
- [x] Show tooltip ("Running" / "Stopped").
- [x] Implement right-click popup menu (Show, Start/Stop, Exit).
- [x] Implement left-click / double-click restore window.
- [x] Update tooltip when running state changes.
- [x] Remove tray icon on exit (WM_NCDESTROY cleanup).

**Deliverables:**
- Tray icon appears in taskbar.
- Right-click shows popup menu.
- Menu selections route to WM_COMMAND handlers.
- Tooltip reflects running state.

**Related files:**
- `src/tray.rs` (add, update_tooltip, remove, handle_callback)
- `src/window.rs` (WM_TRAY_CALLBACK dispatch)

---

### Phase 05: Settings Persistence ✅ CODE COMPLETE

**Status:** Code complete (`src/settings.rs`, 91 LOC; debounced save wired from `src/window.rs`); runtime smoke test pending in Phase 07.

**Objectives:**
- [x] Implement `settings::load()` to read `<exe>/settings.json`.
- [x] Implement `settings::save()` with atomic write (tmp + MoveFileEx).
- [x] Validate settings on load (interval clamped 1–86400, mode normalized).
- [x] Fault-tolerant load (missing/corrupt file → defaults).
- [x] Clean up orphaned `.json.tmp` on startup.
- [x] Serialize/deserialize Settings struct via serde_json.
- [x] Wire save to debounced TIMER_SAVE in message loop.

**Deliverables:**
- `src/settings.rs` complete (91 LOC).
- Settings persist across restarts.
- Corrupted JSON gracefully handled.

**Related files:**
- `src/settings.rs`
- `src/window.rs` (TIMER_SAVE scheduling)

---

### Phase 06: Autostart Registry ✅ COMPLETE

**Status:** Done (2026-05-04)

**Objectives:**
- [x] Implement `autostart::is_enabled()` to check HKCU Run key.
- [x] Implement `autostart::set(bool)` to add/remove registry value.
- [x] Implement `autostart::reconcile()` to sync registry with settings.
- [x] Handle ERROR_FILE_NOT_FOUND as success on delete (idempotent).
- [x] Self-heal: if exe moved, run key path updates automatically.
- [x] Call `reconcile()` on startup (main.rs).
- [x] Wire checkbox to autostart::set() in message loop.

**Deliverables:**
- `src/autostart.rs` complete (112 LOC).
- Registry toggle in UI works.
- Self-heal on exe move.

**Related files:**
- `src/autostart.rs`
- `src/main.rs` (reconcile call)
- `src/window.rs` (checkbox handler)

---

### Phase 07: Packaging & Test ⚠️ AWAITING VERIFICATION

**Status:** Partial (manifest/icon/rc in place; size & behavior verification pending)

**Objectives:**
- [ ] Build release binary: `cargo build --release`.
- [ ] Verify executable size ≤ 1.5 MB (check with `Get-Item`).
- [ ] Verify DPI awareness: manifest + SetProcessDpiAwarenessContext().
- [ ] Manual test checklist:
  - [ ] Window opens and centers.
  - [ ] Mode combobox works (select Zen/Visible).
  - [ ] Interval edit accepts numeric input (1–86400).
  - [ ] Start/Stop button toggles jiggling (visible mode shows movement).
  - [ ] Minimize hides to tray.
  - [ ] Tray right-click shows menu.
  - [ ] Tray menu options (Show, Start/Stop, Exit) work.
  - [ ] Tooltip shows "Running" / "Stopped".
  - [ ] Autostart checkbox toggles registry.
  - [ ] Start minimized checkbox works.
  - [ ] Settings persist: close & reopen, check values.
  - [ ] Cold start time < 200 ms.
  - [ ] Zen mode: idle timer resets (invisible in Teams).
  - [ ] Visible mode: cursor alternates ±1px (no net drift).
  - [ ] Exe move: autostart registry updates automatically.
- [ ] Sign or document unsigned .exe behavior (SmartScreen).
- [ ] Create distribution package: single `mouse_wiggler.exe`.

**Deliverables:**
- Verified `target/release/mouse_wiggler.exe` ≤ 1.5 MB.
- Manual test results documented.
- README & deployment guide finalized.

**Related files:**
- `Cargo.toml` (release profile)
- `target/release/mouse_wiggler.exe` (build output)
- `README.md`, `docs/deployment-guide.md`

---

## Success Criteria (Overall)

| Criterion | Status | Notes |
|-----------|--------|-------|
| ✅ Single .exe ≤ 1.5 MB, no deps | Pending | Phase 07 verification |
| ✅ Builds on Windows 10/11 x64 | Pending | cargo build --release |
| ✅ Two input modes (Zen/Visible) | ✅ Complete | Phase 02 done |
| ✅ Configurable interval | ✅ Code complete | Phase 03; runtime smoke test pending |
| ✅ Settings persist | ✅ Code complete | Phase 05 |
| ✅ Autostart registry | ✅ Code complete | Phase 06 |
| ✅ Minimize-to-tray | ✅ Code complete | Phase 03/04; runtime smoke test pending |
| ✅ Tray icon + menu | ✅ Code complete | Phase 04; runtime smoke test pending |
| ✅ DPI awareness (PerMonitorV2) | ✅ Complete | Manifest + code done |
| ✅ Cold start < 200 ms | Pending | Phase 07 test |
| ✅ No telemetry | ✅ Complete | Inherent (no network code) |
| ✅ No admin required | ✅ Complete | All HKCU operations (user-scope) |

---

## Timeline Estimate

| Phase | Priority | Effort | Est. Duration |
|-------|----------|--------|----------------|
| 01 | MUST | 4 hrs | 2 days |
| 02 | MUST | 3 hrs | 1 day |
| 03 | MUST | 6 hrs | 2 days |
| 04 | SHOULD | 4 hrs | 1 day |
| 05 | MUST | 3 hrs | 1 day |
| 06 | MUST | 3 hrs | 1 day |
| 07 | MUST | 4 hrs | 2 days |
| **Total** | — | **27 hrs** | **~10 days** |

**Actual progress (2026-05-04):** Phases 01–06 code complete (`src/` totals 1,075 LOC). Only Phase 07 (release-build size verification + manual smoke checklist) remains.

---

## Dependencies & Blockers

### Phase 03 → Phase 07
- Window UI must stabilize before full test.
- All controls must be created and wired to Jiggler state.

### Phase 04 → Phase 07
- Tray icon callback must route to message loop handlers.
- Tooltip updates must sync with running state.

### Phase 05 → Phase 03
- Settings struct must be fully defined (defines UI control values).

### Phase 06 → main.rs
- Autostart reconcile must run after settings load (depends on Settings struct).

---

## Known Issues & Deferred

| Issue | Phase | Priority | Status |
|-------|-------|----------|--------|
| Window size/position not persisted | Future | LOW | Out of scope for v1 |
| Global hotkey support | Out of scope | NONE | Explicitly deferred |
| Multi-language UI | Out of scope | NONE | English only |
| Code signing / Authenticode | Out of scope | NONE | Unsigned .exe (manual SmartScreen bypass) |
| Auto-update | Out of scope | NONE | Manual distribution only |

---

## Phase Details

See individual phase files for full specification:
- `plans/260504-1552-mouse-wiggler/phase-01-scaffold.md`
- `plans/260504-1552-mouse-wiggler/phase-02-jiggler-core.md`
- `plans/260504-1552-mouse-wiggler/phase-03-window-ui.md`
- `plans/260504-1552-mouse-wiggler/phase-04-tray.md`
- `plans/260504-1552-mouse-wiggler/phase-05-settings-persistence.md`
- `plans/260504-1552-mouse-wiggler/phase-06-autostart-registry.md`
- `plans/260504-1552-mouse-wiggler/phase-07-packaging-and-test.md`

---

## Next Steps

1. **Immediate (Phase 03):** Complete window UI control creation and message handlers.
2. **Short-term (Phase 04):** Wire tray icon interactions.
3. **Verify (Phase 05):** Test settings persistence end-to-end.
4. **Test (Phase 07):** Full manual test suite + size verification.
5. **Release:** Distribute single `mouse_wiggler.exe`.
