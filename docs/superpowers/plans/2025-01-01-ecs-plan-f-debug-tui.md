# ECS Migration Plan F: Debug TUI Redesign

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Delete `DebugSnapshot`, `ActorSnapshot`, and `build_debug_snapshot()`. Replace `update_status()` with a `render(&World, &Resources)` signature that queries data at render time. Zero per-frame allocations on the debug TUI hot path.

**Architecture:** `bridge.rs` shrinks to just command/log types and display helpers. `DebugConsole::render` takes shared references to `World` and `Resources` and queries each panel's data directly. The stub mirrors the same signature. `main.rs` loses ~160 lines of snapshot construction.

**Prerequisites:** Plans A–D complete. `EcsScene` is the active gameplay path.

**Tech Stack:** Rust 2021, `ratatui = "0.29"` (feature-gated), `hecs = "0.11"`.

---

## File map

| File | Change |
|---|---|
| `src/game/debug_tui/bridge.rs` | Remove `DebugSnapshot`, `ActorSnapshot`, all constructors; keep `DebugCommand`, `DebugLogEntry`, `LogCategory`, display helpers |
| `src/game/debug_tui/view.rs` | Replace `update_status()` with `render(&World, &Resources, canvas)` |
| `src/game/debug_tui/stub.rs` | Mirror new API: `render(&World, &Resources, canvas)`, remove `update_status()` |
| `src/game/debug_tui/mod.rs` | Remove `DebugSnapshot` re-export |
| `src/main.rs` | Delete `build_debug_snapshot()` and the `update_status()` call; call `debug_console.render(&world, &resources, canvas)` |

---

## Task 1: Shrink `bridge.rs` — remove snapshot types

**Files:**
- Modify: `src/game/debug_tui/bridge.rs`

- [ ] **Step 1: Read the full current `bridge.rs`**

```bash
wc -l /home/ddehaven/projects/faery-tale-rs/src/game/debug_tui/bridge.rs
```

Then read to understand which parts to keep.

- [ ] **Step 2: Identify what to delete**

Using `grep` find all structs/types/impls to remove:
```bash
grep -n "^pub struct\|^struct\|^impl\|^pub fn\|^fn" src/game/debug_tui/bridge.rs
```

Delete:
- `DebugSnapshot` struct and all its `impl` blocks
- `ActorSnapshot` struct and all its `impl` blocks
- `fn build_debug_snapshot(...)` (if defined here rather than `main.rs`)
- Any function whose sole purpose is constructing `DebugSnapshot` or `ActorSnapshot`

Keep:
- `DebugCommand` (and its `use` re-export from `debug_command.rs`)
- `DebugLogEntry`, `LogCategory` (and their `use` re-export from `debug_log.rs`)
- Display helper functions: `weapon_short_name`, `facing_name`, `goal_label`, `tactic_label`, `npc_state_name`, `actor_state_name` (any helpers used by the TUI panels directly)
- All `use` imports needed by the above

- [ ] **Step 3: Remove `use crate::game::game_state::DayPhase;`**

`DayPhase` is derived from `GameState`. After `GameState` deletion, this import breaks. Remove it. Time-of-day string formatting in the TUI panels will derive the period directly from `GameClock.daynight` using a helper function.

Add a replacement helper in `bridge.rs`:
```rust
/// Derive a time-of-day label from the raw daynight counter.
/// DAYLEN = 24000; 12 periods, each 2000 ticks wide.
pub fn daynight_period_label(daynight: u16) -> &'static str {
    match daynight / 2000 {
        0            => "Midnight",
        1            => "Late Night",
        2            => "Early Morning",
        3            => "Dawn",
        4            => "Morning",
        5            => "Late Morning",
        6            => "Noon",
        7            => "Afternoon",
        8            => "Late Afternoon",
        9            => "Evening",
        10           => "Dusk",
        11 | _       => "Night",
    }
}
```

- [ ] **Step 4: Run cargo check**

```bash
cargo check 2>&1 | grep "^error" | head -20
```

The expected errors are:
- `DebugSnapshot` used in `stub.rs` — fixed in Task 3
- `DebugSnapshot` used in `view.rs` — fixed in Task 2
- `DebugSnapshot` or `ActorSnapshot` used in `main.rs` — fixed in Task 4
- `DayPhase` used elsewhere — remove or replace

Fix each error that originates from `bridge.rs` itself. Leave `stub.rs` / `view.rs` / `main.rs` errors for their respective tasks.

- [ ] **Step 5: Commit partial change**

```bash
git add src/game/debug_tui/bridge.rs
git commit -m "refactor(debug-tui): remove DebugSnapshot and ActorSnapshot from bridge.rs"
```

---

## Task 2: Update `view.rs` — replace `update_status` with direct queries

**Files:**
- Modify: `src/game/debug_tui/view.rs`

This file is only compiled under `#[cfg(feature = "debug-tui")]`. The changes here are the largest in this plan.

- [ ] **Step 1: Add new render method signature**

In `view.rs`, find `impl DebugConsole`. Replace:
```rust
pub fn update_status(&mut self, status: DebugSnapshot) {
    self.status = status;
}
pub fn render(&mut self) {
    // draws from self.status
}
```

With:
```rust
pub fn render(
    &mut self,
    world: &hecs::World,
    resources: &crate::game::ecs::resources::Resources,
    canvas: &mut sdl3::render::Canvas<sdl3::video::Window>,
) {
    // draw panels by querying world and resources directly
    // ... (implemented in steps below)
}
```

- [ ] **Step 2: Update the status header panel**

Find the status header panel drawing code (reads from `self.status.daynight`, `self.status.fps`, etc.). Replace each field access with a direct resource/query:

```rust
// Before:
let daynight = self.status.daynight;
let fps = self.status.fps;
let region = self.status.region_num;

// After:
let daynight = resources.clock.daynight;
let region = resources.region.region_num;
// fps is still passed as a parameter or computed from a timer stored in DebugConsole
let fps = self.fps_counter.current();
```

For fields that were in `DebugSnapshot` but are now in `Resources`, the mapping is direct. For fields derived from `GameState` that no longer exist in the same form (e.g. `DayPhase`), use the new `daynight_period_label()` helper from `bridge.rs`.

- [ ] **Step 3: Update the hero panel**

```rust
// Before:
let vitality = self.status.vitality;
let hero_x = self.status.hero_x;

// After:
let (vitality, hero_x, hero_y, hunger, fatigue) =
    world.query_one::<(&HeroStats, &Position)>(resources.hero_entity)
         .ok()
         .and_then(|mut q| q.get().map(|(s, p)| (s.vitality, p.x as u16, p.y as u16, s.hunger, s.fatigue)))
         .unwrap_or((0, 0, 0, 0, 0));
```

- [ ] **Step 4: Update the actor watch panel**

```rust
// Before:
for snapshot in &self.status.actors { ... }

// After:
for (_, (pos, facing, ai, health, kind)) in world
    .query::<(&Position, &Facing, &AiState, &Health, &EnemyKind)>()
    .iter()
{
    render_actor_row(pos, facing, ai, health, kind);
}
```

- [ ] **Step 5: Update all other panels**

Work through each panel in the TUI, replacing every `self.status.XYZ` with the equivalent `resources.XYZ` or a `world.get::<&Component>(entity)` call.

Panel → Resource/Query mapping:
| Panel data | Source |
|---|---|
| `fps`, `tps` | Computed locally in `DebugConsole` (keep a fps_counter field) |
| `daynight`, `lightlevel` | `resources.clock.daynight`, `resources.clock.lightlevel` |
| `game_ticks` | `resources.clock.tick_counter` |
| `paused` | `resources.view.paused` |
| `hero_x`, `hero_y` | `world.get::<&Position>(resources.hero_entity)` |
| `vitality`, `brave`, `luck` | `world.get::<&HeroStats>(resources.hero_entity)` |
| `brother` | `world.get::<&BrotherKind>(resources.hero_entity)` |
| `region_num` | `resources.region.region_num` |
| `god_mode_flags` | `resources.brother.god_mode` |
| `vfx_jewel_active` | `resources.clock.light_timer > 0` |
| `vfx_secret_active` | `resources.clock.secret_timer > 0` |
| `princess_captive` etc. | `resources.region.princess` |
| `current_mood` | `resources.region.current_mood` |
| `battleflag` | `resources.region.battleflag` |
| Enemy actors | `world.query::<(&Position, &Facing, &AiState, &Health, &EnemyKind)>()` |
| Missiles | `world.query::<(&Position, &MissileMotion, &MissileKind)>()` |
| World objects | `world.query::<(&Position, &WorldObj)>()` |

- [ ] **Step 6: Remove `self.status: DebugSnapshot` field from `DebugConsole`**

In `DebugConsole` struct, remove:
```rust
status: DebugSnapshot,
```

Remove any initializer for it in `DebugConsole::new()`.

- [ ] **Step 7: Build with debug-tui feature**

```bash
cargo build --features debug-tui 2>&1 | grep "^error"
```
Expected: no errors.

- [ ] **Step 8: Commit**

```bash
git add src/game/debug_tui/view.rs
git commit -m "refactor(debug-tui): DebugConsole::render queries World+Resources directly"
```

---

## Task 3: Update `stub.rs` to match new API

**Files:**
- Modify: `src/game/debug_tui/stub.rs`

- [ ] **Step 1: Replace `stub.rs`**

The stub must mirror the exact public API of the real `DebugConsole`. Remove `update_status`, add the new `render` signature:

```rust
//! Stub `DebugConsole` — no-op implementation when `debug-tui` feature is disabled.
//! Mirrors the public API of `view::DebugConsole` exactly.

use std::io;
use crate::game::debug_tui::bridge::{DebugCommand, DebugLogEntry};

pub struct DebugConsole {
    _priv: (),
}

impl DebugConsole {
    pub fn new() -> Result<Self, io::Error> {
        eprintln!(
            "warning: --debug requested but binary built without `debug-tui` feature; \
             no debug console will open. Rebuild with default features (or \
             `--features debug-tui`) to enable."
        );
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "debug-tui feature disabled",
        ))
    }

    pub fn render(
        &mut self,
        _world: &hecs::World,
        _resources: &crate::game::ecs::resources::Resources,
        _canvas: &mut sdl3::render::Canvas<sdl3::video::Window>,
    ) {}

    pub fn log_entry(&mut self, _entry: DebugLogEntry) {}
    pub fn ingest(&mut self, _entry: DebugLogEntry) {}
    pub fn log(&mut self, _msg: impl Into<String>) {}

    pub fn drain_commands(&mut self) -> Vec<DebugCommand> { Vec::new() }
    pub fn poll_commands(&mut self) -> Vec<DebugCommand> { Vec::new() }
    pub fn take_pause_request(&mut self) -> Option<bool> { None }
    pub fn take_step_request(&mut self) -> u32 { 0 }
    pub fn take_song_request(&mut self) -> Option<usize> { None }
    pub fn take_stop_request(&mut self) -> bool { false }
    pub fn take_cave_mode_request(&mut self) -> Option<bool> { None }
    pub fn take_quit_request(&mut self) -> bool { false }
    pub fn poll_input(&mut self) -> bool { false }
}
```

- [ ] **Step 2: Build without debug-tui feature**

```bash
cargo build 2>&1 | grep "^error"
```
Expected: no errors.

- [ ] **Step 3: Commit**

```bash
git add src/game/debug_tui/stub.rs
git commit -m "refactor(debug-tui): update stub to match new render API"
```

---

## Task 4: Delete `build_debug_snapshot` from `main.rs`

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Find snapshot construction in `main.rs`**

```bash
grep -n "build_debug_snapshot\|update_status\|DebugSnapshot" src/main.rs
```

- [ ] **Step 2: Delete `build_debug_snapshot` function**

Find the `fn build_debug_snapshot(...)` function body (approximately 160 lines). Delete it entirely.

- [ ] **Step 3: Delete `update_status` call**

Find the call site:
```rust
let snapshot = build_debug_snapshot(&gs, total_game_ticks, fps, tps);
debug_console.update_status(snapshot);
```

Delete these lines. The `debug_console.render(&world, &resources, canvas)` call is already present from Plan D.

- [ ] **Step 4: Remove now-unused imports in `main.rs`**

```bash
cargo check 2>&1 | grep "unused import\|^error\[" | head -20
```

Remove any `use` lines that are now unused (typically `DebugSnapshot`, `ActorSnapshot`, `DayPhase`).

- [ ] **Step 5: Build**

```bash
cargo build 2>&1 | grep "^error"
```
Expected: no errors.

- [ ] **Step 6: Build with debug-tui feature**

```bash
cargo build --features debug-tui 2>&1 | grep "^error"
```
Expected: no errors.

- [ ] **Step 7: Commit**

```bash
git add src/main.rs
git commit -m "refactor(main): delete build_debug_snapshot and update_status call (~160 lines)"
```

---

## Task 5: Cleanup — remove remaining snapshot references

- [ ] **Step 1: Find any remaining snapshot uses**

```bash
grep -rn "DebugSnapshot\|ActorSnapshot\|build_debug_snapshot\|update_status" src/ --include="*.rs"
```
Expected: no results.

- [ ] **Step 2: Remove `DebugSnapshot` re-export from `debug_tui/mod.rs`**

In `src/game/debug_tui/mod.rs`, remove:
```rust
pub use bridge::*;
```
Replace with explicit re-exports of what is actually needed:
```rust
pub use bridge::{DebugCommand, DebugLogEntry, LogCategory, daynight_period_label};
```

- [ ] **Step 3: Run full test suite**

```bash
cargo test 2>&1 | grep "^test result"
```
Expected: all test suites pass.

- [ ] **Step 4: Test with debug-tui feature**

```bash
cargo test --features debug-tui 2>&1 | grep "^test result"
```

- [ ] **Step 5: Final commit**

```bash
git add src/game/debug_tui/mod.rs
git commit -m "refactor(debug-tui): explicit re-exports; debug TUI is now zero-copy snapshot-free"
```

---

## Completion check

```bash
# Verify no snapshot types remain
grep -rn "DebugSnapshot\|ActorSnapshot" src/ --include="*.rs"
# Expected: no output

# Verify bridge.rs has shrunk significantly
wc -l src/game/debug_tui/bridge.rs
# Expected: ~150 lines (was ~434)

# Verify both build modes work
cargo build 2>&1 | grep "^error"
cargo build --features debug-tui 2>&1 | grep "^error"

# Verify tests pass in both modes
cargo test 2>&1 | grep "^test result"
cargo test --features debug-tui 2>&1 | grep "^test result"
```

All green. The debug TUI now reads directly from the ECS world — no snapshot, no per-frame allocation, no copy.
