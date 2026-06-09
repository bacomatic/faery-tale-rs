---
title: "Plan N — Combat System (Melee)"
plan: N
status: draft
depends_on: []
touches:
  - src/game/ecs/systems/combat.rs
  - src/game/ecs/systems/damage.rs
  - src/game/ecs/systems/mod.rs
  - src/game/ecs/systems/input.rs
  - src/game/ecs/scene.rs
---

# ECS Migration Plan N: Combat System (Melee)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port melee combat from legacy `src/game/combat.rs` into the ECS via `CombatSystem`. Implement hero melee attack detection, hit probability calculation, damage application to enemies, and enemy counter-attacks. Integrate with the existing `DamageEvent` pattern established by `MissileSystem`.

**Architecture:** `combat::run()` is an existing stub in `src/game/ecs/systems/combat.rs`. This plan replaces that stub with the full melee implementation. Damage is not applied inside `combat::run()` — instead, `DamageEvent` entries are pushed to `res.events.damage`, which a new `damage::run()` system drains each tick. This mirrors how `MissileSystem` already works and keeps hit detection decoupled from health mutation. `input::run()` gains fire-button detection to enter `ActorState::Fighting`. All required ECS component types (`Health`, `CombatState`, `AiState`, `EnemyKind`) already exist from Plans A-D.

**Prerequisites:** None (standalone). All required components already exist from Plans A-D. `MissileSystem` provides the established `DamageEvent` pattern to follow.

**Tech Stack:** Rust 2021, `hecs = "0.11"`, SDL3.

---

## File map

| File | Change |
|------|--------|
| `src/game/ecs/systems/combat.rs` | Replace stub — implement `run()` |
| `src/game/ecs/systems/damage.rs` | **Create** — `DamageSystem::run()` |
| `src/game/ecs/systems/mod.rs` | Add `pub mod damage;` |
| `src/game/ecs/systems/input.rs` | Detect fire button; advance `Fighting(n)` frame counter |
| `src/game/ecs/scene.rs` | Call `damage::run()` after `combat::run()` in schedule |

---

## Background: legacy combat formulas

The following formulas are derived from `reference/logic/combat.md` on the research branch and `docs/spec/combat.md` §10.1–10.3. All must be matched exactly.

### Strike point

The attack origin for hit detection is offset from the hero's tile center in the current facing direction, then jittered by a small random term:

```
xs = hero_x + facing_offset_x(weapon * 2) + rand8() - 3
ys = hero_y + facing_offset_y(weapon * 2) + rand8() - 3
```

`weapon_tip(x, y, facing, weapon_code)` in `src/game/combat.rs` encapsulates this calculation. Call it unchanged — do not re-derive the offset table.

### Reach

Hero reach is bravery-dependent, clamped to [4, 15]:

```
reach_hero = clamp((brave / 20) + 5, 4, 15)
```

Enemy reach is randomized each attack:

```
reach_enemy = 2 + rand4()   // range [2, 5]
```

`combat_reach(is_hero, brave, tick)` in `src/game/combat.rs` encapsulates both. Call it unchanged.

### Hit condition

An attack lands when:
1. Chebyshev distance from strike point to target centre < reach, **and**
2. `res.clock.freeze_timer == 0`

### Damage

```
damage = weapon_code + rand3()    // rand3() = bitrand(2) = 0, 1, or 2
```

Touch attacks (weapon_code == 0) are capped at 5 total damage.

### Enemy counter-attack

After processing all hero-to-enemy hits, each enemy in `NpcState::Fighting` rolls a counter:

```
hits = rand256() > hero_brave
```

If `hits`, emit `DamageEvent` targeting the hero entity.

---

## Background: immunity table

Immunity checks run **before** any `DamageEvent` is pushed. Immune targets are silently skipped.

| Race | Condition | Message |
|------|-----------|---------|
| 0x8a (Spectre) | Always immune | Silent |
| 0x8b (Ghost) | Always immune | Silent |
| 9 (Necromancer) | `weapon_code < 4` | emit `SpeechEvent(58)` |
| 0x89 (Witch) | `weapon_code < 4 && stuff[7] == 0` | emit `SpeechEvent(58)` |

speech_id 58 maps to the "ineffective weapon" message. Verify the exact ID against `reference/logic/dialog_system.md` on the research branch before shipping.

---

## Background: DamageEvent pattern

`MissileSystem` already uses this pattern. Follow it exactly:

```rust
res.events.damage.push(DamageEvent {
    target:             entity,
    amount:             damage as i16,
    weapon:             weapon_code,
    is_friendly_fire:   false,
});
```

`DamageEvent` and `EnemyDiedEvent` are already defined in `src/game/ecs/events.rs`.

---

## Task 1: Implement `CombatSystem::run()`

**Files:**
- Modify: `src/game/ecs/systems/combat.rs`

- [ ] **Step 1: Add imports**

  Replace or extend the current import block at the top of the file:

  ```rust
  use hecs::World;
  use crate::game::ecs::resources::Resources;
  use crate::game::ecs::components::{
      Position, Facing, CombatState, HeroStats, Inventory,
      AiState, EnemyKind, Health,
  };
  use crate::game::ecs::events::DamageEvent;
  use crate::game::actor::ActorState;
  use crate::game::npc::NpcState;
  use crate::game::combat::{weapon_tip, combat_reach};
  ```

- [ ] **Step 2: Implement hero attack pass**

  Replace the stub `pub fn run(...)` body with:

  ```rust
  pub fn run(world: &mut World, res: &mut Resources) {
      // Skip entirely if freeze timer is active.
      if res.clock.freeze_timer > 0 {
          return;
      }

      // --- Hero attack ---
      let hero = res.hero_entity;

      // Only attack while in Fighting state.
      let fighting = match world.get::<CombatState>(hero) {
          Ok(cs) => matches!(cs.state, ActorState::Fighting(_)),
          Err(_) => false,
      };
      if !fighting {
          // Still need to process enemy counter-attacks even when hero is not
          // attacking, so do not return here — fall through to enemy pass.
      } else {
          let (hx, hy, hfacing, weapon_code, brave) = {
              let pos   = world.get::<Position>(hero).expect("hero must have Position");
              let face  = world.get::<Facing>(hero).expect("hero must have Facing");
              let cs    = world.get::<CombatState>(hero).expect("hero must have CombatState");
              let stats = world.get::<HeroStats>(hero).expect("hero must have HeroStats");
              (pos.x, pos.y, face.dir, cs.weapon, stats.brave)
          };

          // Compute jittered strike point.
          let (sx, sy) = weapon_tip(hx, hy, hfacing, weapon_code);
          let reach = combat_reach(true, brave, res.clock.tick);

          // Collect inventory once for Witch immunity check.
          let sun_stone = world
              .get::<Inventory>(hero)
              .map(|inv| inv.stuff[7] > 0)
              .unwrap_or(false);

          // Query all enemies for hit detection.
          let targets: Vec<(hecs::Entity, i16, u8, f32, f32)> = world
              .query::<(&Health, &EnemyKind, &Position)>()
              .iter()
              .map(|(e, (h, ek, p))| (e, h.vitality, ek.race, p.x, p.y))
              .collect();

          for (entity, _vitality, race, ex, ey) in targets {
              // Chebyshev distance from strike point to enemy centre.
              let dist = (sx - ex).abs().max((sy - ey).abs());
              if dist >= reach as f32 {
                  continue;
              }

              // Immunity checks.
              if race == 0x8a || race == 0x8b {
                  // Spectre / Ghost — always immune, silent.
                  continue;
              }
              if race == 9 && weapon_code < 4 {
                  // Necromancer immune to non-enchanted weapons.
                  res.events.speech.push(crate::game::ecs::events::SpeechEvent {
                      speech_id: 58,
                      brother_name: res.brother.active_name.clone(),
                  });
                  continue;
              }
              if race == 0x89 && weapon_code < 4 && !sun_stone {
                  // Witch immune without Sun Stone.
                  res.events.speech.push(crate::game::ecs::events::SpeechEvent {
                      speech_id: 58,
                      brother_name: res.brother.active_name.clone(),
                  });
                  continue;
              }

              // Damage roll.
              let base   = weapon_code as i16;
              let bonus  = crate::game::rng::bitrand(2) as i16; // 0–2
              let amount = if weapon_code == 0 {
                  (base + bonus).min(5)
              } else {
                  base + bonus
              };

              res.events.damage.push(DamageEvent {
                  target:           entity,
                  amount,
                  weapon:           weapon_code,
                  is_friendly_fire: false,
              });
          }
      }

      // --- Enemy counter-attack pass ---
      let brave = world
          .get::<HeroStats>(hero)
          .map(|s| s.brave)
          .unwrap_or(0);

      let enemy_attackers: Vec<(hecs::Entity, f32, f32)> = world
          .query::<(&AiState, &Position)>()
          .iter()
          .filter(|(_, (ai, _))| matches!(ai.state, NpcState::Fighting))
          .map(|(e, (_, p))| (e, p.x, p.y))
          .collect();

      let hero_pos = world.get::<Position>(hero).map(|p| (p.x, p.y)).unwrap_or((0.0, 0.0));

      for (_enemy, ex, ey) in enemy_attackers {
          // Only enemies within their own reach can counter.
          let reach = combat_reach(false, 0, res.clock.tick);
          let dist  = (ex - hero_pos.0).abs().max((ey - hero_pos.1).abs());
          if dist >= reach as f32 {
              continue;
          }

          // Roll: hits if rand256() > hero brave.
          if crate::game::rng::rand256() > brave as u8 {
              res.events.damage.push(DamageEvent {
                  target:           hero,
                  amount:           1, // Enemy touch damage; exact value from spec §10.2
                  weapon:           0,
                  is_friendly_fire: false,
              });
          }
      }
  }
  ```

- [ ] **Step 3: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

  Expected: no errors. Fix any missing `rng` paths or component field names to match the existing codebase — do not change the component definitions.

---

## Task 2: Create `DamageSystem` in `damage.rs`

**Files:**
- Create: `src/game/ecs/systems/damage.rs`

- [ ] **Step 1: Create the file**

  ```rust
  //! Damage system — drains the DamageEvent queue and applies vitality reduction.
  //! Triggers enemy death when vitality reaches zero.

  use hecs::World;
  use crate::game::ecs::resources::Resources;
  use crate::game::ecs::components::{Health, AiState, EnemyKind, Position};
  use crate::game::ecs::events::{DamageEvent, EnemyDiedEvent};
  use crate::game::npc::NpcState;

  /// Drain `res.events.damage`, reduce target vitality, and trigger death when
  /// vitality reaches zero or below.
  ///
  /// Called immediately after `combat::run()` and `missile::run()` in the tick
  /// schedule so that all damage from a single tick is applied together.
  pub fn run(world: &mut World, res: &mut Resources) {
      // Drain into a local Vec to avoid borrow conflicts with world queries.
      let events: Vec<DamageEvent> = res.events.damage.drain(..).collect();

      for ev in events {
          apply_damage(world, res, ev);
      }
  }

  fn apply_damage(world: &mut World, res: &mut Resources, ev: DamageEvent) {
      // Hero is a special case — reduce HeroStats.vitality, not Health component.
      if ev.target == res.hero_entity {
          if let Ok(mut stats) = world.get_mut::<crate::game::ecs::components::HeroStats>(ev.target) {
              stats.vitality = (stats.vitality - ev.amount).max(0);
              // Hero death is handled by death::run() watching vitality == 0.
          }
          return;
      }

      // Enemy target.
      let vitality_after = match world.get_mut::<Health>(ev.target) {
          Ok(mut h) => {
              h.vitality = (h.vitality - ev.amount).max(0);
              h.vitality
          }
          Err(_) => return, // Entity already despawned.
      };

      if vitality_after <= 0 {
          trigger_death(world, res, ev.target, ev.weapon);
      }
  }

  fn trigger_death(world: &mut World, res: &mut Resources, entity: hecs::Entity, weapon: u8) {
      // Collect fields needed for EnemyDiedEvent before mutating AiState.
      let race = world
          .get::<EnemyKind>(entity)
          .map(|ek| ek.race)
          .unwrap_or(0);
      let (x, y) = world
          .get::<Position>(entity)
          .map(|p| (p.x, p.y))
          .unwrap_or((0.0, 0.0));

      // Gold drop is looked up from the NPC table at death time.
      // Placeholder 0 until NpcTable lookup is wired; combat spec §10.3 gives the table.
      let gold = 0u16;

      // Transition NPC to Dying state — sprite system will play the death animation.
      if let Ok(mut ai) = world.get_mut::<AiState>(entity) {
          ai.state = NpcState::Dying;
      }

      res.events.enemy_died.push(EnemyDiedEvent {
          entity,
          race,
          weapon,
          gold,
          x,
          y,
      });
  }
  ```

- [ ] **Step 2: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

  Expected: no errors. `EnemyDiedEvent` struct field names must match what is already defined in `events.rs` — adjust the struct literal fields if they differ.

---

## Task 3: Register `damage` module

**Files:**
- Modify: `src/game/ecs/systems/mod.rs`

- [ ] **Step 1: Add `pub mod damage;`**

  Find the existing block of `pub mod` declarations and append:

  ```rust
  pub mod damage;
  ```

- [ ] **Step 2: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

  Expected: no errors.

---

## Task 4: Update `input.rs` — fire button and Fighting frame counter

**Files:**
- Modify: `src/game/ecs/systems/input.rs`

- [ ] **Step 1: Detect fire button press**

  In the button/key handling section of `input::run()` (or `input::handle_event()`), add:

  ```rust
  // Fire button: SDL Scancode::Space or gamepad button A.
  // On press: enter ActorState::Fighting(0) if not already fighting.
  if input_state.fire_just_pressed() {
      if let Ok(mut cs) = world.get_mut::<CombatState>(res.hero_entity) {
          if !matches!(cs.state, ActorState::Fighting(_)) {
              cs.state = ActorState::Fighting(0);
          }
      }
  }
  ```

  The exact method name for "just pressed" depends on how `InputState` tracks edge detection in this codebase. Match the pattern already used for movement buttons.

- [ ] **Step 2: Advance Fighting frame counter each tick**

  In the per-tick update section of `input::run()`, add the frame counter advance after movement handling:

  ```rust
  // Advance Fighting animation frame counter.
  // Fighting(n) → Fighting(n+1); at n == 7 → back to Still (attack complete).
  if let Ok(mut cs) = world.get_mut::<CombatState>(res.hero_entity) {
      if let ActorState::Fighting(n) = cs.state {
          if n >= 7 {
              cs.state = ActorState::Still;
          } else {
              cs.state = ActorState::Fighting(n + 1);
          }
      }
  }
  ```

- [ ] **Step 3: Map fire button in InputState**

  If `fire_just_pressed()` does not exist, add it. The fire button maps to:
  - Keyboard: `Scancode::Space`
  - Gamepad: Button A (SDL gamepad button index 0)

  Match the pattern used by the existing movement button mapping — do not invent a new abstraction.

- [ ] **Step 4: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

  Expected: no errors.

- [ ] **Step 5: Commit task 1–4**

  ```bash
  git add src/game/ecs/systems/combat.rs \
          src/game/ecs/systems/damage.rs \
          src/game/ecs/systems/mod.rs \
          src/game/ecs/systems/input.rs
  git commit -m "feat(ecs): implement CombatSystem, DamageSystem, fire input"
  ```

---

## Task 5: Wire `damage::run()` into `EcsScene`

**Files:**
- Modify: `src/game/ecs/scene.rs`

- [ ] **Step 1: Add `damage::run()` call in `run_tick()`**

  In `EcsScene::run_tick()`, find the existing `systems::combat::run(...)` call and add `damage::run()` immediately after:

  ```rust
  systems::combat::run(&mut self.world, &mut self.res);
  systems::damage::run(&mut self.world, &mut self.res);   // ← add this line
  systems::missile::run(&mut self.world, &mut self.res);
  ```

  This ordering ensures that melee damage is applied before missile damage within the same tick, matching the legacy `GameplayScene` system schedule.

- [ ] **Step 2: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

  Expected: no errors.

- [ ] **Step 3: Commit**

  ```bash
  git add src/game/ecs/scene.rs
  git commit -m "feat(ecs): call damage::run() after combat::run() in EcsScene tick schedule"
  ```

---

## Task 6: Add unit tests in `combat.rs`

**Files:**
- Modify: `src/game/ecs/systems/combat.rs`

Five tests covering the primary combat paths and both immunity cases.

- [ ] **Step 1: Add test module**

  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      use hecs::World;
      use crate::game::ecs::resources::Resources;
      use crate::game::ecs::components::{
          Position, Facing, CombatState, HeroStats, Inventory,
          AiState, EnemyKind, Health,
      };
      use crate::game::actor::ActorState;
      use crate::game::npc::NpcState;

      /// Spawn a minimal hero in Fighting state and return (World, Resources).
      fn setup_hero_fighting(weapon: u8, brave: i16, hx: f32, hy: f32) -> (World, Resources) {
          let mut world = World::new();
          let hero = world.spawn((
              Position::new(hx, hy),
              Facing::default(),
              CombatState { state: ActorState::Fighting(0), weapon },
              HeroStats { brave, vitality: 100, ..HeroStats::default() },
              Inventory::empty(),
          ));
          let mut res = Resources::new(hero);
          res.clock.freeze_timer = 0;
          (world, res)
      }

      /// Spawn an enemy at the given position with the given race.
      fn spawn_enemy(world: &mut World, race: u8, ex: f32, ey: f32, vitality: i16) -> hecs::Entity {
          world.spawn((
              Position::new(ex, ey),
              EnemyKind { race },
              Health { vitality, max: vitality },
              AiState { state: NpcState::Wandering },
          ))
      }

      // --- hero attacks enemy in range → DamageEvent emitted ---

      #[test]
      fn hero_fighting_enemy_in_range_emits_damage_event() {
          let (mut world, mut res) = setup_hero_fighting(2, 40, 100.0, 100.0);
          // Enemy 10px away — well within reach (brave=40 → reach=7).
          let enemy = spawn_enemy(&mut world, 5, 110.0, 100.0, 20);

          run(&mut world, &mut res);

          assert_eq!(res.events.damage.len(), 1, "expected exactly one DamageEvent");
          assert_eq!(res.events.damage[0].target, enemy);
          assert_eq!(res.events.damage[0].weapon, 2);
          assert!(!res.events.damage[0].is_friendly_fire);
      }

      // --- enemy in Fighting state, brave=0 → high hit probability on hero ---

      #[test]
      fn enemy_counter_attack_brave_zero_high_hit_rate() {
          // brave=0 → rand256() > 0 is almost always true.
          let (mut world, mut res) = setup_hero_fighting(0, 0, 100.0, 100.0);

          // Enemy at exactly 3px away in Fighting state.
          let _enemy = {
              let e = world.spawn((
                  Position::new(103.0, 100.0),
                  EnemyKind { race: 1 },
                  Health { vitality: 20, max: 20 },
                  AiState { state: NpcState::Fighting },
              ));
              e
          };

          // Run many times and count how often hero is hit.
          let trials = 100u32;
          let mut hits = 0u32;
          for _ in 0..trials {
              res.events.damage.clear();
              run(&mut world, &mut res);
              if res.events.damage.iter().any(|ev| ev.target == res.hero_entity) {
                  hits += 1;
              }
          }

          // With brave=0, expect > 90% hit rate.
          assert!(hits > 85, "expected high hit rate with brave=0, got {}/{}", hits, trials);
      }

      // --- Spectre is immune to all damage ---

      #[test]
      fn spectre_is_immune_to_all_damage() {
          let (mut world, mut res) = setup_hero_fighting(5, 80, 100.0, 100.0);
          // Spectre (race 0x8a) 5px away.
          let _spectre = spawn_enemy(&mut world, 0x8a, 105.0, 100.0, 30);

          run(&mut world, &mut res);

          let spectre_events: Vec<_> = res.events.damage.iter()
              .filter(|ev| ev.target != res.hero_entity)
              .collect();
          assert!(spectre_events.is_empty(), "Spectre must not receive any DamageEvent");
          // No speech event either.
          assert!(res.events.speech.is_empty(), "Spectre immunity is silent");
      }

      // --- Witch is immune without Sun Stone, vulnerable with it ---

      #[test]
      fn witch_immune_without_sun_stone_vulnerable_with_it() {
          // Without Sun Stone (stuff[7] == 0) and weapon < 4.
          {
              let (mut world, mut res) = setup_hero_fighting(1, 80, 100.0, 100.0);
              let _witch = spawn_enemy(&mut world, 0x89, 104.0, 100.0, 30);

              run(&mut world, &mut res);

              let witch_hits: Vec<_> = res.events.damage.iter()
                  .filter(|ev| ev.target != res.hero_entity)
                  .collect();
              assert!(witch_hits.is_empty(), "Witch must be immune without Sun Stone");
              assert!(!res.events.speech.is_empty(), "Witch immunity emits speech event");
          }

          // With Sun Stone (stuff[7] == 1) and same weak weapon → should hit.
          {
              let (mut world, mut res) = setup_hero_fighting(1, 80, 100.0, 100.0);
              // Grant Sun Stone.
              if let Ok(mut inv) = world.get_mut::<Inventory>(res.hero_entity) {
                  inv.stuff[7] = 1;
              }
              let witch = spawn_enemy(&mut world, 0x89, 104.0, 100.0, 30);

              run(&mut world, &mut res);

              let witch_hits: Vec<_> = res.events.damage.iter()
                  .filter(|ev| ev.target == witch)
                  .collect();
              assert!(!witch_hits.is_empty(), "Witch must be vulnerable when hero has Sun Stone");
          }
      }
  }
  ```

- [ ] **Step 2: Add `DamageSystem` death test in `damage.rs`**

  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;
      use hecs::World;
      use crate::game::ecs::resources::Resources;
      use crate::game::ecs::components::{Health, AiState, EnemyKind, Position};
      use crate::game::ecs::events::DamageEvent;
      use crate::game::npc::NpcState;

      #[test]
      fn damage_exceeds_vitality_triggers_death_event() {
          let mut world = World::new();
          let hero = world.spawn((
              crate::game::ecs::components::HeroStats { vitality: 100, ..Default::default() },
              Position::new(0.0, 0.0),
          ));
          let mut res = Resources::new(hero);

          // Spawn an enemy with 5 vitality.
          let enemy = world.spawn((
              Health { vitality: 5, max: 5 },
              EnemyKind { race: 3 },
              AiState { state: NpcState::Wandering },
              Position::new(10.0, 10.0),
          ));

          // Push a DamageEvent that exceeds vitality.
          res.events.damage.push(DamageEvent {
              target:           enemy,
              amount:           10,
              weapon:           2,
              is_friendly_fire: false,
          });

          run(&mut world, &mut res);

          // Health should be clamped to 0.
          let health = world.get::<Health>(enemy).unwrap();
          assert_eq!(health.vitality, 0);

          // EnemyDiedEvent must be emitted.
          assert_eq!(res.events.enemy_died.len(), 1);
          assert_eq!(res.events.enemy_died[0].entity, enemy);
          assert_eq!(res.events.enemy_died[0].race, 3);

          // AiState must transition to Dying.
          let ai = world.get::<AiState>(enemy).unwrap();
          assert!(matches!(ai.state, NpcState::Dying));
      }
  }
  ```

- [ ] **Step 3: Run all tests**

  ```bash
  cargo test ecs::systems::combat::tests 2>&1 | grep -E "^test result|FAILED"
  cargo test ecs::systems::damage::tests 2>&1 | grep -E "^test result|FAILED"
  ```

  Expected: `test result: ok. 4 passed` for combat, `test result: ok. 1 passed` for damage.

- [ ] **Step 4: Commit**

  ```bash
  git add src/game/ecs/systems/combat.rs src/game/ecs/systems/damage.rs
  git commit -m "test(ecs): add melee combat and damage system unit tests"
  ```

---

## Task 7: Integration testing

- [ ] **Step 1: Full build**

  ```bash
  cargo build 2>&1 | grep "^error"
  ```

  Expected: no errors.

- [ ] **Step 2: Run full test suite**

  ```bash
  cargo test 2>&1 | grep -E "^test result|FAILED"
  ```

  Expected: all suites pass.

- [ ] **Step 3: Manual smoke test**

  With the game running under `--ecs`:

  1. Start with Julian.
  2. Walk up to a Slime (race 1) — confirm it pursues.
  3. Press Space — Julian's attack animation plays (8 frames at 15 Hz = ~0.5 s).
  4. Slime loses vitality visible in debug TUI (`--features debug-tui`).
  5. Hit Slime until dead — confirm it transitions to Dying animation, then despawns.
  6. Confirm `EnemyDiedEvent` is logged in debug TUI.
  7. Stand still and let a Fighting enemy attack — confirm Julian's vitality decreases.
  8. Walk toward a Spectre — swing weapon — confirm no vitality loss on Spectre.
  9. Swing with weapon < 4 at a Witch (no Sun Stone) — confirm "ineffective" scroll message appears and Witch takes no damage.
  10. Grant Sun Stone via debug console. Swing at Witch again — confirm damage lands.

- [ ] **Step 4: Final commit**

  ```bash
  git add -A
  git commit -m "feat(ecs): melee combat system fully wired — CombatSystem + DamageSystem + fire input"
  ```

---

## Completion check

```bash
cargo build 2>&1 | grep "^error"
cargo test ecs::systems::combat 2>&1 | grep -E "^test result|FAILED"
cargo test ecs::systems::damage 2>&1 | grep -E "^test result|FAILED"
```

All three succeed. `CombatSystem` emits `DamageEvent`s. `DamageSystem` applies health reduction and triggers `EnemyDiedEvent`. Fire button enters `ActorState::Fighting`. `damage::run()` is called in the tick schedule after `combat::run()`.

---

## Spec references

- `docs/spec/combat.md` §10.1–10.3 — melee hit detection, damage formulas, immunity table
- `docs/reqs/combat.md` R-COMBAT-001 to R-COMBAT-006
- `reference/logic/combat.md` (research branch) — original `dohit()` implementation, reach and damage tables
- `reference/logic/dialog_system.md` (research branch) — speech ID for "ineffective weapon" message (verify ID 58)

## Test plan

- `hero_fighting_enemy_in_range_emits_damage_event` — hero in Fighting state with enemy in reach emits DamageEvent
- `enemy_counter_attack_brave_zero_high_hit_rate` — enemy in Fighting state with hero brave=0 hits at high rate
- `spectre_is_immune_to_all_damage` — Spectre (race 0x8a) receives no DamageEvent, no speech event
- `witch_immune_without_sun_stone_vulnerable_with_it` — Witch immune without Sun Stone; emits speech; vulnerable with Sun Stone
- `damage_exceeds_vitality_triggers_death_event` — DamageEvent with amount > vitality sets vitality to 0, emits EnemyDiedEvent, sets NpcState::Dying

## Files touched

| File | Change |
|------|--------|
| `src/game/ecs/systems/combat.rs` | Replace stub — implement `run()` with hero attack and enemy counter-attack |
| `src/game/ecs/systems/damage.rs` | Create `DamageSystem::run()` — drain events, apply vitality, trigger death |
| `src/game/ecs/systems/mod.rs` | Add `pub mod damage;` |
| `src/game/ecs/systems/input.rs` | Detect fire button press; advance `Fighting(n)` frame counter per tick |
| `src/game/ecs/scene.rs` | Call `damage::run()` after `combat::run()` in tick schedule |
