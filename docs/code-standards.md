# Code Standards & Guidelines

Codebase-specific conventions and best practices for Mouse Wiggler.

## Language & Edition

- **Rust edition:** 2021
- **MSRV:** None formally declared (windows 0.58 requires Rust 1.56+).
- **Target triples:** `x86_64-pc-windows-gnu`, `x86_64-pc-windows-msvc` (x64 only; ARM untested).

## File Organization

| File | Purpose | Max LOC |
|------|---------|---------|
| `src/main.rs` | Entry, DPI init, bootstrap | 70 |
| `src/jiggler.rs` | Input injection | 100 |
| `src/autostart.rs` | Registry helpers | 120 |
| `src/settings.rs` | Persistence, validation | 100 |
| `src/window.rs` | GUI, DPI layout, message loop | 850 |
| `src/tray.rs` | Tray icon, menu | 150 |
| `src/dpi.rs` | DPI scaling, UI font | 100 |

**Guideline:** Keep individual modules ≤ 200 LOC where practical. Existing `window.rs` (811 LOC) is acceptable due to tight Win32 integration; future modules should aim for ≤ 150 LOC.

## Naming Conventions

| Scope | Convention | Example |
|-------|-----------|---------|
| **Module files** | `snake_case` (Rust idiom) | `settings.rs`, `autostart.rs` |
| **Functions** | `snake_case` | `current_exe_command()`, `reconcile()` |
| **Types** | `PascalCase` | `Settings`, `Jiggler`, `AppState` |
| **Constants** | `UPPER_CASE` | `RUN_KEY`, `SAVE_DEBOUNCE_MS`, `TIMER_JIGGLE` |
| **Enum variants** | `PascalCase` | `Mode::Zen`, `Mode::Visible` |

## Unsafe & Win32 Boundaries

### Rule 1: Unsafe blocks are OK; document intent.

All Win32 FFI calls are inherently unsafe. Wrap each unsafe block with a comment explaining the invariant upheld.

**Bad:**
```rust
unsafe {
    let n = GetModuleFileNameW(None, &mut buf);
}
```

**Good:**
```rust
// SAFETY: `buf` is a valid stack-allocated array of u16.
// GetModuleFileNameW writes up to buf.len()-1 elements + null terminator.
let n = unsafe { GetModuleFileNameW(None, &mut buf) as usize };
```

### Rule 2: Minimize unsafe scope.

Unsafe block contains only the FFI call; validation/conversion happens outside.

**Bad:**
```rust
unsafe {
    let path = String::from_utf16_lossy(&buf[..n]);
    // ... more code ...
}
```

**Good:**
```rust
let n = unsafe { GetModuleFileNameW(None, &mut buf) as usize };
let path = String::from_utf16_lossy(&buf[..n]);
```

### Rule 3: Pointers require explicit safety narrative.

When taking raw pointers (e.g., for `GWLP_USERDATA` callbacks):

```rust
// SAFETY: AppState lives on heap, owned by window through GWLP_USERDATA.
// It is not dropped until WM_NCDESTROY. Callback accesses are serialized
// through the message loop, preventing concurrent mutation.
let state_ptr = Box::leak(Box::new(state)) as *mut AppState;
```

## Error Handling

### Non-Actionable Errors → Silent Failure

For Win32 API failures where there is no user-actionable recovery:

```rust
// Registry value doesn't exist → treat as "not enabled".
let exists = RegQueryValueExW(...).is_ok();

// SendInput fails (e.g. UIPI) → no-op.
unsafe { SendInput(&[input], ...); }  // failures swallowed

// Settings I/O fails → retry on next debounce.
let _ = fs::write(&tmp, &json);
```

### Actionable Errors → Propagate

Return `Result<>` when caller needs to decide:

```rust
pub fn reconcile(cfg: &Settings) -> Result<()> {
    if is_enabled() == cfg.autostart {
        return Ok(());
    }
    set(cfg.autostart)
}
```

## Memory Safety

### Stack Buffers for Win32 Strings

UTF-16 buffers for Win32 APIs are allocated on stack with fixed size:

```rust
// GetModuleFileNameW writes up to buf.len()-1 elements.
let mut buf = [0u16; 1024];
let n = unsafe { GetModuleFileNameW(None, &mut buf) } as usize;
let path = String::from_utf16_lossy(&buf[..n]);
```

**Size chosen conservatively:** MAX_PATH (260) + slack for UNC paths (~1024).

### Heap Allocation Only Where Necessary

- Settings struct: derived `Deserialize` for JSON parsing.
- AppState: single heap-allocated, owned by window.
- Temporary strings/vecs: scoped to function call.

No persistent allocations in message loop hot path.

## Serde Usage

Settings struct uses derived `#[serde(default)]` for forward/backward compatibility:

```rust
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(default)]
pub struct Settings {
    pub mode: String,
    pub interval_sec: u32,
    pub autostart: bool,
    pub start_minimized: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: "zen".into(),
            interval_sec: 60,
            autostart: false,
            start_minimized: false,
        }
    }
}
```

**Benefit:** Adding new fields in future versions doesn't break old JSON files.

## Constants vs Magic Numbers

All magic numbers are named constants at module or struct level:

```rust
const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const VALUE_NAME: &str = "MouseWiggler";
const SAVE_DEBOUNCE_MS: u32 = 250;
const TIMER_JIGGLE: usize = 1;
```

No bare numbers in code (except loop indices, 0/1 initializers).

## Documentation

### Module-level comments

Every module starts with a brief comment:

```rust
// Jiggler core: emits a synthetic mouse-move event via SendInput.
//
// Two modes:
//   Zen     — dx=0, dy=0. Resets idle timer without moving the cursor.
//   Visible — alternates dx=+1 / -1 each tick; net cursor displacement zero.
```

### Function documentation

Public functions have `///` doc comments:

```rust
/// Inject a single mouse-move event. Failures (e.g. UIPI / locked desktop)
/// are silently swallowed — there is nothing actionable.
pub fn tick(&mut self) {
    ...
}
```

### Inline comments for complex logic

Used sparingly, only where code intent is non-obvious:

```rust
// Atomic rename: tmp -> final.
let from: Vec<u16> = tmp.as_os_str().encode_wide().chain(Some(0)).collect();
let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
```

## Testing

**Status:** No automated unit tests. Manual testing checklist in phase-07 plan.

**Rationale:** Single-threaded Win32 message loop is difficult to unit test. Integration testing (manual) is simpler and more reliable.

## Compilation & Optimization

**Release profile** (Cargo.toml):
```toml
[profile.release]
opt-level = "z"        # Optimize for size
lto = true             # Link-time optimization
codegen-units = 1      # Monolithic codegen (slower build, better optimization)
strip = true           # Strip debug symbols
panic = "abort"        # No unwinding (smaller code)
```

**Build command:**
```powershell
cargo build --release
```

**Verification:**
```powershell
(Get-Item .\target\release\mouse_wiggler.exe).Length / 1MB
```

Target: ≤ 1.5 MB after link + strip.

## Git Conventions

### Commit Format

Use conventional commits (no AI attribution):

```
feat: implement Zen/Visible input modes
fix: handle moved .exe in autostart registry
refactor: split window.rs message handlers
docs: add codebase summary
test: add manual testing checklist
chore: update Cargo.toml dependencies
```

### Commit Scope

- **Atomic:** One feature/fix per commit.
- **Tested:** Verify `cargo build --release` succeeds before committing.
- **Linted:** Run `cargo clippy --release` (warnings are acceptable if intentional).

### Push to main

- Only after full build + manual testing passes.
- Do NOT commit confidential files (.env, API keys, credentials.json).
- Do NOT commit generated artifacts (target/, *.pdb).

## Code Review Checklist

Before merging to main:

- [ ] Builds with `cargo build --release` (no errors).
- [ ] Runs with expected behavior (manual test).
- [ ] No `unsafe` without safety comment.
- [ ] Error handling covers non-actionable failures.
- [ ] Module size ≤ 200 LOC (or justified).
- [ ] Constants named, no magic numbers.
- [ ] Public API has doc comments.
- [ ] Commit message follows conventional format.

## Linting & Warnings

**Clippy:** Warnings are informational. Most are acceptable unless they indicate:
- Unsafe code without clear invariants.
- Unreachable code or dead logic.
- Performance issues (e.g., unneeded allocations in hot path).

**Run before commit:**
```powershell
cargo clippy --release
```

Address warnings if they suggest genuine improvements; suppress with `#[allow(...)]` if intentional.

## Dependencies

**Crate audit:**
```powershell
cargo audit
```

Run before release. Prefer stable, well-maintained crates.

**Current dependencies:**
- `windows` 0.58 — Win32 bindings (maintained by Microsoft).
- `serde` 1 — Serialization (de facto standard).
- `serde_json` 1 — JSON (de facto standard).
- `embed_resource` 2 — Manifest + icon embedding.

## Architecture Principles

1. **Single-threaded message loop** — All operations serialized through Win32.
2. **No async/await** — Simplicity over parallelism.
3. **Silent failures for non-actionable errors** — User doesn't see stack traces.
4. **Minimal state** — AppState owns only what's needed for message handlers.
5. **Win32 API as substrate** — Don't hide or abstract unnecessary layers.

## Future Considerations

- **Modularization:** If window.rs exceeds 650 LOC, extract dialog procedures or control handlers into submodules.
- **Testing:** If complexity grows, add integration tests via `#[cfg(test)]` modules (manual mocking required for Win32).
- **Telemetry:** Explicitly out of scope; don't add logging unless debugging a crash.
