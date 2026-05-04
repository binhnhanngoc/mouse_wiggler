---
phase: 7
name: Packaging & manual test
status: completed
priority: medium
effort: S
---

# Phase 07 — Packaging & Manual Test

## Context
- [plan.md](plan.md)
- Depends on: phase-01..06

## Overview
Embed icon, finalize manifest, verify final binary size, run a manual test checklist that exercises every feature on real hardware.

## Files to Create
- `assets/icon.ico` (multi-resolution: 16, 32, 48, 256 px)
- `app.rc` (final, with icon + manifest)

## Files to Modify
- `build.rs` — confirm `embed_resource::compile("app.rc", ...)`.
- `src/window.rs` / `src/tray.rs` — load icon by resource id `IDI_APP`.

## Implementation Steps

### 1. Source / generate icon
Options (pick one):
- **Generate** with ImageMagick: design 256×256 SVG → convert to multi-res .ico:
  ```
  magick -background none icon.svg -define icon:auto-resize=16,32,48,256 assets/icon.ico
  ```
- **Repurpose** a CC0 mouse/cursor icon and convert.
- Aim for simple, distinguishable at 16×16 (tray size).

### 2. `app.rc`
```
1            24      "app.manifest"
IDI_APP      ICON    "assets/icon.ico"
```
And expose `IDI_APP` constant in Rust:
```rust
pub const IDI_APP: PCWSTR = PCWSTR(1 as *const u16); // resource id 1
```

### 3. Load icon
- Window icon: `LoadIconW(instance, IDI_APP)` → `WM_SETICON` (ICON_SMALL + ICON_BIG) on `WM_CREATE`.
- Tray icon: same handle in `NOTIFYICONDATAW.hIcon`.

### 4. Final manifest review
Confirm `app.manifest` includes:
- `<assemblyIdentity version="0.1.0.0" ...>`
- Common Controls v6 dependency
- `<dpiAwareness>PerMonitorV2</dpiAwareness>`
- `<requestedExecutionLevel level="asInvoker" ...>`

### 5. Release build & size check
```
cargo clean
cargo build --release
```
Verify:
- `target\release\mouse_wiggler.exe` ≤ 1.5MB.
- If oversize: confirm `strip = true`, `lto = true`, `panic = "abort"`. Run `cargo bloat --release` to spot offenders. `serde_json` is the usual suspect — acceptable.

### 6. Manual Test Checklist
On a clean Windows 10 or 11 x64 user account:
- [ ] Double-click `.exe` from a USB stick / arbitrary folder — opens normally.
- [ ] No console window.
- [ ] Window crisp at 100/125/150% DPI.
- [ ] Default mode = Zen, default interval = 60.
- [ ] Click **Start** → button → "Stop". Wait 65s. Observed: Teams stays Available, screensaver doesn't trigger.
- [ ] Switch to Visible mode while running → cursor wiggles ±1px every interval, net drift ≈ 0 over 5 minutes.
- [ ] Change interval to 5s while stopped → start → ticks every 5s.
- [ ] Click **Stop** → button → "Start"; jiggling halts.
- [ ] Click X → window hides, tray icon present.
- [ ] Double-click tray → window restored.
- [ ] Right-click tray → menu: Show / Stop (if running) / Exit.
- [ ] Tray-Exit → process gone, no orphan tray icon.
- [ ] Toggle "Start with Windows" → reg entry appears in `HKCU\...\Run`. Reboot → app launches.
- [ ] Toggle "Start minimized to tray" → next launch starts hidden in tray only.
- [ ] Edit `settings.json` to corrupt JSON → app launches with defaults; file overwritten on next change.
- [ ] Delete `MouseWiggler` reg entry while autostart=on → on next launch, value is restored (reconcile).
- [ ] Move .exe to new folder with autostart on → relaunch → registry path updated.

### 7. Final verification
- [ ] Binary ≤ 1.5MB.
- [ ] Defender scan: no detections (run `Get-MpThreat` after scan, or right-click → Scan).
- [ ] Cold start < 200ms (subjective; or measure via `Measure-Command { Start-Process ... -Wait }` against a quick-exit test build).

## Todo
- [x] Source/generate `assets/icon.ico` (multi-res)
- [x] Finalize `app.rc` with icon + manifest
- [x] Wire `IDI_APP` into window + tray
- [x] `cargo build --release` and confirm size
- [ ] Run full manual test checklist (manual hardware-only testing required)
- [x] (Optional) Run `cargo bloat --release` if size > 1.5MB

## Success Criteria
All checkboxes in §6 pass on a fresh Windows account; binary ≤ 1.5MB; no AV detection.

## Risks
- **Defender heuristic** flagging mouse-input automation: if triggered, document the workaround (Add Exclusion) — don't try to obfuscate.
- **Icon licensing**: if grabbing from the web, ensure CC0 / self-made.

## Next
None — feature complete. Future: hotkeys, schedule, MSIX (deferred).
