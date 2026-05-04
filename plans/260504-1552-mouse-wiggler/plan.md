---
name: Mouse Wiggler (Windows, Rust + windows-rs)
slug: mouse-wiggler
date: 2026-05-04
status: completed
mode: auto
blockedBy: []
blocks: []
---

# Mouse Wiggler — Implementation Plan

Tiny portable Windows app that prevents idle/lock by emitting mouse input on a configurable interval. Rust + raw Win32 via `windows` crate. Single .exe, no installer, ≤ 1.5MB stripped.

## Source Brainstorm
[brainstorm-260504-1552-mouse-wiggler.md](../reports/brainstorm-260504-1552-mouse-wiggler.md)

## Resolved Decisions (from brainstorm open questions)
- **Icon**: source/create in packaging phase (16/32/48px multi-res .ico). Use ImageMagick or simple SVG → ICO conversion.
- **Close (X)**: minimize-to-tray (matches reference app convention). Tray menu **Exit** is the only way to actually quit.
- **Default mode** on first run: **Zen** (invisible, won't disturb).

## Phases

| # | File | Status | Description |
|---|------|--------|-------------|
| 01 | [phase-01-scaffold.md](phase-01-scaffold.md) | completed | Cargo project, release profile, manifest, hello-window |
| 02 | [phase-02-jiggler-core.md](phase-02-jiggler-core.md) | completed | `SendInput` zen + visible, timer state machine |
| 03 | [phase-03-window-ui.md](phase-03-window-ui.md) | completed | Main window, controls, WM handlers, Start/Stop |
| 04 | [phase-04-tray.md](phase-04-tray.md) | completed | Shell_NotifyIcon, popup menu, minimize-to-tray |
| 05 | [phase-05-settings-persistence.md](phase-05-settings-persistence.md) | completed | settings.json load/save next to .exe |
| 06 | [phase-06-autostart-registry.md](phase-06-autostart-registry.md) | completed | HKCU Run key toggle |
| 07 | [phase-07-packaging-and-test.md](phase-07-packaging-and-test.md) | completed | Icon, manifest, manual test checklist, size verify |

## Dependencies
Linear chain — each phase depends on prior. Phase 02 (jiggler) can technically run in parallel with 03 (UI), but recommend sequential for solo dev.

## Stack
- Rust edition 2021
- `windows` crate (Win32 features only)
- `serde` + `serde_json`
- Release: `opt-level="z"`, `lto=true`, `strip=true`, `panic="abort"`, `codegen-units=1`

## Out of Scope
Global hotkeys, schedule/active-hours, MSIX, code signing, telemetry, multi-language UI, auto-update.

## Success Criteria
- Single `.exe` ≤ 1.5MB on clean Win10/11 x64.
- Both modes verifiably keep system "active" past default idle (~5min) at 60s interval.
- Toggle, minimize-to-tray, autostart, settings persistence functional across reboots.
- DPI-aware (no blur at 125/150/200%).
- Cold start < 200ms.
