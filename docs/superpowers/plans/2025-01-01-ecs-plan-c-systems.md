# ECS Migration Plan C: System Extraction

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extract all 18 gameplay systems from `GameplayScene` into standalone functions in `src/game/ecs/systems/`. Each system takes `(&mut World, &mut Resources)` (or `(&World, &Resources)` for read-only systems) and has no dependency on `GameplayScene`. The existing `GameplayScene` is left intact throughout this plan.

**Architecture:** Each system is a module with one primary `pub fn run(world, resources, ...)` function. Systems communicate only through `Resources.events` — never by calling each other directly. Tests for each system operate directly on a `hecs::World` and `Resources` with no `GameplayScene` involved.

**Prerequisites:** Plans A and B complete. All ≥712 existing tests passing.

**Tech Stack:** Rust 2021, `hecs = "0.11"`. Reference existing logic in `src/game/gameplay_scene/` — port it, don't rewrite it.

---

## File map

| File | Change |
|---|---|
| `src/game/ecs/systems/mod.rs` | **Create** — systems module root |
| `src/game/ecs/systems/clock.rs` | **Create** — ClockSystem |
| `src/game/ecs/systems/input.rs` | **Create** — InputSystem |
| `src/game/ecs/systems/movement.rs` | **Create** — MovementSystem |
| `src/game/ecs/systems/carrier.rs` | **Create** — CarrierSystem |
| `src/game/ecs/systems/collision.rs` | **Create** — CollisionSystem |
| `src/game/ecs/systems/door.rs` | **Create** — DoorSystem |
| `src/game/ecs/systems/zone.rs` | **Create** — ZoneSystem |
| `src/game/ecs/systems/npc_ai.rs` | **Create** — NpcAiSystem |
| `src/game/ecs/systems/npc_movement.rs` | **Create** — NpcMovementSystem |
| `src/game/ecs/systems/combat.rs` | **Create** — CombatSystem |
| `src/game/ecs/systems/missile.rs` | **Create** — MissileSystem |
| `src/game/ecs/systems/encounter.rs` | **Create** — EncounterSystem |
| `src/game/ecs/systems/proximity.rs` | **Create** — ProximitySystem |
| `src/game/ecs/systems/item.rs` | **Create** — ItemSystem |
| `src/game/ecs/systems/narrative.rs` | **Create** — NarrativeSystem |
| `src/game/ecs/systems/death.rs` | **Create** — DeathSystem |
| `src/game/ecs/systems/region.rs` | **Create** — RegionSystem |
| `src/game/ecs/systems/render/mod.rs` | **Create** — render systems module |
| `src/game/ecs/systems/render/palette.rs` | **Create** — PaletteSystem |
| `src/game/ecs/systems/render/map.rs` | **Create** — MapRenderSystem |
| `src/game/ecs/systems/render/sprite.rs` | **Create** — SpriteRenderSystem |
| `src/game/ecs/systems/render/hibar.rs` | **Create** — HiBarRenderSystem |
| `src/game/ecs/mod.rs` | Add `pub mod systems;` |

---

## Implementation strategy for each system

For each system:
1. Write a failing test that constructs a minimal `World` + `Resources`, calls `run()`, and asserts the expected mutation.
2. Implement `run()` by porting the relevant logic from `GameplayScene`.
3. Verify the test passes.
4. Verify the full suite still passes.
5. Commit.

The following tasks are ordered by dependency: systems earlier in the list have no dependency on systems later in the list.

---

## Task 1: Create systems module skeleton

**Files:**
- Create: `src/game/ecs/systems/mod.rs`
- Create: stub files for all system modules
- Modify: `src/game/ecs/mod.rs`

- [ ] **Step 1: Create `src/game/ecs/systems/mod.rs`**

```rust
//! Gameplay systems. Each module contains one `pub fn run(...)`.
//! Execution order: clock → input → sleep → movement → carrier →
//! collision → door → zone → npc_ai → npc_movement → combat →
//! missile → encounter → proximity → item → narrative → death → region.

pub mod clock;
pub mod input;
pub mod movement;
pub mod carrier;
pub mod collision;
pub mod door;
pub mod zone;
pub mod npc_ai;
pub mod npc_movement;
pub mod combat;
pub mod missile;
pub mod encounter;
pub mod proximity;
pub mod item;
pub mod narrative;
pub mod death;
pub mod region;
pub mod render;
```

- [ ] **Step 2: Create stub modules**

For each module listed above, create a file with:
```rust
// Stub — implemented in Plan C.
```
Create `src/game/ecs/systems/render/mod.rs`:
```rust
pub mod palette;
pub mod map;
pub mod sprite;
pub mod hibar;
```
Create stubs for each render submodule.

- [ ] **Step 3: Add to `src/game/ecs/mod.rs`**

```rust
pub mod systems;
```

- [ ] **Step 4: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```
Expected: no errors.

- [ ] **Step 5: Commit**

```bash
git add src/game/ecs/systems/ src/game/ecs/mod.rs
git commit -m "chore: scaffold ecs systems module"
```

---

## Task 2: ClockSystem

**Port from:** `src/game/game_state.rs` — `GameState::tick()` (the day/night and timer portion), `daynight_tick()`.

**Files:**
- Modify: `src/game/ecs/systems/clock.rs`

- [ ] **Step 1: Write failing test**

```rust
#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::resources::{Resources, GameClock};
    use crate::game::ecs::events::ClockEvent;
    use super::run;

    fn make_resources(world: &mut World) -> Resources {
        let hero = world.spawn((crate::game::ecs::components::Hero,));
        Resources::new(hero)
    }

    #[test]
    fn daynight_increments() {
        let mut world = World::new();
        let mut res = make_resources(&mut world);
        res.clock.daynight = 100;
        run(&mut world, &mut res);
        assert!(res.clock.daynight > 100 || res.clock.game_days > 0);
    }

    #[test]
    fn freeze_timer_decrements() {
        let mut world = World::new();
        let mut res = make_resources(&mut world);
        res.clock.freeze_timer = 5;
        run(&mut world, &mut res);
        assert_eq!(res.clock.freeze_timer, 4);
    }

    #[test]
    fn freeze_sticky_holds_timer() {
        let mut world = World::new();
        let mut res = make_resources(&mut world);
        res.clock.freeze_timer = 1;
        res.clock.freeze_sticky = true;
        run(&mut world, &mut res);
        // sticky prevents decrement to 0
        assert!(res.clock.freeze_timer > 0);
    }
}
```

- [ ] **Step 2: Run test — verify it fails**

```bash
cargo test ecs::systems::clock 2>&1 | grep -E "FAILED|error\[|^error"
```
Expected: compile error (function `run` not defined).

- [ ] **Step 3: Implement `clock.rs`**

Port from `GameState::daynight_tick()` and the timer decrement section of `GameState::tick()`:

```rust
//! ClockSystem — advances the day/night cycle, decrements spell timers.
//! Port of GameState::tick() timer section and daynight_tick().
//! See docs/spec/daynight-cycle.md for timing constants.

use hecs::World;
use crate::game::ecs::resources::Resources;
use crate::game::ecs::events::ClockEvent;

/// NTSC daynight cycle length (fmain.c: DAYLEN = 24000).
const DAYLEN: u16 = 24000;
/// Number of time periods per day.
const PERIODS_PER_DAY: u8 = 12;

pub fn run(world: &mut World, res: &mut Resources) {
    let clock = &mut res.clock;

    // Advance day/night counter.
    let prev_daynight = clock.daynight;
    clock.daynight = clock.daynight.wrapping_add(1);
    if clock.daynight >= DAYLEN {
        clock.daynight = 0;
        clock.game_days = clock.game_days.wrapping_add(1);
    }

    // Compute lightlevel: triangle wave 0→300→0 over DAYLEN ticks.
    // Reference: docs/spec/palettes-daynight-visuals.md
    let half = DAYLEN / 2;
    clock.lightlevel = if clock.daynight < half {
        (clock.daynight as u32 * 300 / half as u32) as u16
    } else {
        ((DAYLEN - clock.daynight) as u32 * 300 / half as u32) as u16
    };

    // Day period: 12 buckets, each DAYLEN/12 ticks wide.
    let new_period = (clock.daynight as u32 * PERIODS_PER_DAY as u32 / DAYLEN as u32) as u8;
    let prev_period = (prev_daynight as u32 * PERIODS_PER_DAY as u32 / DAYLEN as u32) as u8;
    if new_period != prev_period {
        res.events.clock.push(ClockEvent::NewPeriod { period: new_period });
        res.region.dayperiod = new_period;
    }

    // Tick cycle and flasher counters.
    let clock = &mut res.clock;
    clock.cycle = clock.cycle.wrapping_add(1);
    clock.flasher = clock.flasher.wrapping_add(1);
    clock.tick_counter = clock.tick_counter.wrapping_add(1);

    // Decrement spell timers (sticky mode holds at 1).
    if clock.light_timer > 0 {
        if clock.light_sticky { clock.light_timer = 1; }
        else { clock.light_timer -= 1; }
    }
    if clock.secret_timer > 0 {
        if clock.secret_sticky { clock.secret_timer = 1; }
        else { clock.secret_timer -= 1; }
    }
    if clock.freeze_timer > 0 {
        if clock.freeze_sticky { clock.freeze_timer = 1; }
        else { clock.freeze_timer -= 1; }
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test ecs::systems::clock 2>&1 | grep -E "test.*ok|FAILED"
```
Expected: all 3 clock tests pass.

- [ ] **Step 5: Full suite**

```bash
cargo test 2>&1 | grep "^test result"
```

- [ ] **Step 6: Commit**

```bash
git add src/game/ecs/systems/clock.rs
git commit -m "feat(ecs): ClockSystem — day/night cycle and spell timer ticks"
```

---

## Task 3: CollisionSystem

**Port from:** `src/game/gameplay_scene/actors.rs` — battleflag computation; `src/game/collision.rs` — proximity check functions (these are already pure functions, just wire them).

**Files:**
- Modify: `src/game/ecs/systems/collision.rs`

- [ ] **Step 1: Write failing test**

```rust
#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::components::*;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::spawn::*;
    use super::run;

    #[test]
    fn battleflag_set_when_enemy_nearby() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0,
            HeroStats { vitality:100, brave:0, luck:0, kind:0,
                        wealth:0, hunger:0, fatigue:0, gold:0 },
            Inventory::empty());
        let mut res = Resources::new(hero);
        // Enemy within 300px
        spawn_enemy(&mut world, 150.0, 150.0, 1, 0, 20, 0, 0, 3, 5, 0);
        run(&world, &mut res);
        assert!(res.region.battleflag);
    }

    #[test]
    fn battleflag_clear_when_no_enemies() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0,
            HeroStats { vitality:100, brave:0, luck:0, kind:0,
                        wealth:0, hunger:0, fatigue:0, gold:0 },
            Inventory::empty());
        let mut res = Resources::new(hero);
        res.region.battleflag = true;
        run(&world, &mut res);
        assert!(!res.region.battleflag);
    }

    #[test]
    fn battleflag_clear_when_enemy_far() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 0.0, 0.0, 0,
            HeroStats { vitality:100, brave:0, luck:0, kind:0,
                        wealth:0, hunger:0, fatigue:0, gold:0 },
            Inventory::empty());
        let mut res = Resources::new(hero);
        // Enemy > 300px away
        spawn_enemy(&mut world, 500.0, 500.0, 1, 0, 20, 0, 0, 3, 5, 0);
        run(&world, &mut res);
        assert!(!res.region.battleflag);
    }
}
```

- [ ] **Step 2: Implement `collision.rs`**

```rust
//! CollisionSystem — computes battleflag from entity proximity.
//! Does NOT perform movement collision (that's in MovementSystem/NpcMovementSystem).

use hecs::World;
use crate::game::ecs::components::{Enemy, Position, Health};
use crate::game::ecs::resources::Resources;

/// Distance threshold for battleflag (original: 300px each axis, not Euclidean).
const BATTLE_RANGE: f32 = 300.0;

pub fn run(world: &World, res: &mut Resources) {
    let hero_pos = match world.get::<&Position>(res.hero_entity) {
        Ok(p) => *p,
        Err(_) => return,
    };

    let battleflag = world
        .query::<(&Position, &Health)>()
        .with::<&Enemy>()
        .iter()
        .any(|(_, (pos, health))| {
            !health.is_dead()
                && (pos.x - hero_pos.x).abs() < BATTLE_RANGE
                && (pos.y - hero_pos.y).abs() < BATTLE_RANGE
        });

    res.region.battleflag = battleflag;
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test ecs::systems::collision 2>&1 | grep -E "test.*ok|FAILED"
```
Expected: all 3 tests pass.

- [ ] **Step 4: Full suite, commit**

```bash
cargo test 2>&1 | grep "^test result"
git add src/game/ecs/systems/collision.rs
git commit -m "feat(ecs): CollisionSystem — battleflag from enemy proximity"
```

---

## Task 4: NpcAiSystem

**Port from:** `src/game/gameplay_scene/actors.rs` — `update_actors()` AI decision pass (lines 44–70); `src/game/npc_ai.rs` — `tick_npc()` (already nearly pure).

**Files:**
- Modify: `src/game/ecs/systems/npc_ai.rs`

- [ ] **Step 1: Write failing test**

```rust
#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::components::*;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::spawn::*;
    use crate::game::actor::{Goal, Tactic};
    use crate::game::npc::NpcState;
    use super::run;

    #[test]
    fn ai_sets_walking_when_pursuing_hero() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 200.0, 200.0, 0,
            HeroStats { vitality:100, brave:0, luck:0, kind:0,
                        wealth:0, hunger:0, fatigue:0, gold:0 },
            Inventory::empty());
        let mut res = Resources::new(hero);
        let enemy = spawn_enemy(&mut world, 100.0, 100.0, 1, 0, 20, 0, 0, 3, 0, 0);
        // Set tactic to Pursue so AI will try to walk
        world.get_mut::<AiState>(enemy).unwrap().tactic = Tactic::Pursue;
        world.get_mut::<AiState>(enemy).unwrap().goal = Goal::Attack1;
        run(&mut world, &mut res);
        let state = world.get::<&AiState>(enemy).unwrap().state.clone();
        assert_eq!(state, NpcState::Walking);
    }

    #[test]
    fn frozen_hostile_npc_skips_ai() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 200.0, 200.0, 0,
            HeroStats { vitality:100, brave:0, luck:0, kind:0,
                        wealth:0, hunger:0, fatigue:0, gold:0 },
            Inventory::empty());
        let mut res = Resources::new(hero);
        res.clock.freeze_timer = 5; // frozen
        let enemy = spawn_enemy(&mut world, 100.0, 100.0, 1,
            0 /* RACE_NORMAL < 7 */, 20, 0, 0, 3, 0, 0);
        world.get_mut::<AiState>(enemy).unwrap().state = NpcState::Still;
        run(&mut world, &mut res);
        // Hostile NPC (race < 7) skips AI when frozen — state unchanged
        let state = world.get::<&AiState>(enemy).unwrap().state.clone();
        assert_eq!(state, NpcState::Still);
    }
}
```

- [ ] **Step 2: Implement `npc_ai.rs`**

```rust
//! NpcAiSystem — runs AI decision pass for all enemy entities.
//! Port of update_actors() AI pass and tick_npc() from gameplay_scene/actors.rs
//! and game/npc_ai.rs.
//!
//! This system writes AiState and Facing for each enemy.
//! It does NOT write Position (that is NpcMovementSystem's job).

use hecs::World;
use crate::game::ecs::components::{
    Enemy, ArenaDummy, Position, Facing, AiState, EnemyKind, Health, Speed,
};
use crate::game::ecs::resources::Resources;
use crate::game::actor::{Goal, Tactic};
use crate::game::npc::{NpcState, MAX_NPCS};
use crate::game::npc_ai::{tick_npc_ecs, SetCourseMode};
use crate::game::direction::Direction;

pub fn run(world: &mut World, res: &mut Resources) {
    let hero_pos = match world.get::<&Position>(res.hero_entity) {
        Ok(p) => *p,
        Err(_) => return,
    };
    let hero_dead = world
        .get::<&crate::game::ecs::components::HeroStats>(res.hero_entity)
        .map(|s| s.is_dead())
        .unwrap_or(true);

    let freeze = res.clock.is_frozen();
    let tick = res.clock.tick_counter;
    let xtype = res.region.xtype;
    let turtle_eggs = false; // SPEC-GAP: not yet tracked

    // Snapshot enemy positions for follow/evade targeting.
    let positions: Vec<(hecs::Entity, f32, f32)> = world
        .query::<&Position>()
        .with::<&Enemy>()
        .iter()
        .map(|(e, p)| (e, p.x, p.y))
        .collect();

    // Find leader: first active hostile enemy.
    let leader_entity = world
        .query::<&AiState>()
        .with::<&Enemy>()
        .iter()
        .find(|(_, ai)| {
            !matches!(ai.state, NpcState::Dead) &&
            matches!(ai.goal, Goal::Attack1 | Goal::Attack2 |
                              Goal::Archer1 | Goal::Archer2)
        })
        .map(|(e, _)| e);

    // Collect entities to tick (avoid borrow conflict).
    let enemies: Vec<hecs::Entity> = world
        .query::<()>()
        .with::<&Enemy>()
        .without::<&ArenaDummy>()
        .iter()
        .map(|(e, _)| e)
        .collect();

    for entity in enemies {
        let (race, state) = match world.query_one::<(&EnemyKind, &AiState)>(entity) {
            Ok(mut q) => match q.get() {
                Some((k, ai)) => (k.race, ai.state.clone()),
                None => continue,
            },
            Err(_) => continue,
        };

        if matches!(state, NpcState::Dead) { continue; }

        // Freeze gate: hostile NPCs (race < 7) skip AI when frozen.
        if freeze && race < 7 { continue; }
        // SETFIG races (>= 0x80) skip the goal FSM entirely.
        if race >= 0x80 { continue; }

        let pos = match world.get::<&Position>(entity) {
            Ok(p) => *p,
            Err(_) => continue,
        };

        // Build position snapshot excluding self.
        let others: Vec<(f32, f32)> = positions
            .iter()
            .filter(|(e, _, _)| *e != entity)
            .map(|(_, x, y)| (*x, *y))
            .collect();

        let is_leader = leader_entity == Some(entity);
        let leader_pos = leader_entity
            .filter(|&le| le != entity)
            .and_then(|le| world.get::<&Position>(le).ok().map(|p| (p.x, p.y)));

        // Call the ported AI decision function.
        // tick_npc_ecs is a new variant of tick_npc that works with ECS types.
        if let Ok(mut ai) = world.get_mut::<AiState>(entity) {
            if let Ok(mut facing) = world.get_mut::<Facing>(entity) {
                tick_npc_ecs(
                    &mut ai,
                    &mut facing,
                    pos.x, pos.y,
                    hero_pos.x, hero_pos.y,
                    hero_dead,
                    is_leader,
                    leader_pos,
                    &others,
                    tick,
                    xtype,
                    turtle_eggs,
                    freeze,
                );
            }
        }
    }
}
```

- [ ] **Step 3: Add `tick_npc_ecs` to `src/game/npc_ai.rs`**

This is a thin adapter that calls the existing AI logic but operates on `AiState` + `Facing` components instead of `&mut Npc`. Add to `npc_ai.rs`:

```rust
use crate::game::ecs::components::{AiState, Facing};

/// ECS adapter for the NPC AI tick.
/// Mirrors tick_npc() but operates on component types instead of Npc.
pub fn tick_npc_ecs(
    ai: &mut AiState,
    facing: &mut Facing,
    x: f32, y: f32,
    hero_x: f32, hero_y: f32,
    hero_dead: bool,
    is_leader: bool,
    leader_pos: Option<(f32, f32)>,
    others: &[(f32, f32)],
    tick: u32,
    xtype: u16,
    turtle_eggs: bool,
    freeze: bool,
) {
    // Convert to i32 for compatibility with existing AI functions.
    // Build a temporary Npc-like view and delegate to select_tactic / do_tactic.
    // This avoids duplicating the AI logic.
    use crate::game::npc::{Npc, NpcState};

    let mut tmp = Npc {
        race: 0, // caller has already checked race
        x: x as i16,
        y: y as i16,
        goal: ai.goal.clone(),
        tactic: ai.tactic.clone(),
        state: ai.state.clone(),
        cleverness: ai.cleverness,
        facing: facing.dir,
        ..Npc::default()
    };

    let others_i32: Vec<(i32, i32)> = others.iter().map(|(ox, oy)| (*ox as i32, *oy as i32)).collect();
    let leader_idx = if is_leader { Some(0) } else { None };

    crate::game::npc_ai::tick_npc(
        &mut tmp,
        0,
        hero_x as i32, hero_y as i32,
        hero_dead,
        leader_idx,
        &others_i32,
        tick,
        xtype,
        turtle_eggs,
        freeze,
    );

    ai.goal    = tmp.goal;
    ai.tactic  = tmp.tactic;
    ai.state   = tmp.state;
    facing.dir = tmp.facing;
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test ecs::systems::npc_ai 2>&1 | grep -E "test.*ok|FAILED"
```
Expected: both NPC AI tests pass.

- [ ] **Step 5: Full suite, commit**

```bash
cargo test 2>&1 | grep "^test result"
git add src/game/ecs/systems/npc_ai.rs src/game/npc_ai.rs
git commit -m "feat(ecs): NpcAiSystem — AI decision pass using tick_npc_ecs adapter"
```

---

## Task 5: NpcMovementSystem

**Port from:** `src/game/gameplay_scene/actors.rs` — movement execution pass (lines 72–123).

**Files:**
- Modify: `src/game/ecs/systems/npc_movement.rs`

- [ ] **Step 1: Write failing test**

```rust
#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::components::*;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::spawn::*;
    use crate::game::npc::NpcState;
    use super::run;

    #[test]
    fn walking_npc_moves_toward_hero() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 200.0, 100.0, 0,
            HeroStats { vitality:100, brave:0, luck:0, kind:0,
                        wealth:0, hunger:0, fatigue:0, gold:0 },
            Inventory::empty());
        let mut res = Resources::new(hero);
        let start_x = 100.0f32;
        let enemy = spawn_enemy(&mut world, start_x, 100.0, 1, 0, 20, 0, 0, 3, 0, 0);
        world.get_mut::<AiState>(enemy).unwrap().state = NpcState::Walking;
        // Set speed
        world.get_mut::<Speed>(enemy).unwrap().speed = 3;
        run(&mut world, &mut res);
        let new_x = world.get::<&Position>(enemy).unwrap().x;
        // Should have moved east toward hero
        assert!(new_x > start_x, "NPC should move east: was {start_x}, now {new_x}");
    }

    #[test]
    fn frozen_npc_does_not_move() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 200.0, 100.0, 0,
            HeroStats { vitality:100, brave:0, luck:0, kind:0,
                        wealth:0, hunger:0, fatigue:0, gold:0 },
            Inventory::empty());
        let mut res = Resources::new(hero);
        res.clock.freeze_timer = 5;
        let enemy = spawn_enemy(&mut world, 100.0, 100.0, 1, 0, 20, 0, 0, 3, 0, 0);
        world.get_mut::<AiState>(enemy).unwrap().state = NpcState::Walking;
        run(&mut world, &mut res);
        let x = world.get::<&Position>(enemy).unwrap().x;
        assert_eq!(x, 100.0, "Frozen NPC should not move");
    }
}
```

- [ ] **Step 2: Implement `npc_movement.rs`**

```rust
//! NpcMovementSystem — executes movement for Walking enemy entities.
//! Port of update_actors() movement pass from gameplay_scene/actors.rs.

use hecs::World;
use crate::game::ecs::components::{Enemy, ArenaDummy, Position, Facing, AiState, Speed};
use crate::game::ecs::resources::Resources;
use crate::game::npc::NpcState;
use crate::game::collision;

pub fn run(world: &mut World, res: &mut Resources) {
    if res.clock.is_frozen() { return; }

    let hero_pos = match world.get::<&Position>(res.hero_entity) {
        Ok(p) => *p,
        Err(_) => return,
    };

    // Snapshot all enemy positions for collision avoidance.
    let snapshot: Vec<(hecs::Entity, f32, f32)> = world
        .query::<&Position>()
        .with::<&Enemy>()
        .iter()
        .map(|(e, p)| (e, p.x, p.y))
        .collect();

    let enemies: Vec<hecs::Entity> = world
        .query::<&AiState>()
        .with::<&Enemy>()
        .without::<&ArenaDummy>()
        .iter()
        .filter(|(_, ai)| ai.state == NpcState::Walking)
        .map(|(e, _)| e)
        .collect();

    let mut any_moved = false;

    for entity in enemies {
        let (facing, speed, old_x, old_y) = match world
            .query_one::<(&Facing, &Speed, &Position)>(entity)
        {
            Ok(mut q) => match q.get() {
                Some((f, s, p)) => (f.dir, s.speed as i32, p.x, p.y),
                None => continue,
            },
            Err(_) => continue,
        };

        // Build collision list: hero + other active, alive enemies.
        let others: Vec<(i32, i32)> = std::iter::once((hero_pos.x as i32, hero_pos.y as i32))
            .chain(
                snapshot.iter()
                    .filter(|(e, _, _)| *e != entity)
                    .map(|(_, x, y)| (*x as i32, *y as i32))
            )
            .collect();

        // Compute new position using existing collision helpers.
        let (dx, dy) = crate::game::gameplay_scene::push_offset_pub(facing, speed);
        let new_x = (old_x as i32 + dx).rem_euclid(0x8000) as f32;
        let new_y = (old_y as i32 + dy) as f32;

        let world_ref = res.map.world.as_ref();
        let can_move = collision::proxcheck(world_ref, new_x as i32, new_y as i32)
            && !collision::actor_collides(new_x as i32, new_y as i32, &others);

        if can_move {
            if let Ok(mut pos) = world.get_mut::<Position>(entity) {
                pos.x = new_x;
                pos.y = new_y;
                any_moved = true;
            }
        }
    }

    // fmain.c:1650 — any NPC's successful walk resets the hero's frustflag.
    if any_moved {
        if let Ok(mut frust) = world.get_mut::<crate::game::ecs::components::FrustFlag>(res.hero_entity) {
            frust.count = 0;
        }
    }
}
```

- [ ] **Step 3: Expose `push_offset` as `pub` in `gameplay_scene/mod.rs`**

Find `fn push_offset` in `src/game/gameplay_scene/mod.rs` and change to `pub(crate) fn push_offset` and add a public wrapper or re-export so `systems/npc_movement.rs` can call it. Alternatively, move it to `src/game/direction.rs` as `Direction::push_offset`.

The cleanest approach is to add a method to `Direction` in `direction.rs`:
```rust
impl Direction {
    /// Returns the (dx, dy) pixel offset for one step in this direction at the given distance.
    pub fn push_offset(self, distance: i32) -> (i32, i32) {
        match self {
            Direction::NW   => (-distance, -distance),
            Direction::N    => (0, -distance),
            Direction::NE   => (distance, -distance),
            Direction::E    => (distance, 0),
            Direction::SE   => (distance, distance),
            Direction::S    => (0, distance),
            Direction::SW   => (-distance, distance),
            Direction::W    => (-distance, 0),
            Direction::None => (0, 0),
        }
    }
}
```

Then update `gameplay_scene/mod.rs` to call `facing.push_offset(distance)` instead of its own private function, and `npc_movement.rs` to call `facing.push_offset(speed)`.

- [ ] **Step 4: Run tests**

```bash
cargo test ecs::systems::npc_movement 2>&1 | grep -E "test.*ok|FAILED"
```
Expected: both movement tests pass.

- [ ] **Step 5: Full suite, commit**

```bash
cargo test 2>&1 | grep "^test result"
git add src/game/ecs/systems/npc_movement.rs src/game/direction.rs src/game/gameplay_scene/mod.rs
git commit -m "feat(ecs): NpcMovementSystem — execute Walking enemy movement with collision"
```

---

## Task 6: DeathSystem

**Port from:** `src/game/gameplay_scene/scene_impl.rs` — dying/goodfairy logic; `src/game/gameplay_scene/` — brother succession.

**Files:**
- Modify: `src/game/ecs/systems/death.rs`

- [ ] **Step 1: Write failing test**

```rust
#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::components::*;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::spawn::*;
    use super::run;

    #[test]
    fn dead_hero_emits_brother_died_event() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0,
            HeroStats { vitality: 0, brave:0, luck:0, kind:0,
                        wealth:0, hunger:0, fatigue:0, gold:0 },
            Inventory::empty());
        let mut res = Resources::new(hero);
        // No luck gate, no goodfairy
        run(&mut world, &mut res);
        assert!(!res.events.brother.is_empty(),
            "Should emit BrotherDiedEvent when hero vitality <= 0");
    }

    #[test]
    fn alive_hero_emits_no_death_event() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0,
            HeroStats { vitality: 10, brave:0, luck:0, kind:0,
                        wealth:0, hunger:0, fatigue:0, gold:0 },
            Inventory::empty());
        let mut res = Resources::new(hero);
        run(&mut world, &mut res);
        assert!(res.events.brother.is_empty());
    }
}
```

- [ ] **Step 2: Implement `death.rs`**

```rust
//! DeathSystem — detects hero death and emits BrotherDiedEvent.
//! Successor spawning and Bones entity creation are handled by ItemSystem
//! consuming BrotherDiedEvent.
//! Port of dying/goodfairy logic from gameplay_scene/scene_impl.rs.
//! See docs/spec/death-revival.md.

use hecs::World;
use crate::game::ecs::components::{Hero, HeroStats, Position, Inventory, BrotherKind};
use crate::game::ecs::resources::Resources;
use crate::game::ecs::events::BrotherDiedEvent;

pub fn run(world: &mut World, res: &mut Resources) {
    let (vitality, luck) = match world
        .query_one::<&HeroStats>(res.hero_entity)
        .ok()
        .and_then(|mut q| q.get().map(|s| (s.vitality, s.luck)))
    {
        Some(v) => v,
        None => return,
    };

    if vitality > 0 { return; }

    // Already in dying sequence — goodfairy countdown.
    if res.encounter.dying {
        if res.encounter.goodfairy > 0 {
            res.encounter.goodfairy -= 1;
            return; // still counting down
        }
        // Goodfairy countdown expired — hero truly dead.
        res.encounter.dying = false;
    } else {
        // First death tick — check luck gate (SPEC §14.3).
        if !res.encounter.luck_gate_fired && luck > 0 {
            let roll = (res.clock.tick_counter.wrapping_mul(2654435761)) as i16;
            if roll.abs() % 100 < luck {
                // Luck saves the hero — restore 1 vitality.
                if let Ok(mut stats) = world.get_mut::<HeroStats>(res.hero_entity) {
                    stats.vitality = 1;
                }
                res.encounter.luck_gate_fired = true;
                return;
            }
        }
        // Start goodfairy countdown (SPEC §14.4: 60 ticks).
        res.encounter.dying = true;
        res.encounter.goodfairy = 60;
        return;
    }

    // Hero is confirmed dead — emit event.
    let (pos, inventory, brother_id) = {
        let pos = world.get::<&Position>(res.hero_entity).map(|p| *p).unwrap_or(Position::new(0.0, 0.0));
        let inv = world.get::<&Inventory>(res.hero_entity).map(|i| i.stuff).unwrap_or([0; 36]);
        let bid = world.get::<&BrotherKind>(res.hero_entity).map(|b| b.id).unwrap_or(0);
        (pos, inv, bid)
    };

    res.events.brother.push(BrotherDiedEvent {
        brother_id,
        x: pos.x,
        y: pos.y,
        stuff: inventory,
    });

    res.encounter.luck_gate_fired = false;
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test ecs::systems::death 2>&1 | grep -E "test.*ok|FAILED"
```
Expected: both death tests pass.

- [ ] **Step 4: Full suite, commit**

```bash
cargo test 2>&1 | grep "^test result"
git add src/game/ecs/systems/death.rs
git commit -m "feat(ecs): DeathSystem — hero death detection and BrotherDiedEvent"
```

---

## Task 7: Remaining systems (ZoneSystem, DoorSystem, MissileSystem, EncounterSystem, ProximitySystem, ItemSystem, NarrativeSystem, RegionSystem, render systems)

Each remaining system follows the same pattern as Tasks 2–6:
1. Write failing test(s) covering the core behavior
2. Implement `run()` by porting from the corresponding `GameplayScene` method
3. Run targeted tests
4. Run full suite
5. Commit with message `feat(ecs): <SystemName> — <one-line description>`

The detailed port logic for each is in the corresponding `GameplayScene` sub-module. Reference the spec files in `docs/spec/` for behavioral correctness. Key source locations:

| System | Port from | Spec |
|---|---|---|
| `zone.rs` | `gameplay_scene/` zone checks | `docs/spec/ai-encounters.md` |
| `door.rs` | `gameplay_scene/` door bump logic | `docs/spec/doors-buildings.md` |
| `missile.rs` | `gameplay_scene/actors.rs` dragon/archer firing + `combat.rs` | `docs/spec/combat.md` |
| `encounter.rs` | `encounter.rs` `try_trigger_encounter` + `spawn_encounter_group` | `docs/spec/ai-encounters.md` |
| `proximity.rs` | `gameplay_scene/proximity.rs` | `docs/spec/npcs-dialogue.md` |
| `item.rs` | `gameplay_scene/items.rs` | `docs/spec/inventory-items.md` |
| `narrative.rs` | `gameplay_scene/` narrative queue | `docs/spec/intro-narrative.md` |
| `region.rs` | `gameplay_scene/region.rs` `on_region_changed` | `docs/spec/world-structure.md` |
| `render/palette.rs` | `gameplay_scene/region.rs` palette computation | `docs/spec/palettes-daynight-visuals.md` |
| `render/map.rs` | `map_renderer.rs` `compose()` | `docs/spec/display-rendering.md` |
| `render/sprite.rs` | `gameplay_scene/rendering.rs` actor blit loop | `docs/spec/characters-animation.md` |
| `render/hibar.rs` | `gameplay_scene/rendering.rs` `render_hibar()` | `docs/spec/ui-menus.md` |

For each remaining system, commit individually so rollback is cheap. Minimum test coverage: one test for the primary behavior and one for an edge case (empty world, no-op condition, etc.).

- [ ] **Step 1: Implement ZoneSystem with tests**
- [ ] **Step 2: Implement DoorSystem with tests**
- [ ] **Step 3: Implement MissileSystem with tests**
- [ ] **Step 4: Implement EncounterSystem with tests**
- [ ] **Step 5: Implement ProximitySystem with tests**
- [ ] **Step 6: Implement ItemSystem with tests**
- [ ] **Step 7: Implement NarrativeSystem with tests**
- [ ] **Step 8: Implement RegionSystem with tests**
- [ ] **Step 9: Implement PaletteSystem with tests**
- [ ] **Step 10: Implement MapRenderSystem with tests**
- [ ] **Step 11: Implement SpriteRenderSystem with tests**
- [ ] **Step 12: Implement HiBarRenderSystem with tests**

---

## Completion check

All systems exist as standalone functions. All existing tests (≥712) still pass. New system tests added: ≥2 per system (≥36 new tests total).

```bash
cargo test 2>&1 | grep "^test result"
```

No `GameplayScene` code has been modified. The old code path continues to work.
