# Unify Time System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `GameState.daynight` the single source of truth for game time, eliminating the desynchronized `GameClock.game_ticks` wall-clock that causes the debug display to show wrong time and `/time` commands to appear to revert.

**Architecture:** Remove the redundant wall-clock/day-phase methods from `GameClock` (leaving it as a pure tick-rate pacer). Derive all displayed time (day, hour, minute, phase) from `GameState.daynight` + a new `game_days` counter. Add a `daynight_to_wall_clock()` helper to `GameState`. Wire main.rs and debug console to read from `GameState` instead of `GameClock`. Remove the `A`/`Shift+A` key handlers (they only modified `GameClock` and had no gameplay effect); the `/time` debug command is the proper replacement.

**Tech Stack:** Rust, SDL2 (no new dependencies)

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `src/game/game_state.rs` | Modify | Add `game_days: u32`, add `daynight_to_wall_clock()` helper, increment `game_days` on daynight wrap |
| `src/game/game_clock.rs` | Modify | Remove `get_game_wall_clock`, `set_game_wall_clock`, `advance_game_wall_clock_to`, `advance_game_wall_clock_by`, `get_game_days`, `get_day_phase`, `DayPhase` enum. Keep tick pacer (`update()`, `pause/resume`, `reset_game_ticks`). Move `DayPhase` to `game_state.rs`. |
| `src/game/debug_console.rs` | Modify | Update `DebugStatus` to add `daynight` and `lightlevel` fields, derive displayed time from `daynight` |
| `src/main.rs` | Modify | Build `DebugStatus` from `GameState` fields instead of `GameClock` wall-clock methods. Remove `A`/`Shift+A` key handlers. Remove the `last_minute` / dirty-on-minute-change logic (it reads from `GameClock`). |
| `src/game/gameplay_scene.rs` | Modify | `SetDayPhase`/`SetGameTime` handlers should also recompute `lightlevel` and `dayperiod` immediately |

---

### Task 1: Add `game_days` counter and wall-clock helper to `GameState`

**Files:**
- Modify: `src/game/game_state.rs:63-68` (add `game_days` field near `daynight`)
- Modify: `src/game/game_state.rs:184-185` (init `game_days: 0`)
- Modify: `src/game/game_state.rs:269-295` (increment `game_days` on daynight wrap)
- Modify: `src/game/game_state.rs` (add `daynight_to_wall_clock()` method)

- [ ] **Step 1: Add the `game_days` field and initializer**

In `src/game/game_state.rs`, add `game_days` after `daynight`:

```rust
    // Cycle counters
    /// 0–24000 wrapping
    pub daynight: u16,
    /// Number of full day cycles completed
    pub game_days: u32,
    /// Derived triangle wave 0–300–0
    pub lightlevel: u16,
```

And in `GameState::new()`, initialize it:

```rust
            daynight: 8000,
            game_days: 0,
            lightlevel: 300,
```

- [ ] **Step 2: Increment `game_days` on daynight wrap in `daynight_tick()`**

In `src/game/game_state.rs`, inside `daynight_tick()`, after the wrap check:

```rust
        let prev = self.daynight;
        self.daynight = self.daynight.wrapping_add(1);
        if self.daynight >= 24000 {
            self.daynight = 0;
            self.game_days += 1;
        }
```

- [ ] **Step 3: Add `daynight_to_wall_clock()` helper method**

Add this method to the `impl GameState` block:

```rust
    /// Derive (day, hour, minute) from the authoritative `daynight` counter.
    /// - hour: 0–23 (each hour = 1000 daynight ticks)
    /// - minute: 0–59
    pub fn daynight_to_wall_clock(&self) -> (u32, u32, u32) {
        let hour = (self.daynight / 1000) as u32;
        let remainder = (self.daynight % 1000) as u32;
        let minute = remainder * 60 / 1000;
        (self.game_days, hour, minute)
    }
```

- [ ] **Step 4: Add `daynight_day_phase()` helper method**

Add this method to the `impl GameState` block, using the same boundaries as the original `dayperiod` logic:

```rust
    /// Derive the current day phase from `daynight`, matching original fmain.c
    /// segment boundaries: 0=midnight, 6000=morning, 12000=midday, 18000=evening.
    pub fn daynight_day_phase(&self) -> u8 {
        self.dayperiod
    }
```

- [ ] **Step 5: Run tests**

Run: `cargo test --quiet`
Expected: all tests pass (no behavioral change yet)

- [ ] **Step 6: Commit**

```bash
git add src/game/game_state.rs
git commit -m "feat(time): add game_days counter and wall-clock helper to GameState

GameState.daynight is now the single source of truth for game time.
Added game_days counter (incremented on daynight wrap) and
daynight_to_wall_clock() to derive (day, hour, minute) for display.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 2: Move `DayPhase` enum from `game_clock.rs` to `game_state.rs`

**Files:**
- Modify: `src/game/game_state.rs` (add `DayPhase` enum)
- Modify: `src/game/game_clock.rs` (remove `DayPhase` enum, remove `get_day_phase()`)
- Modify: `src/game/debug_console.rs` (update import path)

- [ ] **Step 1: Add `DayPhase` enum to `game_state.rs`**

At the top of `src/game/game_state.rs` (after imports), add:

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum DayPhase {
    #[default]
    Midnight = 0,
    Morning = 4,
    Midday = 6,
    Evening = 9,
}
```

And add a conversion method to `impl GameState`:

```rust
    /// Get the current day phase from dayperiod.
    pub fn get_day_phase(&self) -> DayPhase {
        match self.dayperiod {
            0 => DayPhase::Midnight,
            1 => DayPhase::Morning,
            2 => DayPhase::Midday,
            3 => DayPhase::Evening,
            _ => DayPhase::Midnight,
        }
    }
```

- [ ] **Step 2: Remove `DayPhase` and `get_day_phase()` from `game_clock.rs`**

Remove the `DayPhase` enum (lines 60–67) and the `get_day_phase()` method (lines 236–245) from `src/game/game_clock.rs`.

- [ ] **Step 3: Update import in `debug_console.rs`**

Change:
```rust
use crate::game::game_clock::DayPhase;
```
To:
```rust
use crate::game::game_state::DayPhase;
```

- [ ] **Step 4: Update any other imports of `DayPhase`**

Search for `game_clock::DayPhase` in all `.rs` files and update to `game_state::DayPhase`. This likely includes `src/main.rs` if it imports `DayPhase` directly.

- [ ] **Step 5: Build and test**

Run: `cargo test --quiet`
Expected: all tests pass

- [ ] **Step 6: Commit**

```bash
git add src/game/game_state.rs src/game/game_clock.rs src/game/debug_console.rs src/main.rs
git commit -m "refactor(time): move DayPhase enum from GameClock to GameState

DayPhase now lives next to dayperiod, the field it describes.
GameState::get_day_phase() replaces GameClock::get_day_phase().

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 3: Wire debug display to read time from `GameState.daynight`

**Files:**
- Modify: `src/game/debug_console.rs:43-64` (add `daynight` and `lightlevel` to `DebugStatus`)
- Modify: `src/main.rs:453-476` (gameplay status from GameState)
- Modify: `src/main.rs:479-498` (intro status — use defaults since no GameState)
- Modify: `src/main.rs:520-538` (no-scene status — use defaults)

- [ ] **Step 1: Add `daynight` and `lightlevel` to `DebugStatus`**

In `src/game/debug_console.rs`, add fields to `DebugStatus`:

```rust
pub struct DebugStatus {
    pub fps: f64,
    pub game_day: u32,
    pub game_hour: u32,
    pub game_minute: u32,
    pub day_phase: DayPhase,
    pub daynight: u16,
    pub lightlevel: u16,
    pub game_ticks: u64,
    pub paused: bool,
    // ... rest unchanged
}
```

- [ ] **Step 2: Update gameplay `DebugStatus` construction in `main.rs`**

In `src/main.rs`, the gameplay branch (around line 453), replace the `GameClock`-derived time with `GameState`-derived time:

```rust
                let (gday, ghour, gminute) = gs.state.daynight_to_wall_clock();
                let status = DebugStatus {
                    fps: game_fps,
                    game_day: gday,
                    game_hour: ghour,
                    game_minute: gminute,
                    day_phase: gs.state.get_day_phase(),
                    daynight: gs.state.daynight,
                    lightlevel: gs.state.lightlevel,
                    game_ticks: clock.game_ticks,
                    paused: clock.paused,
                    // ... rest unchanged
                };
```

- [ ] **Step 3: Update intro/no-scene `DebugStatus` construction**

For the intro scene and no-scene branches, there's no `GameState`. Use zeros/defaults:

```rust
                    day_phase: DayPhase::default(),
                    daynight: 0,
                    lightlevel: 0,
```

These branches don't show gameplay time anyway.

- [ ] **Step 4: Build and test**

Run: `cargo test --quiet`
Expected: all tests pass

- [ ] **Step 5: Commit**

```bash
git add src/game/debug_console.rs src/main.rs
git commit -m "fix(time): debug display reads time from GameState.daynight

The debug status header now derives day/hour/minute from the
authoritative GameState.daynight counter instead of the disconnected
GameClock.game_ticks counter. This fixes the start time showing
00:00 instead of the correct 08:00.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 4: Fix `/time` command to recompute lightlevel and dayperiod

**Files:**
- Modify: `src/game/gameplay_scene.rs:1815-1826`

- [ ] **Step 1: Update `SetDayPhase` and `SetGameTime` handlers**

In `src/game/gameplay_scene.rs`, update both handlers to also recompute derived state:

```rust
            SetDayPhase { phase } => {
                self.state.daynight = phase;
                let raw = self.state.daynight / 40;
                self.state.lightlevel = if raw >= 300 { 600 - raw } else { raw };
                self.state.dayperiod = ((self.state.daynight / 6000) as u8).min(3);
            }
            SetGameTime { hour, minute } => {
                let ticks = (hour as u16).saturating_mul(1000)
                    + (minute as u16).saturating_mul(1000) / 60;
                self.state.daynight = ticks % 24000;
                let raw = self.state.daynight / 40;
                self.state.lightlevel = if raw >= 300 { 600 - raw } else { raw };
                self.state.dayperiod = ((self.state.daynight / 6000) as u8).min(3);
            }
```

- [ ] **Step 2: Build and test**

Run: `cargo test --quiet`
Expected: all tests pass

- [ ] **Step 3: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "fix(time): /time command recomputes lightlevel and dayperiod

SetDayPhase and SetGameTime now immediately update lightlevel and
dayperiod after setting daynight, preventing the one-frame-stale
display that made /time appear to revert.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 5: Remove dead `GameClock` wall-clock methods and orphaned key handlers

**Files:**
- Modify: `src/game/game_clock.rs` (remove wall-clock methods)
- Modify: `src/main.rs` (remove `A`/`Shift+A` handlers, remove `last_minute` logic)

- [ ] **Step 1: Remove unused wall-clock methods from `GameClock`**

In `src/game/game_clock.rs`, remove these methods (they are no longer called anywhere):
- `get_game_days()` (line ~183)
- `get_game_wall_clock()` (line ~190)
- `set_game_wall_clock()` (line ~202)
- `advance_game_wall_clock_to()` (line ~212)
- `advance_game_wall_clock_by()` (line ~226)

Also remove the constants that are only used by these methods if they're no longer referenced:
- `TICKS_PER_DAY` — check if used elsewhere first
- `TICKS_PER_HOUR` — check if used elsewhere first
- `TICKS_PER_MINUTE` — check if used elsewhere first

- [ ] **Step 2: Remove `A`/`Shift+A` key handlers from `main.rs`**

In `src/main.rs`, remove the `Scancode::A` match arm (lines ~272-279) that calls the now-deleted `advance_game_wall_clock_to/by` methods. These had no gameplay effect (only modified `GameClock`, not `GameState`).

- [ ] **Step 3: Remove `last_minute` dirty-check logic from `main.rs`**

In `src/main.rs`, remove the `last_minute` variable and the block at line ~324 that reads `clock.get_game_wall_clock()` to detect minute changes. This was driving debug redraws from the wrong time source.

- [ ] **Step 4: Build and test**

Run: `cargo test --quiet`
Expected: all tests pass, no compiler warnings about dead code

- [ ] **Step 5: Commit**

```bash
git add src/game/game_clock.rs src/main.rs
git commit -m "refactor(time): remove dead GameClock wall-clock methods

GameClock no longer tracks its own parallel wall-clock. The A/Shift+A
key handlers that only modified GameClock (with no gameplay effect)
are removed. The /time debug command is the proper way to manipulate
game time.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 6: Update `game_clock.rs` comment block and remove stale 60 Hz reference

**Files:**
- Modify: `src/game/game_clock.rs:108` (fix stale comment about 1/60 second)

- [ ] **Step 1: Fix stale comment**

The comment block in `game_clock.rs` around line 108 says:

```
    Assumption is that this happens every tick (1/60 second), so a full day is 24000 ticks,
    or 400 seconds (6 minutes 40 seconds) of real time. Each hour is 1000 ticks (16.67 seconds).
```

Update to:

```
    The game loop runs at 30 Hz (NTSC interlaced frame rate). A full day is 24000 ticks,
    or 800 seconds (13 minutes 20 seconds) of real time. Each hour is 1000 ticks (33.3 seconds).
```

- [ ] **Step 2: Remove stale constants if no longer used**

If `TICKS_PER_DAY`, `TICKS_PER_HOUR`, `TICKS_PER_MINUTE` are no longer referenced after Task 5, remove them. If they're still used (e.g., by the `last_minute` check removed in Task 5), they should already be gone.

- [ ] **Step 3: Build and test**

Run: `cargo test --quiet`
Expected: all tests pass

- [ ] **Step 4: Commit**

```bash
git add src/game/game_clock.rs
git commit -m "docs(time): fix stale 60 Hz comment in game_clock.rs

Updated timing math in comments to reflect the 30 Hz game tick rate.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 7: Add unit test for `daynight_to_wall_clock()`

**Files:**
- Modify: `src/game/game_state.rs` (add test)

- [ ] **Step 1: Write tests**

Add to the existing `#[cfg(test)] mod tests` block in `game_state.rs`:

```rust
    #[test]
    fn test_daynight_to_wall_clock_midnight() {
        let mut s = GameState::new();
        s.daynight = 0;
        s.game_days = 0;
        let (day, hour, minute) = s.daynight_to_wall_clock();
        assert_eq!((day, hour, minute), (0, 0, 0));
    }

    #[test]
    fn test_daynight_to_wall_clock_start() {
        let s = GameState::new();
        let (day, hour, minute) = s.daynight_to_wall_clock();
        assert_eq!((day, hour, minute), (0, 8, 0), "game starts at 08:00");
    }

    #[test]
    fn test_daynight_to_wall_clock_noon() {
        let mut s = GameState::new();
        s.daynight = 12000;
        let (day, hour, minute) = s.daynight_to_wall_clock();
        assert_eq!((day, hour, minute), (0, 12, 0));
    }

    #[test]
    fn test_daynight_to_wall_clock_2330() {
        let mut s = GameState::new();
        s.daynight = 23500;
        let (day, hour, minute) = s.daynight_to_wall_clock();
        assert_eq!((day, hour, minute), (0, 23, 30));
    }

    #[test]
    fn test_game_days_increments_on_wrap() {
        let mut s = GameState::new();
        s.daynight = 23999;
        s.game_days = 0;
        s.daynight_tick();
        assert_eq!(s.daynight, 0);
        assert_eq!(s.game_days, 1);
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test --quiet`
Expected: all 5 new tests pass

- [ ] **Step 3: Commit**

```bash
git add src/game/game_state.rs
git commit -m "test(time): add unit tests for daynight_to_wall_clock and game_days

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```
