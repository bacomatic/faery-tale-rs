---
title: "Plan W — Debug TUI Extras"
plan: W
status: draft
depends_on: [V]
touches: [src/main.rs, src/game/ecs/debug_tui/bridge.rs, src/game/ecs/debug_tui/snapshot.rs]
---

# ECS Migration Plan W: Debug TUI Extras

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Populate the existing `DebugSnapshot` with actor watch data (position/state of hero + nearby entities) and hero extras (quest state, timer values, pending narrative).

**Architecture:** The `DebugSnapshot` struct serves as a bridge between `EcsScene` and `DebugConsole`. Main.rs constructs the snapshot from ECS data each frame, then passes it to the TUI for display. This plan populates the currently-empty fields with real ECS data.

**Prerequisites:** Plans A-D complete; Plan V (QuestState available). DebugSnapshot already wired to TUI.

**Tech Stack:** Rust 2021, hecs ECS queries, crossterm TUI.

---

## File map

| File | Change |
|------|--------|
| `src/main.rs` | Populate DebugSnapshot with ECS data |
| `src/game/ecs/debug_tui/bridge.rs` | Add actor snapshot conversion helpers |
| `src/game/ecs/debug_tui/snapshot.rs` | Verify DebugSnapshot field completeness |

---

## Task 1: Examine current DebugSnapshot structure

**Files:**
- Read: `src/game/ecs/debug_tui/snapshot.rs` (or wherever DebugSnapshot is defined)

- [ ] **Step 1: Read DebugSnapshot definition**

```bash
find . -name "*.rs" -exec grep -l "struct DebugSnapshot" {} \;
```

- [ ] **Step 2: Identify empty fields**

Look for fields that are currently zeroed or have placeholder values:
- `actors: Vec<ActorSnapshot>` (likely empty)
- Hero extras fields (max_vitality, hero_weapon, etc.)
- Quest state fields

- [ ] **Step 3: Note ActorSnapshot structure**

Examine what fields `ActorSnapshot` contains:
- position (x, y)
- state/facing
- health/vitality
- weapon info
- race/goal/tactic for enemies

---

## Task 2: Add actor snapshot conversion helpers

**Files:**
- Modify: `src/game/ecs/debug_tui/bridge.rs` (create if doesn't exist)

- [ ] **Step 1: Create or extend bridge.rs with actor conversion**

```rust
//! Bridge helpers for converting ECS data to DebugSnapshot format
use crate::game::ecs::components::*;
use crate::game::ecs::debug_tui::snapshot::{DebugSnapshot, ActorSnapshot};
use crate::game::debug_tui::{actor_state_u8, actor_state_name, facing_name, weapon_short_name, race_label, goal_name, tactic_name};
use hecs::World;

/// Convert ECS entities to ActorSnapshot vectors for debug display
pub fn build_actor_snapshots(
    world: &World,
    hero_entity: hecs::Entity,
    max_actors: usize,
) -> Vec<ActorSnapshot> {
    let mut actors = Vec::with_capacity(max_actors);
    
    // Always include hero as slot 0
    if let Ok((pos, facing, stats, combat, health, motion)) = 
        world.get::<(Position, Facing, HeroStats, CombatState, Health, ActorMotion)>(hero_entity) {
        
        actors.push(ActorSnapshot {
            slot: 0,
            x: pos.x as i32,
            y: pos.y as i32,
            state_u8: actor_state_u8(combat.state),
            state_name: actor_state_name(combat.state),
            facing: facing_name(*facing),
            health: health.vitality,
            max_health: stats.vitality,
            weapon: combat.weapon,
            weapon_name: weapon_short_name(combat.weapon),
            race: "Hero".to_string(),
            goal: "Player".to_string(),
            tactic: "None".to_string(),
        });
    }
    
    // Add enemies up to max_actors limit
    let mut enemy_count = 0;
    for (entity, (pos, facing, kind, combat, health, motion)) in 
        world.query::<(&Position, &Facing, &EnemyKind, &CombatState, &Health, &ActorMotion)>().iter() {
        
        if actors.len() >= max_actors {
            break;
        }
        
        // Skip hero entity if it somehow has Enemy component
        if entity == hero_entity {
            continue;
        }
        
        actors.push(ActorSnapshot {
            slot: (enemy_count + 1) as u8,
            x: pos.x as i32,
            y: pos.y as i32,
            state_u8: actor_state_u8(combat.state),
            state_name: actor_state_name(combat.state),
            facing: facing_name(*facing),
            health: health.vitality,
            max_health: health.max_vitality,
            weapon: combat.weapon,
            weapon_name: weapon_short_name(combat.weapon),
            race: race_label(kind.race),
            goal: goal_name(kind.goal),
            tactic: tactic_name(kind.tactic),
        });
        
        enemy_count += 1;
    }
    
    actors
}

/// Extract hero-specific extra data for debug display
pub fn build_hero_extras(
    world: &World,
    hero_entity: hecs::Entity,
    res: &crate::game::ecs::resources::Resources,
) -> HeroExtras {
    let mut extras = HeroExtras::default();
    
    if let Ok((stats, combat, facing, motion, carrier)) = 
        world.get::<(&HeroStats, &CombatState, &Facing, &ActorMotion, &CarrierMount)>(hero_entity) {
        
        // Calculate max vitality from stats
        extras.max_vitality = 15 + (stats.brave / 4);
        
        // Weapon info
        extras.hero_weapon = combat.weapon;
        extras.hero_weapon_name = weapon_short_name(combat.weapon);
        
        // State and facing
        extras.hero_state_u8 = actor_state_u8(combat.state);
        extras.hero_state_name = actor_state_name(combat.state);
        extras.hero_facing = facing_name(*facing);
        
        // Environment
        extras.hero_environ = motion.environ;
        
        // Carrier info
        extras.active_carrier = carrier.active_carrier;
        extras.active_carrier_name = carrier_name(carrier.active_carrier);
    }
    
    // Timer values from Resources
    extras.jewel_timer = res.clock.light_timer;
    extras.orb_timer = res.clock.secret_timer;
    extras.freeze_timer = res.clock.freeze_timer;
    
    extras
}

/// Helper to get carrier name from ID
fn carrier_name(carrier_id: u8) -> String {
    match carrier_id {
        0 => "None".to_string(),
        1 => "Raft".to_string(),
        2 => "Turtle".to_string(),
        3 => "Swan".to_string(),
        _ => format!("Unknown({})", carrier_id),
    }
}

/// Hero extra data for debug display
#[derive(Debug, Default, Clone)]
pub struct HeroExtras {
    pub max_vitality: i16,
    pub hero_weapon: u8,
    pub hero_weapon_name: String,
    pub hero_state_u8: u8,
    pub hero_state_name: String,
    pub hero_facing: String,
    pub hero_environ: u8,
    pub active_carrier: u8,
    pub active_carrier_name: String,
    pub jewel_timer: u16,
    pub orb_timer: u16,
    pub freeze_timer: u16,
}
```

- [ ] **Step 2: Verify DebugSnapshot can use these helpers**

Check if `DebugSnapshot` needs to be updated to accept `HeroExtras` or if fields should be populated individually.

---

## Task 3: Update DebugSnapshot if needed

**Files:**
- Modify: `src/game/ecs/debug_tui/snapshot.rs`

- [ ] **Step 1: Check if HeroExtras struct is needed**

If DebugSnapshot has many individual hero fields, either:
1. Keep individual fields and populate them directly, or
2. Group them into a HeroExtras struct

- [ ] **Step 2: Add HeroExtras field if needed**

```rust
#[derive(Debug, Default, Clone)]
pub struct DebugSnapshot {
    // ... existing fields ...
    pub hero_extras: HeroExtras,
    // ... rest of fields ...
}
```

- [ ] **Step 3: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors.

---

## Task 4: Populate DebugSnapshot in main.rs

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Locate DebugSnapshot construction**

Find where `DebugSnapshot` is built for the debug console (look for `dc.update_status()` call).

- [ ] **Step 2: Add actor watch population**

```rust
// In the ECS branch, where DebugSnapshot is constructed:
use crate::game::ecs::debug_tui::bridge::{build_actor_snapshots, build_hero_extras};

// Replace empty actors vector with real data:
snapshot.actors = build_actor_snapshots(
    &ecs_scene.world,
    ecs_scene.res.hero_entity,
    20, // Max actors per spec
);
```

- [ ] **Step 3: Add hero extras population**

```rust
// Populate hero-specific fields:
let hero_extras = build_hero_extras(
    &ecs_scene.world,
    ecs_scene.res.hero_entity,
    &ecs_scene.res,
);

// If using individual fields:
snapshot.max_vitality = hero_extras.max_vitality;
snapshot.hero_weapon = hero_extras.hero_weapon;
snapshot.hero_weapon_name = hero_extras.hero_weapon_name;
snapshot.hero_state_u8 = hero_extras.hero_state_u8;
snapshot.hero_state_name = hero_extras.hero_state_name;
snapshot.hero_facing = hero_extras.hero_facing;
snapshot.hero_environ = hero_extras.hero_environ;
snapshot.active_carrier = hero_extras.active_carrier;
snapshot.active_carrier_name = hero_extras.active_carrier_name;
snapshot.jewel_timer = hero_extras.jewel_timer;
snapshot.orb_timer = hero_extras.orb_timer;
snapshot.freeze_timer = hero_extras.freeze_timer;

// Or if using HeroExtras struct:
snapshot.hero_extras = hero_extras;
```

- [ ] **Step 4: Add quest state population**

```rust
// Populate quest state from Plan V:
snapshot.quest_princess_rescues = ecs_scene.res.quest.princess_rescues;
snapshot.quest_statues_collected = ecs_scene.res.quest.statues_collected;
snapshot.quest_writ_obtained = ecs_scene.res.quest.writ_obtained;
snapshot.quest_rose_obtained = ecs_scene.res.quest.rose_obtained;
snapshot.quest_crystal_shard_obtained = ecs_scene.res.quest.crystal_shard_obtained;
snapshot.quest_sun_stone_obtained = ecs_scene.res.quest.sun_stone_obtained;
snapshot.quest_golden_lasso_obtained = ecs_scene.res.quest.golden_lasso_obtained;
snapshot.quest_talisman_obtained = ecs_scene.res.quest.talisman_obtained;
snapshot.quest_king_bone_obtained = ecs_scene.res.quest.king_bone_obtained;
snapshot.quest_can_enter_azal = ecs_scene.res.quest.can_enter_azal();
snapshot.quest_has_won = ecs_scene.res.quest.has_won();
snapshot.quest_progress_percent = ecs_scene.res.quest.progress_percent();
```

- [ ] **Step 5: Add pending narrative population**

```rust
// Populate narrative queue info:
snapshot.narrative_pending_count = ecs_scene.res.narr_queue.pending.len() as u32;
snapshot.narrative_active = ecs_scene.res.narr_queue.active.is_some();
snapshot.narrative_timer = ecs_scene.res.narr_queue.timer;

// Add first few pending narrative items for preview
snapshot.narrative_preview = ecs_scene.res.narr_queue.pending
    .iter()
    .take(3)
    .map(|narr| match narr {
        crate::game::ecs::resources::NarrEvent::Placard(text) => format!("PLACARD: {}", text),
        crate::game::ecs::resources::NarrEvent::Speech(text) => format!("SPEECH: {}", text),
    })
    .collect();
```

- [ ] **Step 6: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors (may need to add imports or adjust field names).

---

## Task 5: Add unit tests for bridge functions

**Files:**
- Modify: `src/game/ecs/debug_tui/bridge.rs` (add tests module)

- [ ] **Step 1: Add test module**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::ecs::components::*;
    use crate::game::ecs::resources::Resources;
    use hecs::World;
    
    #[test]
    fn test_build_actor_snapshots_includes_hero() {
        let mut world = World::new();
        let hero = world.spawn((
            Position::new(100.0, 200.0),
            Facing::South,
            HeroStats { vitality: 50, brave: 10, luck: 8, kind: 12, wealth: 5, hunger: 0, fatigue: 0, gold: 0 },
            CombatState { state: ActorState::Standing, weapon: 1 },
            Health { vitality: 45, max_vitality: 50 },
            ActorMotion { environ: 0, ..Default::default() },
            CarrierMount { active_carrier: 0 },
        ));
        
        let actors = build_actor_snapshots(&world, hero, 10);
        
        assert_eq!(actors.len(), 1);
        assert_eq!(actors[0].slot, 0);
        assert_eq!(actors[0].x, 100);
        assert_eq!(actors[0].y, 200);
        assert_eq!(actors[0].health, 45);
        assert_eq!(actors[0].max_health, 50);
        assert_eq!(actors[0].weapon, 1);
        assert_eq!(actors[0].race, "Hero");
    }
    
    #[test]
    fn test_build_actor_snapshots_includes_enemies() {
        let mut world = World::new();
        let hero = world.spawn((
            Position::new(0.0, 0.0),
            Facing::North,
            HeroStats::default(),
            CombatState::default(),
            Health::default(),
            ActorMotion::default(),
            CarrierMount::default(),
        ));
        
        // Add some enemies
        for i in 0..3 {
            world.spawn((
                Position::new((i * 50) as f32, 100.0),
                Facing::East,
                EnemyKind { race: 1, goal: 2, tactic: 0 },
                CombatState { state: ActorState::Walking(5), weapon: 0 },
                Health { vitality: 30, max_vitality: 30 },
                ActorMotion::default(),
            ));
        }
        
        let actors = build_actor_snapshots(&world, hero, 10);
        
        assert_eq!(actors.len(), 4); // 1 hero + 3 enemies
        assert_eq!(actors[0].slot, 0); // Hero
        assert_eq!(actors[1].slot, 1); // First enemy
        assert_eq!(actors[2].slot, 2); // Second enemy
        assert_eq!(actors[3].slot, 3); // Third enemy
        
        // Check enemy data
        assert_eq!(actors[1].x, 0);
        assert_eq!(actors[1].y, 100);
        assert_eq!(actors[1].health, 30);
        assert_eq!(actors[2].x, 50);
        assert_eq!(actors[3].x, 100);
    }
    
    #[test]
    fn test_build_actor_snapshots_respects_max_limit() {
        let mut world = World::new();
        let hero = world.spawn((
            Position::new(0.0, 0.0),
            Facing::North,
            HeroStats::default(),
            CombatState::default(),
            Health::default(),
            ActorMotion::default(),
            CarrierMount::default(),
        ));
        
        // Add more enemies than the limit
        for i in 0..15 {
            world.spawn((
                Position::new((i * 10) as f32, 100.0),
                Facing::East,
                EnemyKind { race: 1, goal: 2, tactic: 0 },
                CombatState::default(),
                Health::default(),
                ActorMotion::default(),
            ));
        }
        
        let actors = build_actor_snapshots(&world, hero, 5);
        
        assert_eq!(actors.len(), 5); // 1 hero + 4 enemies (limited)
    }
    
    #[test]
    fn test_build_hero_extras() {
        let mut world = World::new();
        let hero = world.spawn((
            HeroStats { vitality: 80, brave: 16, luck: 8, kind: 12, wealth: 5, hunger: 0, fatigue: 0, gold: 0 },
            CombatState { state: ActorState::Fighting(10), weapon: 2 },
            Facing::West,
            ActorMotion { environ: 3, ..Default::default() },
            CarrierMount { active_carrier: 1 },
        ));
        
        let mut res = Resources::new(hero);
        res.clock.light_timer = 100;
        res.clock.secret_timer = 200;
        res.clock.freeze_timer = 300;
        
        let extras = build_hero_extras(&world, hero, &res);
        
        assert_eq!(extras.max_vitality, 15 + (16 / 4)); // 15 + 4 = 19
        assert_eq!(extras.hero_weapon, 2);
        assert_eq!(extras.hero_state_u8, actor_state_u8(ActorState::Fighting(10)));
        assert_eq!(extras.hero_facing, "West");
        assert_eq!(extras.hero_environ, 3);
        assert_eq!(extras.active_carrier, 1);
        assert_eq!(extras.active_carrier_name, "Raft");
        assert_eq!(extras.jewel_timer, 100);
        assert_eq!(extras.orb_timer, 200);
        assert_eq!(extras.freeze_timer, 300);
    }
    
    #[test]
    fn test_carrier_name() {
        assert_eq!(carrier_name(0), "None");
        assert_eq!(carrier_name(1), "Raft");
        assert_eq!(carrier_name(2), "Turtle");
        assert_eq!(carrier_name(3), "Swan");
        assert_eq!(carrier_name(99), "Unknown(99)");
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test ecs::debug_tui::bridge::tests 2>&1 | grep "^test result"
```

Expected: all tests pass.

---

## Task 6: Integration testing

- [ ] **Step 1: Test actor watch in debug console**

```bash
cargo run --features debug-tui -- --ecs
```

1. Start game with debug TUI enabled
2. Press backtick (`) to open debug console
3. Type `/actors` command
4. Verify hero appears in slot 0 with correct position/stats
5. Spawn some enemies (if debug commands allow)
6. Verify enemies appear in subsequent slots

- [ ] **Step 2: Test hero extras display**

1. In debug console, check hero stats display
2. Verify max_vitality is calculated correctly from brave stat
3. Verify weapon name matches equipped weapon
4. Verify facing direction updates when hero moves
5. Verify carrier info updates when mounting/dismounting

- [ ] **Step 3: Test quest state display**

1. Pick up some quest items
2. Check debug console for quest progress
3. Verify statues collected count updates
4. Verify princess rescue count updates
5. Verify can_enter_azal status after 5 statues

- [ ] **Step 4: Test narrative queue display**

1. Trigger some narrative events (talk to NPCs, enter zones)
2. Check debug console for narrative queue info
3. Verify pending count and active status
4. Verify preview text shows upcoming messages

---

## Task 7: Performance verification

- [ ] **Step 1: Check snapshot construction performance**

The debug snapshot should be cheap to construct (only copying data, not complex queries).

```bash
# Add some timing debug prints if needed to verify < 1ms per frame
```

- [ ] **Step 2: Verify memory usage**

Ensure the snapshot doesn't grow unbounded and is properly cleaned up each frame.

---

## Task 8: Final verification

- [ ] **Step 1: Full build check**

```bash
cargo build --features debug-tui 2>&1 | grep "^error"
```

Expected: no errors.

- [ ] **Step 2: Run test suite**

```bash
cargo test --features debug-tui 2>&1 | grep "^test result"
```

Expected: all tests pass.

- [ ] **Step 3: Manual smoke test**

```bash
cargo run --features debug-tui -- --ecs 2>&1 | head -5
```

Expected: game starts, debug console accessible.

- [ ] **Step 4: Commit changes**

```bash
git add src/main.rs src/game/ecs/debug_tui/bridge.rs src/game/ecs/debug_tui/snapshot.rs
git commit -m "feat(ecs): populate DebugSnapshot with real ECS data for debug TUI

- Add bridge helpers to convert ECS entities to ActorSnapshot format
- Populate hero extras (stats, weapon, timers, carrier info)
- Add quest state display from QuestState resource
- Add narrative queue preview for pending messages
- Add comprehensive unit tests for data conversion
- Verify debug console shows live actor positions and states"
```

---

## Completion check

```bash
cargo build --features debug-tui 2>&1 | grep "^error"
cargo test --features debug-tui 2>&1 | grep "^test result"
cargo run --features debug-tui -- --ecs 2>&1 | head -5
```

All three succeed. Debug TUI shows live actor data, hero extras, quest progress, and narrative queue.

---

## Context

### Why does the debug TUI still use a snapshot?

One of the goals of moving to ECS was to eliminate the `DebugSnapshot` push model — instead of serializing game state into a bag-of-scalars every frame, the TUI could read ECS data directly. The snapshot exists today because `DebugConsole` (crossterm TUI) and `EcsScene` are both owned by `main.rs` and both need to be mutably borrowed in the same frame: `dc.drain_commands()` takes `&mut DebugConsole` while the ECS tick takes `&mut EcsScene`. Rust cannot hold both borrows simultaneously, so `main.rs` copies data out of the ECS into a `DebugSnapshot` and hands that to the console.

### Is there a better architecture?

Yes — pass a read-only view of the ECS *into* `DebugConsole::render()` and `DebugConsole::update_status()` at call sites, instead of a pre-built snapshot. This requires refactoring `DebugConsole` to hold `&EcsScene` or equivalent for the duration of `render()`, which is possible because `render()` only reads. However:

- `DebugConsole::render()` is called after the ECS tick (no overlap).
- The existing `DebugSnapshot` struct is only ~130 lines; the copy cost is negligible.
- The real win from ECS was eliminating `GameState` (a 400-field God Object), not eliminating the TUI snapshot.

### Decision for this plan

Keep `DebugSnapshot` as the bridge type — it is small, cheap to copy, and avoids complex lifetime threading through the TUI crate. The snapshot fields that are currently missing (actors, quest state, hero extras) will be populated from the ECS world before `update_status()` is called. If a future plan wants to refactor to a direct reference, that is a separate architectural decision.

---

## Dependencies
Plans A-D complete; Plan V (QuestState available). DebugSnapshot already wired to TUI.

---

## Spec references
- `docs/DEBUG_SPECIFICATION.md` §DebugSnapshot Data Model
- Debug TUI commands reference (`/actors`, `/quest`, `/hero`)