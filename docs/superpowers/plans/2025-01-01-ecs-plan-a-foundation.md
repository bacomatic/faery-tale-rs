# ECS Migration Plan A: Foundation

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `hecs` dependency, promote `Direction` to a first-class field type everywhere, introduce `SetCourseMode` enum, and add rotation methods — leaving all 712 tests passing.

**Architecture:** This plan makes no gameplay changes. It is purely a type-safety refactor. `Direction` moves from a conversion utility into the canonical type for all facing/direction fields across `Actor`, `Npc`, `GameState`, and every function that takes or returns a direction. `SetCourseMode` replaces the `SC_*` u8 constants in `npc_ai.rs`. All existing tests must pass without modification; new tests verify the new methods.

**Tech Stack:** Rust 2021, `hecs = "0.11"`, existing `cargo test` suite (712 tests across 4 test binaries).

---

## File map

| File | Change |
|---|---|
| `Cargo.toml` | Add `hecs = "0.11"` dependency |
| `src/game/gameplay_scene/mod.rs` | Move `Direction` to `src/game/direction.rs`; add rotation methods; update `push_offset`, `facing_toward`, `compass_dir_for_facing` signatures |
| `src/game/direction.rs` | **Create** — canonical home for `Direction` enum + impl |
| `src/game/mod.rs` | `pub mod direction;` + re-export |
| `src/game/actor.rs` | `facing: u8` → `facing: Direction` |
| `src/game/npc.rs` | `facing: u8` → `facing: Direction`; update `from_bytes` |
| `src/game/game_state.rs` | `facing: u8` → `facing: Direction`; update default |
| `src/game/npc_ai.rs` | Replace `SC_*` consts with `SetCourseMode` enum; update `set_course` signature; replace `COM2 [[u8;3];3]` with `[[Direction;3];3]`; replace `& 7` arithmetic with `Direction::rotate_by` |
| `src/game/combat.rs` | `fire_missile` `dir: u8` → `dir: Direction`; `weapon_tip` `facing: u8` → `facing: Direction`; `Missile::facing()` returns `Direction` |
| `src/game/collision.rs` | `newx`/`newy` `dir: u8` → `dir: Direction` |
| `src/game/hiscreen.rs` | `facing_char(dir: u8)` → `facing_char(dir: Direction)` |
| `src/game/gameplay_scene/rendering.rs` | `facing_to_frame_base`/`facing_to_fight_frame_base` `facing: u8` → `facing: Direction` |
| `src/game/gameplay_scene/combat_logic.rs` | All `facing: u8` params → `Direction` |
| `src/game/gameplay_scene/input.rs` | `dir as u8` cast → use `Direction` directly |
| `src/game/gameplay_scene/actors.rs` | `npc.facing = 4` → `npc.facing = Direction::S` |
| `src/game/gameplay_scene/carriers.rs` | `facing.wrapping_add(off as u8) & 7` → `facing.rotate_by(off)` |
| `src/game/magic.rs` | `state.facing = 0` → `state.facing = Direction::N` |
| `src/game/encounter.rs` | `facing: 0` → `facing: Direction::N`; `facing: 4` → `facing: Direction::S` |
| `src/game/debug_tui/bridge.rs` | `hero_facing: u8` and `ActorSnapshot.facing: u8` stay `u8` — explicit `as u8` cast at snapshot boundary |
| `src/game/gameplay_scene/tests.rs` | Update raw literal assertions to use `Direction::X as u8` or compare `Direction` directly |
| `src/game/npc_ai.rs` (tests) | Update `npc.facing == 3` → `npc.facing == Direction::E` etc. |

---

## Task 1: Add hecs dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add dependency**

Open `Cargo.toml` and add to `[dependencies]`:
```toml
hecs = "0.11"
```

- [ ] **Step 2: Verify it compiles**

```bash
cd /home/ddehaven/projects/faery-tale-rs
cargo build 2>&1 | grep -E "^error|Compiling hecs"
```
Expected: `Compiling hecs v0.11.x` appears, no errors.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add hecs 0.11 dependency"
```

---

## Task 2: Create `src/game/direction.rs` with rotation methods

**Files:**
- Create: `src/game/direction.rs`
- Modify: `src/game/mod.rs`
- Modify: `src/game/gameplay_scene/mod.rs` (remove the `Direction` definition, keep the re-export)

- [ ] **Step 1: Write failing tests for rotation methods**

Create `src/game/direction.rs` with tests only (no impl yet):

```rust
//! Canonical direction type for all facing/heading values.
//! Amiga DIR_* encoding: NW=0, N=1, NE=2, E=3, SE=4, S=5, SW=6, W=7.
//! Value 9 is the DIRECTION_STILL sentinel (Direction::None).

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Direction {
    NW   = 0,
    #[default]
    N    = 1,
    NE   = 2,
    E    = 3,
    SE   = 4,
    S    = 5,
    SW   = 6,
    W    = 7,
    None = 9,
}

impl From<u8> for Direction {
    fn from(v: u8) -> Self {
        match v {
            0 => Direction::NW,
            1 => Direction::N,
            2 => Direction::NE,
            3 => Direction::E,
            4 => Direction::SE,
            5 => Direction::S,
            6 => Direction::SW,
            7 => Direction::W,
            _ => Direction::None,
        }
    }
}

impl Direction {
    /// Rotate clockwise by `steps` (positive = CW, negative = CCW).
    /// `Direction::None` is returned unchanged.
    pub fn rotate_by(self, steps: i8) -> Direction {
        if self == Direction::None {
            return Direction::None;
        }
        Direction::from(((self as i8).wrapping_add(steps)).rem_euclid(8) as u8)
    }

    /// One step clockwise.
    pub fn rotate_cw(self) -> Direction {
        self.rotate_by(1)
    }

    /// One step counter-clockwise.
    pub fn rotate_ccw(self) -> Direction {
        self.rotate_by(-1)
    }

    /// Opposite direction (180°).
    pub fn opposite(self) -> Direction {
        self.rotate_by(4)
    }

    /// True if this is a cardinal (N/S/E/W).
    pub fn is_cardinal(self) -> bool {
        matches!(self, Direction::N | Direction::S | Direction::E | Direction::W)
    }

    /// True if this is a diagonal (NW/NE/SW/SE).
    pub fn is_diagonal(self) -> bool {
        matches!(self, Direction::NW | Direction::NE | Direction::SW | Direction::SE)
    }
}

#[cfg(test)]
mod tests {
    use super::Direction;

    #[test]
    fn amiga_discriminants() {
        assert_eq!(Direction::NW as u8, 0);
        assert_eq!(Direction::N  as u8, 1);
        assert_eq!(Direction::NE as u8, 2);
        assert_eq!(Direction::E  as u8, 3);
        assert_eq!(Direction::SE as u8, 4);
        assert_eq!(Direction::S  as u8, 5);
        assert_eq!(Direction::SW as u8, 6);
        assert_eq!(Direction::W  as u8, 7);
        assert_eq!(Direction::None as u8, 9);
    }

    #[test]
    fn from_u8_all_values() {
        assert_eq!(Direction::from(0u8), Direction::NW);
        assert_eq!(Direction::from(1u8), Direction::N);
        assert_eq!(Direction::from(2u8), Direction::NE);
        assert_eq!(Direction::from(3u8), Direction::E);
        assert_eq!(Direction::from(4u8), Direction::SE);
        assert_eq!(Direction::from(5u8), Direction::S);
        assert_eq!(Direction::from(6u8), Direction::SW);
        assert_eq!(Direction::from(7u8), Direction::W);
        assert_eq!(Direction::from(8u8), Direction::None);
        assert_eq!(Direction::from(9u8), Direction::None);
        assert_eq!(Direction::from(255u8), Direction::None);
    }

    #[test]
    fn rotate_cw_full_circle() {
        let dirs = [
            Direction::NW, Direction::N, Direction::NE, Direction::E,
            Direction::SE, Direction::S, Direction::SW, Direction::W,
        ];
        for i in 0..8 {
            assert_eq!(dirs[i].rotate_cw(), dirs[(i + 1) % 8]);
        }
    }

    #[test]
    fn rotate_ccw_full_circle() {
        let dirs = [
            Direction::NW, Direction::N, Direction::NE, Direction::E,
            Direction::SE, Direction::S, Direction::SW, Direction::W,
        ];
        for i in 0..8 {
            assert_eq!(dirs[i].rotate_ccw(), dirs[(i + 7) % 8]);
        }
    }

    #[test]
    fn opposite() {
        assert_eq!(Direction::N.opposite(),  Direction::S);
        assert_eq!(Direction::S.opposite(),  Direction::N);
        assert_eq!(Direction::E.opposite(),  Direction::W);
        assert_eq!(Direction::W.opposite(),  Direction::E);
        assert_eq!(Direction::NE.opposite(), Direction::SW);
        assert_eq!(Direction::SW.opposite(), Direction::NE);
        assert_eq!(Direction::NW.opposite(), Direction::SE);
        assert_eq!(Direction::SE.opposite(), Direction::NW);
    }

    #[test]
    fn none_rotate_returns_none() {
        assert_eq!(Direction::None.rotate_cw(),   Direction::None);
        assert_eq!(Direction::None.rotate_ccw(),  Direction::None);
        assert_eq!(Direction::None.rotate_by(3),  Direction::None);
        assert_eq!(Direction::None.opposite(),    Direction::None);
    }

    #[test]
    fn rotate_by_2() {
        assert_eq!(Direction::N.rotate_by(2),  Direction::NE);
        assert_eq!(Direction::N.rotate_by(-2), Direction::NW);
        assert_eq!(Direction::W.rotate_by(2),  Direction::N);
    }

    #[test]
    fn is_cardinal_and_diagonal() {
        assert!(Direction::N.is_cardinal());
        assert!(Direction::S.is_cardinal());
        assert!(Direction::E.is_cardinal());
        assert!(Direction::W.is_cardinal());
        assert!(!Direction::NE.is_cardinal());

        assert!(Direction::NE.is_diagonal());
        assert!(Direction::NW.is_diagonal());
        assert!(Direction::SE.is_diagonal());
        assert!(Direction::SW.is_diagonal());
        assert!(!Direction::N.is_diagonal());
    }
}
```

- [ ] **Step 2: Add module to `src/game/mod.rs`**

Open `src/game/mod.rs` and add:
```rust
pub mod direction;
pub use direction::Direction;
```

- [ ] **Step 3: Run tests to verify they pass**

```bash
cargo test direction 2>&1 | grep -E "test.*direction|FAILED|ok\."
```
Expected: all direction tests pass.

- [ ] **Step 4: Remove `Direction` definition from `gameplay_scene/mod.rs`, re-export from there**

In `src/game/gameplay_scene/mod.rs`, find the `Direction` enum definition (lines ~265–293) and the `From<u8>` impl. Replace the full definition with a re-export:

```rust
pub use crate::game::direction::Direction;
```

- [ ] **Step 5: Verify full test suite still passes**

```bash
cargo test 2>&1 | grep -E "^test result"
```
Expected: 4 lines, all `ok`.

- [ ] **Step 6: Commit**

```bash
git add src/game/direction.rs src/game/mod.rs src/game/gameplay_scene/mod.rs
git commit -m "refactor: extract Direction enum to game::direction with rotation methods"
```

---

## Task 3: Replace `SC_*` constants with `SetCourseMode` enum

**Files:**
- Modify: `src/game/npc_ai.rs`

- [ ] **Step 1: Add `SetCourseMode` enum above the constants**

In `src/game/npc_ai.rs`, find the block starting at line 114 (`/// set_course modes`). Replace the seven `pub const SC_*: u8` lines with:

```rust
/// set_course modes (from fmain2.c).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetCourseMode {
    /// Smart seek — suppress minor axis when one is 2× the other.
    Smart,
    /// Smart + ±1 facing deviation when distance < 40.
    Deviate1,
    /// Smart + ±2 facing deviation when distance < 30.
    Deviate2,
    /// Flee — negate direction before lookup.
    Flee,
    /// Bumble — skip axis suppression, allow true diagonals.
    Bumble,
    /// Aim only — set facing but do NOT set state to Walking.
    Aim,
    /// Direct — target_x/y are raw deltas, not world positions.
    Direct,
}
```

- [ ] **Step 2: Update `set_course` signature and body**

Replace the `set_course` function signature and all `mode == SC_*` / `mode <= SC_FLEE` comparisons:

```rust
pub fn set_course(npc: &mut Npc, target_x: i32, target_y: i32, mode: SetCourseMode) {
    use SetCourseMode::*;

    let (dx, dy) = if mode == Direct {
        (target_x, target_y)
    } else {
        (target_x - npc.x as i32, target_y - npc.y as i32)
    };

    if dx == 0 && dy == 0 {
        npc.state = NpcState::Still;
        return;
    }

    let (dx, dy) = if mode == Flee { (-dx, -dy) } else { (dx, dy) };

    let adx = dx.abs();
    let ady = dy.abs();

    // Axis suppression: Smart/Deviate1/Deviate2/Flee suppress minor axis.
    // Bumble/Aim/Direct skip this.
    let (eff_dx, eff_dy) = if matches!(mode, Smart | Deviate1 | Deviate2 | Flee) {
        let sx = if ady > adx * 2 { 0 } else { dx };
        let sy = if adx > ady * 2 { 0 } else { dy };
        (sx, sy)
    } else {
        (dx, dy)
    };

    // com2 lookup: map (xsign, ysign) → Direction.
    const COM2: [[Direction; 3]; 3] = [
        [Direction::NW, Direction::N,    Direction::NE],  // eff_dy < 0
        [Direction::W,  Direction::None, Direction::E ],  // eff_dy = 0
        [Direction::SW, Direction::S,    Direction::SE],  // eff_dy > 0
    ];
    let xi = (eff_dx.signum() + 1) as usize;
    let yi = (eff_dy.signum() + 1) as usize;
    let mut facing = COM2[yi][xi];

    if facing == Direction::None {
        npc.state = NpcState::Still;
        return;
    }

    // Deviation: Deviate1/Deviate2 add random ±N when close to target.
    if mode == Deviate1 && (adx + ady) < 40 {
        let coin = (npc.x.wrapping_mul(7).wrapping_add(npc.y)) & 1;
        facing = if coin == 0 { facing.rotate_cw() } else { facing.rotate_ccw() };
    } else if mode == Deviate2 && (adx + ady) < 30 {
        let coin = (npc.x.wrapping_mul(13).wrapping_add(npc.y)) & 1;
        facing = if coin == 0 { facing.rotate_by(2) } else { facing.rotate_by(-2) };
    }

    npc.facing = facing;
    if mode != Aim {
        npc.state = NpcState::Walking;
    }
}
```

- [ ] **Step 3: Update all `set_course` call sites in `npc_ai.rs`**

Replace every `SC_*` argument with `SetCourseMode::*`. Find all calls with:
```bash
grep -n "set_course" src/game/npc_ai.rs
```

Each `set_course(npc, x, y, SC_SMART)` becomes `set_course(npc, x, y, SetCourseMode::Smart)`, etc.:
- `SC_SMART`    → `SetCourseMode::Smart`
- `SC_DEVIATE1` → `SetCourseMode::Deviate1`
- `SC_DEVIATE2` → `SetCourseMode::Deviate2`
- `SC_FLEE`     → `SetCourseMode::Flee`
- `SC_BUMBLE`   → `SetCourseMode::Bumble`
- `SC_AIM`      → `SetCourseMode::Aim`
- `SC_DIRECT`   → `SetCourseMode::Direct`

- [ ] **Step 4: Fix the `Direction` import in `npc_ai.rs`**

At the top of `npc_ai.rs`, add or update the import so `Direction` and `SetCourseMode` are in scope:
```rust
use crate::game::direction::Direction;
```
(The `COM2` table inside `set_course` uses `Direction` directly.)

- [ ] **Step 5: Run tests**

```bash
cargo test 2>&1 | grep -E "^test result|^error"
```
Expected: 4 `ok` lines, no errors.

- [ ] **Step 6: Commit**

```bash
git add src/game/npc_ai.rs
git commit -m "refactor: replace SC_* u8 constants with SetCourseMode enum in npc_ai"
```

---

## Task 4: Promote `Direction` to field type in `Actor`, `Npc`, `GameState`

**Files:**
- Modify: `src/game/actor.rs`
- Modify: `src/game/npc.rs`
- Modify: `src/game/game_state.rs`

- [ ] **Step 1: Update `Actor.facing`**

In `src/game/actor.rs`, change:
```rust
pub facing: u8, // 0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW
```
to:
```rust
pub facing: Direction,
```

Add the import at the top of the file:
```rust
use crate::game::direction::Direction;
```

- [ ] **Step 2: Update `Npc.facing`**

In `src/game/npc.rs`, change:
```rust
pub facing: u8,
```
to:
```rust
pub facing: Direction,
```

Add import:
```rust
use crate::game::direction::Direction;
```

Update `Npc::from_bytes` where it sets `facing: 0` — change to `facing: Direction::N`.

Update `Npc::default()` if manually implemented (or derive `Default` relying on `Direction`'s `#[default]` of `N`).

- [ ] **Step 3: Update `GameState.facing`**

In `src/game/game_state.rs`, change:
```rust
pub facing: u8,
```
to:
```rust
pub facing: Direction,
```

Add import:
```rust
use crate::game::direction::Direction;
```

Find the `GameState` default/new initializer and change `facing: 0` to `facing: Direction::N`.

- [ ] **Step 4: Run cargo check to find all broken call sites**

```bash
cargo check 2>&1 | grep "^error" | head -40
```

This will list every file that assigned a `u8` to a `Direction` field. Work through each one in the next steps.

- [ ] **Step 5: Fix all broken call sites**

For each error, apply the appropriate fix:

**Pattern: `npc.facing = 4`** → `npc.facing = Direction::SE` (or whichever direction the comment says)

Key literal → Direction mappings (Amiga encoding):
- `0` → `Direction::NW`
- `1` → `Direction::N`
- `2` → `Direction::NE`
- `3` → `Direction::E`
- `4` → `Direction::SE`
- `5` → `Direction::S`
- `6` → `Direction::SW`
- `7` → `Direction::W`

Files to fix and their patterns (from the audit):

`src/game/gameplay_scene/actors.rs` line ~189:
```rust
// Before:
npc.facing = 4;
// After:
npc.facing = Direction::SE; // Dragon always faces south (SPEC §21.5)
```

`src/game/magic.rs` line ~521:
```rust
// Before:
state.facing = 0;
// After:
state.facing = Direction::NW;
```

`src/game/encounter.rs` line ~287:
```rust
// Before:
facing: 0,
// After:
facing: Direction::N,
```

`src/game/encounter.rs` line ~417:
```rust
// Before:
facing: 4,
// After:
facing: Direction::SE,
```

`src/game/gameplay_scene/input.rs` (the `dir as u8` cast):
```rust
// Before:
let facing = if dir != Direction::None { dir as u8 } else { self.state.facing };
self.state.facing = facing;
// After:
let facing = if dir != Direction::None { dir } else { self.state.facing };
self.state.facing = facing;
```

`src/game/npc_ai.rs` random facing lines:
```rust
// Before:
npc.facing = ((r >> 4) & 7) as u8;
// After:
npc.facing = Direction::from(((r >> 4) & 7) as u8);
```

`src/game/gameplay_scene/actors.rs` sync loop (line ~147):
```rust
// Before:
actor.facing = npc.facing;
// After — already same type now, no cast needed:
actor.facing = npc.facing;
```

- [ ] **Step 6: Run full test suite**

```bash
cargo test 2>&1 | grep -E "^test result|^error"
```
Expected: 4 `ok` lines, no errors.

- [ ] **Step 7: Commit**

```bash
git add src/game/actor.rs src/game/npc.rs src/game/game_state.rs \
        src/game/gameplay_scene/actors.rs src/game/gameplay_scene/input.rs \
        src/game/magic.rs src/game/encounter.rs src/game/npc_ai.rs
git commit -m "refactor: promote Direction to field type in Actor, Npc, GameState"
```

---

## Task 5: Update all function signatures to use `Direction`

**Files:**
- Modify: `src/game/gameplay_scene/mod.rs` (`push_offset`, `facing_toward`, `compass_dir_for_facing`)
- Modify: `src/game/combat.rs` (`fire_missile dir:`, `weapon_tip facing:`, `Missile::facing()`)
- Modify: `src/game/collision.rs` (`newx dir:`, `newy dir:`)
- Modify: `src/game/hiscreen.rs` (`facing_char dir:`)
- Modify: `src/game/gameplay_scene/rendering.rs` (`facing_to_frame_base`, `facing_to_fight_frame_base`)
- Modify: `src/game/gameplay_scene/combat_logic.rs` (all `facing: u8` params)

- [ ] **Step 1: Update `push_offset` in `gameplay_scene/mod.rs`**

```rust
// Before:
fn push_offset(facing: u8, distance: i32) -> (i32, i32) {
    match Direction::from(facing) {
// After:
fn push_offset(facing: Direction, distance: i32) -> (i32, i32) {
    match facing {
```

- [ ] **Step 2: Update `facing_toward` return type**

```rust
// Before:
fn facing_toward(sx: i32, sy: i32, tx: i32, ty: i32) -> u8 {
    ...
    if dy > 0 { Direction::S as u8 } else { Direction::N as u8 }
    ...
}
// After:
fn facing_toward(sx: i32, sy: i32, tx: i32, ty: i32) -> Direction {
    ...
    if dy > 0 { Direction::S } else { Direction::N }
    ...
    // Replace all `Direction::X as u8` with `Direction::X`
}
```

- [ ] **Step 3: Update `compass_dir_for_facing`**

```rust
// Before:
fn compass_dir_for_facing(facing: u8) -> usize {
// After:
fn compass_dir_for_facing(facing: Direction) -> usize {
```
The body uses `facing` as a `usize` index. Change:
```rust
// Before:
facing as usize
// After:
facing as u8 as usize
```
(Only `as u8` cast is at this one display boundary — acceptable.)

- [ ] **Step 4: Update `fire_missile` in `combat.rs`**

```rust
// Before:
pub fn fire_missile(
    missiles: &mut [Missile; MAX_MISSILES],
    x: i32, y: i32,
    dir: u8,
    weapon: u8,
    is_friendly: bool,
    speed: i32,
) -> Option<usize>
// After:
pub fn fire_missile(
    missiles: &mut [Missile; MAX_MISSILES],
    x: i32, y: i32,
    dir: Direction,
    weapon: u8,
    is_friendly: bool,
    speed: i32,
) -> Option<usize>
```
Inside the body, `dir` is used to compute `dx`/`dy` via `push_offset(dir, speed)` — now type-matches directly.

- [ ] **Step 5: Update `weapon_tip` in `combat.rs`**

```rust
// Before:
pub fn weapon_tip(abs_x: i32, abs_y: i32, facing: u8, wt: i16) -> (i32, i32)
// After:
pub fn weapon_tip(abs_x: i32, abs_y: i32, facing: Direction, wt: i16) -> (i32, i32)
```

- [ ] **Step 6: Update `Missile::facing()` return type**

```rust
// Before:
pub fn facing(&self) -> u8 {
    match (self.dx.signum(), self.dy.signum()) {
        (-1, -1) => Direction::NW as u8,
        ...
    }
}
// After:
pub fn facing(&self) -> Direction {
    match (self.dx.signum(), self.dy.signum()) {
        (-1, -1) => Direction::NW,
        (0,  -1) => Direction::N,
        (1,  -1) => Direction::NE,
        (1,   0) => Direction::E,
        (1,   1) => Direction::SE,
        (0,   1) => Direction::S,
        (-1,  1) => Direction::SW,
        (-1,  0) => Direction::W,
        _        => Direction::None,
    }
}
```

- [ ] **Step 7: Update `newx` and `newy` in `collision.rs`**

```rust
// Before:
pub fn newx(x: u16, dir: u8, dist: i32) -> u16
pub fn newy(y: u16, dir: u8, dist: i32) -> u16
// After:
pub fn newx(x: u16, dir: Direction, dist: i32) -> u16
pub fn newy(y: u16, dir: Direction, dist: i32) -> u16
```
Add import: `use crate::game::direction::Direction;`

Inside the body, `dir` is used via `push_offset(dir, dist)` — already typed.

- [ ] **Step 8: Update `facing_char` in `hiscreen.rs`**

```rust
// Before:
pub fn facing_char(dir: u8) -> char {
// After:
pub fn facing_char(dir: Direction) -> char {
```
Add import. The body uses `dir` in a match — update match arms to `Direction::N => ...` etc.

- [ ] **Step 9: Update `facing_to_frame_base` and `facing_to_fight_frame_base` in `rendering.rs`**

```rust
// Before:
pub(super) fn facing_to_frame_base(facing: u8) -> usize
pub(super) fn facing_to_fight_frame_base(facing: u8) -> usize
// After:
pub(super) fn facing_to_frame_base(facing: Direction) -> usize
pub(super) fn facing_to_fight_frame_base(facing: Direction) -> usize
```
The bodies use `facing` in a match. Update the match arms to use `Direction::X`.

- [ ] **Step 10: Update `combat_logic.rs` facing params**

```bash
grep -n "facing: u8" src/game/gameplay_scene/combat_logic.rs
```
Update each found signature from `facing: u8` to `facing: Direction`. Update any `Direction::from(facing)` call sites inside to just use `facing` directly.

- [ ] **Step 11: Update `carriers.rs` rotation**

Find the line:
```rust
let probe_dir = facing.wrapping_add(off as u8) & 7;
```
Replace with:
```rust
let probe_dir = facing.rotate_by(off as i8);
```
where `facing` is now `Direction`. Update the `probe_dir` type to `Direction` and update its downstream uses.

- [ ] **Step 12: Run cargo check, fix remaining call sites**

```bash
cargo check 2>&1 | grep "^error" | head -40
```
Fix any remaining `u8` → `Direction` mismatches found. These are typically places that call `facing_toward()` and then pass the result directly to `fire_missile()` or `weapon_tip()` — they now flow through without casts.

- [ ] **Step 13: Run full test suite**

```bash
cargo test 2>&1 | grep -E "^test result|^error"
```
Expected: 4 `ok` lines, no errors.

- [ ] **Step 14: Commit**

```bash
git add src/game/gameplay_scene/mod.rs src/game/combat.rs src/game/collision.rs \
        src/game/hiscreen.rs src/game/gameplay_scene/rendering.rs \
        src/game/gameplay_scene/combat_logic.rs src/game/gameplay_scene/carriers.rs
git commit -m "refactor: all facing/direction function signatures use Direction type"
```

---

## Task 6: Fix `debug_tui/bridge.rs` snapshot boundary and update tests

**Files:**
- Modify: `src/game/debug_tui/bridge.rs`
- Modify: `src/game/gameplay_scene/tests.rs`
- Modify: `src/game/npc_ai.rs` (test section)
- Modify: `src/game/npc.rs` (test fixtures)

- [ ] **Step 1: Fix bridge.rs snapshot boundary**

In `src/game/debug_tui/bridge.rs`, the `ActorSnapshot` fields `facing: u8` and `hero_facing: u8` stay as `u8` — they are display values for the TUI, not game logic. The conversion happens explicitly at the snapshot-building boundary.

Find `ActorSnapshot::from_actor` and `from_npc` — update to cast explicitly:
```rust
// from_actor:
facing: a.facing as u8,

// from_npc:
facing: n.facing as u8,
```

Find `facing_name(facing: u8)` — no change needed (already takes u8).

Find any place `hero_facing` is set in the snapshot builder in `main.rs`:
```rust
// Before (if present):
hero_facing: gs.state.facing,
// After:
hero_facing: gs.state.facing as u8,
```

- [ ] **Step 2: Update test assertions in `npc_ai.rs` tests**

```bash
grep -n "npc\.facing ==" src/game/npc_ai.rs | head -20
```

Replace numeric assertions with `Direction` comparisons:
```rust
// Before:
assert_eq!(npc.facing, 3); // DIR_E
// After:
assert_eq!(npc.facing, Direction::E);

// Before:
assert_eq!(npc.facing, 1); // DIR_N
// After:
assert_eq!(npc.facing, Direction::N);
```

- [ ] **Step 3: Update test fixture initializers in `npc.rs` and `gameplay_scene/tests.rs`**

```bash
grep -n "facing: [0-9]" src/game/npc.rs src/game/gameplay_scene/tests.rs src/game/npc_ai.rs
```

Replace each:
```rust
// Before:
facing: 2, // East
// After:
facing: Direction::NE,

// Before:
facing: 0,
// After:
facing: Direction::NW,

// Before:
facing: 4,
// After:
facing: Direction::SE,
```

Use the Amiga encoding: NW=0, N=1, NE=2, E=3, SE=4, S=5, SW=6, W=7.

- [ ] **Step 4: Run full test suite**

```bash
cargo test 2>&1 | grep -E "^test result|^error"
```
Expected: 4 `ok` lines. Test count should be ≥ 712 (new direction tests added).

- [ ] **Step 5: Final commit**

```bash
git add -A
git commit -m "refactor: fix snapshot boundary casts and update test fixtures to use Direction"
```

---

## Completion check

All 712+ tests pass. The codebase now has:
- Zero `facing: u8` fields in game structs (only in the debug TUI snapshot boundary)
- Zero `& 7` direction masks outside `direction.rs`
- Zero `Direction::X as u8` casts outside `direction.rs` and the serialization/TUI boundary
- `SetCourseMode` enum in `npc_ai.rs` with exhaustive match
- `hecs` in `Cargo.toml` (unused until Plan B)

Run the full suite one final time:
```bash
cargo test 2>&1 | grep "^test result"
```
