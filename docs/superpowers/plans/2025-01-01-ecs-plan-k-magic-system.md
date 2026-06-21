# ECS Migration Plan K: Magic System (ECS)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the complete magic item system from `src/game/magic.rs` (legacy `GameState`-based) to the ECS architecture, enabling the MAGIC menu to dispatch spell casts through `magic_dispatch_ecs()`, which reads/writes hero inventory and spell timers from ECS components and resources, emits appropriate events, and triggers visual effects.

**Architecture:** The legacy `magic.rs` already contains a spec-faithful `use_magic()` implementation (`docs/spec/magic.md` §19). Plan K adds `magic_dispatch_ecs()` that performs the same logic using ECS components (`Inventory`, `HeroStats`, `Position`, `Facing`, `CarrierMount`, `Health`, `EnemyKind`, `ArenaDummy`) and resources (`GameClock`, `RegionState`, `ViewState`, `BrotherRoster`). The MAGIC menu (already wired in Plan I) calls this function via `MenuAction::CastSpell`. Scroll messages are emitted by the caller from `faery.toml` `[narr]` tables or the documented hardcoded literal in `reference/logic/dialog_system.md`.

**Prerequisites:** Plans I (menu dispatch) and J (inventory). Plans A-D complete.

**Tech Stack:** Rust 2021, `hecs = "0.11"`, SDL3.

---

## File map

| File | Change |
|---|---|
| `src/game/ecs/resources.rs` | Replace VFX placeholders with real types |
| `src/game/world_data.rs` | Add `WorldData::sector_at_pos()` helper |
| `src/game/magic.rs` | Add `magic_dispatch_ecs()` + ECS unit tests |
| `src/game/ecs/scene.rs` | Wire `CastSpell` in `dispatch_menu_action()` |

---

## Task 1: Replace VFX placeholders in resources.rs

**Files:**
- Modify: `src/game/ecs/resources.rs`

- [ ] **Step 1: Remove placeholder types (lines 242–248)**

Remove these lines:
```rust
// Placeholder — real type will be moved from gfx_effects in Plan D.
#[derive(Debug, Clone, Default)]
pub struct WitchEffectPlaceholder;

/// Placeholder — real type will be moved from gfx_effects in Plan D.
#[derive(Debug, Clone, Default)]
pub struct TeleportEffectPlaceholder;
```

- [ ] **Step 2: Import real VFX types**

Add at top with other imports:
```rust
use crate::game::gfx_effects::{WitchEffect, TeleportEffect};
```

- [ ] **Step 3: Update VfxState struct**

Replace the struct with:
```rust
/// Active visual effects.
#[derive(Default)]
pub struct VfxState {
    pub witch_effect: WitchEffect,
    pub teleport_effect: TeleportEffect,
}
```

- [ ] **Step 4: Update VfxState::default()**

`WitchEffect` and `TeleportEffect` both implement `Default`, so the derived `Default` on `VfxState` remains valid. No further change is needed.

- [ ] **Step 5: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors related to VfxState.

---

## Task 2: Add hero-sector lookup helper to world_data.rs

The original `use_magic()` reads `state.hero_sector` to gate the Blue Stone. In ECS, the sector is stored in the loaded `WorldData` map, so we add a helper that computes the sector index at the hero’s world pixel position.

**Files:**
- Modify: `src/game/world_data.rs`

- [ ] **Step 1: Add `WorldData::sector_at_pos()`**

Insert after `sector_at()`:

```rust
impl WorldData {
    /// Sector index at absolute world pixel coordinates.
    /// Mirrors the original `hero_sector` update: `mapxy[hero_x / 256][hero_y / 256]`.
    pub fn sector_at_pos(&self, x: f32, y: f32) -> u16 {
        let mx = (x as u16 >> 8) as usize;
        let my = (y as u16 >> 8) as usize;
        self.sector_at(mx, my) as u16
    }
}
```

- [ ] **Step 2: Add unit test**

Append to the existing `#[cfg(test)]` module in `src/game/world_data.rs`:

```rust
#[test]
fn sector_at_pos_reads_map_mem() {
    let mut w = WorldData::empty();
    // Set sector (54, 43) to index 144.
    w.map_mem[43 * 128 + 54] = 144;
    let x = ((54u16 << 8) | 85) as f32;
    let y = ((43u16 << 8) | 64) as f32;
    assert_eq!(w.sector_at_pos(x, y), 144);
}
```

- [ ] **Step 3: Verify compile**

```bash
cargo test world_data::tests::sector_at_pos 2>&1 | grep "^test result"
```

Expected: test passes.

---

## Task 3: Implement `magic_dispatch_ecs()`

**Files:**
- Modify: `src/game/magic.rs`

- [ ] **Step 1: Add ECS imports**

Add with the other imports at the top of the file:

```rust
use hecs::World;
use crate::game::ecs::resources::Resources;
use crate::game::ecs::components::{ArenaDummy, CarrierMount, Enemy, EnemyKind, Facing, Health, HeroStats, Inventory, Position};
```

- [ ] **Step 2: Implement `magic_dispatch_ecs()`**

Add after the existing `use_magic()` function:

```rust
/// Dispatch a MAGIC menu cast using ECS components/resources.
/// Mirrors `use_magic()` (fmain.c:3300-3365) and `docs/spec/magic.md` §19.
/// Scroll messages are the caller's responsibility.
pub fn magic_dispatch_ecs(item_idx: usize, world: &mut World, res: &mut Resources) -> MagicResult {
    if item_idx < ITEM_BLUE_STONE || item_idx > ITEM_JADE_SKULL {
        return MagicResult::NoOwned;
    }

    let hero = res.hero_entity;
    let inventory = match world.get::<&Inventory>(hero) {
        Ok(inv) => inv,
        Err(_) => return MagicResult::NoOwned,
    };
    if inventory.stuff[item_idx] == 0 {
        return MagicResult::NoOwned;
    }
    drop(inventory);

    let result = match item_idx {
        ITEM_BLUE_STONE => stone_ring_effect_ecs(world, res, hero),
        ITEM_GREEN_JEWEL => {
            res.clock.light_timer = res.clock.light_timer.saturating_add(LIGHT_TIMER_INCREMENT);
            MagicResult::Applied
        }
        ITEM_GLASS_VIAL => {
            let capped = apply_vial_heal_ecs(world, hero);
            MagicResult::Healed { capped }
        }
        ITEM_CRYSTAL_ORB => {
            res.clock.secret_timer = res.clock.secret_timer.saturating_add(SECRET_TIMER_INCREMENT);
            MagicResult::Applied
        }
        ITEM_BIRD_TOTEM => {
            if res.region.region_num > 7 && !res.brother.cheat1 {
                return MagicResult::Suppressed;
            }
            res.view.viewstatus = 1;
            MagicResult::Applied
        }
        ITEM_GOLD_RING => {
            if let Ok(cm) = world.get::<&CarrierMount>(hero) {
                if cm.riding > 1 {
                    return MagicResult::Suppressed;
                }
            }
            res.clock.freeze_timer = res.clock.freeze_timer.saturating_add(FREEZE_TIMER_INCREMENT);
            MagicResult::Applied
        }
        ITEM_JADE_SKULL => skull_effect_ecs(world, res, hero),
        _ => return MagicResult::NoOwned,
    };

    if matches!(result, MagicResult::Applied | MagicResult::Healed { .. } | MagicResult::StoneTeleport { .. } | MagicResult::MassKill { .. }) {
        if let Ok(mut inv) = world.get::<&mut Inventory>(hero) {
            inv.stuff[item_idx] -= 1;
        }
    }
    result
}
```

- [ ] **Step 3: Implement ECS effect helpers**

Add after `magic_dispatch_ecs()`:

```rust
fn stone_ring_effect_ecs(world: &mut World, res: &mut Resources, hero: hecs::Entity) -> MagicResult {
    let mut q = world.query_one::<(&Position, &Facing)>(hero);
    let (hero_x, hero_y, facing_dir) = match q.get() {
        Ok((pos, facing)) => (pos.x as u16, pos.y as u16, facing.dir),
        Err(_) => return MagicResult::Suppressed,
    };
    drop(q);

    let hero_sector = res.map.world.as_ref()
        .map(|w| w.sector_at_pos(hero_x as f32, hero_y as f32))
        .unwrap_or(0);
    if hero_sector != STONE_RING_SECTOR {
        return MagicResult::Suppressed;
    }

    let hx_frac = (hero_x & 255) / 85;
    let hy_frac = (hero_y & 255) / 64;
    if hx_frac != 1 || hy_frac != 1 {
        return MagicResult::Suppressed;
    }

    let current = match find_current_ring(hero_x, hero_y) {
        Some(c) => c,
        None => return MagicResult::Suppressed,
    };

    let dest = (current + facing_dir as usize + 1) % STONE_RINGS.len();
    let (dx, dy) = STONE_RINGS[dest];
    let new_x = ((dx as u16) << 8) | (hero_x & 255);
    let new_y = ((dy as u16) << 8) | (hero_y & 255);

    if let Ok(mut pos) = world.get::<&mut Position>(hero) {
        pos.x = new_x as f32;
        pos.y = new_y as f32;
    }

    // TODO: drag the active carrier along with the hero (SPEC §21.7).
    // `res.carrier_entity` is not yet wired by the carrier system.

    let capped = apply_vial_heal_ecs(world, hero);
    MagicResult::StoneTeleport { capped }
}

fn apply_vial_heal_ecs(world: &mut World, hero: hecs::Entity) -> bool {
    let heal = rand8() + 4;
    if let Ok(mut stats) = world.get::<&mut HeroStats>(hero) {
        let cap = heal_cap(stats.brave);
        let raw = stats.vitality + heal;
        if raw > cap {
            stats.vitality = cap;
            true
        } else {
            stats.vitality = raw;
            false
        }
    } else {
        false
    }
}

fn skull_effect_ecs(world: &mut World, res: &mut Resources, _hero: hecs::Entity) -> MagicResult {
    let in_battle = res.region.battleflag;
    let mut slain: Vec<usize> = Vec::new();

    for (entity, health, enemy_kind) in world
        .query::<(hecs::Entity, &mut Health, &EnemyKind)>()
        .with::<&Enemy>()
        .without::<&ArenaDummy>()
        .iter()
    {
        if health.vitality > 0 && enemy_kind.race < 7 {
            health.vitality = 0;
            slain.push(entity.id() as usize);
        }
    }

    if slain.is_empty() {
        MagicResult::Suppressed
    } else {
        MagicResult::MassKill { slain, in_battle }
    }
}
```

- [ ] **Step 4: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors.

---

## Task 4: Wire `CastSpell` into `dispatch_menu_action()`

**Files:**
- Modify: `src/game/ecs/scene.rs`

- [ ] **Step 1: Add magic imports**

Near the top of the file, add:

```rust
use crate::game::magic::{magic_dispatch_ecs, MagicResult, ITEM_BLUE_STONE};
```

- [ ] **Step 2: Add `game_lib` parameter to `dispatch_menu_action()`**

Change the signature from:

```rust
fn dispatch_menu_action(&mut self, action: MenuAction, _resources: &mut SceneResources<'_, '_>) -> bool {
```

to:

```rust
fn dispatch_menu_action(&mut self, action: MenuAction, game_lib: &GameLibrary, _resources: &mut SceneResources<'_, '_>) -> bool {
```

- [ ] **Step 3: Update the call site in `update()`**

Change:

```rust
if self.dispatch_menu_action(action, resources) {
```

to:

```rust
if self.dispatch_menu_action(action, game_lib, resources) {
```

- [ ] **Step 4: Add the `CastSpell` branch**

Replace:

```rust
MenuAction::CastSpell(_) => {}
```

with:

```rust
MenuAction::CastSpell(hit) => {
    let item_idx = ITEM_BLUE_STONE + hit as usize;
    let result = magic_dispatch_ecs(item_idx, &mut self.world, &mut self.res);
    let name = game_lib
        .get_brother(self.res.brother.active_brother)
        .map(|b| b.name.as_str())
        .unwrap_or("Hero");

    match result {
        MagicResult::NoOwned => {
            self.res.events.message.push(crate::game::ecs::events::MessageEvent {
                text: crate::game::events::event_msg(&game_lib.narr, 21, name),
            });
        }
        MagicResult::Healed { capped: false } | MagicResult::StoneTeleport { capped: false } => {
            self.res.events.message.push(crate::game::ecs::events::MessageEvent {
                text: "That feels a lot better!".to_string(),
            });
        }
        MagicResult::MassKill { in_battle: true, .. } => {
            self.res.events.message.push(crate::game::ecs::events::MessageEvent {
                text: crate::game::events::event_msg(&game_lib.narr, 34, name),
            });
        }
        _ => {}
    }
}
```

- [ ] **Step 5: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors.

---

## Task 5: Add unit tests in magic.rs

**Files:**
- Modify: `src/game/magic.rs`

- [ ] **Step 1: Add the ECS test module**

Append after the existing `#[cfg(test)] mod tests`:

```rust
#[cfg(test)]
mod ecs_tests {
    use super::*;
    use hecs::World;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::components::{Inventory, HeroStats, Position, Facing, CarrierMount, EnemyKind, Health, ArenaDummy, Speed, Loot, SpriteRef};
    use crate::game::ecs::spawn::spawn_enemy;
    use crate::game::direction::Direction;

    fn test_hero(vitality: i16, brave: i16) -> HeroStats {
        HeroStats {
            vitality,
            brave,
            luck: 20,
            kind: 15,
            wealth: 20,
            hunger: 0,
            fatigue: 0,
            gold: 0,
        }
    }

    fn spawn_hero_with_inv(world: &mut World, slot: usize, count: u8) -> hecs::Entity {
        let mut stuff = [0u8; 36];
        stuff[slot] = count;
        world.spawn((
            Inventory { stuff },
            test_hero(30, 200),
            Position::new(0.0, 0.0),
            Facing::new(Direction::N),
            CarrierMount::default(),
        ))
    }

    #[test]
    fn green_jewel_adds_light_timer() {
        let mut world = World::new();
        let hero = spawn_hero_with_inv(&mut world, ITEM_GREEN_JEWEL, 1);
        let mut res = Resources::new(hero);
        res.clock.light_timer = 0;

        let result = magic_dispatch_ecs(ITEM_GREEN_JEWEL, &mut world, &mut res);
        assert_eq!(result, MagicResult::Applied);
        assert_eq!(res.clock.light_timer, LIGHT_TIMER_INCREMENT);
        assert_eq!(world.get::<&Inventory>(hero).unwrap().stuff[ITEM_GREEN_JEWEL], 0);
    }

    #[test]
    fn skull_no_inventory_returns_no_owned() {
        let mut world = World::new();
        let hero = world.spawn((
            Inventory::empty(),
            test_hero(30, 0),
            Position::new(0.0, 0.0),
            Facing::new(Direction::N),
            CarrierMount::default(),
        ));
        let mut res = Resources::new(hero);

        let result = magic_dispatch_ecs(ITEM_JADE_SKULL, &mut world, &mut res);
        assert_eq!(result, MagicResult::NoOwned);
    }

    #[test]
    fn vial_heals_vitality_uncapped() {
        let mut world = World::new();
        let hero = spawn_hero_with_inv(&mut world, ITEM_GLASS_VIAL, 1);
        let mut res = Resources::new(hero);

        let result = magic_dispatch_ecs(ITEM_GLASS_VIAL, &mut world, &mut res);
        assert!(matches!(result, MagicResult::Healed { capped: false }));
        let stats = world.get::<&HeroStats>(hero).unwrap();
        assert!(stats.vitality > 30 && stats.vitality <= 30 + 11);
    }

    #[test]
    fn vial_heal_capped_by_brave() {
        let mut world = World::new();
        let mut stuff = [0u8; 36];
        stuff[ITEM_GLASS_VIAL] = 1;
        let hero = world.spawn((
            Inventory { stuff },
            test_hero(38, 20), // cap = 15 + 20/4 = 20
            Position::new(0.0, 0.0),
            Facing::new(Direction::N),
            CarrierMount::default(),
        ));
        let mut res = Resources::new(hero);

        let result = magic_dispatch_ecs(ITEM_GLASS_VIAL, &mut world, &mut res);
        assert!(matches!(result, MagicResult::Healed { capped: true }));
        let stats = world.get::<&HeroStats>(hero).unwrap();
        assert_eq!(stats.vitality, 20);
    }

    #[test]
    fn ring_blocked_when_riding_returns_suppressed() {
        let mut world = World::new();
        let mut stuff = [0u8; 36];
        stuff[ITEM_GOLD_RING] = 1;
        let hero = world.spawn((
            Inventory { stuff },
            test_hero(30, 0),
            Position::new(0.0, 0.0),
            Facing::new(Direction::N),
            CarrierMount { riding: 2, ..Default::default() },
        ));
        let mut res = Resources::new(hero);

        let result = magic_dispatch_ecs(ITEM_GOLD_RING, &mut world, &mut res);
        assert_eq!(result, MagicResult::Suppressed);
        assert_eq!(res.clock.freeze_timer, 0);
    }

    #[test]
    fn ring_allowed_on_foot() {
        let mut world = World::new();
        let hero = spawn_hero_with_inv(&mut world, ITEM_GOLD_RING, 1);
        let mut res = Resources::new(hero);

        let result = magic_dispatch_ecs(ITEM_GOLD_RING, &mut world, &mut res);
        assert_eq!(result, MagicResult::Applied);
        assert_eq!(res.clock.freeze_timer, FREEZE_TIMER_INCREMENT);
    }

    #[test]
    fn orb_adds_secret_timer() {
        let mut world = World::new();
        let hero = spawn_hero_with_inv(&mut world, ITEM_CRYSTAL_ORB, 1);
        let mut res = Resources::new(hero);
        res.clock.secret_timer = 0;

        let result = magic_dispatch_ecs(ITEM_CRYSTAL_ORB, &mut world, &mut res);
        assert_eq!(result, MagicResult::Applied);
        assert_eq!(res.clock.secret_timer, SECRET_TIMER_INCREMENT);
    }

    #[test]
    fn totem_sets_viewstatus_overworld() {
        let mut world = World::new();
        let hero = spawn_hero_with_inv(&mut world, ITEM_BIRD_TOTEM, 1);
        let mut res = Resources::new(hero);
        res.region.region_num = 7;

        let result = magic_dispatch_ecs(ITEM_BIRD_TOTEM, &mut world, &mut res);
        assert_eq!(result, MagicResult::Applied);
        assert_eq!(res.view.viewstatus, 1);
    }

    #[test]
    fn totem_suppressed_underground_without_cheat() {
        let mut world = World::new();
        let hero = spawn_hero_with_inv(&mut world, ITEM_BIRD_TOTEM, 1);
        let mut res = Resources::new(hero);
        res.region.region_num = 8;
        res.brother.cheat1 = false;

        let result = magic_dispatch_ecs(ITEM_BIRD_TOTEM, &mut world, &mut res);
        assert_eq!(result, MagicResult::Suppressed);
        assert_eq!(res.view.viewstatus, 0);
    }

    #[test]
    fn totem_allowed_underground_with_cheat() {
        let mut world = World::new();
        let hero = spawn_hero_with_inv(&mut world, ITEM_BIRD_TOTEM, 1);
        let mut res = Resources::new(hero);
        res.region.region_num = 8;
        res.brother.cheat1 = true;

        let result = magic_dispatch_ecs(ITEM_BIRD_TOTEM, &mut world, &mut res);
        assert_eq!(result, MagicResult::Applied);
        assert_eq!(res.view.viewstatus, 1);
    }

    #[test]
    fn stone_ring_teleports_hero() {
        let mut world = World::new();
        let mut stuff = [0u8; 36];
        stuff[ITEM_BLUE_STONE] = 1;
        let hero = world.spawn((
            Inventory { stuff },
            test_hero(30, 200),
            Position::new(((54u16 << 8) | 85) as f32, ((43u16 << 8) | 64) as f32),
            Facing::new(Direction::NW),
            CarrierMount::default(),
        ));
        let mut res = Resources::new(hero);
        let mut map = crate::game::world_data::WorldData::empty();
        map.map_mem[43 * 128 + 54] = STONE_RING_SECTOR as u8;
        res.map.world = Some(map);

        let result = magic_dispatch_ecs(ITEM_BLUE_STONE, &mut world, &mut res);
        assert!(matches!(result, MagicResult::StoneTeleport { capped: false }));
        let pos = world.get::<&Position>(hero).unwrap();
        let expected_x = ((71u16 << 8) | 85) as f32;
        let expected_y = ((77u16 << 8) | 64) as f32;
        assert!((pos.x - expected_x).abs() < 0.1);
        assert!((pos.y - expected_y).abs() < 0.1);
    }

    #[test]
    fn stone_ring_wrong_sector_suppressed() {
        let mut world = World::new();
        let hero = spawn_hero_with_inv(&mut world, ITEM_BLUE_STONE, 1);
        let mut res = Resources::new(hero);
        let map = crate::game::world_data::WorldData::empty();
        // Deliberately leave sector index 0 at the hero position.
        res.map.world = Some(map);

        let result = magic_dispatch_ecs(ITEM_BLUE_STONE, &mut world, &mut res);
        assert_eq!(result, MagicResult::Suppressed);
        assert_eq!(world.get::<&Inventory>(hero).unwrap().stuff[ITEM_BLUE_STONE], 1);
    }

    #[test]
    fn skull_mass_kill_affects_race_lt_7() {
        let mut world = World::new();
        let hero = spawn_hero_with_inv(&mut world, ITEM_JADE_SKULL, 1);
        let mut res = Resources::new(hero);
        res.region.battleflag = true;

        let killable = spawn_enemy(&mut world, 10.0, 10.0, 1, 3, 10, 0, 0, 0, 0, 0);
        let immune = spawn_enemy(&mut world, 20.0, 20.0, 1, 7, 10, 0, 0, 0, 0, 0);
        let dummy = world.spawn((
            crate::game::ecs::components::Enemy,
            ArenaDummy,
            Position::new(30.0, 30.0),
            Facing::new(Direction::N),
            EnemyKind { npc_type: 1, race: 3 },
            Health::new(10),
            Speed { speed: 0 },
            Loot::default(),
            SpriteRef { cfile_idx: 0 },
        ));

        let result = magic_dispatch_ecs(ITEM_JADE_SKULL, &mut world, &mut res);
        assert!(matches!(result, MagicResult::MassKill { in_battle: true, .. }));
        assert!(world.get::<&Health>(killable).unwrap().vitality <= 0);
        assert!(world.get::<&Health>(immune).unwrap().vitality > 0);
        assert!(world.get::<&Health>(dummy).unwrap().vitality > 0);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test magic::ecs_tests 2>&1 | grep "^test result"
```

Expected: all tests pass.

---

## Task 6: Integration testing

- [ ] **Step 1: Build with magic system**

```bash
cargo build 2>&1 | grep "^error"
```

Expected: no errors.

- [ ] **Step 2: Run the full test suite**

```bash
cargo test 2>&1 | grep "^test result"
```

Expected: all tests pass.

- [ ] **Step 3: Manual review / validation**

Do **not** commit. Leave the changes on the feature branch for the user to validate.

---

## Completion check

```bash
cargo build 2>&1 | grep "^error"
cargo test 2>&1 | grep "^test result"
```

Both succeed. The magic system is fully ported to ECS and preserves the original spec behavior.

---

## Spec references

- `docs/spec/magic.md` §19 — complete magic specification
- `docs/reqs/magic.md` R-MAGIC-001 through R-MAGIC-010
- `reference/logic/magic.md` (research branch) — fmain.c logic trace
- `reference/logic/dialog_system.md` (research branch) — hardcoded scroll messages

## Test plan

- Green Jewel adds `light_timer`
- Skull with no inventory returns `NoOwned`
- Vial heals vitality (uncapped and capped cases)
- Ring blocked when `riding > 1` returns `Suppressed`
- Stone ring teleports when centered on the correct map sector
- Totem sets `viewstatus = 1` (map view)
- Totem suppressed underground without `cheat1`
- Orb adds `secret_timer`
- Mass kill affects enemies with `race < 7` and ignores arena dummies
- `CastSpell` wired in `dispatch_menu_action` with canonical messages

## Files touched

| File | Change |
|------|--------|
| `src/game/ecs/resources.rs` | Replace VFX placeholders with `WitchEffect` / `TeleportEffect` |
| `src/game/world_data.rs` | Add `WorldData::sector_at_pos()` helper + test |
| `src/game/magic.rs` | Add `magic_dispatch_ecs()` + ECS helpers + tests |
| `src/game/ecs/scene.rs` | Wire `CastSpell` in `dispatch_menu_action()` |
