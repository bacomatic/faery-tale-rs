---
title: "Plan O — Encounter System"
plan: O
status: draft
depends_on: [G, N]
touches:
  - src/game/ecs/systems/encounter.rs
  - src/game/ecs/resources.rs
  - src/game/ecs/scene.rs
---

# ECS Migration Plan O: Encounter System

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `EncounterSystem` to trigger random enemy encounters during gameplay based on terrain type, day/night cycle, and probability tables; spawn enemy groups using the encounter chart; and manage encounter state through `Resources.encounter` and `Resources.region`.

**Architecture:** `encounter::run()` is called every gameplay tick from `EcsScene::run_tick()`. It applies a 5-gate filter, selects an encounter type from the chart, and spawns a group of up to 4 enemies scattered around an origin point 150–213 px from the hero. Encounter state (current type, encounter number) is written into `Resources.region`.

**Prerequisites:** Plans A–D complete. Plan G (region loaded, `xtype` available). Plan N (combat system functional).

**Tech Stack:** Rust 2021, `hecs = "0.11"`, SDL3.

---

## File map

| File | Change |
|---|---|
| `src/game/ecs/systems/encounter.rs` | **Create** — `run()` + `try_trigger_encounter()` + `spawn_encounter_group()` + unit tests |
| `src/game/ecs/resources.rs` | Add `EncounterTables` struct and `encounter_tables` field to `Resources` |
| `src/game/ecs/scene.rs` | Verify `encounter::run()` is present in `run_tick()` schedule |

---

## Context

### Trigger gates (from legacy `encounter.rs`)

All five gates must pass before any spawn is attempted:

| Gate | Condition | Meaning |
|------|-----------|---------|
| 1 | `tick & 31 == 0` | Only attempt every 32 ticks |
| 2 | `active_enemy_count < 4` | At most 3 enemies already on screen |
| 3 | `xtype < 50` | Not in a forced-peace or town zone |
| 4 | `res.carrier_entity.is_none()` | Hero is not riding a carrier |
| 5 | Danger roll | `rand64() <= (2 + xtype)` outdoor (region ≤ 7) or `(5 + xtype)` indoor |

Gates are checked in order; short-circuit on the first failure. Gate 5 uses region index to distinguish outdoor (`region_num <= 7`) from indoor.

### Encounter chart

`ENCOUNTER_CHART_FULL[11]` lives in `src/game/encounter.rs`. Each entry is an `EnemyTypeStats`:

| Index | Name | Notable fields |
|-------|------|---------------|
| 0 | ogre | high HP, low cleverness |
| 1 | orc | mid HP |
| 2 | wraith | low HP, high cleverness |
| 3 | skeleton | low HP |
| 4 | snake | low HP, swamp only |
| 5 | salamander | — |
| 6 | spider | forced in spider region |
| 7 | dark_knight | — |
| 8 | loraii | — |
| 9 | necromancer | high cleverness |
| 10 | woodcutter | — |

`ENCOUNTER_TO_NPC_TYPE[11]` maps encounter index → npc_type constant used by the spawn helpers.

`WEAPON_PROBS[32]` is indexed by `arms * 4 + rand4()` → weapon code. `rand4()` returns 0–3.

### Encounter type selection

```
base = rand4()  // 0–3: ogre / orc / wraith / skeleton
if xtype == 7 (swamp):
    wraith (2) → remap to snake (4)
if xtype == 8 (spider region):
    force spider (6)
if xtype == 49:
    force wraith (2)
```

### Spawn group geometry

- **Origin**: 150–213 px from hero in a random direction (uniformly sampled angle, radius = `150 + rand64() % 64`).
- **Per-enemy scatter**: ±31 px around origin. Up to 15 position retries per enemy.
- **Collision rejection**: reject using the original game's actor bounding box (`collision.rs::actor_collides`): a position collides when `|dx| < 11` AND `|dy| < 9` relative to the hero or any already-placed enemy. (NOT a 32 px radius.) Also reject out-of-bounds positions.
- Spawn at most 4 enemies per encounter event regardless of group composition.

### `EnemyTypeStats` fields (for reference)

```rust
// Actual struct in src/game/encounter.rs — note field names `clever` and `cfile`.
pub struct EnemyTypeStats {
    pub hp:        i16,
    pub arms:      u8,
    pub clever:    u8,
    pub treasure:  u8,
    pub cfile:     u8,
}
```

---

## Task 1: Add `EncounterTables` to `resources.rs`

**Files:**
- Modify: `src/game/ecs/resources.rs`

- [ ] **Step 1: Define `EncounterTables`**

```rust
/// Static lookup tables used by EncounterSystem.
/// All fields are references to statics in `src/game/encounter.rs`.
pub struct EncounterTables {
    pub chart:        &'static [crate::game::encounter::EnemyTypeStats; 11],
    pub npc_type_map: &'static [u8; 11],
    pub weapon_probs: &'static [u8; 32],
}

impl Default for EncounterTables {
    fn default() -> Self {
        Self {
            chart:        &crate::game::encounter::ENCOUNTER_CHART_FULL,
            npc_type_map: &crate::game::encounter::ENCOUNTER_TO_NPC_TYPE,
            weapon_probs: &crate::game::encounter::WEAPON_PROBS,
        }
    }
}
```

- [ ] **Step 2: Add field to `Resources`**

In `Resources`, add:

```rust
pub encounter_tables: EncounterTables,
```

- [ ] **Step 3: Initialize in `Resources::new()`**

```rust
encounter_tables: EncounterTables::default(),
```

- [ ] **Step 4: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no new errors from this change.

- [ ] **Step 5: Commit**

```bash
git add src/game/ecs/resources.rs
git commit -m "feat(ecs): add EncounterTables to Resources"
```

---

## Task 2: Implement `try_trigger_encounter()`

**Files:**
- Create/modify: `src/game/ecs/systems/encounter.rs`

This helper encapsulates the 5-gate logic and returns the chosen encounter type index (0–10) if all gates pass, or `None` otherwise.

- [ ] **Step 1: Implement the helper**

```rust
/// Apply all five encounter trigger gates.
/// Returns Some(encounter_type 0–10) if an encounter should fire, None otherwise.
fn try_trigger_encounter(
    world: &hecs::World,
    res: &Resources,
) -> Option<usize> {
    // Gate 1: tick cadence
    if res.clock.tick & 31 != 0 {
        return None;
    }

    // Gate 2: enemy cap — count active (alive) enemies
    let active_count = count_active_enemies(world);
    if active_count >= 4 {
        return None;
    }

    // Gate 3: peace zone
    let xtype = res.region.xtype as i32;
    if xtype >= 50 {
        return None;
    }

    // Gate 4: not riding a carrier
    if res.carrier_entity.is_some() {
        return None;
    }

    // Gate 5: danger roll
    let threshold = if res.region.region_num <= 7 {
        2 + xtype
    } else {
        5 + xtype
    };
    if crate::game::rng::rand64() > threshold as u64 {
        return None;
    }

    // Select encounter type
    Some(select_encounter_type(xtype as u8))
}
```

- [ ] **Step 2: Implement `count_active_enemies()`**

```rust
fn count_active_enemies(world: &hecs::World) -> usize {
    world
        .query::<(&crate::game::ecs::components::Enemy,
                  &crate::game::ecs::components::Health)>()
        .iter()
        .filter(|(_, (_, hp))| !hp.is_dead())
        .count()
}
```

- [ ] **Step 3: Implement `select_encounter_type()`**

```rust
fn select_encounter_type(xtype: u8) -> usize {
    // Forced overrides first
    if xtype == 49 {
        return 2; // wraith
    }
    if xtype == 8 {
        return 6; // spider
    }

    // Base random selection: 0–3
    let mut enc = crate::game::rng::rand4() as usize;

    // Swamp: wraith → snake
    if xtype == 7 && enc == 2 {
        enc = 4; // snake
    }

    enc
}
```

- [ ] **Step 4: Commit**

```bash
git add src/game/ecs/systems/encounter.rs
git commit -m "feat(ecs): implement try_trigger_encounter() with 5-gate logic"
```

---

## Task 3: Implement `spawn_encounter_group()`

**Files:**
- Modify: `src/game/ecs/systems/encounter.rs`

- [ ] **Step 1: Implement the spawn helper**

```rust
/// Spawn up to 4 enemies of `encounter_type` around the hero.
/// Returns the number of enemies actually spawned.
fn spawn_encounter_group(
    world: &mut hecs::World,
    res: &Resources,
    encounter_type: usize,
) -> usize {
    // Re-check cap (race-guard — should already be < 4 from try_trigger_encounter).
    if count_active_enemies(world) >= 4 {
        return 0;
    }

    let tables = &res.encounter_tables;
    let stats  = &tables.chart[encounter_type];
    let npc_type = tables.npc_type_map[encounter_type];

    // Compute origin: 150–213 px from hero in random direction.
    let hero_pos = hero_position(world, res);
    let origin   = random_origin(hero_pos, 150, 213);

    // Collect occupied positions: hero + already-active enemies.
    let mut occupied: Vec<(f32, f32)> = occupied_positions(world, res, hero_pos);

    let mut spawned = 0usize;

    for _ in 0..4 {
        if let Some(pos) = find_spawn_position(origin, &occupied, 15, 31.0) {
            let weapon = tables.weapon_probs
                [(stats.arms as usize * 4)
                 + crate::game::rng::rand4() as usize % 4];
            // Race is derived from the encounter type, NOT hardcoded to 0.
            // Matches src/game/encounter.rs::spawn_encounter (combat immunity
            // logic in Plan N depends on the correct race).
            let race = match encounter_type {
                2     => crate::game::npc::RACE_WRAITH, // wraith
                3 | 5 => crate::game::npc::RACE_UNDEAD, // skeleton / salamander
                4     => crate::game::npc::RACE_SNAKE,  // snake
                _     => crate::game::npc::RACE_ENEMY,
            };
            crate::game::ecs::spawn::spawn_enemy(
                world,
                pos.0, pos.1,
                npc_type,
                race,
                stats.hp,
                weapon,
                stats.treasure,
                4,                  // speed (default)
                stats.clever,
                stats.cfile,
            );
            occupied.push(pos);
            spawned += 1;
        }
    }

    spawned
}
```

- [ ] **Step 2: Implement geometry helpers**

```rust
fn hero_position(world: &hecs::World, res: &Resources) -> (f32, f32) {
    world
        .get::<&crate::game::ecs::components::Position>(res.hero_entity)
        .map(|p| (p.x, p.y))
        .unwrap_or((0.0, 0.0))
}

fn random_origin(hero: (f32, f32), min_r: u32, max_r: u32) -> (f32, f32) {
    let r     = min_r + (crate::game::rng::rand64() as u32 % (max_r - min_r));
    let angle = (crate::game::rng::rand64() as f32) * std::f32::consts::TAU
                / u64::MAX as f32;
    (hero.0 + r as f32 * angle.cos(),
     hero.1 + r as f32 * angle.sin())
}

fn occupied_positions(
    world: &hecs::World,
    res: &Resources,
    hero_pos: (f32, f32),
) -> Vec<(f32, f32)> {
    use crate::game::ecs::components::{Enemy, Health, Position};
    let mut out = vec![hero_pos];
    for (_, (_, _, pos)) in world
        .query::<(&Enemy, &Health, &Position)>()
        .iter()
        .filter(|(_, (_, hp, _))| !hp.is_dead())
    {
        out.push((pos.x, pos.y));
    }
    out
}

/// Try up to `max_tries` scatter positions around `origin` (±`scatter` px).
/// Accept the first position that does not collide (per the original game's
/// actor bounding box: |dx| < 11 AND |dy| < 9) with any occupied slot.
fn find_spawn_position(
    origin: (f32, f32),
    occupied: &[(f32, f32)],
    max_tries: u32,
    scatter: f32,
) -> Option<(f32, f32)> {
    let scatter_range = scatter as i32 * 2 + 1;
    for _ in 0..max_tries {
        let dx = (crate::game::rng::rand64() as i32 % scatter_range) - scatter as i32;
        let dy = (crate::game::rng::rand64() as i32 % scatter_range) - scatter as i32;
        let pos = (origin.0 + dx as f32, origin.1 + dy as f32);
        // Reuse the canonical actor bounding-box check from collision.rs.
        let others: Vec<(i32, i32)> =
            occupied.iter().map(|&(ox, oy)| (ox as i32, oy as i32)).collect();
        if !crate::game::collision::actor_collides(pos.0 as i32, pos.1 as i32, &others) {
            return Some(pos);
        }
    }
    None
}
```

- [ ] **Step 3: Commit**

```bash
git add src/game/ecs/systems/encounter.rs
git commit -m "feat(ecs): implement spawn_encounter_group() with geometry helpers"
```

---

## Task 4: Implement `encounter::run()`

**Files:**
- Modify: `src/game/ecs/systems/encounter.rs`

- [ ] **Step 1: Write `run()`**

```rust
use hecs::World;
use crate::game::ecs::resources::Resources;

pub fn run(world: &mut World, res: &mut Resources) {
    if let Some(encounter_type) = try_trigger_encounter(world, res) {
        let n = spawn_encounter_group(world, res, encounter_type);
        if n > 0 {
            res.region.encounter_type   = encounter_type as u8;
            res.region.encounter_number = res.region.encounter_number.wrapping_add(1);
        }
    }
}
```

`encounter_type` and `encounter_number` must be fields on `res.region` (type `RegionState` or equivalent). Add them if they do not already exist — see Task 5.

- [ ] **Step 2: Verify `encounter::run()` is in the `run_tick()` schedule**

Open `src/game/ecs/scene.rs` and confirm the line:

```rust
systems::encounter::run(&mut self.world, &mut self.res);
```

is present in `run_tick()` between `systems::missile::run` and `systems::proximity::run`, matching the schedule shown in Plan D.

- [ ] **Step 3: Compile check**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/game/ecs/systems/encounter.rs src/game/ecs/scene.rs
git commit -m "feat(ecs): implement encounter::run() and wire into system schedule"
```

---

## Task 5: Add `encounter_type` and `encounter_number` to `RegionState`

**Files:**
- Modify: `src/game/ecs/resources.rs`

If `RegionState` does not already contain these fields, add them:

- [ ] **Step 1: Add fields**

```rust
pub struct RegionState {
    // ... existing fields ...
    /// Index (0–10) of the last triggered encounter type.
    pub encounter_type:   u8,
    /// Running count of encounters triggered this session.
    pub encounter_number: u8,
}
```

- [ ] **Step 2: Initialize in `RegionState::default()` / `new()`**

```rust
encounter_type:   0,
encounter_number: 0,
```

- [ ] **Step 3: Commit**

```bash
git add src/game/ecs/resources.rs
git commit -m "feat(ecs): add encounter_type/encounter_number fields to RegionState"
```

---

## Task 6: Unit tests

**Files:**
- Modify: `src/game/ecs/systems/encounter.rs`

Add a `#[cfg(test)]` module at the bottom of `encounter.rs`.

- [ ] **Step 1: Test — no spawn in peace zone**

```rust
#[test]
fn no_spawn_in_peace_zone() {
    let (mut world, mut res) = test_fixtures::minimal();
    res.clock.tick       = 0;   // gate 1 passes
    res.region.xtype     = 50;  // gate 3 fails
    res.carrier_entity   = None;
    assert!(try_trigger_encounter(&world, &res).is_none());
}
```

- [ ] **Step 2: Test — no spawn on non-gated tick**

```rust
#[test]
fn no_spawn_on_non_gated_tick() {
    let (mut world, mut res) = test_fixtures::minimal();
    res.clock.tick       = 1;   // gate 1 fails: 1 & 31 != 0
    res.region.xtype     = 10;
    res.carrier_entity   = None;
    assert!(try_trigger_encounter(&world, &res).is_none());
}
```

- [ ] **Step 3: Test — no spawn while riding carrier**

```rust
#[test]
fn no_spawn_while_riding_carrier() {
    let (mut world, mut res) = test_fixtures::minimal();
    res.clock.tick     = 0;
    res.region.xtype   = 10;
    res.carrier_entity = Some(world.spawn(()));   // gate 4 fails
    assert!(try_trigger_encounter(&world, &res).is_none());
}
```

- [ ] **Step 4: Test — enemy cap respected**

```rust
#[test]
fn enemy_cap_blocks_spawn() {
    use crate::game::ecs::components::{Enemy, Health};
    let (mut world, mut res) = test_fixtures::minimal();
    // Spawn 4 live enemies.
    for _ in 0..4 {
        world.spawn((Enemy, Health::new(10)));
    }
    res.clock.tick   = 0;
    res.region.xtype = 10;
    res.carrier_entity = None;
    // Gate 2 fails: 4 active enemies already present.
    assert!(try_trigger_encounter(&world, &res).is_none());
}
```

- [ ] **Step 5: Test — `EncounterTables::default()` provides valid references**

```rust
#[test]
fn encounter_tables_default_is_valid() {
    let tables = super::super::resources::EncounterTables::default();
    assert_eq!(tables.chart.len(),        11);
    assert_eq!(tables.npc_type_map.len(), 11);
    assert_eq!(tables.weapon_probs.len(), 32);
}
```

- [ ] **Step 6: Run tests**

```bash
cargo test -p faery-tale-rs encounter 2>&1
```

Expected: 5 tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/game/ecs/systems/encounter.rs
git commit -m "test(ecs): add 5 unit tests for EncounterSystem gates and tables"
```

---

## Spec references

- `docs/spec/ai-encounters.md` §12 — encounter generation, danger levels, monster type selection
- `docs/reqs/ai-encounters.md` R-AI-023 to R-AI-028
- `reference/logic/encounters.md` (research branch) — complete encounter mechanics including gate logic, type selection tables, and spawn geometry

---

## Dependencies

| Plan | Reason |
|------|--------|
| A–D | ECS scaffolding, components, resources, scene |
| G | Region system — provides `xtype`, `region_num` in `res.region` |
| N | Combat system — `Health::is_dead()`, combat components must exist |

---

## Completion check

```bash
cargo build 2>&1 | grep "^error"
cargo test -p faery-tale-rs encounter 2>&1 | grep "^test result"
```

Both succeed. `EncounterSystem` is live in the tick schedule and all 5 unit tests pass.
