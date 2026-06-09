---
title: "Plan G — Region Transitions"
plan: G
status: draft
depends_on: []
touches:
  - src/game/ecs/systems/region.rs
  - src/game/ecs/systems/zone.rs
  - src/game/ecs/scene.rs
  - src/game/ecs/resources.rs
---

# Plan G — Region Transitions

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

## Goal

Implement full region transitions in `RegionSystem`: world-data reloading, NPC/SetFig/GroundItem despawning and respawning, zone-list extraction, region palette application, and camera repositioning. Transitions are triggered by `RegionTransitionEvent` (emitted by `DoorSystem` or `ZoneSystem`).

## Context

### Current state
- **`src/game/ecs/systems/region.rs` stub** (lines 13–23): drains `res.events.region`, updates `res.region.region_num` only. No loading, no spawn, no palette.
- **`RegionTransitionEvent`** (`events.rs` 110–114): carries `new_region: u8`, `dest_x: f32`, `dest_y: f32`.
- **`RegionState`** (`resources.rs` 44–58): `region_num`, `new_region`, encounter state, palette.
- **`MapData`** (`resources.rs` 153–156): holds `Option<WorldData>` and `Option<MapRenderer>`.
- **`EcsScene::load_world()`** (`scene.rs` 210–283): lazy-loads ADF + WorldData + MapRenderer on first frame.
- **`ZoneSystem`** (`systems/zone.rs`): has an empty zones list (line 22) — must be populated on load.
- **`NpcTable::load()`** (`npc.rs` 240–249): loads NPC records from ADF cfile for a region.
- **`spawn_enemy()`, `spawn_setfig()`** (`spawn.rs`): create ECS entities.
- **`GameLibrary.zones`** (`game_library.rs` line 200): `Vec<ZoneConfig>` per region.

### Key data structures
| Type | Location | Purpose |
|------|----------|---------|
| `WorldData` | `world_data.rs` | Sector/map/terrain/image data for one region |
| `MapRenderer` | `map_renderer.rs` | Renders map tiles to framebuffer |
| `Npc` | `npc.rs` 66–89 | NPC record: type, race, position, vitality, AI state |
| `ZoneConfig` | `game_library.rs` 80–90 | Zone rect: label, etype, x1/y1/x2/y2, flags |

## Dependencies

**None.** Plans A–F must be complete and operational.

## Implementation steps

- [ ] **Step 1 — Add `adf` field to `Resources`**
  File: `src/game/ecs/resources.rs`
  Add `pub adf: Option<std::sync::Arc<crate::game::adf::AdfDisk>>` to the `Resources` struct.
  Initialize as `None` in `Resources::new()`.

- [ ] **Step 2 — Add `zones` field to `Resources`**
  File: `src/game/ecs/resources.rs`
  Add `pub zones: Vec<crate::game::game_library::ZoneConfig>` to `Resources`.
  Initialize as `Vec::new()` in `Resources::new()`.

- [ ] **Step 3 — Store `AdfDisk` in `EcsScene`; pass to `Resources`**
  File: `src/game/ecs/scene.rs`
  Add `adf: std::sync::Arc<crate::game::adf::AdfDisk>` field to `EcsScene` struct.
  In `new()`, open the ADF once (using `game_lib.disk`) and store in both `self.adf` and `self.res.adf = Some(adf.clone())`.

- [ ] **Step 4 — Implement `reload_region()` helper in `EcsScene`**
  File: `src/game/ecs/scene.rs`
  Extract reusable logic from `load_world()` into:
  ```rust
  fn reload_region(&mut self, region: u8, dest_x: f32, dest_y: f32, game_lib: &GameLibrary)
  ```
  This method must:
  1. Despawn all `Enemy` + `SetFig` + `GroundItem` entities (`collect` entity list, then `world.despawn` each)
  2. Load `WorldData` + `MapRenderer` from `self.res.adf` for `region`
  3. Spawn NPCs from `NpcTable::load(adf, region)` using `spawn_enemy()` / `spawn_setfig()`
  4. Populate `self.res.zones` from `game_lib.zones` filtered to this region
  5. Call `region_palette()` + set `self.res.palette.dirty = true`
  6. Reposition hero: `world.get_mut::<Position>(hero_entity)?.set(dest_x, dest_y)`
  7. Snap camera: `res.camera.map_x = (dest_x - 144.0).rem_euclid(0x8000.0)`, same for y

- [ ] **Step 5 — Implement `region::run()`**
  File: `src/game/ecs/systems/region.rs`
  Replace the stub with:
  ```rust
  pub fn run(world: &mut World, res: &mut Resources, game_lib: &GameLibrary) {
      let events = std::mem::take(&mut res.events.region);
      for ev in events {
          // Delegate to EcsScene::reload_region() via helper or inline the logic here.
          // At minimum: clear entities, reload world data, spawn NPCs, extract zones,
          // apply palette, reposition hero, update res.region.region_num.
      }
  }
  ```
  Note: the simplest approach is to keep all region-load logic in `EcsScene::reload_region()` and have `region::run()` drain events and call it via a function pointer or by inlining here.

- [ ] **Step 6 — Update `ZoneSystem` to use `res.zones`**
  File: `src/game/ecs/systems/zone.rs`
  Replace the empty `zones` slice with `&res.zones` so zone entry/exit detection actually works.

- [ ] **Step 7 — Pass `game_lib` to `region::run()` in `run_tick()`**
  File: `src/game/ecs/scene.rs`
  Update the call site in `run_tick()`:
  ```rust
  systems::region::run(&mut self.world, &mut self.res, game_lib);
  ```

## Spec references
- `docs/spec/world-structure.md` §2.2–2.5 — region loading, coordinate hierarchy
- `docs/spec/display-rendering.md` — palette application, camera snap
- `docs/spec/ai-encounters.md` — zone detection and encounter generation
- `reference/logic/doors.md` (research branch) — door transition triggers

## Test plan
| Test | Setup | Expected |
|------|-------|----------|
| `test_region_clears_old_entities` | Spawn `Enemy` + `SetFig`; run transition | Both despawned; hero remains |
| `test_region_num_updated` | Emit `RegionTransitionEvent { new_region: 3, .. }` | `res.region.region_num == 3` |
| `test_zones_populated` | Region load; `game_lib` has zones | `res.zones` non-empty |
| `test_hero_repositioned` | Transition with `dest_x=500, dest_y=600` | Hero `Position` == (500, 600) |
| `test_camera_snapped` | Same | Camera offset derived from new hero pos |

## Files touched
| File | Change |
|------|--------|
| `src/game/ecs/resources.rs` | Add `adf` and `zones` fields |
| `src/game/ecs/scene.rs` | Add `adf` field; `reload_region()`; pass `game_lib` to `region::run()` |
| `src/game/ecs/systems/region.rs` | Implement `run()` |
| `src/game/ecs/systems/zone.rs` | Use `res.zones` instead of empty slice |