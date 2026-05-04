---
date: 2026-05-04
type: completion
project: mouse_wiggler
tags: [rust, windows, win32, implementation]
---

# Mouse Wiggler — Implementation Complete

**Date**: 2026-05-04 16:21
**Component**: All 7 phases
**Status**: Code complete, smoke-tested, manual hardware verification pending

## What Shipped

Single-pass implementation of all phases in one cohesive Rust codebase. `cargo build --release` produces `target\release\mouse_wiggler.exe` at **550,912 bytes (~538 KB)** — well below the 1.5 MB target.

Modules:
- `main.rs` — entry, DPI awareness, settings load, autostart reconcile
- `jiggler.rs` — `SendInput`-based Zen / Visible modes with sign-flipping dx for net-zero drift
- `window.rs` — class registration, control creation w/ DPI scaling, `WndProc`, `AppState` lifecycle (Box leak via `WM_NCCREATE` → reclaim in `WM_NCDESTROY`)
- `tray.rs` — `Shell_NotifyIconW`, popup menu, hide-on-close
- `settings.rs` — JSON persistence next to .exe, atomic tmp+rename via `MoveFileExW`, 250ms debounce
- `autostart.rs` — `HKCU\...\Run` toggle + reconcile-on-launch (self-healing on .exe move)

Icon: SVG → multi-res `assets/icon.ico` (16/32/48/64/128/256) via ImageMagick. Embedded via `app.rc` + `embed-resource`.

## Code Review Findings & Fixes

`code-reviewer` agent ran a soundness audit. One real concern (H1) plus low-impact polish:

**H1 — Re-entrancy aliasing UB (FIXED).** Two sub-cases:
1. `ID_TRAY_EXIT` in `handle_command` called `DestroyWindow(st.hwnd)` while `&mut *p` was still live; the synchronous `WM_NCDESTROY` dispatch dropped `Box<AppState>` under the borrow. Fixed by hoisting the Exit branch into `WndProc` itself, outside any borrow.
2. `tray::show_menu(&AppState)` held an immutable borrow across `TrackPopupMenu`, which pumps messages and re-enters `WndProc` (each handler taking its own `&mut *p`). Fixed by changing `handle_callback` to take `*mut AppState` and `show_menu` to take `(hwnd: HWND, running: bool)` by value — no borrow crosses the modal pump.

**M1 (FIXED)** — Defensive bounds in `read_edit_u32` (`len > 16` → reject; `n.min(buf.len())`).
**M2 (FIXED)** — Best-effort cleanup of stale `settings.json.tmp` at startup.
**M5 (DEFERRED)** — `WM_DPICHANGED` not handled; documented as known v1 limitation per scope.

## Notable Implementation Details

- `windows` crate **0.58** uses `Param<HWND>` rather than `Option<HWND>` for many APIs, requiring `HWND::default()` / `HWND(null_mut())` for null handles instead of `None`. Several core symbols moved across modules between versions: `COLOR_BTNFACE` is in `Graphics::Gdi`, `SS_LEFT` in `System::SystemServices`, `BST_CHECKED`/`BST_UNCHECKED` in `UI::Controls`, `UpdateWindow` in `Graphics::Gdi`.
- Mixing `i32` style constants (`BS_AUTOCHECKBOX`, `CBS_DROPDOWNLIST`, `ES_NUMBER`) with `WINDOW_STYLE` (u32 wrapper) requires explicit `WINDOW_STYLE(c as u32)` packaging — abstracted via `style_with_class()` helper.
- `HMENU` for child control IDs constructed as `HMENU(id as usize as *mut _)` — Rust forbids u16-to-pointer direct casts; need usize bridge.
- `Cell<bool>` on `AppState` to suppress save-on-init while controls are being populated.

## What Was NOT Done in This Session

Manual hardware checklist (Phase 07 §6) — all click-driven verification: Start/Stop with stopwatch, Teams availability over 5 min, visible-mode cursor wiggle observation, autostart reboot test, settings.json round-trip via UI, registry reconcile after .exe move. These need a real desktop user.

## Next Steps

User to run the Phase 07 manual checklist on a clean Windows 10/11 account from a USB stick. If Defender flags it, document the exclusion workaround.
