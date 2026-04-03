# NPC Goal/Tactic AI System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the original game's AI decision system (do_tactic, set_course, tactic selection) so NPCs behave organically — re-aiming probabilistically, using 10+ tactical behaviors — instead of laser-tracking the hero every frame.

**Architecture:** Add AI fields (goal, tactic, facing, state, cleverness) to the `Npc` struct. Create a new `npc_ai.rs` module with three pure functions (`set_course`, `do_tactic`, `select_tactic`) ported from fmain.c/fmain2.c. Rewire `update_actors()` to run AI then movement. Keep Actor array for rendering, synced from NPC positions each frame.

**Tech Stack:** Rust, SDL2. Existing `collision` module helpers (`proxcheck`, `newx`, `newy`, `calc_dist`). Goal/Tactic enums from `actor.rs`.

**Spec:** `docs/superpowers/specs/2026-04-02-npc-goal-tactic-ai-design.md`

---

## File Map

| File | Action | Responsibility |
|------|--------|---------------|
| `src/game/npc.rs` | Modify | Add goal/tactic/facing/state/cleverness fields, NpcState enum; refactor `tick()` to use stored facing |
| `src/game/npc_ai.rs` | Create | `set_course()`, `do_tactic()`, `select_tactic()` — pure AI functions |
| `src/game/encounter.rs` | Modify | Add cleverness to encounter chart; initialize AI fields on spawn |
| `src/game/gameplay_scene.rs` | Modify | Rewrite `update_actors()` AI pipeline; sync NPC→Actor; battleflag |
| `src/game/mod.rs` | Modify | Add `pub mod npc_ai;` |
| `src/game/actor.rs` | Read-only | Goal/Tactic enums reused |
| `src/game/collision.rs` | Read-only | `proxcheck`, `newx`, `newy`, `calc_dist` reused |
| `src/game/game_state.rs` | Read-only | `battleflag`, `hero_x`, `hero_y`, `xtype`, `actors`, `anix` |

---

## Task 1: Add NpcState enum and AI fields to Npc struct

**Files:**
- Modify: `src/game/npc.rs` (struct at line 62, `from_bytes` at line 78, `Default`)

- [ ] **Step 1: Write failing test for NpcState and new Npc fields**

Add to `src/game/npc.rs` test module (after the existing tests):

```rust
#[test]
fn test_npc_ai_fields_default() {
    let npc = Npc::default();
    assert_eq!(npc.goal, Goal::None);
    assert_eq!(npc.tactic, Tactic::None);
    assert_eq!(npc.facing, 0);
    assert_eq!(npc.state, NpcState::Still);
    assert_eq!(npc.cleverness, 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test test_npc_ai_fields_default`
Expected: FAIL — `NpcState`, `Goal`, `Tactic` not found on `Npc`, fields don't exist.

- [ ] **Step 3: Add NpcState enum and AI fields to Npc**

At the top of `src/game/npc.rs`, add the import after the existing `use` statement:

```rust
use crate::game::actor::{Goal, Tactic};
```

Before the `Npc` struct definition, add:

```rust
/// Lightweight NPC state for AI decisions (distinct from ActorState which carries animation data).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum NpcState {
    #[default]
    Still,
    Walking,
    Fighting,
    Shooting,
    Dying,
    Dead,
    Sinking,
}
```

Add five fields to the `Npc` struct (after the existing `active` field):

```rust
pub struct Npc {
    pub npc_type: u8,
    pub race: u8,
    pub x: i16,
    pub y: i16,
    pub vitality: i16,
    pub gold: i16,
    pub speed: u8,
    pub weapon: u8,
    pub active: bool,
    pub goal: Goal,
    pub tactic: Tactic,
    pub facing: u8,
    pub state: NpcState,
    pub cleverness: u8,
}
```

Update `from_bytes()` to initialize the new fields with defaults:

```rust
    pub fn from_bytes(data: &[u8]) -> Self {
        if data.len() < 16 {
            return Npc::default();
        }
        Npc {
            npc_type: data[0],
            race: data[1],
            x: i16::from_be_bytes([data[2], data[3]]),
            y: i16::from_be_bytes([data[4], data[5]]),
            vitality: i16::from_be_bytes([data[6], data[7]]),
            gold: i16::from_be_bytes([data[8], data[9]]),
            speed: data[10],
            weapon: data[11],
            active: data[0] != NPC_TYPE_NONE,
            goal: Goal::None,
            tactic: Tactic::None,
            facing: 0,
            state: NpcState::Still,
            cleverness: 0,
        }
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test test_npc_ai_fields_default`
Expected: PASS

- [ ] **Step 5: Run full test suite**

Run: `cargo test`
Expected: All tests pass. Existing tests that construct `Npc` with `..Default::default()` will pick up the new fields automatically.

- [ ] **Step 6: Commit**

```bash
git add src/game/npc.rs
git commit -m "feat(npc): add NpcState enum and AI fields (goal, tactic, facing, state, cleverness)

Add Goal, Tactic, facing, NpcState, and cleverness fields to the Npc struct
in preparation for porting the original game's do_tactic/set_course AI system.
NpcState is a lightweight enum distinct from ActorState (which carries animation data)."
```

---

## Task 2: Create npc_ai.rs with `set_course()`

**Files:**
- Create: `src/game/npc_ai.rs`
- Modify: `src/game/mod.rs` (line 50, add `pub mod npc_ai;`)

- [ ] **Step 1: Register the module**

In `src/game/mod.rs`, add after the `pub mod npc;` line (currently line 50):

```rust
pub mod npc_ai;
```

- [ ] **Step 2: Write failing tests for set_course**

Create `src/game/npc_ai.rs` with tests only:

```rust
//! NPC AI decision system — ports do_tactic/set_course/select_tactic from fmain.c/fmain2.c.

use crate::game::npc::{Npc, NpcState};

/// set_course modes (from fmain2.c).
pub const SC_SMART: u8 = 0;    // Smart seek — suppress minor axis
pub const SC_DEVIATE1: u8 = 1; // Smart + ±1 deviation when close
pub const SC_DEVIATE2: u8 = 2; // Smart + ±2 deviation when close
pub const SC_FLEE: u8 = 3;     // Flee — negate direction
pub const SC_BUMBLE: u8 = 4;   // Bumble — skip axis suppression
pub const SC_AIM: u8 = 5;      // Aim only — set facing, don't walk
pub const SC_DIRECT: u8 = 6;   // Direct — target is raw delta, not position

/// Compute facing and state from position delta.
/// Ports set_course from fmain2.c.
pub fn set_course(npc: &mut Npc, target_x: i32, target_y: i32, mode: u8) {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::actor::{Goal, Tactic};
    use crate::game::npc::RACE_ENEMY;

    fn make_npc(x: i16, y: i16) -> Npc {
        Npc {
            npc_type: 6,
            race: RACE_ENEMY,
            x,
            y,
            vitality: 10,
            active: true,
            ..Default::default()
        }
    }

    #[test]
    fn test_set_course_smart_east() {
        let mut npc = make_npc(100, 100);
        set_course(&mut npc, 200, 100, SC_SMART);
        assert_eq!(npc.facing, 2); // East
        assert_eq!(npc.state, NpcState::Walking);
    }

    #[test]
    fn test_set_course_smart_north() {
        let mut npc = make_npc(100, 100);
        set_course(&mut npc, 100, 50, SC_SMART);
        assert_eq!(npc.facing, 0); // North
        assert_eq!(npc.state, NpcState::Walking);
    }

    #[test]
    fn test_set_course_smart_axis_suppression() {
        // Target far east, slightly south — should suppress Y axis → pure East.
        let mut npc = make_npc(100, 100);
        set_course(&mut npc, 300, 110, SC_SMART);
        assert_eq!(npc.facing, 2); // East (Y suppressed)
    }

    #[test]
    fn test_set_course_smart_diagonal() {
        // Target equally far NE — should get NE (facing 1).
        let mut npc = make_npc(100, 100);
        set_course(&mut npc, 200, 0, SC_SMART);
        assert_eq!(npc.facing, 1); // NE
    }

    #[test]
    fn test_set_course_flee_reversal() {
        // Flee mode: target east → NPC should face west.
        let mut npc = make_npc(100, 100);
        set_course(&mut npc, 200, 100, SC_FLEE);
        assert_eq!(npc.facing, 6); // West (flee from east)
        assert_eq!(npc.state, NpcState::Walking);
    }

    #[test]
    fn test_set_course_aim_only() {
        // Aim mode: set facing but don't change state to Walking.
        let mut npc = make_npc(100, 100);
        npc.state = NpcState::Still;
        set_course(&mut npc, 200, 100, SC_AIM);
        assert_eq!(npc.facing, 2); // East
        assert_eq!(npc.state, NpcState::Still); // NOT Walking
    }

    #[test]
    fn test_set_course_direct_mode() {
        // Direct mode: target_x/y are raw deltas, not world positions.
        let mut npc = make_npc(100, 100);
        set_course(&mut npc, 1, 0, SC_DIRECT); // raw delta: +X, 0Y → East
        assert_eq!(npc.facing, 2);
    }

    #[test]
    fn test_set_course_same_position() {
        // Same position → state should be Still (com2 returns 9).
        let mut npc = make_npc(100, 100);
        set_course(&mut npc, 100, 100, SC_SMART);
        assert_eq!(npc.state, NpcState::Still);
    }

    #[test]
    fn test_set_course_bumble_no_axis_suppression() {
        // Bumble mode: should NOT suppress minor axis, allowing true diagonals.
        let mut npc = make_npc(100, 100);
        // Far east, slightly south — in SMART mode this suppresses Y.
        // In BUMBLE mode it should keep the diagonal.
        set_course(&mut npc, 300, 130, SC_BUMBLE);
        assert_eq!(npc.facing, 3); // SE (not suppressed to E)
    }
}
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test npc_ai`
Expected: FAIL — `set_course` contains `todo!()`.

- [ ] **Step 4: Implement set_course**

Replace the `todo!()` in `set_course` with the full implementation:

```rust
/// Compute facing and state from position delta.
/// Ports set_course from fmain2.c.
///
/// Modes 0–3: smart seek with axis suppression (if one axis > 2× the other, zero the minor).
/// Mode 0: plain smart seek.
/// Mode 1: + deviation ±1 when dist < 40.
/// Mode 2: + deviation ±2 when dist < 30.
/// Mode 3: flee (negate direction signs before lookup).
/// Mode 4: bumble (skip axis suppression — true diagonals).
/// Mode 5: aim only (set facing but do NOT set state to Walking).
/// Mode 6: direct (target_x/y are raw deltas, not world positions).
pub fn set_course(npc: &mut Npc, target_x: i32, target_y: i32, mode: u8) {
    let (dx, dy) = if mode == SC_DIRECT {
        (target_x, target_y)
    } else {
        (target_x - npc.x as i32, target_y - npc.y as i32)
    };

    if dx == 0 && dy == 0 {
        npc.state = NpcState::Still;
        return;
    }

    // Flee mode: negate direction.
    let (dx, dy) = if mode == SC_FLEE {
        (-dx, -dy)
    } else {
        (dx, dy)
    };

    let adx = dx.abs();
    let ady = dy.abs();

    // Axis suppression: modes 0–3 suppress minor axis if major > 2× minor.
    // Mode 4 (bumble) skips this entirely.
    let (eff_dx, eff_dy) = if mode <= SC_FLEE {
        let sx = if ady > adx * 2 { 0 } else { dx };
        let sy = if adx > ady * 2 { 0 } else { dy };
        (sx, sy)
    } else {
        (dx, dy)
    };

    // com2 lookup: map (xsign, ysign) to compass direction 0–7.
    // Index: signum + 1 → 0/1/2.
    const COM2: [[u8; 3]; 3] = [
        [7, 0, 1],   // dy < 0: NW, N, NE
        [6, 9, 2],   // dy = 0: W, STILL, E
        [5, 4, 3],   // dy > 0: SW, S, SE
    ];
    let xi = (eff_dx.signum() + 1) as usize;
    let yi = (eff_dy.signum() + 1) as usize;
    let mut facing = COM2[yi][xi];

    if facing == 9 {
        npc.state = NpcState::Still;
        return;
    }

    // Deviation: modes 1 and 2 add random ±N when close to target.
    if mode == SC_DEVIATE1 && (adx + ady) < 40 {
        let dev = 1i8;
        let coin = (npc.x.wrapping_mul(7).wrapping_add(npc.y)) & 1;
        if coin == 0 {
            facing = (facing.wrapping_add(dev as u8)) & 7;
        } else {
            facing = (facing.wrapping_sub(dev as u8)) & 7;
        }
    } else if mode == SC_DEVIATE2 && (adx + ady) < 30 {
        let dev = 2i8;
        let coin = (npc.x.wrapping_mul(13).wrapping_add(npc.y)) & 1;
        if coin == 0 {
            facing = (facing.wrapping_add(dev as u8)) & 7;
        } else {
            facing = (facing.wrapping_sub(dev as u8)) & 7;
        }
    }

    npc.facing = facing;
    if mode != SC_AIM {
        npc.state = NpcState::Walking;
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test npc_ai`
Expected: All `test_set_course_*` tests PASS.

- [ ] **Step 6: Commit**

```bash
git add src/game/npc_ai.rs src/game/mod.rs
git commit -m "feat(npc_ai): add set_course() with 7 modes

Ports set_course from fmain2.c. Supports smart seek (mode 0), deviation ±1/±2
(modes 1,2), flee (mode 3), bumble (mode 4), aim-only (mode 5), and direct
delta (mode 6). Uses com2 lookup table for 8-way facing computation with
axis suppression for modes 0-3."
```

---

## Task 3: Add `do_tactic()` to npc_ai.rs

**Files:**
- Modify: `src/game/npc_ai.rs`

- [ ] **Step 1: Write failing tests for do_tactic**

Add to the `tests` module in `src/game/npc_ai.rs`:

```rust
    #[test]
    fn test_do_tactic_pursue_gates_reaim() {
        // do_tactic with Pursue should only re-aim ~12.5% of the time.
        // Run 1000 iterations, count how many change facing.
        let mut reaim_count = 0u32;
        for tick in 0..1000u32 {
            let mut npc = make_npc(100, 100);
            npc.tactic = Tactic::Pursue;
            npc.goal = Goal::Attack1;
            npc.facing = 0; // facing North
            do_tactic(&mut npc, 200, 100, None, &[], tick);
            if npc.facing != 0 {
                reaim_count += 1;
            }
        }
        // ~12.5% = 125 ± margin. Should be between 50 and 250.
        assert!(reaim_count > 50, "too few re-aims: {reaim_count}");
        assert!(reaim_count < 250, "too many re-aims: {reaim_count}");
    }

    #[test]
    fn test_do_tactic_random_sets_random_facing() {
        // Random tactic should set state=Walking with a random facing.
        let mut npc = make_npc(100, 100);
        npc.tactic = Tactic::Random;
        npc.goal = Goal::Attack1;
        npc.state = NpcState::Still;
        do_tactic(&mut npc, 200, 100, None, &[], 42);
        // After triggering, state should be Walking (when gate passes).
        // We can't predict exact tick, but over many ticks it should trigger.
        let mut triggered = false;
        for tick in 0..100u32 {
            let mut npc2 = make_npc(100, 100);
            npc2.tactic = Tactic::Random;
            npc2.goal = Goal::Attack1;
            npc2.state = NpcState::Still;
            do_tactic(&mut npc2, 200, 100, None, &[], tick);
            if npc2.state == NpcState::Walking {
                triggered = true;
                break;
            }
        }
        assert!(triggered, "Random tactic should trigger within 100 ticks");
    }

    #[test]
    fn test_do_tactic_backup_flees() {
        // Backup tactic should flee (face away from hero).
        let mut triggered_away = false;
        for tick in 0..100u32 {
            let mut npc = make_npc(100, 100);
            npc.tactic = Tactic::Backup;
            npc.goal = Goal::Archer1;
            npc.facing = 2; // East (toward hero)
            do_tactic(&mut npc, 200, 100, None, &[], tick);
            if npc.facing == 6 { // West = away from hero at (200,100)
                triggered_away = true;
                break;
            }
        }
        assert!(triggered_away, "Backup should eventually face away from hero");
    }

    #[test]
    fn test_do_tactic_attack2_higher_reaim_rate() {
        // Attack2 goal should re-aim ~25% (1/4) vs ~12.5% (1/8).
        let mut reaim_a1 = 0u32;
        let mut reaim_a2 = 0u32;
        for tick in 0..1000u32 {
            let mut npc1 = make_npc(100, 100);
            npc1.tactic = Tactic::Pursue;
            npc1.goal = Goal::Attack1;
            npc1.facing = 0;
            do_tactic(&mut npc1, 200, 100, None, &[], tick);
            if npc1.facing != 0 { reaim_a1 += 1; }

            let mut npc2 = make_npc(100, 100);
            npc2.tactic = Tactic::Pursue;
            npc2.goal = Goal::Attack2;
            npc2.facing = 0;
            do_tactic(&mut npc2, 200, 100, None, &[], tick);
            if npc2.facing != 0 { reaim_a2 += 1; }
        }
        assert!(reaim_a2 > reaim_a1, "Attack2 should re-aim more often: a1={reaim_a1}, a2={reaim_a2}");
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test npc_ai`
Expected: FAIL — `do_tactic` not defined.

- [ ] **Step 3: Add RNG helper and do_tactic**

Add above `set_course` in `src/game/npc_ai.rs`:

```rust
use crate::game::actor::{Goal, Tactic};

/// Simple deterministic RNG for AI decisions.
/// Uses the same LCG family as encounter.rs.
fn ai_rand(tick: u32, salt: u32) -> u32 {
    tick.wrapping_mul(2246822519).wrapping_add(salt).wrapping_mul(1664525).wrapping_add(1013904223)
}

/// Execute the current tactic — gates set_course behind a probabilistic check.
/// Ports do_tactic from fmain2.c:2075.
///
/// `leader_idx`: if Some(i), the index of the leader NPC in `npcs` for Follow/Evade.
/// `npcs`: read-only snapshot of NPC positions for Follow/Evade targets.
pub fn do_tactic(
    npc: &mut Npc,
    hero_x: i32,
    hero_y: i32,
    leader_idx: Option<usize>,
    npcs: &[(i32, i32)],
    tick: u32,
) {
    let r = ai_rand(tick, npc.x as u32 ^ npc.y as u32);

    // Probabilistic gate: ~12.5% for most goals, ~25% for Attack2/Archer2.
    let mask = match npc.goal {
        Goal::Attack2 | Goal::Archer2 => 3,  // !(rand & 3) → 25%
        _ => 7, // !(rand & 7) → 12.5%
    };
    if (r & mask) != 0 {
        return; // No re-aim this tick.
    }

    match npc.tactic {
        Tactic::Pursue => {
            set_course(npc, hero_x, hero_y, SC_SMART);
        }
        Tactic::Shoot => {
            // Check if aligned on cardinal/diagonal axis for firing.
            let xd = (hero_x - npc.x as i32).abs();
            let yd = (hero_y - npc.y as i32).abs();
            let aligned = xd < 8 || yd < 8 || (xd > yd.saturating_sub(5) && xd < yd + 7);
            if aligned && (r >> 8) & 1 == 0 {
                // Aim at hero, set shooting state.
                set_course(npc, hero_x, hero_y, SC_AIM);
                npc.state = NpcState::Shooting;
            } else {
                // Not aligned — maneuver toward hero.
                set_course(npc, hero_x, hero_y, SC_SMART);
            }
        }
        Tactic::Random => {
            // Random facing, walk.
            npc.facing = ((r >> 4) & 7) as u8;
            npc.state = NpcState::Walking;
        }
        Tactic::BumbleSeek => {
            set_course(npc, hero_x, hero_y, SC_BUMBLE);
        }
        Tactic::Backup => {
            set_course(npc, hero_x, hero_y, SC_FLEE);
        }
        Tactic::Follow => {
            // Follow the leader NPC + 20px Y offset.
            if let Some(li) = leader_idx {
                if li < npcs.len() {
                    let (lx, ly) = npcs[li];
                    set_course(npc, lx, ly + 20, SC_SMART);
                } else {
                    // Fallback to random if leader invalid.
                    npc.facing = ((r >> 4) & 7) as u8;
                    npc.state = NpcState::Walking;
                }
            } else {
                npc.facing = ((r >> 4) & 7) as u8;
                npc.state = NpcState::Walking;
            }
        }
        Tactic::Evade => {
            // Aim at neighboring NPC + 20px Y offset (deviation mode 2).
            if let Some(li) = leader_idx {
                let neighbor = if li + 1 < npcs.len() { li + 1 } else if li > 0 { li - 1 } else { li };
                if neighbor < npcs.len() {
                    let (nx, ny) = npcs[neighbor];
                    set_course(npc, nx, ny + 20, SC_DEVIATE2);
                }
            }
        }
        Tactic::EggSeek => {
            // Aim at fixed turtle egg coordinates.
            set_course(npc, 23087, 5667, SC_SMART);
            npc.state = NpcState::Walking;
        }
        Tactic::Frust | Tactic::None => {
            // Frustrated or idle — do nothing this tick.
        }
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test npc_ai`
Expected: All `test_do_tactic_*` tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/game/npc_ai.rs
git commit -m "feat(npc_ai): add do_tactic() with probabilistic re-aim gate

Ports do_tactic from fmain2.c:2075. Gates set_course behind !(rand & 7)
(~12.5%) for most goals, !(rand & 3) (~25%) for Attack2/Archer2. Implements
all 8 tactic branches: Pursue, Shoot, Random, BumbleSeek, Backup, Follow,
Evade, EggSeek."
```

---

## Task 4: Add `select_tactic()` to npc_ai.rs

**Files:**
- Modify: `src/game/npc_ai.rs`

- [ ] **Step 1: Write failing tests for select_tactic**

Add to the `tests` module in `src/game/npc_ai.rs`:

```rust
    #[test]
    fn test_select_tactic_dead_hero_causes_flee() {
        let mut npc = make_npc(100, 100);
        npc.goal = Goal::Attack1;
        npc.tactic = Tactic::Pursue;
        select_tactic(&mut npc, 200, 100, true, None, 0, 42);
        assert_eq!(npc.goal, Goal::Flee);
    }

    #[test]
    fn test_select_tactic_low_vitality_flees() {
        let mut npc = make_npc(100, 100);
        npc.goal = Goal::Attack1;
        npc.vitality = 1;
        select_tactic(&mut npc, 200, 100, false, None, 0, 42);
        assert_eq!(npc.goal, Goal::Flee);
    }

    #[test]
    fn test_select_tactic_archer_close_backups() {
        // Archer too close → Backup tactic.
        let mut backed_up = false;
        for tick in 0..200u32 {
            let mut npc = make_npc(100, 100);
            npc.goal = Goal::Archer1;
            npc.weapon = 4; // bow
            npc.tactic = Tactic::Pursue;
            select_tactic(&mut npc, 120, 110, false, None, 0, tick);
            if npc.tactic == Tactic::Backup {
                backed_up = true;
                break;
            }
        }
        assert!(backed_up, "Archer should select Backup when hero is close");
    }

    #[test]
    fn test_select_tactic_archer_in_range_shoots() {
        // Archer in range → Shoot tactic.
        let mut shooting = false;
        for tick in 0..200u32 {
            let mut npc = make_npc(100, 100);
            npc.goal = Goal::Archer1;
            npc.weapon = 4; // bow
            npc.tactic = Tactic::Pursue;
            select_tactic(&mut npc, 160, 140, false, None, 0, tick);
            if npc.tactic == Tactic::Shoot {
                shooting = true;
                break;
            }
        }
        assert!(shooting, "Archer should select Shoot when hero is in range");
    }

    #[test]
    fn test_select_tactic_melee_close_range_fighting() {
        // Melee NPC very close → Fighting state.
        let mut fighting = false;
        for tick in 0..200u32 {
            let mut npc = make_npc(100, 100);
            npc.goal = Goal::Attack1;
            npc.weapon = 1; // dirk (melee)
            select_tactic(&mut npc, 105, 105, false, None, 0, tick);
            if npc.state == NpcState::Fighting {
                fighting = true;
                break;
            }
        }
        assert!(fighting, "Melee NPC should enter Fighting at close range");
    }

    #[test]
    fn test_select_tactic_no_weapon_confused() {
        // NPC with no weapon → tactic should become Random (confused).
        let mut confused = false;
        for tick in 0..200u32 {
            let mut npc = make_npc(100, 100);
            npc.goal = Goal::Attack1;
            npc.weapon = 0; // no weapon
            select_tactic(&mut npc, 200, 100, false, None, 0, tick);
            if npc.tactic == Tactic::Random {
                confused = true;
                break;
            }
        }
        assert!(confused, "Weaponless NPC should get tactic=Random (confused)");
    }

    #[test]
    fn test_select_tactic_flee_goal_stays_backup() {
        let mut npc = make_npc(100, 100);
        npc.goal = Goal::Flee;
        npc.tactic = Tactic::Pursue;
        select_tactic(&mut npc, 200, 100, false, None, 0, 42);
        assert_eq!(npc.tactic, Tactic::Backup);
    }

    #[test]
    fn test_select_tactic_stand_goal_stays_still() {
        let mut npc = make_npc(100, 100);
        npc.goal = Goal::Stand;
        select_tactic(&mut npc, 200, 100, false, None, 0, 42);
        assert_eq!(npc.state, NpcState::Still);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test npc_ai`
Expected: FAIL — `select_tactic` not defined.

- [ ] **Step 3: Implement select_tactic**

Add to `src/game/npc_ai.rs`, after `do_tactic`:

```rust
/// Goal value for close-range melee threshold computation.
fn goal_value(goal: &Goal) -> i32 {
    match goal {
        Goal::Attack1 => 0,
        Goal::Attack2 => 1,
        Goal::Archer1 => 3,
        Goal::Archer2 => 4,
        _ => 0,
    }
}

/// Select tactic for this NPC based on goal, distance, state.
/// Ports the tactic decision tree from fmain.c:2500-2595.
///
/// `hero_dead`: true if hero is dead/falling.
/// `leader_idx`: index of leader NPC for follower logic.
/// `xtype`: terrain type from game state.
/// `tick`: current game tick for RNG.
pub fn select_tactic(
    npc: &mut Npc,
    hero_x: i32,
    hero_y: i32,
    hero_dead: bool,
    leader_idx: Option<usize>,
    xtype: u16,
    tick: u32,
) {
    let r = ai_rand(tick, npc.x as u32 ^ (npc.y as u32).wrapping_mul(3));

    // === Goal overrides (checked every tick) ===

    // Hero dead → flee or follow leader.
    if hero_dead {
        npc.goal = if leader_idx.is_some() {
            Goal::Follower
        } else {
            Goal::Flee
        };
    }

    // Vitality critically low → flee.
    if npc.vitality < 2 {
        npc.goal = Goal::Flee;
    }

    // High xtype + non-special race → flee (original: xtype > 59 && race != special).
    if xtype > 59 && npc.race < 4 {
        npc.goal = Goal::Flee;
    }

    // === Non-hostile goal modes (bypass tactic tree) ===
    match npc.goal {
        Goal::Flee => {
            npc.tactic = Tactic::Backup;
            return;
        }
        Goal::Follower => {
            npc.tactic = Tactic::Follow;
            return;
        }
        Goal::Stand => {
            set_course(npc, hero_x, hero_y, SC_AIM);
            npc.state = NpcState::Still;
            return;
        }
        Goal::None | Goal::User | Goal::Leader => {
            npc.state = NpcState::Still;
            return;
        }
        _ => {} // Attack/Archer goals continue to tactic tree.
    }

    // === Close-range melee check (every tick, bypasses re-aim gate) ===
    let xd = (hero_x - npc.x as i32).abs();
    let yd = (hero_y - npc.y as i32).abs();

    let is_melee = npc.weapon < 4; // weapons 0-3 are melee (dirk, mace, sword, etc.)
    if is_melee {
        let mut thresh = 14 - goal_value(&npc.goal);
        if npc.race == 7 { // DKnight
            thresh = 16;
        }
        if xd < thresh && yd < thresh {
            // Close-range melee: aim directly and fight.
            set_course(npc, hero_x, hero_y, SC_DIRECT);
            npc.state = NpcState::Fighting;
            return;
        }
    }

    // === Recalculation gate (probabilistic, varies by goal) ===
    let gate_mask = match npc.goal {
        Goal::Attack1 | Goal::Archer1 => 3,  // ~25%
        _ => 15, // ~6.25%
    };
    if (r & gate_mask) != 0 {
        return; // Keep current tactic this tick.
    }

    // === Tactic decision tree ===

    // Snake race + turtle eggs special case.
    if npc.race == 4 { // RACE_SNAKE
        npc.tactic = Tactic::EggSeek;
        return;
    }

    // No weapon → confused.
    if npc.weapon == 0 {
        npc.tactic = Tactic::Random;
        return;
    }

    // Low vitality + 50% chance → evade.
    if npc.vitality < 6 && (r >> 8) & 1 == 0 {
        npc.tactic = Tactic::Evade;
        return;
    }

    // Archer-specific range brackets.
    let is_archer = matches!(npc.goal, Goal::Archer1 | Goal::Archer2);
    if is_archer {
        if xd < 40 && yd < 30 {
            npc.tactic = Tactic::Backup; // Too close.
            return;
        }
        if xd < 70 && yd < 70 {
            npc.tactic = Tactic::Shoot; // In range.
            return;
        }
        // Far away → pursue.
        npc.tactic = Tactic::Pursue;
        return;
    }

    // Default melee → pursue.
    npc.tactic = Tactic::Pursue;
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test npc_ai`
Expected: All `test_select_tactic_*` tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/game/npc_ai.rs
git commit -m "feat(npc_ai): add select_tactic() decision tree

Ports the tactic selection logic from fmain.c:2500-2595. Handles goal
overrides (hero dead, low vitality, high xtype), close-range melee
transition, probabilistic recalculation gate (25% for Attack1/Archer1,
6.25% for others), and full decision tree (confused, evade, archer range
brackets, default pursue)."
```

---

## Task 5: Refactor Npc::tick() to use stored facing

**Files:**
- Modify: `src/game/npc.rs` (tick method at line 102)

- [ ] **Step 1: Write failing test for facing-based movement**

Add to `src/game/npc.rs` test module:

```rust
#[test]
fn test_npc_tick_uses_stored_facing() {
    use crate::game::npc::NpcState;
    let mut npc = Npc {
        npc_type: NPC_TYPE_ORC,
        race: RACE_ENEMY,
        x: 1000,
        y: 1000,
        vitality: 10,
        active: true,
        facing: 2, // East
        state: NpcState::Walking,
        ..Default::default()
    };
    let old_x = npc.x;
    npc.tick(None, false);
    assert!(npc.x > old_x, "Walking east should increase X");
}

#[test]
fn test_npc_tick_still_does_not_move() {
    use crate::game::npc::NpcState;
    let mut npc = Npc {
        npc_type: NPC_TYPE_ORC,
        race: RACE_ENEMY,
        x: 1000,
        y: 1000,
        vitality: 10,
        active: true,
        facing: 2,
        state: NpcState::Still,
        ..Default::default()
    };
    let old_x = npc.x;
    let old_y = npc.y;
    npc.tick(None, false);
    assert_eq!(npc.x, old_x);
    assert_eq!(npc.y, old_y);
}

#[test]
fn test_npc_tick_blocked_becomes_frust() {
    use crate::game::npc::NpcState;
    use crate::game::actor::Tactic;
    // With a WorldData that blocks all terrain, NPC should become frustrated.
    // Without WorldData (None), proxcheck passes, so we skip this for now.
    // This test validates state transition when all 3 directions are blocked.
    let mut npc = Npc {
        npc_type: NPC_TYPE_ORC,
        race: RACE_ENEMY,
        x: 1000,
        y: 1000,
        vitality: 10,
        active: true,
        facing: 2,
        state: NpcState::Walking,
        ..Default::default()
    };
    // With no WorldData (None), proxcheck always passes → should move.
    npc.tick(None, false);
    assert_eq!(npc.state, NpcState::Walking); // Still walking (not frustrated)
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_npc_tick_uses_stored_facing`
Expected: FAIL — `tick()` signature doesn't match (still expects hero_x, hero_y).

- [ ] **Step 3: Refactor tick() to use stored facing**

Replace the entire `tick()` method in `src/game/npc.rs`:

```rust
    /// Execute one frame of movement using stored `facing` and `state`.
    /// The AI layer (select_tactic → do_tactic → set_course) sets facing/state
    /// before this runs. Only moves when state == Walking.
    ///
    /// If all three directions (primary + ±1 deviation) are blocked,
    /// sets state to Still and tactic to Frust.
    ///
    /// `world`: terrain data for collision checks (None = always passable).
    /// `indoor`: true for indoor regions (region >= 8), affects Y wrapping.
    pub fn tick(
        &mut self,
        world: Option<&crate::game::world_data::WorldData>,
        indoor: bool,
    ) {
        use crate::game::collision::{proxcheck, newx, newy};
        use crate::game::actor::Tactic;

        if !self.active || self.state != NpcState::Walking {
            return;
        }

        let facing = self.facing;
        let dist = 2i32;

        let proposed_x = newx(self.x as u16, facing, dist);
        let proposed_y = newy(self.y as u16, facing, dist, indoor);

        // Race-specific terrain bypass: wraith (race 2) skips terrain checks.
        let terrain_passable = self.race == RACE_WRAITH
            || proxcheck(world, proposed_x as i32, proposed_y as i32);

        if terrain_passable {
            self.x = proposed_x as i16;
            self.y = proposed_y as i16;
        } else {
            // Wall-sliding: try clockwise then counter-clockwise deviation.
            let dev_cw = (facing + 1) & 7;
            let cw_x = newx(self.x as u16, dev_cw, dist);
            let cw_y = newy(self.y as u16, dev_cw, dist, indoor);
            if proxcheck(world, cw_x as i32, cw_y as i32) {
                self.x = cw_x as i16;
                self.y = cw_y as i16;
            } else {
                let dev_ccw = (facing.wrapping_sub(1)) & 7;
                let ccw_x = newx(self.x as u16, dev_ccw, dist);
                let ccw_y = newy(self.y as u16, dev_ccw, dist, indoor);
                if proxcheck(world, ccw_x as i32, ccw_y as i32) {
                    self.x = ccw_x as i16;
                    self.y = ccw_y as i16;
                } else {
                    // Fully blocked — frustrated.
                    self.state = NpcState::Still;
                    self.tactic = Tactic::Frust;
                }
            }
        }
    }
```

- [ ] **Step 4: Update existing tick tests to match new signature**

Update the existing tests in `src/game/npc.rs` that call `npc.tick(hero_x, hero_y, world, indoor)`:

`test_npc_tick_chase` — change to set facing/state, then call `tick(None, false)`:
```rust
    #[test]
    fn test_npc_tick_chase() {
        let mut npc = Npc {
            npc_type: NPC_TYPE_ORC,
            race: RACE_ENEMY,
            x: 0, y: 0,
            vitality: 10,
            gold: 5,
            speed: 2,
            active: true,
            facing: 2,  // East
            state: NpcState::Walking,
            ..Default::default()
        };
        npc.tick(None, false);
        assert!(npc.x > 0); // should have moved east
    }
```

`test_npc_tick_moves_toward_hero_with_direction_lut` — change similarly:
```rust
    #[test]
    fn test_npc_tick_moves_toward_hero_with_direction_lut() {
        let mut npc = Npc {
            npc_type: NPC_TYPE_ORC,
            race: RACE_ENEMY,
            x: 1000,
            y: 1000,
            vitality: 10,
            gold: 0,
            speed: 2,
            active: true,
            facing: 2,  // East
            state: NpcState::Walking,
            ..Default::default()
        };
        let old_x = npc.x;
        npc.tick(None, false);
        assert!(npc.x > old_x, "NPC should move east");
    }
```

- [ ] **Step 5: Run full test suite**

Run: `cargo test`
Expected: All tests pass. The compilation may reveal call sites in `gameplay_scene.rs` that still use the old `tick()` signature — that's expected and will be fixed in Task 7.

- [ ] **Step 6: Temporarily fix the gameplay_scene.rs call site to compile**

In `src/game/gameplay_scene.rs`, update the `npc.tick()` call (around line 1354) from:

```rust
let adjacent = npc.tick(hero_x, hero_y, self.map_world.as_ref(), indoor);
```

to:

```rust
npc.tick(self.map_world.as_ref(), indoor);
let adjacent = false; // TODO: Task 7 rewrites update_actors with AI pipeline
```

- [ ] **Step 7: Run full test suite again**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/game/npc.rs src/game/gameplay_scene.rs
git commit -m "refactor(npc): tick() uses stored facing instead of recomputing

tick() no longer takes hero_x/hero_y — the AI layer (set_course/do_tactic)
is responsible for setting facing and state. Movement only executes when
state == Walking. Fully blocked NPCs transition to Still + Frust tactic.
Temporary stub in update_actors() until Task 7 wires the AI pipeline."
```

---

## Task 6: Update encounter spawning with AI fields

**Files:**
- Modify: `src/game/encounter.rs`

- [ ] **Step 1: Write failing test for cleverness and AI field initialization**

Add to `src/game/encounter.rs` test module:

```rust
    #[test]
    fn test_spawn_encounter_has_ai_fields() {
        use crate::game::actor::{Goal, Tactic};
        use crate::game::npc::NpcState;

        let npc = spawn_encounter(0, 100, 100, 42); // Ogre
        assert_eq!(npc.cleverness, 0);
        assert!(matches!(npc.goal, Goal::Attack1 | Goal::Attack2));
        assert_eq!(npc.tactic, Tactic::Pursue);
        assert_eq!(npc.state, NpcState::Walking);
    }

    #[test]
    fn test_spawn_encounter_archer_goal() {
        use crate::game::actor::Goal;

        // DKnight (type 7): arms=7, weapon prob produces bow (4) often.
        // Force a tick that gives weapon=4 (bow).
        for tick in 0..100u32 {
            let npc = spawn_encounter(7, 100, 100, tick);
            if npc.weapon >= 4 {
                assert!(matches!(npc.goal, Goal::Archer1 | Goal::Archer2),
                    "Bow/wand wielder should get Archer goal, got {:?}", npc.goal);
                return;
            }
        }
        // If no bow was rolled in 100 ticks, that's fine — skip.
    }

    #[test]
    fn test_spawn_encounter_cleverness_wraith() {
        let npc = spawn_encounter(2, 100, 100, 42); // Wraith: clever=1
        assert_eq!(npc.cleverness, 1);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_spawn_encounter_has_ai_fields`
Expected: FAIL — `cleverness` field not set, `goal`/`tactic`/`state` not initialized.

- [ ] **Step 3: Update spawn_encounter to set AI fields**

In `src/game/encounter.rs`, add imports at the top:

```rust
use crate::game::actor::{Goal, Tactic};
use crate::game::npc::NpcState;
```

Update the `spawn_encounter` function body. Replace the `Npc { ... }` struct construction:

```rust
pub fn spawn_encounter(encounter_type: usize, hero_x: i16, hero_y: i16, tick: u32) -> Npc {
    let etype = encounter_type.min(10);
    let stats = &ENCOUNTER_CHART_FULL[etype];
    let wp_idx = (stats.arms as usize * 4 + rand4_from_tick(tick.wrapping_add(etype as u32)) as usize).min(31);
    let weapon = WEAPON_PROBS[wp_idx];
    let race = match etype {
        2 => RACE_WRAITH,
        3 | 5 => RACE_UNDEAD,
        4 => RACE_SNAKE,
        _ => RACE_ENEMY,
    };

    // Assign goal based on weapon type and cleverness.
    let is_ranged = weapon >= 4; // bow or wand
    let goal = if is_ranged {
        if stats.clever > 0 { Goal::Archer2 } else { Goal::Archer1 }
    } else {
        if stats.clever > 0 { Goal::Attack2 } else { Goal::Attack1 }
    };

    Npc {
        npc_type: etype as u8,
        race,
        x: hero_x.saturating_add(50),
        y: hero_y.saturating_add(50),
        vitality: stats.hp,
        gold: stats.treasure as i16 * 5,
        speed: 2,
        weapon,
        active: true,
        goal,
        tactic: Tactic::Pursue,
        facing: 4, // South (toward hero, roughly)
        state: NpcState::Walking,
        cleverness: stats.clever,
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test encounter`
Expected: All encounter tests pass.

- [ ] **Step 5: Run full test suite**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/game/encounter.rs
git commit -m "feat(encounter): initialize AI fields on NPC spawn

Set goal (Attack1/2 or Archer1/2 based on weapon and cleverness), tactic
(Pursue), facing, state (Walking), and cleverness from the encounter chart
when spawning new NPCs."
```

---

## Task 7: Rewrite update_actors() with AI pipeline

**Files:**
- Modify: `src/game/gameplay_scene.rs` (update_actors at line 1301)

- [ ] **Step 1: Write test for AI pipeline integration**

Add to `src/game/gameplay_scene.rs` test module (at end of file):

```rust
    #[test]
    fn test_update_actors_runs_ai_pipeline() {
        use crate::game::npc::{Npc, NpcTable, NPC_TYPE_ORC, RACE_ENEMY, NpcState, MAX_NPCS};
        use crate::game::actor::{Goal, Tactic};

        // Verify that after multiple ticks, NPCs don't all end up
        // at the same position (proves they're not laser-tracking).
        let mut table = NpcTable { npcs: std::array::from_fn(|_| Npc::default()) };
        for i in 0..4 {
            table.npcs[i] = Npc {
                npc_type: NPC_TYPE_ORC,
                race: RACE_ENEMY,
                x: 200,
                y: 200,
                vitality: 10,
                active: true,
                goal: Goal::Attack1,
                tactic: Tactic::Pursue,
                facing: 4,
                state: NpcState::Walking,
                cleverness: 0,
                ..Default::default()
            };
        }

        // Simulate 100 ticks of AI + movement (without world data).
        let hero_x = 500i32;
        let hero_y = 500i32;
        for tick in 0..100u32 {
            use crate::game::npc_ai::{select_tactic, do_tactic};
            let positions: Vec<(i32, i32)> = table.npcs.iter()
                .map(|n| (n.x as i32, n.y as i32))
                .collect();
            let leader_idx = table.npcs.iter().position(|n| n.active);
            for npc in &mut table.npcs {
                if !npc.active { continue; }
                select_tactic(npc, hero_x, hero_y, false, leader_idx, 0, tick);
                do_tactic(npc, hero_x, hero_y, leader_idx, &positions, tick);
                npc.tick(None, false);
            }
        }

        // After 100 ticks, NPCs should NOT all be at the same position.
        let positions: Vec<(i16, i16)> = table.npcs[0..4].iter().map(|n| (n.x, n.y)).collect();
        let all_same = positions.iter().all(|p| *p == positions[0]);
        // With probabilistic AI, they should have diverged.
        assert!(!all_same, "NPCs should have diverged, but all at {:?}", positions[0]);
    }
```

- [ ] **Step 2: Run test to verify it fails (or passes with current stub)**

Run: `cargo test test_update_actors_runs_ai_pipeline`
Expected: This may compile and run depending on the current state. If it passes, good — it validates the AI pipeline works end-to-end. If it fails, the next step fixes it.

- [ ] **Step 3: Rewrite update_actors()**

Replace the `update_actors` method in `src/game/gameplay_scene.rs` (lines 1301–1387). The new version:

```rust
    fn update_actors(&mut self, _delta: u32) {
        use crate::game::npc_ai::{select_tactic, do_tactic};
        use crate::game::npc::NpcState;
        use crate::game::actor::{Goal, Tactic};

        let hero_x = self.state.hero_x as i32;
        let hero_y = self.state.hero_y as i32;
        let hero_dead = self.state.vitality <= 0;
        let xtype = self.state.xtype;
        let indoor = self.state.region_num >= 8;
        let tick = self.state.game_tick;

        if let Some(ref mut table) = self.npc_table {
            // Snapshot NPC positions for Follow/Evade targeting.
            let positions: Vec<(i32, i32)> = table.npcs.iter()
                .map(|n| (n.x as i32, n.y as i32))
                .collect();

            // Determine leader: first active hostile NPC.
            let leader_idx = table.npcs.iter().position(|n| {
                n.active && matches!(n.goal,
                    Goal::Attack1 | Goal::Attack2 | Goal::Archer1 | Goal::Archer2)
            });

            // 1. AI decision pass.
            for npc in &mut table.npcs {
                if !npc.active { continue; }
                select_tactic(npc, hero_x, hero_y, hero_dead, leader_idx, xtype, tick);
                do_tactic(npc, hero_x, hero_y, leader_idx, &positions, tick);
            }

            // 2. Movement execution pass.
            for npc in &mut table.npcs {
                if !npc.active { continue; }
                npc.tick(self.map_world.as_ref(), indoor);
            }

            // 3. Battleflag: true if any active NPC within 300px.
            let any_nearby = table.npcs.iter().any(|n| {
                n.active
                    && (n.x as i32 - hero_x).abs() < 300
                    && (n.y as i32 - hero_y).abs() < 300
            });
            self.state.battleflag = any_nearby;

            // 4. Sync NPC positions → Actor array for rendering.
            let anix = self.state.anix;
            let mut actor_idx = 1; // Skip actor 0 (player).
            for npc in &table.npcs {
                if !npc.active { continue; }
                if actor_idx >= anix { break; }
                let actor = &mut self.state.actors[actor_idx];
                actor.abs_x = npc.x as u16;
                actor.abs_y = npc.y as u16;
                actor.facing = npc.facing;
                actor.moving = npc.state == NpcState::Walking;
                actor.state = match npc.state {
                    NpcState::Walking => crate::game::actor::ActorState::Walking,
                    NpcState::Fighting => crate::game::actor::ActorState::Fighting(0),
                    NpcState::Shooting => crate::game::actor::ActorState::Shooting(0),
                    NpcState::Dying => crate::game::actor::ActorState::Dying,
                    NpcState::Dead => crate::game::actor::ActorState::Dead,
                    NpcState::Sinking => crate::game::actor::ActorState::Sinking,
                    NpcState::Still => crate::game::actor::ActorState::Still,
                };
                actor_idx += 1;
            }
        }

        // 5. Archer missile firing (from NPC Shooting state).
        if self.archer_cooldown > 0 {
            self.archer_cooldown -= 1;
        } else if let Some(ref table) = self.npc_table {
            for npc in &table.npcs {
                if !npc.active { continue; }
                if npc.state != NpcState::Shooting { continue; }
                let ax = npc.x as i32;
                let ay = npc.y as i32;
                if (hero_x - ax).abs().max((hero_y - ay).abs()) > 150 { continue; }
                let dir = facing_toward(ax, ay, hero_x, hero_y);
                use crate::game::combat::fire_missile;
                fire_missile(&mut self.missiles, ax, ay, dir, 3, false);
                self.archer_cooldown = 15;
                break;
            }
        }
    }
```

- [ ] **Step 4: Add `game_tick` field to GameState if needed**

Check if `self.state.game_tick` exists. If not, add a `pub game_tick: u32` field to `GameState` in `src/game/game_state.rs` (initialized to 0 in `new()`), and increment it in the main update loop (`gameplay_scene.rs`, inside the `for _ in 0..delta_ticks` loop).

If `game_tick` already exists (may be called `tick` or `frame_count`), use that instead.

- [ ] **Step 5: Verify compile**

Run: `cargo build`
Expected: Compiles without errors.

- [ ] **Step 6: Run full test suite**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/game/gameplay_scene.rs src/game/game_state.rs
git commit -m "feat(gameplay): rewrite update_actors() with AI pipeline

Replace laser-tracking NPC movement with the full AI pipeline:
select_tactic → do_tactic → set_course → tick(). NPCs now re-aim
probabilistically (~12.5%) instead of every frame. Sync NPC positions
to Actor array for rendering. Set battleflag from NPC proximity.
Fire archer missiles from NPC Shooting state."
```

---

## Task 8: Remove dead Actor-based movement code

**Files:**
- Modify: `src/game/gameplay_scene.rs`

- [ ] **Step 1: Remove the old Actor goal-based movement loop**

The old code in `update_actors()` that iterated `self.state.actors[1..anix]` with `match actor.goal { ... }` setting `vel_x`/`vel_y` is now replaced by the NPC AI pipeline. If any remnant of this loop still exists after Task 7, remove it. The Actor array is now write-only (synced from NPC positions).

- [ ] **Step 2: Remove `direction_to_target()` from npc.rs if unused**

Check if `direction_to_target()` (lines 39–59 of npc.rs) is still called anywhere. If not, remove it. The AI now uses `set_course()` from `npc_ai.rs` instead.

- [ ] **Step 3: Run full test suite**

Run: `cargo test`
Expected: All tests pass. Remove any tests that referenced `direction_to_target()` if the function is removed.

- [ ] **Step 4: Commit**

```bash
git add src/game/gameplay_scene.rs src/game/npc.rs
git commit -m "cleanup: remove dead Actor movement code and direction_to_target

The NPC AI pipeline (select_tactic → do_tactic → set_course → tick)
now handles all NPC movement. The old Actor-based vel_x/vel_y movement
and direction_to_target helper are no longer needed."
```

---

## Task 9: Final validation and smoke test

**Files:** None (read-only validation)

- [ ] **Step 1: Run full test suite**

Run: `cargo test`
Expected: All tests pass, including new npc_ai tests.

- [ ] **Step 2: Run game with debug mode**

Run: `cargo run -- --debug --skip-intro`
Expected: Game starts. Walk into enemy territory. Observe:
- NPCs don't laser-track — they wander, occasionally re-aim
- Archers back up when too close, shoot when in range
- Multiple NPCs spread out rather than stacking on same pixel
- Battleflag activates near enemies (battle music plays)

- [ ] **Step 3: Count test coverage**

Run: `cargo test 2>&1 | grep "test result"`
Document the number of tests and confirm all pass.

- [ ] **Step 4: Final commit (if any adjustments)**

```bash
git add -A
git commit -m "test: validate NPC AI system end-to-end"
```
