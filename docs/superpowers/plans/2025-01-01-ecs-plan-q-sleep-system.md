---
title: "Plan Q — Sleep System"
plan: Q
status: draft
depends_on: []
touches: [src/game/ecs/systems/sleep.rs, src/game/ecs/systems/mod.rs, src/game/ecs/scene.rs]
---

# ECS Migration Plan Q: Sleep System

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the sleep system as an ECS system that advances time 64x faster, recovers fatigue and health, and wakes on specific conditions. Replace the placeholder skip comment in `EcsScene::run_tick()` with a full sleep system.

**Architecture:** The sleep system is a standalone ECS system in `src/game/ecs/systems/sleep.rs`. When `EncounterContext.sleeping` is true, `EcsScene::run_tick()` calls `systems::sleep::run()` and returns immediately, bypassing all other gameplay systems. Each sleep tick accelerates the daynight clock by +63 (plus the normal +1 from the clock system = 64x speed), decrements fatigue, and conditionally heals the hero. The system checks three wake conditions each tick and clears the sleeping flag when any is met.

**Prerequisites:** Plans A-D complete. No additional dependencies.

**Tech Stack:** Rust 2021, `hecs = "0.11"`, SDL3.

---

## File map

| File | Change |
|------|--------|
| `src/game/ecs/systems/sleep.rs` | **Create** — `run()` + helpers + unit tests |
| `src/game/ecs/systems/mod.rs` | Add `pub mod sleep;` |
| `src/game/ecs/scene.rs` | Replace sleep skip comment with `systems::sleep::run()` call |

---

## Background: current state

- `src/game/ecs/scene.rs` line 479: comment `// sleep system not yet ported — skipped`
- `EncounterContext.sleeping: bool` in `resources.rs` line 178 — gates sleep mode
- `HeroStats.fatigue: i16` and `HeroStats.hunger: i16` in `components.rs` lines 104-105
- `GameClock.daynight: u16` (0-23999 wrapping), `game_days: u32`, `lightlevel: u16` — in `resources.rs`

## Background: sleep mechanics (SPEC SS18.4)

Each tick while `EncounterContext.sleeping` is true:

1. **Daynight acceleration**: `daynight += 63`. Combined with the normal +1 from the clock system (which runs before sleep in `run_tick()`), this yields 64x real-time speed. The daynight counter wraps at 24000 and the lightlevel triangle wave is refreshed from the new daynight value.

2. **Fatigue recovery**: `fatigue -= 1`, clamped at 0 via `saturating_sub(1)`.

3. **Healing**: When `(daynight & 0x3FF) == 0` AND `!battleflag`, the hero gains +1 vitality up to the cap of `15 + brave / 4`. The `0x3FF` mask means healing triggers every 1024 daynight units. At 64x speed, ~4 healing events occur per full day cycle (24000 / 1024 ~= 23, but the hero typically sleeps only a fraction of the cycle).

## Background: wake conditions

The hero wakes (any one condition is sufficient):

| # | Condition | Meaning |
|---|-----------|---------|
| 1 | `fatigue == 0` | Fully rested |
| 2 | `fatigue < 30 AND daynight in [9000, 10000)` | Dawn wake window — hero wakes near sunrise if reasonably rested |
| 3 | `battleflag AND 1-in-64 chance` | Enemy proximity — random wakeup when enemies are near |

On wake:
- `res.encounter.sleeping = false`
- Snap hero Y position: `pos.y = (pos.y as u16 & 0xFFE0) as f32` — aligns to 32-pixel grid

## Background: daynight compression arithmetic

Normal gameplay: +1/tick at 15 Hz = 24000 / 15 = 1600 seconds (~26.7 minutes) for a full day cycle.

Sleep mode: +64/tick at 15 Hz = 24000 / (64 * 15) = 25 seconds for a full day cycle. A hero with fatigue=100 needs ~100 ticks to reach fatigue=0, which is ~6.7 real seconds.

## Background: lightlevel triangle wave

The daynight counter drives a triangle wave for ambient lighting (spec
`daynight-cycle.md §17.2`; matches `game_state.rs`):

```
lightlevel = daynight / 40
if lightlevel >= 300:
    lightlevel = 600 - lightlevel
```

This ramps up to a peak of 300 at `daynight == 12000`, then back down — NOT a
`daynight / 12` ramp to 999. The sleep system must refresh this after each +63
increment so that the palette system sees smooth day/night progression during
sleep.

---

## Task 1: Create `src/game/ecs/systems/sleep.rs`

**Files:**
- Create: `src/game/ecs/systems/sleep.rs`

- [ ] **Step 1: Create the file with module-level doc comment and imports**

  ```rust
  //! Sleep system — accelerates daynight, recovers fatigue/health, wakes on conditions.
  //!
  //! Called from `EcsScene::run_tick()` when `EncounterContext.sleeping` is true.
  //! Bypasses all other gameplay systems for the duration of sleep.

  use hecs::World;
  use crate::game::ecs::resources::Resources;
  use crate::game::ecs::components::HeroStats;
  ```

- [ ] **Step 2: Implement `pub fn run()`**

  The top-level entry point. Calls helpers in order, then checks wake conditions:

  ```rust
  /// Run one sleep tick: advance daynight, recover fatigue, maybe heal, maybe wake.
  pub fn run(world: &mut World, res: &mut Resources) {
      advance_daynight(res);
      decrement_fatigue(world, res);
      apply_healing(world, res);

      if should_wake(world, res) {
          wake_hero(world, res);
      }
  }
  ```

- [ ] **Step 3: Implement `advance_daynight()`**

  Adds 63 to the daynight counter (the clock system already added +1 this tick), wraps at 24000, and refreshes the lightlevel triangle wave:

  ```rust
  /// Advance daynight by 63 (sleep acceleration) and refresh the lightlevel.
  fn advance_daynight(res: &mut Resources) {
      let dn = &mut res.clock.daynight;
      *dn = (*dn + 63) % 24000;

      // Refresh lightlevel (daynight-cycle.md §17.2; matches game_state.rs).
      let mut ll = *dn / 40;
      if ll >= 300 {
          ll = 600 - ll;
      }
      res.clock.lightlevel = ll;
  }
  ```

- [ ] **Step 4: Implement `decrement_fatigue()`**

  ```rust
  /// Decrement hero fatigue by 1, clamped at 0.
  fn decrement_fatigue(world: &mut World, res: &mut Resources) {
      if let Ok(mut stats) = world.get::<&mut HeroStats>(res.hero_entity) {
          stats.fatigue = stats.fatigue.saturating_sub(1);
      }
  }
  ```

- [ ] **Step 5: Implement `apply_healing()`**

  Healing fires when `(daynight & 0x3FF) == 0` and the hero is not in active combat. Vitality is capped at `15 + brave / 4`:

  ```rust
  /// Heal +1 vitality when daynight is aligned to 0x400 boundary and no battle is active.
  fn apply_healing(world: &mut World, res: &mut Resources) {
      if (res.clock.daynight & 0x3FF) != 0 {
          return;
      }
      if res.encounter.battleflag {
          return;
      }

      if let Ok(mut stats) = world.get::<&mut HeroStats>(res.hero_entity) {
          let cap = 15 + stats.brave / 4;
          if stats.vitality < cap {
              stats.vitality += 1;
          }
      }
  }
  ```

- [ ] **Step 6: Implement `should_wake()`**

  Checks the three wake conditions:

  ```rust
  /// Check whether the hero should wake up this tick.
  fn should_wake(world: &World, res: &Resources) -> bool {
      let fatigue = match world.get::<&HeroStats>(res.hero_entity) {
          Ok(stats) => stats.fatigue,
          Err(_) => return true, // No hero stats — wake defensively.
      };

      // Condition 1: fully rested.
      if fatigue == 0 {
          return true;
      }

      // Condition 2: dawn wake window — reasonably rested near sunrise.
      if fatigue < 30 && (9000..10000).contains(&res.clock.daynight) {
          return true;
      }

      // Condition 3: enemy proximity — random wake when battleflag is set.
      if res.encounter.battleflag {
          // 1-in-64 chance per tick.
          if (res.clock.tick_counter & 0x3F) == 0 {
              return true;
          }
      }

      false
  }
  ```

  > **Note on RNG:** The original uses `(daynight & 0x3F) == 0` as a pseudo-random 1-in-64 gate. The `tick_counter` serves the same purpose here — it increments each tick and the low 6 bits cycle through all 64 values. If `Resources` does not expose a suitable tick counter, use `res.clock.daynight & 0x3F` instead, which yields the same statistical frequency.

- [ ] **Step 7: Implement `wake_hero()`**

  Clears the sleeping flag and snaps the hero's Y position to the 32-pixel grid:

  ```rust
  /// Wake the hero: clear sleeping flag and snap Y to 32-pixel grid.
  fn wake_hero(world: &mut World, res: &mut Resources) {
      res.encounter.sleeping = false;

      if let Ok(mut pos) = world.get::<&mut crate::game::ecs::components::Position>(res.hero_entity) {
          pos.y = (pos.y as u16 & 0xFFE0) as f32;
      }
  }
  ```

- [ ] **Step 8: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

  Expected: no errors. If `EncounterContext.battleflag` or `GameClock.tick_counter` do not exist, check the actual field names in `resources.rs` and adjust accordingly.

---

## Task 2: Add `pub mod sleep;` to systems/mod.rs

**Files:**
- Modify: `src/game/ecs/systems/mod.rs`

- [ ] **Step 1: Add module declaration**

  Add `pub mod sleep;` in alphabetical order alongside the other system module declarations in `src/game/ecs/systems/mod.rs`.

- [ ] **Step 2: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

  Expected: no errors.

---

## Task 3: Wire into `run_tick()` in scene.rs

**Files:**
- Modify: `src/game/ecs/scene.rs`

- [ ] **Step 1: Replace the sleep skip comment**

  Find the comment `// sleep system not yet ported — skipped` in `EcsScene::run_tick()` and replace it with the actual system call. The sleep system must short-circuit all other gameplay systems:

  ```rust
  if self.res.encounter.sleeping {
      systems::sleep::run(&mut self.world, &mut self.res);
      return;  // skip all other systems
  }
  ```

  This matches the pattern already established in the Plan D reference implementation (scene.rs lines 170-173), where the sleep branch exits `run_tick()` early.

- [ ] **Step 2: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

  Expected: no errors.

- [ ] **Step 3: Commit**

  ```bash
  git add src/game/ecs/systems/sleep.rs src/game/ecs/systems/mod.rs src/game/ecs/scene.rs
  git commit -m "feat(ecs): implement sleep system — daynight acceleration, fatigue recovery, wake conditions"
  ```

---

## Task 4: Add unit tests in sleep.rs

**Files:**
- Modify: `src/game/ecs/systems/sleep.rs`

Six tests covering all sleep mechanics and wake conditions.

- [ ] **Step 1: Add test module with setup helper**

  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      use hecs::World;
      use crate::game::ecs::resources::Resources;
      use crate::game::ecs::components::{HeroStats, Position};

      /// Spawn a minimal hero with given stats and position, return (World, Resources).
      fn setup(fatigue: i16, vitality: i16, brave: i16, daynight: u16) -> (World, Resources) {
          let mut world = World::new();
          let hero = world.spawn((
              HeroStats {
                  fatigue,
                  vitality,
                  brave,
                  ..HeroStats::default()
              },
              Position { x: 100.0, y: 123.5 },
          ));
          let mut res = Resources::new(hero);
          res.clock.daynight = daynight;
          res.encounter.sleeping = true;
          res
      }
  ```

- [ ] **Step 2: Test `daynight_advances_by_63`**

  ```rust
      #[test]
      fn daynight_advances_by_63() {
          let (mut world, mut res) = setup(100, 10, 40, 1000);

          run(&mut world, &mut res);

          // daynight should be 1000 + 63 = 1063
          assert_eq!(res.clock.daynight, 1063);
      }
  ```

- [ ] **Step 3: Test `fatigue_decrements`**

  ```rust
      #[test]
      fn fatigue_decrements() {
          let (mut world, mut res) = setup(50, 10, 40, 1000);

          run(&mut world, &mut res);

          let stats = world.get::<&HeroStats>(res.hero_entity).unwrap();
          assert_eq!(stats.fatigue, 49, "fatigue must decrement by 1 each sleep tick");
          // Still sleeping — fatigue > 0 and not in dawn window.
          assert!(res.encounter.sleeping);
      }
  ```

- [ ] **Step 4: Test `wakes_when_fatigue_zero`**

  ```rust
      #[test]
      fn wakes_when_fatigue_zero() {
          // fatigue=1 → after decrement it becomes 0 → should wake.
          let (mut world, mut res) = setup(1, 10, 40, 1000);

          run(&mut world, &mut res);

          let stats = world.get::<&HeroStats>(res.hero_entity).unwrap();
          assert_eq!(stats.fatigue, 0);
          assert!(!res.encounter.sleeping, "must wake when fatigue reaches 0");
      }
  ```

- [ ] **Step 5: Test `wakes_in_dawn_window_with_low_fatigue`**

  ```rust
      #[test]
      fn wakes_in_dawn_window_with_low_fatigue() {
          // fatigue=20 (<30), daynight will be 9500+63=9563 which is in [9000,10000).
          // But we need daynight to already be in the window when should_wake checks.
          // After advance_daynight: 9500 + 63 = 9563. After decrement: fatigue = 19.
          // should_wake sees fatigue=19 (<30) and daynight=9563 (in [9000,10000)) → wake.
          let (mut world, mut res) = setup(20, 10, 40, 9500);

          run(&mut world, &mut res);

          assert!(!res.encounter.sleeping, "must wake in dawn window with low fatigue");
      }
  ```

- [ ] **Step 6: Test `snaps_y_on_wake`**

  ```rust
      #[test]
      fn snaps_y_on_wake() {
          // fatigue=1 → wakes after one tick. pos.y = 123.5 → snapped.
          // 123 as u16 = 123 = 0x007B. 0x007B & 0xFFE0 = 0x0060 = 96.
          let (mut world, mut res) = setup(1, 10, 40, 1000);

          run(&mut world, &mut res);

          let pos = world.get::<&Position>(res.hero_entity).unwrap();
          assert_eq!(pos.y, 96.0, "Y must snap to 32-pixel grid on wake (0xFFE0 mask)");
      }
  ```

- [ ] **Step 7: Test `healing_at_64x_rate`**

  ```rust
      #[test]
      fn healing_at_64x_rate() {
          // Start at daynight=0, vitality=5, brave=40. Cap = 15 + 40/4 = 25.
          // Run 64 ticks. daynight advances 63 per tick → 64*63 = 4032 units.
          // Healing fires when (daynight & 0x3FF) == 0, i.e. at multiples of 1024.
          // In range 0..4032: hits at 0 (initial? depends on order), 1024, 2048, 3072.
          // That's ~3-4 healing events. Vitality should increase by 3-4.
          let (mut world, mut res) = setup(200, 5, 40, 0);
          res.encounter.battleflag = false;

          for _ in 0..64 {
              run(&mut world, &mut res);
          }

          let stats = world.get::<&HeroStats>(res.hero_entity).unwrap();
          assert!(stats.vitality > 5, "vitality must increase during sleep");
          assert!(stats.vitality <= 25, "vitality must not exceed cap (15 + brave/4)");
          // Expect approximately 3-4 healing events in 64 ticks.
          let healed = stats.vitality - 5;
          assert!(healed >= 3 && healed <= 5,
              "expected 3-5 healing events in 64 sleep ticks, got {}", healed);
      }
  }
  ```

- [ ] **Step 8: Run tests**

  ```bash
  cargo test systems::sleep::tests 2>&1 | grep -E "^test result|FAILED"
  ```

  Expected: `test result: ok. 6 passed`.

- [ ] **Step 9: Commit**

  ```bash
  git add src/game/ecs/systems/sleep.rs
  git commit -m "test(ecs): add 6 unit tests for sleep system"
  ```

---

## Completion check

```bash
cargo build 2>&1 | grep "^error"
cargo test systems::sleep 2>&1 | grep -E "^test result|FAILED"
```

Both succeed. The sleep system is fully implemented, wired into `run_tick()`, and covered by 6 unit tests.

---

## Spec references

- `docs/spec/survival.md` SS18.4 — sleep mechanics (fatigue recovery, healing, wake conditions)
- `docs/spec/daynight-cycle.md` SS17.1-17.2 — daynight counter, lightlevel triangle wave
- `docs/reqs/survival.md` R-SURV-010 — sleep processing requirements

## Test plan

- `daynight_advances_by_63` — after one `run()`, daynight == start + 63
- `fatigue_decrements` — stats.fatigue decremented by 1, still sleeping
- `wakes_when_fatigue_zero` — sleeping set to false when fatigue reaches 0
- `wakes_in_dawn_window_with_low_fatigue` — daynight=9563, fatigue=19 -> wakes
- `snaps_y_on_wake` — pos.y = 123.5 -> aligned to 96.0 (0xFFE0 grid)
- `healing_at_64x_rate` — 64 sleep ticks -> ~3-5 healing events

## Files touched

| File | Change |
|------|--------|
| `src/game/ecs/systems/sleep.rs` | Create with `run()` + 5 helpers + 6 unit tests |
| `src/game/ecs/systems/mod.rs` | Add `pub mod sleep;` |
| `src/game/ecs/scene.rs` | Replace sleep skip comment with `systems::sleep::run()` call |
