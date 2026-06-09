# ECS Migration Plan K: Magic System (ECS)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the complete magic item system from `src/game/magic.rs` (legacy GameState-based) to the ECS architecture, enabling the MAGIC menu to dispatch spell casts through `magic_dispatch_ecs()`, which reads/writes hero inventory and spell timers from ECS components and resources, emits appropriate events, and triggers visual effects.

**Architecture:** The legacy `magic.rs` contains a complete, correct implementation that operates on `GameState` fields. Plan K creates `magic_dispatch_ecs()` that performs the same logic using ECS components (Inventory, HeroStats, Position, CarrierMount) and Resources (clock timers, region data, event queues). The MAGIC menu (Plan I) calls this function via `MenuAction::CastSpell`.

**Prerequisites:** Plans I (menu dispatch) and J (inventory). Plans A-D complete.

**Tech Stack:** Rust 2021, `hecs = "0.11"`, SDL3.

---

## File map

| File | Change |
|---|---|
| `src/game/ecs/resources.rs` | Replace VFX placeholders with real types |
| `src/game/magic.rs` | Add `magic_dispatch_ecs()` + unit tests |
| `src/game/ecs/scene.rs` | Wire CastSpell in dispatch_menu_action |
| `src/game/world_data.rs` | Add `compute_sector()` helper |

---

## Task 1: Replace VFX placeholders in resources.rs

**Files:**
- Modify: `src/game/ecs/resources.rs`

- [ ] **Step 1: Remove placeholder types (lines 192–198)**

Remove these lines:
```rust
// Placeholder types until gfx_effects are ported
pub struct WitchEffectPlaceholder;
pub struct TeleportEffectPlaceholder;
```

- [ ] **Step 2: Import real VFX types**

Add at top with other imports:
```rust
use crate::game::gfx_effects::{WitchEffect, TeleportEffect};
```

- [ ] **Step 3: Update VfxState struct**

Replace the placeholder fields with real types:
```rust
pub struct VfxState {
    pub witch_effect: WitchEffect,
    pub teleport_effect: TeleportEffect,
}
```

- [ ] **Step 4: Update VfxState::default()**

Replace placeholder construction with real type construction (may need Default implementations or new() methods).

- [ ] **Step 5: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors related to VfxState.

---

## Task 2: Add compute_sector() helper

**Files:**
- Modify: `src/game/world_data.rs` (or create `src/game/ecs/geometry.rs`)

- [ ] **Step 1: Add compute_sector function**

```rust
/// Compute sector ID from world coordinates.
/// Matches fmain.c sector calculation: (y << 8) | x, where x/y are 8-bit sector coordinates.
pub fn compute_sector(x: f32, y: f32) -> u16 {
    let sx = (x as u16) >> 8;
    let sy = (y as u16) >> 8;
    (sy << 8) | sx
}
```

- [ ] **Step 2: Add unit test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compute_sector_matches_legacy() {
        // Test origin
        assert_eq!(compute_sector(0.0, 0.0), 0);
        // Test sector (1, 2) -> 0x0201
        assert_eq!(compute_sector(300.0, 600.0), 0x0201);
        // Test sector (255, 255) -> 0xFFFF
        assert_eq!(compute_sector(65535.0, 65535.0), 0xFFFF);
    }
}
```

- [ ] **Step 3: Add to module exports**

If created in new file, add to `src/game/ecs/mod.rs`:
```rust
pub mod geometry;
pub use geometry::compute_sector;
```

---

## Task 3: Implement magic_dispatch_ecs()

**Files:**
- Modify: `src/game/magic.rs`

- [ ] **Step 1: Add ECS imports**

```rust
use hecs::World;
use crate::game::ecs::resources::Resources;
use crate::game::ecs::components::{Inventory, HeroStats, Position, Facing, CarrierMount};
use crate::game::world_data::compute_sector;
```

- [ ] **Step 2: Implement magic_dispatch_ecs function**

```rust
/// Dispatch magic item effect using ECS architecture.
/// Returns MagicResult indicating outcome.
pub fn magic_dispatch_ecs(item_slot: usize, world: &mut hecs::World, res: &mut Resources) -> MagicResult {
    // Validate slot range
    if item_slot < 9 || item_slot > 15 {
        return MagicResult::NoOwned;
    }

    // Get hero components
    let hero_entity = res.hero_entity;
    let (inventory, stats, position, _facing, carrier_mount) = match world.query_one::<(&Inventory, &HeroStats, &Position, &Facing, &CarrierMount)>(hero_entity) {
        Ok(q) => match q.get() {
            Ok(components) => components,
            Err(_) => return MagicResult::NoOwned,
        },
        Err(_) => return MagicResult::NoOwned,
    };

    // Check ownership
    if inventory.stuff[item_slot] == 0 {
        return MagicResult::NoOwned;
    }

    // Map slot to effect
    let result = match item_slot {
        9 => lantern_effect(world, res, hero_entity),
        10 => vial_effect(world, res, hero_entity),
        11 => orb_effect(world, res, hero_entity),
        12 => totem_effect(world, res, hero_entity),
        13 => ring_effect(world, res, hero_entity, carrier_mount.riding),
        14 => skull_effect(world, res, hero_entity),
        15 => stone_ring_effect(world, res, hero_entity, position.x, position.y),
        _ => MagicResult::NoOwned,
    };

    // Decrement inventory on successful use
    if matches!(result, MagicResult::Applied | MagicResult::Healed { .. } | MagicResult::StoneTeleport { .. } | MagicResult::MassKill { .. }) {
        if let Ok(mut inv) = world.get_mut::<Inventory>(hero_entity) {
            inv.stuff[item_slot] -= 1;
        }
    }

    result
}
```

- [ ] **Step 3: Implement effect helpers**

```rust
fn lantern_effect(world: &mut World, res: &mut Resources, hero_entity: hecs::Entity) -> MagicResult {
    res.clock.light_timer += 760;
    MagicResult::Applied
}

fn vial_effect(world: &mut World, res: &mut Resources, hero_entity: hecs::Entity) -> MagicResult {
    if let Ok(mut stats) = world.get_mut::<HeroStats>(hero_entity) {
        let old_vitality = stats.vitality;
        stats.vitality = (stats.vitality + 50).min(100);
        let healed = stats.vitality - old_vitality;
        if healed > 0 {
            res.events.message.push(MessageEvent { text: format!("Healed {} vitality", healed) });
            return MagicResult::Healed { capped: healed < 50 };
        }
    }
    MagicResult::Suppressed
}

fn orb_effect(world: &mut World, res: &mut Resources, hero_entity: hecs::Entity) -> MagicResult {
    res.clock.secret_timer += 360;
    MagicResult::Applied
}

fn totem_effect(world: &mut World, res: &mut Resources, hero_entity: hecs::Entity) -> MagicResult {
    if res.region.region_num > 7 && res.brother.cheat1 == 0 {
        return MagicResult::Suppressed;
    }
    res.view.viewstatus = 1; // Map view
    MagicResult::Applied
}

fn ring_effect(world: &mut World, res: &mut Resources, hero_entity: hecs::Entity, riding: u8) -> MagicResult {
    if riding > 1 {
        return MagicResult::Suppressed;
    }
    res.clock.freeze_timer += 100;
    MagicResult::Applied
}

fn skull_effect(world: &mut World, res: &mut Resources, hero_entity: hecs::Entity) -> MagicResult {
    let mut slain = 0;
    let in_battle = res.encounter.battleflag;
    
    // Query all enemy entities
    for (entity, (health, enemy_kind)) in world.query::<(&mut Health, &EnemyKind)>().iter() {
        if enemy_kind.race < 7 {
            health.current = 0; // Kill enemy
            slain += 1;
            if let Ok(mut stats) = world.get_mut::<HeroStats>(hero_entity) {
                stats.brave -= 1;
            }
        }
    }
    
    if slain > 0 {
        res.events.message.push(MessageEvent { text: format!("Mass kill: {} enemies slain", slain) });
        MagicResult::MassKill { slain, in_battle }
    } else {
        MagicResult::Suppressed
    }
}

fn stone_ring_effect(world: &mut World, res: &mut Resources, hero_entity: hecs::Entity, x: f32, y: f32) -> MagicResult {
    let sector = compute_sector(x, y);
    if sector != 144 {
        return MagicResult::Suppressed;
    }
    
    // Check if centered on stone (approximate)
    let stone_x = ((x as u16) & 0xFF00) + 128; // Center of sector
    let stone_y = ((y as u16) & 0xFF00) + 128;
    let dx = (x - stone_x as f32).abs();
    let dy = (y - stone_y as f32).abs();
    
    if dx > 20.0 || dy > 20.0 {
        return MagicResult::Suppressed;
    }
    
    // Teleport to next stone and heal
    if let Ok(mut pos) = world.get_mut::<Position>(hero_entity) {
        pos.set(stone_x as f32 + 256.0, stone_y as f32); // Next stone
    }
    
    if let Ok(mut stats) = world.get_mut::<HeroStats>(hero_entity) {
        let old_vitality = stats.vitality;
        stats.vitality = (stats.vitality + 25).min(100);
        let healed = stats.vitality - old_vitality;
        res.events.message.push(MessageEvent { text: format!("Teleported! Healed {} vitality", healed) });
        MagicResult::StoneTeleport { capped: healed < 25 }
    } else {
        MagicResult::Applied
    }
}
```

- [ ] **Step 4: Add missing component imports**

Add to `src/game/ecs/components.rs` if missing:
```rust
pub struct EnemyKind {
    pub race: u8,
}

pub struct Health {
    pub current: i16,
    pub max: i16,
}
```

---

## Task 4: Wire into dispatch_menu_action()

**Files:**
- Modify: `src/game/ecs/scene.rs`

- [ ] **Step 1: Add magic import**

```rust
use crate::game::magic::magic_dispatch_ecs;
```

- [ ] **Step 2: Add CastSpell branch**

In `dispatch_menu_action()` method, add:
```rust
MenuAction::CastSpell(hit) => {
    let item_slot = 9 + hit as usize;
    let result = magic_dispatch_ecs(item_slot, &mut self.world, &mut self.res);
    match result {
        MagicResult::NoOwned => {
            self.messages.push("if only I had some Magic!".into());
        }
        MagicResult::Applied | MagicResult::Suppressed => {
            // No message needed
        }
        MagicResult::Healed { .. } | MagicResult::StoneTeleport { .. } |
        MagicResult::MassKill { .. } => {
            // Events already emitted by magic_dispatch_ecs
        }
    }
}
```

- [ ] **Step 3: Add MagicResult import**

```rust
use crate::game::magic::MagicResult;
```

---

## Task 5: Add unit tests in magic.rs

**Files:**
- Modify: `src/game/magic.rs`

- [ ] **Step 1: Add test module**

```rust
#[cfg(test)]
mod ecs_tests {
    use super::*;
    use hecs::World;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::components::{Inventory, HeroStats, Position, Facing, CarrierMount};

    #[test]
    fn lantern_adds_light_timer() {
        let mut world = World::new();
        let hero = world.spawn((
            Inventory { stuff: [0; 32], /* set stuff[9] = 1 */ },
            HeroStats::default(),
            Position::new(0.0, 0.0),
            Facing::default(),
            CarrierMount { riding: 0 },
        ));
        
        let mut res = Resources::new(hero);
        res.clock.light_timer = 0;
        
        // Set inventory to have lantern
        if let Ok(mut inv) = world.get_mut::<Inventory>(hero) {
            inv.stuff[9] = 1;
        }
        
        let result = magic_dispatch_ecs(9, &mut world, &mut res);
        assert!(matches!(result, MagicResult::Applied));
        assert_eq!(res.clock.light_timer, 760);
    }

    #[test]
    fn skull_no_inventory_returns_no_owned() {
        let mut world = World::new();
        let hero = world.spawn((
            Inventory::empty(),
            HeroStats::default(),
            Position::new(0.0, 0.0),
            Facing::default(),
            CarrierMount { riding: 0 },
        ));
        
        let mut res = Resources::new(hero);
        let result = magic_dispatch_ecs(14, &mut world, &mut res);
        assert!(matches!(result, MagicResult::NoOwned));
    }

    #[test]
    fn vial_heals_vitality_uncapped() {
        let mut world = World::new();
        let hero = world.spawn((
            Inventory { stuff: [0; 32] },
            HeroStats { vitality: 30, ..Default::default() },
            Position::new(0.0, 0.0),
            Facing::default(),
            CarrierMount { riding: 0 },
        ));
        
        let mut res = Resources::new(hero);
        
        // Set inventory to have vial
        if let Ok(mut inv) = world.get_mut::<Inventory>(hero) {
            inv.stuff[10] = 1;
        }
        
        let result = magic_dispatch_ecs(10, &mut world, &mut res);
        assert!(matches!(result, MagicResult::Healed { capped: false }));
        
        if let Ok(stats) = world.get::<HeroStats>(hero) {
            assert_eq!(stats.vitality, 80); // 30 + 50, uncapped
        }
    }

    #[test]
    fn ring_blocked_when_riding_returns_suppressed() {
        let mut world = World::new();
        let hero = world.spawn((
            Inventory { stuff: [0; 32] },
            HeroStats::default(),
            Position::new(0.0, 0.0),
            Facing::default(),
            CarrierMount { riding: 2 }, // Riding bird
        ));
        
        let mut res = Resources::new(hero);
        
        // Set inventory to have ring
        if let Ok(mut inv) = world.get_mut::<Inventory>(hero) {
            inv.stuff[13] = 1;
        }
        
        let result = magic_dispatch_ecs(13, &mut world, &mut res);
        assert!(matches!(result, MagicResult::Suppressed));
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test magic::ecs_tests 2>&1 | grep "^test result"
```

Expected: all 4 tests pass.

---

## Task 6: Integration testing

- [ ] **Step 1: Build with magic system**

```bash
cargo build 2>&1 | grep "^error"
```

Expected: no errors.

- [ ] **Step 2: Test magic menu in game**

Manual test with running game:
1. Start game with hero that has magic items
2. Open MAGIC menu
3. Cast each spell type
4. Verify effects match legacy behavior

- [ ] **Step 3: Commit**

```bash
git add src/game/magic.rs src/game/ecs/resources.rs src/game/ecs/scene.rs src/game/world_data.rs
git commit -m "feat(ecs): port magic system to ECS architecture"
```

---

## Completion check

```bash
cargo build 2>&1 | grep "^error"
cargo test magic::ecs_tests 2>&1 | grep "^test result"
```

Both succeed. Magic system is fully ported to ECS.

---

## Spec references

- `docs/spec/magic.md` §19 — complete magic specification
- `docs/reqs/magic.md` R-MAGIC-001 through R-MAGIC-010
- `reference/logic/magic.md` (research branch) — fmain.c logic trace

## Test plan

- Lantern adds light_timer
- Skull with no inventory returns NoOwned
- Vial heals vitality (uncapped case)
- Ring blocked when riding > 1 returns Suppressed
- Stone ring teleports when centered on sector 144
- Totem sets viewstatus = 1 (map view)
- Orb adds secret_timer
- Mass kill affects enemies with race < 7

## Files touched

| File | Change |
|------|--------|
| `src/game/ecs/resources.rs` | Replace VFX placeholders |
| `src/game/magic.rs` | Add `magic_dispatch_ecs()` + tests |
| `src/game/ecs/scene.rs` | Wire CastSpell in dispatch_menu_action |
| `src/game/world_data.rs` | Add `compute_sector()` |