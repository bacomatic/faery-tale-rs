---
title: "Plan R — Brother Succession"
plan: R
status: draft
depends_on: [G]
touches: [src/game/ecs/scene.rs, src/game/scene.rs, src/main.rs]
---

# ECS Migration Plan R: Brother Succession

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the complete brother succession system. When a hero dies and the goodfairy countdown expires, emit `BrotherDiedEvent`, spawn a Bones entity at the death location, select the next living brother, spawn the new hero with saved inventory, and return `SceneResult::GameOver` when all brothers are dead.

**Architecture:** The death system (already implemented) emits `BrotherDiedEvent` when the goodfairy countdown reaches 0. `EcsScene::drain_brother_deaths()` consumes these events: it spawns a Bones entity at the death position, saves the dead brother's inventory to `BrotherRoster.inactive_inventories`, queries for the next living brother in succession order (Julian -> Phillip -> Kevin), and spawns the successor at the Tambry coordinates with fresh stats and their previously-saved inventory. If no brothers remain alive, the method returns true and the scene returns `SceneResult::GameOver`. The new `GameOver` variant is added to `SceneResult` in `src/game/scene.rs` and handled in `main.rs`.

**Prerequisites:** Plan G (region loading for spawn position). Plans A-D complete.

**Tech Stack:** Rust 2021, `hecs = "0.11"`, SDL3.

---

## File map

| File | Change |
|------|--------|
| `src/game/scene.rs` | Add `GameOver` variant to `SceneResult` enum |
| `src/game/ecs/scene.rs` | Add `next_living_brother()`, `drain_brother_deaths()`, call in `update()` |
| `src/main.rs` | Handle `SceneResult::GameOver` in the scene update match |

---

## Background: BrotherRoster (resources.rs lines 64-89)

The `BrotherRoster` resource tracks which brother is active and stores per-brother inventories:

```rust
pub struct BrotherRoster {
    pub active_brother: usize,          // 0=Julian, 1=Phillip, 2=Kevin
    pub brother: u8,                    // raw save code (1=Julian, 2=Phillip, 3=Kevin)
    pub inactive_inventories: [[u8; 36]; 3], // saved inventories per brother
}
```

When a brother dies, his current inventory is saved into `inactive_inventories[brother_id]` so that the successor (or a later brother who finds the Bones entity) can recover it.

## Background: goodfairy countdown (EncounterContext)

When hero vitality reaches 0:
1. `dying` is set to true.
2. `goodfairy` is initialized to 255.
3. Each tick, `goodfairy` decrements by 1.
4. When `goodfairy` reaches 0, the death system emits `BrotherDiedEvent` and resets the dying state.

This countdown gives the player a brief window where a nearby NPC with the goodfairy ability could revive the hero. If no revival occurs, succession proceeds.

## Background: BrotherDiedEvent (events.rs lines 72-80)

```rust
pub struct BrotherDiedEvent {
    pub brother_id: u8,
    pub x: f32,
    pub y: f32,
    pub stuff: [u8; 36],
}
```

The event carries the dead brother's ID, death position, and final inventory snapshot. The position is used for Bones entity placement. The inventory is saved to `inactive_inventories` and can be recovered by another brother finding the Bones.

## Background: brother succession order

Brothers succeed in fixed order: Julian (0) -> Phillip (1) -> Kevin (2). If a brother is already dead (has a Bones entity in the world), they are skipped. The first living brother not in the dead set becomes the active hero.

Examples:
- Julian dies -> Phillip becomes active (if alive)
- Julian and Phillip both dead -> Kevin becomes active
- All three dead -> game over

## Background: brother spawn position

Per spec SS20.4, the successor always spawns at Tambry: `(19036.0, 15755.0)` in region 3. This is the brothers' home village and the canonical respawn point for succession events.

## Background: Bones entity

A Bones entity marks the death location of a brother. It has:
- `Position` — the death coordinates
- `BrotherKind` — identifies which brother died (`id` field)
- `Bones` marker component
- Stored inventory — the dead brother's `stuff[36]` array

When a living brother walks over a Bones entity, they can loot the dead brother's inventory (handled by the item/proximity system, not this plan).

## Background: SceneResult enum (scene.rs lines 16-23)

Current variants:

```rust
pub enum SceneResult {
    Continue,
    Done,
    Quit,
}
```

This plan adds `GameOver` to signal that all brothers are dead and the game should end.

---

## Task 1: Add `GameOver` variant to `SceneResult`

**Files:**
- Modify: `src/game/scene.rs`

- [ ] **Step 1: Add the variant**

  In `src/game/scene.rs`, add `GameOver` to the `SceneResult` enum:

  ```rust
  pub enum SceneResult {
      Continue,
      Done,
      GameOver,
      Quit,
  }
  ```

  Alphabetical ordering is not required — place `GameOver` between `Done` and `Quit` for logical grouping (normal flow -> game over -> user quit).

- [ ] **Step 2: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

  Expected: possible `non-exhaustive patterns` warnings in match statements elsewhere that match on `SceneResult`. These will be resolved in Task 4 (main.rs) and should not block this step if they are only warnings.

- [ ] **Step 3: Commit**

  ```bash
  git add src/game/scene.rs
  git commit -m "feat: add GameOver variant to SceneResult enum"
  ```

---

## Task 2: Implement `next_living_brother()` in scene.rs

**Files:**
- Modify: `src/game/ecs/scene.rs`

- [ ] **Step 1: Add required imports**

  In `src/game/ecs/scene.rs`, ensure the following are imported:

  ```rust
  use crate::game::ecs::components::{BrotherKind, Bones};
  use std::collections::HashSet;
  ```

- [ ] **Step 2: Implement the helper method**

  Add as a private method on `EcsScene`:

  ```rust
  impl EcsScene {
      /// Find the next living brother in succession order after `just_died`.
      ///
      /// Queries all Bones entities in the world to determine which brothers
      /// are already dead. Adds `just_died` to the dead set. Returns the first
      /// brother ID (0, 1, or 2) not in the dead set, or None if all are dead.
      fn next_living_brother(&self, just_died: u8) -> Option<u8> {
          let dead_ids: HashSet<u8> = self.world
              .query::<&BrotherKind>()
              .with::<&Bones>()
              .iter()
              .map(|(_, bk)| bk.id)
              .chain(std::iter::once(just_died))
              .collect();

          (0u8..3).find(|id| !dead_ids.contains(id))
      }
  }
  ```

  > **Note:** This method is identical to the one shown in Plan D's reference implementation (scene.rs lines 239-249), except it does not take a `game_lib` parameter since it only needs to query the world for existing Bones entities. The `game_lib` parameter is needed by `drain_brother_deaths()` to look up successor stats.

- [ ] **Step 3: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

  Expected: no errors. If `BrotherKind` or `Bones` are not yet defined as components, they should already exist from Plan B. If not, stub them:
  ```rust
  pub struct BrotherKind { pub id: u8 }
  pub struct Bones;
  ```

---

## Task 3: Implement `drain_brother_deaths()` in scene.rs

**Files:**
- Modify: `src/game/ecs/scene.rs`

- [ ] **Step 1: Add required imports**

  Ensure these are available in `src/game/ecs/scene.rs`:

  ```rust
  use crate::game::ecs::spawn::{spawn_bones, spawn_hero};
  use crate::game::ecs::components::{HeroStats, Inventory};
  use crate::game::game_library::GameLibrary;
  ```

- [ ] **Step 2: Implement the method**

  Add as a method on `EcsScene`:

  ```rust
  impl EcsScene {
      /// Consume all `BrotherDiedEvent`s: spawn Bones, select successor, or signal game over.
      ///
      /// Returns `true` if all brothers are dead (game over), `false` otherwise.
      fn drain_brother_deaths(&mut self, game_lib: &GameLibrary) -> bool {
          // Drain brother death events from this tick.
          let events: Vec<_> = self.res.events.brother.drain(..).collect();

          for ev in events {
              // 1. Spawn a Bones entity at the death location.
              spawn_bones(
                  &mut self.world,
                  ev.x,
                  ev.y,
                  self.res.region.region_num,
                  ev.brother_id,
                  ev.stuff,
              );

              // 2. Save the dead brother's inventory.
              self.res.brother.inactive_inventories[ev.brother_id as usize] = ev.stuff;

              // 3. Find the next living brother.
              match self.next_living_brother(ev.brother_id) {
                  Some(successor) => {
                      // 4a. Load successor stats from GameLibrary.
                      let cfg = &game_lib.brothers[successor as usize];
                      let stats = HeroStats {
                          vitality: 100,
                          brave:    cfg.brave  as i16,
                          luck:     cfg.luck   as i16,
                          kind:     cfg.kind   as i16,
                          wealth:   cfg.wealth as i16,
                          hunger:   0,
                          fatigue:  0,
                          gold:     0,
                      };

                      // 4b. Load successor's saved inventory.
                      let inv = Inventory {
                          stuff: self.res.brother.inactive_inventories[successor as usize],
                      };

                      // 4c. Despawn the current (dead) hero entity.
                      self.world.despawn(self.res.hero_entity).ok();

                      // 4d. Spawn successor at Tambry (19036.0, 15755.0).
                      let new_hero = spawn_hero(
                          &mut self.world,
                          19036.0,
                          15755.0,
                          successor,
                          stats,
                          inv,
                      );

                      // 4e. Update resource references.
                      self.res.hero_entity = new_hero;
                      self.res.brother.active_brother = successor as usize;
                      self.res.brother.brother = successor + 1; // raw save code is 1-indexed

                      // 4f. Trigger region load for Tambry's region.
                      self.res.region.new_region = 3;
                  }
                  None => {
                      // 5. All brothers dead — game over.
                      return true;
                  }
              }
          }

          false
      }
  }
  ```

  > **Note on Tambry coordinates:** The hardcoded spawn position `(19036.0, 15755.0)` and region 3 come from spec SS20.4. These are the canonical succession spawn coordinates for all brothers.

  > **Note on region transition:** Setting `res.region.new_region = 3` triggers the region loading system in the next `update()` cycle, which will load region 3's terrain, NPCs, and objects. This relies on Plan G being complete.

- [ ] **Step 3: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

  Expected: no errors. Verify that `spawn_bones()` accepts the arguments shown. If its signature differs, adjust the call to match.

---

## Task 4: Call `drain_brother_deaths()` in `update()`

**Files:**
- Modify: `src/game/ecs/scene.rs`

- [ ] **Step 1: Add the call after the tick loop**

  In `EcsScene::update()`, after the `while self.tick_accum >= MS_PER_TICK` loop and before the render call, add the succession check:

  ```rust
  // Process brother deaths accumulated during tick(s).
  if self.drain_brother_deaths(game_lib) {
      return SceneResult::GameOver;
  }
  ```

  The full `update()` flow becomes:

  ```rust
  fn update(...) -> SceneResult {
      self.tick_accum += delta_ticks;

      // Process pending debug commands.
      for cmd in self.debug.poll_commands() {
          crate::game::ecs::debug_commands::handle(cmd, &mut self.world, &mut self.res);
      }

      // Run gameplay ticks at 15 Hz.
      while self.tick_accum >= MS_PER_TICK {
          self.tick_accum -= MS_PER_TICK;
          self.run_tick(game_lib);
      }

      // Process brother deaths — may trigger succession or game over.
      if self.drain_brother_deaths(game_lib) {
          return SceneResult::GameOver;
      }

      // Render at presentation frame rate.
      self.render(canvas, play_tex, game_lib, resources);

      // ... rest of update() ...

      SceneResult::Continue
  }
  ```

  Placing the death drain between ticks and rendering ensures:
  - All death events from the tick batch are processed before rendering.
  - The successor (if any) is spawned and visible in the first rendered frame after death.
  - `GameOver` is returned before wasting a render call on a dead world.

- [ ] **Step 2: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

  Expected: no errors.

- [ ] **Step 3: Commit**

  ```bash
  git add src/game/ecs/scene.rs
  git commit -m "feat(ecs): implement brother succession — drain_brother_deaths() + next_living_brother()"
  ```

---

## Task 5: Handle `SceneResult::GameOver` in main.rs

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add the match arm**

  In `src/main.rs`, find the match on `SceneResult` returned by `scene.update()`. Add the `GameOver` arm:

  ```rust
  SceneResult::GameOver => {
      // TODO(Plan R): Show game-over placard(5) "And so ends our sad tale..."
      break 'running;
  }
  ```

  > **Note on game-over placard:** The original game displays placard ID 5 with the text "And so ends our sad tale..." before returning to the title screen. The placard system may not yet be implemented; the `break 'running` is sufficient for now and a TODO comment marks the gap. The placard text must come from the authoritative source (`faery.toml` or `reference/logic/placard.md`), not be hardcoded in Rust.

- [ ] **Step 2: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

  Expected: no errors. The `GameOver` arm must be present in every match on `SceneResult`. If there are other match sites (e.g., in a scene manager or test harness), add `SceneResult::GameOver => { break; }` or equivalent to each.

- [ ] **Step 3: Verify no unhandled match warnings**

  ```bash
  cargo check 2>&1 | grep "non-exhaustive"
  ```

  Expected: no warnings.

- [ ] **Step 4: Commit**

  ```bash
  git add src/main.rs
  git commit -m "feat: handle SceneResult::GameOver in main.rs"
  ```

---

## Task 6: Add unit tests

**Files:**
- Modify: `src/game/ecs/scene.rs`

Five tests covering succession logic and game-over detection. These tests operate on `EcsScene` internals and require constructing minimal world states.

- [ ] **Step 1: Add test module with setup helpers**

  ```rust
  #[cfg(test)]
  mod succession_tests {
      use super::*;
      use hecs::World;
      use crate::game::ecs::resources::Resources;
      use crate::game::ecs::components::{HeroStats, Inventory, Position, BrotherKind, Bones};
      use crate::game::ecs::spawn::{spawn_hero, spawn_bones};
      use crate::game::ecs::events::BrotherDiedEvent;

      /// Create a minimal EcsScene with Julian as the active hero.
      fn setup_scene() -> EcsScene {
          // Construct a minimal scene. This may need adjustment based on
          // EcsScene::new() signature and GameLibrary availability in tests.
          let mut world = World::new();
          let hero = spawn_hero(
              &mut world,
              100.0, 200.0,
              0, // Julian
              HeroStats::default(),
              Inventory::empty(),
          );
          let mut res = Resources::new(hero);
          res.brother.active_brother = 0;
          res.brother.brother = 1; // raw save code: Julian=1
          EcsScene {
              world,
              res,
              tick_accum: 0,
              debug: DebugConsole::new(),
          }
      }
  ```

- [ ] **Step 2: Test `next_living_brother_returns_successor`**

  ```rust
      #[test]
      fn next_living_brother_returns_successor() {
          let scene = setup_scene();

          // Julian (0) just died — Phillip (1) should be next.
          let next = scene.next_living_brother(0);
          assert_eq!(next, Some(1), "Phillip should succeed Julian");
      }
  ```

- [ ] **Step 3: Test `next_living_brother_skips_dead`**

  ```rust
      #[test]
      fn next_living_brother_skips_dead() {
          let mut scene = setup_scene();

          // Phillip (1) already has Bones in the world — he's dead.
          spawn_bones(&mut scene.world, 500.0, 600.0, 0, 1, [0u8; 36]);

          // Julian (0) just died — Phillip is dead too — Kevin (2) should be next.
          let next = scene.next_living_brother(0);
          assert_eq!(next, Some(2), "Kevin should succeed when Phillip is already dead");
      }
  ```

- [ ] **Step 4: Test `next_living_brother_returns_none_all_dead`**

  ```rust
      #[test]
      fn next_living_brother_returns_none_all_dead() {
          let mut scene = setup_scene();

          // Phillip and Kevin already have Bones in the world.
          spawn_bones(&mut scene.world, 500.0, 600.0, 0, 1, [0u8; 36]);
          spawn_bones(&mut scene.world, 700.0, 800.0, 0, 2, [0u8; 36]);

          // Julian (0) just died — all brothers dead.
          let next = scene.next_living_brother(0);
          assert_eq!(next, None, "should return None when all brothers are dead");
      }
  ```

- [ ] **Step 5: Test `drain_deaths_spawns_bones`**

  ```rust
      #[test]
      fn drain_deaths_spawns_bones() {
          let mut scene = setup_scene();
          let game_lib = GameLibrary::test_default(); // or appropriate test constructor

          // Push a BrotherDiedEvent for Julian.
          scene.res.events.brother.push(BrotherDiedEvent {
              brother_id: 0,
              x: 1000.0,
              y: 2000.0,
              stuff: [42u8; 36],
          });

          let game_over = scene.drain_brother_deaths(&game_lib);
          assert!(!game_over, "should not be game over — Phillip and Kevin are alive");

          // Verify a Bones entity was spawned.
          let bones_count = scene.world
              .query::<(&Bones, &BrotherKind, &Position)>()
              .iter()
              .filter(|(_, (_, bk, pos))| {
                  bk.id == 0 && pos.x == 1000.0 && pos.y == 2000.0
              })
              .count();
          assert_eq!(bones_count, 1, "exactly one Bones entity for Julian at death position");

          // Verify inventory was saved.
          assert_eq!(scene.res.brother.inactive_inventories[0], [42u8; 36]);
      }
  ```

  > **Note:** `GameLibrary::test_default()` is a placeholder for whatever test constructor is available. If `GameLibrary` cannot be easily constructed in tests, extract the brother config lookup into a trait or pass the config directly. Adjust the test setup to match the actual test infrastructure.

- [ ] **Step 6: Test `drain_deaths_returns_true_all_dead`**

  ```rust
      #[test]
      fn drain_deaths_returns_true_all_dead() {
          let mut scene = setup_scene();
          let game_lib = GameLibrary::test_default();

          // Phillip and Kevin already dead.
          spawn_bones(&mut scene.world, 500.0, 600.0, 0, 1, [0u8; 36]);
          spawn_bones(&mut scene.world, 700.0, 800.0, 0, 2, [0u8; 36]);

          // Julian dies — all brothers now dead.
          scene.res.events.brother.push(BrotherDiedEvent {
              brother_id: 0,
              x: 100.0,
              y: 200.0,
              stuff: [0u8; 36],
          });

          let game_over = scene.drain_brother_deaths(&game_lib);
          assert!(game_over, "should be game over when all brothers are dead");
      }
  }
  ```

- [ ] **Step 7: Run tests**

  ```bash
  cargo test succession_tests 2>&1 | grep -E "^test result|FAILED"
  ```

  Expected: `test result: ok. 5 passed`.

- [ ] **Step 8: Commit**

  ```bash
  git add src/game/ecs/scene.rs
  git commit -m "test(ecs): add 5 unit tests for brother succession system"
  ```

---

## Completion check

```bash
cargo build 2>&1 | grep "^error"
cargo test succession_tests 2>&1 | grep -E "^test result|FAILED"
```

Both succeed. The brother succession system is fully implemented: `BrotherDiedEvent` is consumed, Bones entities are spawned, successors are selected and spawned at Tambry, and `SceneResult::GameOver` is returned when all brothers are dead.

---

## Spec references

- `docs/spec/death-revival.md` SS20.2 — goodfairy countdown mechanics
- `docs/spec/death-revival.md` SS20.4 — succession rules, Tambry spawn position, inventory transfer
- `docs/spec/death-revival.md` SS20.5 — Bones entity creation and data
- `docs/spec/death-revival.md` SS20.7 — game over condition (all brothers dead)

## Test plan

- `next_living_brother_returns_successor` — Julian dead -> returns Phillip (1)
- `next_living_brother_skips_dead` — Julian dead + Phillip has Bones -> returns Kevin (2)
- `next_living_brother_returns_none_all_dead` — all Bones in world -> returns None
- `drain_deaths_spawns_bones` — event pushed -> Bones entity exists at correct position with correct inventory
- `drain_deaths_returns_true_all_dead` — all brothers dead -> returns true (game over)

## Files touched

| File | Change |
|------|--------|
| `src/game/scene.rs` | Add `GameOver` variant to `SceneResult` |
| `src/game/ecs/scene.rs` | Add `next_living_brother()`, `drain_brother_deaths()`, call in `update()`, 5 unit tests |
| `src/main.rs` | Handle `SceneResult::GameOver` in scene update match |
