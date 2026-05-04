---
phase: 1
name: Scaffold project
status: completed
priority: high
effort: S
---

# Phase 01 — Scaffold

## Context
- [plan.md](plan.md)
- [brainstorm](../reports/brainstorm-260504-1552-mouse-wiggler.md)

## Overview
Create Cargo project, configure size-optimized release profile, add app manifest (DPI + CommCtrl v6), get an empty window on screen.

## Requirements
- `cargo run --release` produces a window titled "Mouse Wiggler".
- Manifest applied (DPI awareness, common controls v6).
- Release binary builds with size flags.

## Files to Create
- `Cargo.toml`
- `build.rs`
- `app.manifest`
- `src/main.rs`
- `.gitignore`

## Implementation Steps

### 1. `Cargo.toml`
```toml
[package]
name = "mouse_wiggler"
version = "0.1.0"
edition = "2021"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dependencies.windows]
version = "0.58"
features = [
    "Win32_Foundation",
    "Win32_System_LibraryLoader",
    "Win32_System_Registry",
    "Win32_UI_WindowsAndMessaging",
    "Win32_UI_Input_KeyboardAndMouse",
    "Win32_UI_Shell",
    "Win32_UI_HiDpi",
    "Win32_Graphics_Gdi",
]

[build-dependencies]
embed-resource = "2"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

### 2. `app.manifest`
Standard manifest with `<dpiAwareness>PerMonitorV2</dpiAwareness>` and Microsoft.Windows.Common-Controls v6 dependency. Reference: official Microsoft template.

### 3. `build.rs`
```rust
fn main() {
    embed_resource::compile("app.rc", embed_resource::NONE);
}
```
Stub `app.rc` (to be expanded in phase 07 for icon):
```
1 24 "app.manifest"
```

### 4. `src/main.rs` — minimal `WinMain`-equivalent
- `#![windows_subsystem = "windows"]` (no console)
- Register window class, `CreateWindowExW`, message loop with `GetMessageW` / `TranslateMessage` / `DispatchMessageW`.
- WndProc handles `WM_DESTROY` → `PostQuitMessage`.

### 5. Verify
- `cargo build --release`
- Inspect `target\release\mouse_wiggler.exe` size — note baseline (~400-700KB expected).
- Run: empty window appears, closes cleanly.

## Todo
- [x] Init `Cargo.toml` with deps + release profile
- [x] Add `app.manifest` (DPI + CommCtrl v6)
- [x] Add `build.rs` + `app.rc` stub
- [x] `src/main.rs` empty window + message loop
- [x] `.gitignore` for `/target`, `settings.json`
- [x] Verify release build runs and shows window

## Success Criteria
- Empty 240×170 window opens at center of primary monitor.
- No console window flashes.
- Release build < 1MB at this stage.

## Risks
- `embed-resource` toolchain quirks on MSVC vs MinGW — pin to MSVC (`x86_64-pc-windows-msvc`).

## Next
Phase 02 — jiggler core.
