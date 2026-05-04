---
date: 2026-05-04
type: kickoff
project: mouse_wiggler
tags: [planning, rust, windows, win32]
---

# Mouse Wiggler — Project Kickoff

**Date**: 2026-05-04 16:00  
**Severity**: N/A  
**Component**: Project initialization  
**Status**: Planning complete, implementation ready

## What Happened

Brainstorm + design session completed. Stack selected, architecture finalized, implementation plan generated with 7 sequential phases and task backlog (tasks #4-#10) ready for execution. All scope decisions locked in before coding begins.

## Key Technical Decisions

**Stack & Constraints**
- Language: Rust with `windows` crate (raw Win32 bindings)
- Binary target: ≤1.5MB single .exe (opt-level=z, LTO, strip, panic=abort)
- Minimum Windows version: Windows 7+ (via Win32 API compatibility)

**UI Design**
- Main window: ~240x170px, non-resizable, always-on-top during operation
- Controls: Mode dropdown (Zen/Visible), interval input (integer ms), Start/Stop button
- Persistence: Autostart + start-minimized checkboxes saved atomically to settings.json

**Tray Integration**
- Shell_NotifyIconW for system tray presence
- X button minimizes to tray; tray Exit actually quits
- Left-click restores window; right-click context menu

**Settings Persistence**
- Location: settings.json alongside .exe (no AppData bloat)
- Atomic writes: write to .tmp, rename on success (crash-safe)
- Debounce: 250ms to prevent rapid file churn
- Autostart: HKCU\Software\Microsoft\Windows\CurrentVersion\Run\MouseWiggler with self-healing logic

**Default Behavior**
- First run: Zen mode selected
- Interval: 60000ms (1 minute) default

## Out of Scope (Deferred)

Hotkey support, scheduling, MSIX/installer, code signing, telemetry, multi-language UI, advanced DPI scaling for multi-monitor setups.

## Artifacts

- **Brainstorm report**: `plans/reports/brainstorm-260504-1552-mouse-wiggler.md`
- **Plan directory**: `plans/260504-1552-mouse-wiggler/`
  - `plan.md` (overview)
  - 7 phase files: scaffold → core jiggler → window UI → tray → settings → autostart → packaging/test
- **Task backlog**: Tasks #4-#10 with linear blockedBy dependency chain

## Tracked Risks

1. Antivirus software flagging mouse jiggler pattern as suspicious (mitigation: document behavior, consider code signing)
2. Enterprise GPO policies blocking SendInput() even from user-mode (out of scope for personal use)
3. DPI scaling on multi-monitor systems (deferred to post-MVP)

## Next Steps

Begin Phase 01 (Cargo project scaffold + Rust skeleton) when implementation starts. All prerequisite research complete; ready to build.

---

**Status**: DONE
