# Project Overview & Product Requirements (PDR)

## Problem Statement

Users engage in legitimate activities (reading, watching, thinking) that don't generate mouse/keyboard input, causing systems to auto-lock or mark them as "Away" in collaboration tools (Teams, Slack, Discord), interrupting workflow and creating false unavailability signals.

## Target Users

- Knowledge workers (developers, writers, analysts) during focused work
- Remote employees subject to aggressive idle policies
- Personal/home office use cases only (not enterprise deployment)

## Functional Requirements

| Req | Priority | Specification |
|-----|----------|----------------|
| **FR1** | MUST | **Two input modes:** Zen (dx=0, dy=0 invisible) and Visible (±1px alternating, net drift=0) |
| **FR2** | MUST | **Configurable interval** (user-settable, 1–86400 sec, default 60 sec) |
| **FR3** | MUST | **Simple UI** (minimize to tray, Start/Stop button, mode selector, interval editor) |
| **FR4** | MUST | **Persistent settings** (JSON file next to .exe; auto-load on startup) |
| **FR5** | MUST | **Autostart registry** (HKCU Run key, user-toggleable, self-heals on exe move) |
| **FR6** | SHOULD | **Minimize to tray on startup** (start_minimized setting) |
| **FR7** | SHOULD | **Tray popup menu** (Show, Start/Stop, Exit) |
| **FR8** | NICE | **DPI awareness** (PerMonitorV2 manifest) |

## Non-Functional Requirements

| Req | Priority | Specification |
|-----|----------|----------------|
| **NFR1** | MUST | **Single .exe distribution** (no installer, no MSI, no MSIX) |
| **NFR2** | MUST | **No runtime dependencies** (Win32 API only; no .NET, no Visual C++ redist) |
| **NFR3** | MUST | **No admin elevation** (user-scope operations only) |
| **NFR4** | MUST | **Cold start < 200 ms** (snap startup) |
| **NFR5** | MUST | **Executable size ≤ 1.5 MB** (stripped, optimized) |
| **NFR6** | MUST | **No telemetry or crash reporting** (privacy-first) |
| **NFR7** | MUST | **No code signing** (unsigned .exe; SmartScreen bypass is manual) |
| **NFR8** | MUST | **Single-threaded message loop** (no async, no background threads) |
| **NFR9** | SHOULD | **Robust error handling** (silent failures for non-actionable Win32 errors; UIPI/locked-desktop = no-op) |

## In Scope

- Zen mode (invisible idle-timer reset)
- Visible mode (±1px movement, net zero drift)
- Configurable interval (1–86400 sec)
- JSON settings persistence
- HKCU Run autostart (no admin required)
- Minimize-to-tray UI
- Start/Stop button, mode selector, interval editor
- Tray popup menu (Show, Start/Stop, Exit)
- DPI awareness (PerMonitorV2)
- Release-optimized single .exe ≤ 1.5 MB

## Out of Scope

- Global hotkeys (keyboard shortcuts; UI interaction only)
- Scheduling (fixed intervals always; no calendars or time-of-day logic)
- MSIX/Windows Store packaging
- Code signing or Authenticode
- Telemetry, crash dumps, or usage analytics
- Multi-language UI (English only)
- Auto-update or update-checking mechanism
- Network connectivity
- Mobile or non-Windows platforms

## Success Criteria

1. ✅ Builds to single `.exe` ≤ 1.5 MB (no dependencies)
2. ✅ Runs on Windows 10/11 x64 (no admin required)
3. ✅ Prevents Teams/Slack "Away" markers during Zen mode
4. ✅ Accepts all FR1–FR8 requirements without breaking changes
5. ✅ Cold start ≤ 200 ms
6. ✅ Settings persist across restarts
7. ✅ Autostart registry can be toggled from UI and survives exe move
8. ✅ Manual testing passes per phase-07 checklist

## Architecture Decision: Single-Threaded Message Loop

**Rationale:** Win32 message loops are inherently single-threaded. UI operations, timer callbacks, and registry queries are all serialized through `GetMessageW()` / `DispatchMessageW()`. Attempting background threads adds complexity (synchronization, UIPI, cross-thread marshaling) with minimal benefit for this use case.

**Trade-off:** Input injection (`SendInput`) is instantaneous; no observable blocking. Settings save is I/O-bound but rare and debounced (250 ms). Message loop remains responsive.

## Stack

- **Language:** Rust 2021 edition
- **Crates:** `windows` 0.58, `serde`, `serde_json`
- **Build:** `embed_resource` 2 (manifest + icon embedding)
- **Target:** x86_64-pc-windows-gnu (or -msvc)
- **Optimizer:** LTO + size optimization (`opt-level = "z"`)
