---
title: "Plan H — Door System"
plan: H
status: draft
depends_on: [G]
touches:
  - src/game/ecs/systems/door.rs
  - src/game/ecs/resources.rs
  - src/game/ecs/scene.rs
---

# Plan H — Door System

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

## Goal

Implement door detection and region transition in `DoorSystem`. When the hero's position overlaps a door rectangle, emit a `RegionTransitionEvent` with the destination region and spawn coordinates. Track opened doors to prevent re-triggering. Emit door-open SFX.

## Context

### Current state
- **`src/game/ecs/systems/door.rs` stub** (lines 1–24): reads hero position, does nothing else (TODO comment).
- **`DoorEntry`** (`src/game/doors.rs` 5–14): `src_region`, `src_x/y`, `dst_region`, `dst_x/y`, `door_type`.
- **Helper functions** already exist in `src/game/doors.rs`:
  - `doorfind(table, region_num, hero_x, hero_y) -> Option<DoorEntry>` — outdoor grid-aligned detection
  - `doorfind_exit(table, hero_x, hero_y) -> Option<DoorEntry>` — indoor exit detection
  - `entry_spawn(door) -> (u16, u16)` — spawn position when entering
  - `exit_spawn(door) -> (u16, u16)` — spawn position when exiting
- **`GameLibrary.doors: Vec<DoorConfig>`** (`game_library.rs` 198–200): loaded from `faery.toml`.
- **`MapData`** (`resources.rs` 153–156): currently `world` + `renderer` only. Doors go here.
- **`RegionTransitionEvent`** (`events.rs` 109–114): already defined.

### How the old gameplay scene handled doors
1. Each tick, check hero position against door table for current region.
2. `doorfind()` for outdoor (region_num < 8); `doorfind_exit()` for indoor (region_num ≥ 8).
3. On hit: emit region transition with destination + spawn coords; play door-open SFX.

## Dependencies
**Plan G must complete first.** Plan G establishes the region-load infrastructure (`MapData` population on region load). Plan H adds `MapData.doors` to that same pattern.

## Implementation steps

- [ ] **Step 1 — Add `doors` and `opened_doors` to `MapData`**
  File: `src/game/ecs/resources.rs`
  Extend `MapData` struct:
  ```rust
  pub doors:        Vec<crate::game::doors::DoorEntry>,
  pub opened_doors: std::collections::HashSet<usize>,
  ```
  Initialize both as empty in `MapData::default()`.

- [ ] **Step 2 — Populate door table on region load**
  File: `src/game/ecs/scene.rs` (in `reload_region()` from Plan G)
  After loading `WorldData`, add:
  ```rust
  self.res.map.doors.clear();
  self.res.map.opened_doors.clear();
  for door_cfg in game_lib.doors.iter().filter(|d| d.src_region == region) {
      self.res.map.doors.push(DoorEntry {
          src_region: door_cfg.src_region,
          src_x: door_cfg.src_x, src_y: door_cfg.src_y,
          dst_region: door_cfg.dst_region,
          dst_x: door_cfg.dst_x, dst_y: door_cfg.dst_y,
          door_type: door_cfg.door_type,
      });
  }
  ```

- [ ] **Step 3 — Update `door::run()` call to accept `game_lib`**
  File: `src/game/ecs/scene.rs`
  Change call in `run_tick()`:
  ```rust
  systems::door::run(&self.world, &mut self.res, game_lib);
  ```

- [ ] **Step 4 — Implement `door::run()`**
  File: `src/game/ecs/systems/door.rs`
  Replace stub with:
  ```rust
  pub fn run(world: &World, res: &mut Resources, _game_lib: &GameLibrary) {
      // 1. Get hero position
      // 2. Choose doorfind() vs doorfind_exit() based on res.region.region_num
      // 3. On hit: check opened_doors; if new, insert idx, compute spawn, emit
      //    RegionTransitionEvent + SfxEvent { sfx_id: 12 }
  }
  ```
  Exact logic:
  - `hero_x/y` from `world.get::<&Position>(res.hero_entity)`
  - For `region_num < 8`: use `doors::doorfind(&res.map.doors, region_num, hero_x as u16, hero_y as u16)`
  - For `region_num >= 8`: use `doors::doorfind_exit(&res.map.doors, hero_x as u16, hero_y as u16)`
  - Find index of matched door in `res.map.doors`; skip if in `res.map.opened_doors`
  - Insert index into `res.map.opened_doors`
  - Compute spawn: if `region_num < 8` → `doors::entry_spawn(&door)` else `doors::exit_spawn(&door)`
  - `res.events.region.push(RegionTransitionEvent { new_region: door.dst_region, dest_x: spawn.0 as f32, dest_y: spawn.1 as f32 })`
  - `res.events.sfx.push(SfxEvent { sfx_id: 12 })`

## Spec references
- `docs/spec/doors-buildings.md` §16.1–16.6 — door structure, type constants, region transitions
- `reference/logic/doors.md` (research branch) — detailed door logic and edge cases

## Test plan
| Test | Setup | Expected |
|------|-------|----------|
| `test_door_triggers_transition` | Hero at door position; door in `res.map.doors` | `RegionTransitionEvent` emitted |
| `test_door_no_retrigger` | Same door idx in `opened_doors` | No event emitted |
| `test_door_emits_sfx` | Hero at door position | `SfxEvent { sfx_id: 12 }` emitted |
| `test_no_doors_no_panic` | Empty `res.map.doors` | No panic, no events |
| `test_indoor_uses_doorfind_exit` | `region_num = 8`, hero at exit | `doorfind_exit` path used |

## Files touched
| File | Change |
|------|--------|
| `src/game/ecs/resources.rs` | Add `doors` + `opened_doors` to `MapData` |
| `src/game/ecs/scene.rs` | Populate door table in `reload_region()`; pass `game_lib` to `door::run()` |
| `src/game/ecs/systems/door.rs` | Implement `run()` |