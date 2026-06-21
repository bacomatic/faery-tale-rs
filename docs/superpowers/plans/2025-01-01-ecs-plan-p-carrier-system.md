---
title: "Plan P — Carrier / Transport System"
plan: P
status: draft
depends_on: []
touches:
  - src/game/ecs/systems/carrier.rs
  - src/game/ecs/scene.rs
---

# ECS Migration Plan P: Carrier / Transport System

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `CarrierSystem` to manage raft, turtle, swan, and dragon carrier mount state, mount/dismount detection, autonomous movement, and sprite rendering.

**Architecture:** `carrier::run()` is called every gameplay tick from `EcsScene::run_tick()`. It queries `CarrierMount` on the hero entity and dispatches to the appropriate sub-handler (raft, turtle, swan). Dragon is a stationary hostile NPC and is not ridden; it is handled entirely by the combat system. A carrier rendering pass is added to `blit_actors_inner()` in `scene.rs` so carrier sprites are drawn after enemies but before the HUD.

**Prerequisites:** Plans A–D complete. No other plan dependencies — this is a standalone system.

**Tech Stack:** Rust 2021, `hecs = "0.11"`, SDL3.

---

## File map

| File | Change |
|---|---|
| `src/game/ecs/systems/carrier.rs` | **Create** — `run()` with raft, turtle, swan sub-handlers + unit tests |
| `src/game/ecs/scene.rs` | Add carrier rendering pass to `blit_actors_inner()` |

---

## Context

### `CarrierMount` component (`components.rs` lines 148–159)

```rust
pub struct CarrierMount {
    pub riding:          i16,  // 0=none, 1=raft, 5=turtle, 11=swan
    pub flying:          i16,  // 1=flying (swan only)
    pub swan_vx:         f32,
    pub swan_vy:         f32,
    pub active_carrier:  i16,  // 0=none, 5=raft, 6=turtle, 1=swan (CARRIER_* in game_state.rs)
    pub on_raft:         bool,
    pub raftprox:        i16,
    pub wcarry:          u8,   // actor slot: 1=raft, 3=turtle/swan
}
```

`CarrierMount` is a component on the hero entity only. All three carrier types read and write it via `world.get_mut::<CarrierMount>(res.hero_entity)`.

### Inventory slots referenced

| `stuff` index | Item |
|---------------|------|
| `stuff[5]` | Golden lasso (required for swan) |
| `stuff[6]` | Turtle shell (consumed on turtle summon) |

### Terrain constants

| Value | Terrain |
|-------|---------|
| 3–5 | Water / shore (raft auto-mounts) |
| 5 | Deep water (turtle navigates to) |
| 8 | Lava (blocks swan dismount) |

### `CarrierKind` component (on carrier entities)

```rust
pub enum CarrierKind { Raft, Turtle, Swan }
```

Carrier entities are spawned into the world with `(Carrier, Position, Facing, CarrierKind, SpriteRef)`. `res.carrier_entity` holds the current carrier's `Entity` handle, or `None` when the hero is not mounted.

### Sprite sheet mapping

| Carrier | cfile index | Frame selection |
|---------|-------------|----------------|
| Raft | 4 | frame 0 |
| Turtle | 5 | frame 0 |
| Swan (flying) | 11 | `facing.dir` |
| Swan (grounded) | 4 | frame 1 |

### Integration points

- `door::run()`: check `res.carrier_entity.is_some()` to block door entry while mounted.
- `encounter::run()`: check `res.carrier_entity.is_some()` to suppress encounters (Plan O, Gate 4).
- `movement::run()`: when `mount.flying == 1`, skip terrain collision for the hero.

---

## Task 1: Implement raft logic

**Files:**
- Create: `src/game/ecs/systems/carrier.rs`

The raft auto-mounts when the hero steps onto water terrain and auto-dismounts when the hero reaches land. There is no explicit player action — proximity and terrain drive the transition.

- [ ] **Step 1: Implement `update_raft()`**

```rust
fn update_raft(world: &mut World, res: &mut Resources) {
    let hero_pos  = hero_position(world, res);
    let terrain   = res.region.terrain_at(hero_pos.0 as i32, hero_pos.1 as i32);
    let on_water  = matches!(terrain, 3..=5);

    let mut mount = world
        .get_mut::<CarrierMount>(res.hero_entity)
        .expect("hero must have CarrierMount");

    if mount.riding == 0 && on_water {
        // Mount: look for a raft entity within 9 px.
        if let Some((raft_ent, raft_pos)) = nearest_carrier(world, res, CarrierKind::Raft) {
            let dist = dist2(hero_pos, raft_pos).sqrt();
            if dist <= 9.0 {
                mount.riding         = 1;
                mount.active_carrier = 5;  // CARRIER_RAFT
                mount.on_raft        = true;
                mount.wcarry         = 1;
                res.carrier_entity   = Some(raft_ent);
            }
        }
    } else if mount.riding == 1 {
        // Tick: snap raft to hero position.
        if let Some(raft_ent) = res.carrier_entity {
            if let Ok(mut rpos) = world.get_mut::<Position>(raft_ent) {
                rpos.x = hero_pos.0;
                rpos.y = hero_pos.1;
            }
        }
        // Dismount: no longer on water terrain.
        if !on_water {
            mount.riding         = 0;
            mount.active_carrier = 0;
            mount.on_raft        = false;
            mount.wcarry         = 0;
            res.carrier_entity   = None;
        }
    }
}
```

- [ ] **Step 2: Implement `nearest_carrier()` helper**

```rust
fn nearest_carrier(
    world: &World,
    res: &Resources,
    kind: CarrierKind,
) -> Option<(Entity, (f32, f32))> {
    use crate::game::ecs::components::{CarrierKind as CK, Position};
    let hero = hero_position(world, res);
    world
        .query::<(&CK, &Position)>()
        .iter()
        .filter(|(_, (k, _))| std::mem::discriminant(*k) == std::mem::discriminant(&kind))
        .min_by(|(_, (_, a)), (_, (_, b))| {
            dist2(hero, (a.x, a.y))
                .partial_cmp(&dist2(hero, (b.x, b.y)))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(e, (_, p))| (e, (p.x, p.y)))
}
```

- [ ] **Step 3: Commit**

```bash
git add src/game/ecs/systems/carrier.rs
git commit -m "feat(ecs): implement raft mount/dismount/snap logic in CarrierSystem"
```

---

## Task 2: Implement turtle logic

**Files:**
- Modify: `src/game/ecs/systems/carrier.rs`

The turtle is summoned on-demand by the player using a shell item. Once summoned, it moves autonomously toward deep water. The hero mounts and dismounts by proximity.

- [ ] **Step 1: Implement `update_turtle()`**

```rust
fn update_turtle(world: &mut World, res: &mut Resources) {
    let hero_pos = hero_position(world, res);

    let riding = {
        let mount = world
            .get::<CarrierMount>(res.hero_entity)
            .expect("hero must have CarrierMount");
        mount.riding
    };

    // Summon: action pressed + shell in inventory + not already riding + not in central region.
    if riding == 0
        && res.input.action_just_pressed()
        && inventory_count(world, res, 6) > 0
        && !res.region.is_central()
    {
        decrement_inventory(world, res, 6);
        let turtle = crate::game::ecs::spawn::spawn_turtle(world, hero_pos.0, hero_pos.1);
        let mut mount = world
            .get_mut::<CarrierMount>(res.hero_entity)
            .expect("hero must have CarrierMount");
        mount.riding         = 5;
        mount.active_carrier = 6;  // CARRIER_TURTLE
        mount.wcarry         = 3;
        res.carrier_entity   = Some(turtle);
        return;
    }

    // Autonomous turtle movement: probe 4 directions for terrain 5 (deep water).
    if let Some(turtle_ent) = res.carrier_entity {
        if riding == 5 {
            autonomous_turtle_move(world, res, turtle_ent);
            // Dismount: hero has drifted > 16 px from turtle.
            if let Ok(tpos) = world.get::<Position>(turtle_ent) {
                let d = dist2(hero_pos, (tpos.x, tpos.y)).sqrt();
                if d > 16.0 {
                    let mut mount = world
                        .get_mut::<CarrierMount>(res.hero_entity)
                        .expect("hero must have CarrierMount");
                    mount.riding         = 0;
                    mount.active_carrier = 0;
                    mount.wcarry         = 0;
                    res.carrier_entity   = None;
                }
            }
        } else if riding == 0 {
            // Mount: turtle already exists, hero walks within 16 px.
            if let Ok(tpos) = world.get::<Position>(turtle_ent) {
                let d = dist2(hero_pos, (tpos.x, tpos.y)).sqrt();
                if d <= 16.0 {
                    let mut mount = world
                        .get_mut::<CarrierMount>(res.hero_entity)
                        .expect("hero must have CarrierMount");
                    mount.riding         = 5;
                    mount.active_carrier = 6;  // CARRIER_TURTLE
                    mount.wcarry         = 3;
                }
            }
        }
    }
}
```

- [ ] **Step 2: Implement `autonomous_turtle_move()`**

```rust
/// Move the turtle one step per tick toward terrain-5 (deep water).
/// Probes 4 cardinal directions; moves to the first valid deep-water position.
/// Turtle speed is always 3 px (spec §21.3 / fmain.c:1521-1522).
fn autonomous_turtle_move(world: &mut World, res: &Resources, turtle: Entity) {
    const DIRS: [(f32, f32); 4] = [(0.0, -3.0), (0.0, 3.0), (-3.0, 0.0), (3.0, 0.0)];
    let current = world
        .get::<Position>(turtle)
        .map(|p| (p.x, p.y))
        .unwrap_or_default();

    for &(dx, dy) in &DIRS {
        let nx = current.0 + dx;
        let ny = current.1 + dy;
        let t  = res.region.terrain_at(nx as i32, ny as i32);
        if t == 5 {
            if let Ok(mut pos) = world.get_mut::<Position>(turtle) {
                pos.x = nx;
                pos.y = ny;
            }
            return;
        }
    }
    // No deep-water neighbor found; turtle stays put.
}
```

- [ ] **Step 3: Commit**

```bash
git add src/game/ecs/systems/carrier.rs
git commit -m "feat(ecs): implement turtle summon, autonomous movement, and proximity mount"
```

---

## Task 3: Implement swan logic

**Files:**
- Modify: `src/game/ecs/systems/carrier.rs`

The swan provides inertial flight. Velocity accumulates from directional input each tick, is clamped to ±32 (x) / ±40 (y), and the position is updated by `vx/4, vy/4`. The outdoor coordinate space wraps at `0x8000`. Dismount requires the action button, low speed, passable terrain, and non-lava ground.

- [ ] **Step 1: Implement `update_swan()`**

```rust
/// X-axis velocity deltas indexed by 8-direction `dir` (0=N, clockwise).
const XDIR: [f32; 8] = [0.0, 8.0, 8.0, 8.0, 0.0, -8.0, -8.0, -8.0];
/// Y-axis velocity deltas indexed by direction.
const YDIR: [f32; 8] = [-8.0, -8.0, 0.0, 8.0, 8.0, 8.0, 0.0, -8.0];

fn update_swan(world: &mut World, res: &mut Resources) {
    let hero_pos = hero_position(world, res);
    let riding = {
        let mount = world.get::<CarrierMount>(res.hero_entity).expect("CarrierMount");
        mount.riding
    };

    // Board: hero within 16 px of a swan entity + golden lasso in inventory.
    if riding == 0 && inventory_count(world, res, 5) > 0 {
        if let Some((swan_ent, swan_pos)) = nearest_carrier(world, res, CarrierKind::Swan) {
            if dist2(hero_pos, swan_pos).sqrt() <= 16.0 {
                let mut mount = world.get_mut::<CarrierMount>(res.hero_entity).expect("CarrierMount");
                mount.riding         = 11;
                mount.active_carrier = 1;  // CARRIER_SWAN
                mount.flying         = 1;
                mount.swan_vx        = 0.0;
                mount.swan_vy        = 0.0;
                mount.wcarry         = 3;
                res.carrier_entity   = Some(swan_ent);
            }
        }
        return;
    }

    if riding != 11 {
        return;
    }

    let dir = res.input.direction() as usize;

    // Accumulate inertial velocity from input direction.
    let (new_vx, new_vy, new_fx, new_fy) = {
        let mount = world.get::<CarrierMount>(res.hero_entity).expect("CarrierMount");
        let vx = (mount.swan_vx + XDIR[dir]).clamp(-32.0, 32.0);
        let vy = (mount.swan_vy + YDIR[dir]).clamp(-40.0, 40.0);
        let fx = vx / 4.0;
        let fy = vy / 4.0;
        (vx, vy, fx, fy)
    };

    // Update position on hero entity with outdoor coordinate wrap.
    if let Ok(mut pos) = world.get_mut::<Position>(res.hero_entity) {
        pos.x = (pos.x + new_fx) as i16 as f32;
        let raw_x = pos.x as i16;
        pos.x = raw_x.rem_euclid(0x4000) as f32; // 0x8000 / 2 for tile space
        pos.y += new_fy;
    }

    // Write updated velocities back.
    {
        let mut mount = world.get_mut::<CarrierMount>(res.hero_entity).expect("CarrierMount");
        mount.swan_vx = new_vx;
        mount.swan_vy = new_vy;
    }

    // Mirror position onto swan entity.
    if let Some(swan_ent) = res.carrier_entity {
        if let Ok(mut spos) = world.get_mut::<Position>(swan_ent) {
            let hpos = hero_position(world, res);
            spos.x = hpos.0;
            spos.y = hpos.1;
        }
    }

    // Dismount: action button + speed < 15 + passable terrain + not lava.
    if res.input.action_just_pressed() {
        let speed = {
            let mount = world.get::<CarrierMount>(res.hero_entity).expect("CarrierMount");
            mount.swan_vx.abs().max(mount.swan_vy.abs())
        };
        let pos    = hero_position(world, res);
        let t      = res.region.terrain_at(pos.0 as i32, pos.1 as i32);
        let safe   = t != 8 && res.region.terrain_passable(t);

        if speed < 15.0 && safe {
            let mut mount = world.get_mut::<CarrierMount>(res.hero_entity).expect("CarrierMount");
            mount.riding         = 0;
            mount.active_carrier = 0;
            mount.flying         = 0;
            mount.swan_vx        = 0.0;
            mount.swan_vy        = 0.0;
            mount.wcarry         = 0;
            res.carrier_entity   = None;
        }
    }
}
```

- [ ] **Step 2: Commit**

```bash
git add src/game/ecs/systems/carrier.rs
git commit -m "feat(ecs): implement swan inertial flight, outdoor wrap, and dismount logic"
```

---

## Task 4: Implement `carrier::run()`

**Files:**
- Modify: `src/game/ecs/systems/carrier.rs`

- [ ] **Step 1: Write the public `run()` entry point**

```rust
use hecs::World;
use crate::game::ecs::resources::Resources;

pub fn run(world: &mut World, res: &mut Resources) {
    update_raft(world, res);
    update_turtle(world, res);
    update_swan(world, res);
}
```

All three sub-handlers are safe to call every tick — each checks `mount.riding` internally and returns immediately if the relevant carrier is not active.

- [ ] **Step 2: Verify `carrier::run()` is in the schedule**

Confirm `src/game/ecs/scene.rs` contains, in order:

```rust
systems::movement::run(&mut self.world, &mut self.res);
systems::carrier::run(&mut self.world, &mut self.res);
systems::collision::run(&self.world, &mut self.res);
```

This matches the Plan D schedule. `carrier::run()` must follow `movement::run()` so the hero position is already updated for this tick when carriers snap/wrap.

- [ ] **Step 3: Compile check**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/game/ecs/systems/carrier.rs src/game/ecs/scene.rs
git commit -m "feat(ecs): implement carrier::run() and verify schedule placement"
```

---

## Task 5: Add carrier rendering pass

**Files:**
- Modify: `src/game/ecs/scene.rs`

Carrier sprites must be drawn after enemy sprites but before the HUD so they appear in the correct z-order.

- [ ] **Step 1: Add `blit_carriers()` helper**

Inside the `render` block of `scene.rs` (or a dedicated `systems::render::carrier` module), add:

```rust
fn blit_carriers(world: &World, res: &Resources, canvas: &mut Canvas<Window>) {
    use crate::game::ecs::components::{Carrier, CarrierKind, Facing, Position, SpriteRef};
    for (_, (_, kind, pos, facing, sref)) in
        world.query::<(&Carrier, &CarrierKind, &Position, &Facing, &SpriteRef)>().iter()
    {
        let (cfile_idx, frame) = match kind {
            CarrierKind::Raft                             => (4u8, 0u8),
            CarrierKind::Turtle                           => (5u8, 0u8),
            CarrierKind::Swan if res.carrier_mount_flying => (11u8, facing.dir as u8),
            CarrierKind::Swan                             => (4u8, 1u8),
        };
        crate::game::ecs::render::blit_sprite(
            canvas, res, sref, cfile_idx, frame,
            pos.x as i32, pos.y as i32,
        );
    }
}
```

`res.carrier_mount_flying` is a convenience accessor that reads `CarrierMount.flying` from the hero entity. If this accessor does not exist, add it to `Resources` or inline the world query.

- [ ] **Step 2: Call `blit_carriers()` in the render method**

In `EcsScene::render()`, add the call between enemy sprite rendering and HUD rendering:

```rust
fn render(
    &mut self,
    canvas: &mut Canvas<Window>,
    play_tex: &mut Texture,
    game_lib: &GameLibrary,
    resources: &mut SceneResources<'_, '_>,
) {
    systems::render::palette::run(&self.world, &mut self.res, game_lib);
    systems::render::map::run(&self.world, &mut self.res, canvas, play_tex);
    systems::render::sprite::run(&self.world, &self.res, canvas);
    blit_carriers(&self.world, &self.res, canvas);          // ← new
    systems::render::hibar::run(&self.world, &self.res, canvas, resources);
}
```

- [ ] **Step 3: Compile check**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/game/ecs/scene.rs
git commit -m "feat(ecs): add carrier sprite rendering pass in blit_actors_inner"
```

---

## Task 6: Unit tests

**Files:**
- Modify: `src/game/ecs/systems/carrier.rs`

Add a `#[cfg(test)]` module at the bottom of `carrier.rs`.

- [ ] **Step 1: Test — raft mounts within 9 px on water terrain**

```rust
#[test]
fn raft_mounts_on_water_within_9px() {
    let (mut world, mut res) = test_fixtures::with_raft_at(8.0, 0.0); // 8 px away
    res.region.set_terrain_at(0, 0, 4); // terrain 4 = water
    update_raft(&mut world, &mut res);
    let mount = world.get::<CarrierMount>(res.hero_entity).unwrap();
    assert_eq!(mount.riding, 1);
    assert!(mount.on_raft);
    assert!(res.carrier_entity.is_some());
}
```

- [ ] **Step 2: Test — raft snaps to hero position each tick**

```rust
#[test]
fn raft_snaps_to_hero_each_tick() {
    let (mut world, mut res) = test_fixtures::with_mounted_raft();
    // Move hero to (50, 75).
    world.get_mut::<Position>(res.hero_entity).unwrap().x = 50.0;
    world.get_mut::<Position>(res.hero_entity).unwrap().y = 75.0;
    res.region.set_terrain_at(50, 75, 4);
    update_raft(&mut world, &mut res);
    let raft_ent = res.carrier_entity.unwrap();
    let rpos = world.get::<Position>(raft_ent).unwrap();
    assert_eq!((rpos.x as i32, rpos.y as i32), (50, 75));
}
```

- [ ] **Step 3: Test — turtle moves only to terrain-5 positions**

```rust
#[test]
fn turtle_moves_only_to_deep_water() {
    let (mut world, mut res) = test_fixtures::with_turtle_at(0.0, 0.0);
    // Mark (0, 3) as terrain 5 (deep water), all others as terrain 0.
    // Turtle steps 3 px per tick, so the deep-water probe lands at (0, 3).
    res.region.set_terrain_at(0, 3, 5);
    let turtle_ent = res.carrier_entity.unwrap();
    autonomous_turtle_move(&mut world, &res, turtle_ent);
    let pos = world.get::<Position>(turtle_ent).unwrap();
    assert_eq!((pos.x as i32, pos.y as i32), (0, 3));
}
```

- [ ] **Step 4: Test — swan velocity accumulates and is clamped**

```rust
#[test]
fn swan_velocity_clamped_at_limits() {
    let (mut world, mut res) = test_fixtures::with_mounted_swan();
    // Drive east (dir=2) for many ticks — vx should clamp at 32.
    for _ in 0..20 {
        res.input.set_direction(2); // east
        update_swan(&mut world, &mut res);
    }
    let mount = world.get::<CarrierMount>(res.hero_entity).unwrap();
    assert!(mount.swan_vx <= 32.0, "vx must not exceed clamp");
    assert!(mount.swan_vy <= 40.0, "vy must not exceed clamp");
}
```

- [ ] **Step 5: Test — swan dismount blocked at high velocity or lava**

```rust
#[test]
fn swan_dismount_blocked_on_lava_or_high_speed() {
    // Case A: high velocity.
    {
        let (mut world, mut res) = test_fixtures::with_mounted_swan();
        world.get_mut::<CarrierMount>(res.hero_entity).unwrap().swan_vx = 30.0;
        res.input.set_action_just_pressed(true);
        res.region.set_terrain_at(0, 0, 2); // passable, non-lava
        update_swan(&mut world, &mut res);
        let mount = world.get::<CarrierMount>(res.hero_entity).unwrap();
        assert_eq!(mount.riding, 11, "still riding at high speed");
    }
    // Case B: lava terrain.
    {
        let (mut world, mut res) = test_fixtures::with_mounted_swan();
        world.get_mut::<CarrierMount>(res.hero_entity).unwrap().swan_vx = 0.0;
        res.input.set_action_just_pressed(true);
        res.region.set_terrain_at(0, 0, 8); // lava
        update_swan(&mut world, &mut res);
        let mount = world.get::<CarrierMount>(res.hero_entity).unwrap();
        assert_eq!(mount.riding, 11, "still riding over lava");
    }
}
```

- [ ] **Step 6: Run tests**

```bash
cargo test -p faery-tale-rs carrier 2>&1
```

Expected: 5 tests pass.

- [ ] **Step 7: Commit**

```bash
git add src/game/ecs/systems/carrier.rs
git commit -m "test(ecs): add 5 unit tests for CarrierSystem raft/turtle/swan"
```

---

## Spec references

- `docs/spec/carriers.md` §21.1–21.7 — carrier mechanics (raft auto-mount, turtle summon and AI, swan inertial flight, dismount conditions)
- `docs/reqs/carriers.md` R-CARRY-001 to R-CARRY-015
- `reference/logic/carrier-transport.md` (research branch) — complete fmain.c carrier logic including velocity clamping constants, terrain checks, and outdoor wrap

---

## Dependencies

| Plan | Reason |
|------|--------|
| A–D | ECS scaffolding, components, resources, scene |

No other plans required. `carrier::run()` integrates with `movement::run()`, `door::run()`, and `encounter::run()` only through `res.carrier_entity` — a simple `Option<Entity>` field set by this system and read by others.

---

## Completion check

```bash
cargo build 2>&1 | grep "^error"
cargo test -p faery-tale-rs carrier 2>&1 | grep "^test result"
```

Both succeed. Raft, turtle, and swan carriers mount, tick, and dismount correctly. Carrier sprites render in the correct z-order. All 5 unit tests pass.
