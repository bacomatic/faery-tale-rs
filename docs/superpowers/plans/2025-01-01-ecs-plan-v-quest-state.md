---
title: "Plan V — Quest State + Tracking"
plan: V
status: draft
depends_on: [G, L]
touches: [src/game/ecs/resources.rs, src/game/ecs/systems/item.rs, src/game/persist.rs, proto/faery_save.proto]
---

# ECS Migration Plan V: Quest State + Tracking

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement a `QuestState` struct to track quest progress (princess rescues, statues collected, key items obtained) and integrate it into `Resources` for save/load and debug snapshot.

**Architecture:** `QuestState` lives in `Resources` as a centralized quest progress tracker. Item pickups, zone entries, and narrative events update quest state. The persist system serializes/deserializes quest state to/from protobuf.

**Prerequisites:** Plan G (region entities), Plan L (narrative for rescue sequences). Plans A-D.

**Tech Stack:** Rust 2021, protobuf serialization, hecs ECS.

---

## File map

| File | Change |
|------|--------|
| `src/game/ecs/resources.rs` | Add QuestState struct + quest field |
| `src/game/ecs/systems/item.rs` | Hook item pickup → update QuestState |
| `src/game/persist.rs` | Serialize/deserialize QuestState |
| `proto/faery_save.proto` | Add 8 new fields |

---

## Task 1: Define QuestState struct in resources.rs

**Files:**
- Modify: `src/game/ecs/resources.rs`

- [ ] **Step 1: Add QuestState struct before Resources struct**

```rust
/// Tracks overall quest progress across the game world
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QuestState {
    /// Number of princesses rescued (0-3)
    pub princess_rescues: u8,
    /// Number of statues collected for Azal gate (0-5)
    pub statues_collected: u8,
    /// Whether the Writ from the King has been obtained
    pub writ_obtained: bool,
    /// Whether the Rose has been obtained
    pub rose_obtained: bool,
    /// Whether the Crystal Shard has been obtained
    pub crystal_shard_obtained: bool,
    /// Whether the Sun Stone (Witch vulnerability) has been obtained
    pub sun_stone_obtained: bool,
    /// Whether the Golden Lasso (swan flight) has been obtained
    pub golden_lasso_obtained: bool,
    /// Whether the Talisman (win condition) has been obtained
    pub talisman_obtained: bool,
    /// Whether the King's Bone has been obtained
    pub king_bone_obtained: bool,
}

impl QuestState {
    /// Check if player can enter Azal (requires 5 statues)
    pub fn can_enter_azal(&self) -> bool {
        self.statues_collected >= 5
    }
    
    /// Check if player has won the game (all princesses rescued + talisman)
    pub fn has_won(&self) -> bool {
        self.princess_rescues >= 3 && self.talisman_obtained
    }
    
    /// Get progress percentage for debug display
    pub fn progress_percent(&self) -> u8 {
        let major_items = self.princess_rescues as u8 + 
                         self.statues_collected + 
                         (self.talisman_obtained as u8);
        // Major items: 3 princesses + 5 statues + 1 talisman = 9 total
        (major_items * 100) / 9
    }
}
```

- [ ] **Step 2: Add quest field to Resources struct**

```rust
pub struct Resources {
    // ... existing fields ...
    /// Current quest progress tracking
    pub quest: QuestState,
    // ... rest of fields ...
}
```

- [ ] **Step 3: Initialize quest in Resources::new()**

```rust
impl Resources {
    pub fn new(hero_entity: hecs::Entity) -> Self {
        Self {
            // ... existing field initializations ...
            quest: QuestState::default(),
            // ... rest of fields ...
        }
    }
}
```

- [ ] **Step 4: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors.

---

## Task 2: Update proto file with 8 new fields

**Files:**
- Modify: `proto/faery_save.proto`

- [ ] **Step 1: Add quest state fields to FaerySave message**

Find the existing `princess` field and add the new fields after it:

```protobuf
message FaerySave {
    // ... existing fields up to princess ...
    uint32 princess = 160;  // Existing field
    
    // Quest state fields (Plan V)
    uint32 statues_collected = 161;
    bool writ_obtained = 162;
    bool rose_obtained = 163;
    bool crystal_shard_obtained = 164;
    bool sun_stone_obtained = 165;
    bool golden_lasso_obtained = 166;
    bool talisman_obtained = 167;
    bool king_bone_obtained = 168;
    
    // ... rest of existing fields ...
}
```

- [ ] **Step 2: Regenerate protobuf code**

```bash
cd proto && make generate
```

Or if using build.rs:
```bash
cargo build
```

- [ ] **Step 3: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors from protobuf generation.

---

## Task 3: Serialize QuestState in ecs_to_proto()

**Files:**
- Modify: `src/game/persist.rs`

- [ ] **Step 1: Locate ecs_to_proto() function**

Find the function that converts ECS state to protobuf (around line 542-581 based on context).

- [ ] **Step 2: Add quest state serialization**

```rust
pub fn ecs_to_proto(scene: &EcsScene) -> FaerySave {
    let mut proto = FaerySave::new();
    
    // ... existing field assignments ...
    
    // Quest state (Plan V)
    proto.set_statues_collected(scene.res.quest.statues_collected as u32);
    proto.set_writ_obtained(scene.res.quest.writ_obtained);
    proto.set_rose_obtained(scene.res.quest.rose_obtained);
    proto.set_crystal_shard_obtained(scene.res.quest.crystal_shard_obtained);
    proto.set_sun_stone_obtained(scene.res.quest.sun_stone_obtained);
    proto.set_golden_lasso_obtained(scene.res.quest.golden_lasso_obtained);
    proto.set_talisman_obtained(scene.res.quest.talisman_obtained);
    proto.set_king_bone_obtained(scene.res.quest.king_bone_obtained);
    
    // ... rest of existing assignments ...
    
    proto
}
```

- [ ] **Step 3: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors.

---

## Task 4: Deserialize QuestState in proto_to_ecs()

**Files:**
- Modify: `src/game/persist.rs`

- [ ] **Step 1: Locate proto_to_ecs() function**

Find the function that converts protobuf back to ECS state.

- [ ] **Step 2: Add quest state deserialization**

```rust
pub fn proto_to_ecs(proto: &FaerySave, scene: &mut EcsScene) -> Result<(), String> {
    // ... existing field restorations ...
    
    // Quest state (Plan V)
    scene.res.quest.statues_collected = proto.get_statues_collected() as u8;
    scene.res.quest.writ_obtained = proto.get_writ_obtained();
    scene.res.quest.rose_obtained = proto.get_rose_obtained();
    scene.res.quest.crystal_shard_obtained = proto.get_crystal_shard_obtained();
    scene.res.quest.sun_stone_obtained = proto.get_sun_stone_obtained();
    scene.res.quest.golden_lasso_obtained = proto.get_golden_lasso_obtained();
    scene.res.quest.talisman_obtained = proto.get_talisman_obtained();
    scene.res.quest.king_bone_obtained = proto.get_king_bone_obtained();
    
    // ... rest of existing restorations ...
    
    Ok(())
}
```

- [ ] **Step 3: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors.

---

## Task 5: Hook item pickup in systems/item.rs

**Files:**
- Modify: `src/game/ecs/systems/item.rs`

- [ ] **Step 1: Locate handle_take() function**

Find where items are processed when the hero picks them up.

- [ ] **Step 2: Add quest state updates after item pickup**

```rust
use crate::game::ecs::resources::QuestState;

// In handle_take() after marking item invisible:
match obj.ob_id {
    22 => {
        // Talisman (win condition)
        res.quest.talisman_obtained = true;
        res.messages.push("You found the Talisman of Fate!".to_string());
    }
    7 => {
        // Sun Stone (Witch vulnerability)
        res.quest.sun_stone_obtained = true;
        res.messages.push("You found the Sun Stone!".to_string());
    }
    5 => {
        // Golden Lasso (swan flight)
        res.quest.golden_lasso_obtained = true;
        res.messages.push("You found the Golden Lasso!".to_string());
    }
    6 => {
        // Shell (summon turtle) - if this is a quest item
        // Add shell handling if needed for quest
    }
    // Add other quest item ob_ids as they are identified
    // These would need to be cross-referenced with the actual object data
    statue_ob_ids if statue_ob_ids >= 100 && statue_ob_ids <= 104 => {
        // Statue collection (5 statues for Azal gate)
        res.quest.statues_collected = (res.quest.statues_collected + 1).min(5);
        res.messages.push(format!("Statue collected: {}/5", res.quest.statues_collected));
        
        if res.quest.can_enter_azal() {
            res.messages.push("All statues collected! The path to Azal is open.".to_string());
        }
    }
    writ_ob_id if writ_ob_id == 200 => {
        // King's Writ
        res.quest.writ_obtained = true;
        res.messages.push("You received the King's Writ!".to_string());
    }
    rose_ob_id if rose_ob_id == 201 => {
        // Rose
        res.quest.rose_obtained = true;
        res.messages.push("You found the mystical Rose!".to_string());
    }
    crystal_ob_id if crystal_ob_id == 202 => {
        // Crystal Shard
        res.quest.crystal_shard_obtained = true;
        res.messages.push("You found a Crystal Shard!".to_string());
    }
    bone_ob_id if bone_ob_id == 203 => {
        // King's Bone
        res.quest.king_bone_obtained = true;
        res.messages.push("You found the King's Bone!".to_string());
    }
    _ => {
        // Non-quest item, no quest state update
    }
}
```

- [ ] **Step 3: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors (though ob_id values may need adjustment based on actual data).

---

## Task 6: Add princess rescue tracking

**Files:**
- Modify: `src/game/ecs/systems/zone.rs` or appropriate system

- [ ] **Step 1: Locate princess rescue logic**

Find where princess rescue is detected (likely in zone or narrative system).

- [ ] **Step 2: Update princess rescue count**

```rust
// When a princess is rescued:
res.quest.princess_rescues = (res.quest.princess_rescues + 1).min(3);
res.messages.push(format!("Princess rescued! {}/3", res.quest.princess_rescues));

if res.quest.princess_rescues >= 3 {
    res.messages.push("All princesses have been rescued!".to_string());
}
```

- [ ] **Step 3: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors.

---

## Task 7: Add unit tests

**Files:**
- Modify: `src/game/ecs/resources.rs` (add tests module)

- [ ] **Step 1: Add comprehensive test suite**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_quest_state_defaults_to_zero() {
        let quest = QuestState::default();
        assert_eq!(quest.princess_rescues, 0);
        assert_eq!(quest.statues_collected, 0);
        assert!(!quest.writ_obtained);
        assert!(!quest.rose_obtained);
        assert!(!quest.crystal_shard_obtained);
        assert!(!quest.sun_stone_obtained);
        assert!(!quest.golden_lasso_obtained);
        assert!(!quest.talisman_obtained);
        assert!(!quest.king_bone_obtained);
    }
    
    #[test]
    fn test_can_enter_azal_requires_5() {
        let mut quest = QuestState::default();
        
        // Should not be able to enter with 4 statues
        quest.statues_collected = 4;
        assert!(!quest.can_enter_azal());
        
        // Should be able to enter with 5 statues
        quest.statues_collected = 5;
        assert!(quest.can_enter_azal());
        
        // Should still be able with more than 5 (capped)
        quest.statues_collected = 10;
        assert!(quest.can_enter_azal());
    }
    
    #[test]
    fn test_has_won_requires_all_princesses_and_talisman() {
        let mut quest = QuestState::default();
        
        // Should not win with only princesses
        quest.princess_rescues = 3;
        assert!(!quest.has_won());
        
        // Should not win with only talisman
        quest.princess_rescues = 0;
        quest.talisman_obtained = true;
        assert!(!quest.has_won());
        
        // Should win with both
        quest.princess_rescues = 3;
        quest.talisman_obtained = true;
        assert!(quest.has_won());
    }
    
    #[test]
    fn test_progress_percent_calculation() {
        let mut quest = QuestState::default();
        assert_eq!(quest.progress_percent(), 0);
        
        // 1 princess = 11% (1/9 * 100)
        quest.princess_rescues = 1;
        assert_eq!(quest.progress_percent(), 11);
        
        // 1 princess + 5 statues = 66% (6/9 * 100)
        quest.statues_collected = 5;
        assert_eq!(quest.progress_percent(), 66);
        
        // All major items = 100%
        quest.princess_rescues = 3;
        quest.talisman_obtained = true;
        assert_eq!(quest.progress_percent(), 100);
    }
    
    #[test]
    fn test_quest_state_equality() {
        let quest1 = QuestState {
            princess_rescues: 2,
            statues_collected: 3,
            talisman_obtained: true,
            ..Default::default()
        };
        
        let quest2 = QuestState {
            princess_rescues: 2,
            statues_collected: 3,
            talisman_obtained: true,
            ..Default::default()
        };
        
        let quest3 = QuestState {
            princess_rescues: 1,
            statues_collected: 3,
            talisman_obtained: true,
            ..Default::default()
        };
        
        assert_eq!(quest1, quest2);
        assert_ne!(quest1, quest3);
    }
    
    #[test]
    fn test_quest_state_clone() {
        let quest1 = QuestState {
            princess_rescues: 2,
            statues_collected: 3,
            writ_obtained: true,
            ..Default::default()
        };
        
        let quest2 = quest1.clone();
        assert_eq!(quest1, quest2);
        
        // Verify independence
        quest2.princess_rescues = 3;
        assert_ne!(quest1, quest2);
    }
}
```

- [ ] **Step 2: Add persist tests**

```rust
#[cfg(test)]
mod persist_tests {
    use super::*;
    use crate::game::ecs::scene::EcsScene;
    use crate::game::ecs::resources::QuestState;
    
    #[test]
    fn test_quest_roundtrip_save_load() {
        // Create a scene with quest progress
        let mut scene = create_test_scene();
        scene.res.quest.princess_rescues = 2;
        scene.res.quest.statues_collected = 4;
        scene.res.quest.talisman_obtained = true;
        scene.res.quest.sun_stone_obtained = true;
        
        // Save to proto
        let proto = ecs_to_proto(&scene);
        
        // Load into new scene
        let mut new_scene = create_test_scene();
        proto_to_ecs(&proto, &mut new_scene).unwrap();
        
        // Verify quest state preserved
        assert_eq!(new_scene.res.quest.princess_rescues, 2);
        assert_eq!(new_scene.res.quest.statues_collected, 4);
        assert!(new_scene.res.quest.talisman_obtained);
        assert!(new_scene.res.quest.sun_stone_obtained);
        assert!(!new_scene.res.quest.writ_obtained); // Should remain false
    }
    
    fn create_test_scene() -> EcsScene {
        // Helper to create minimal test scene
        todo!("Implement test helper")
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test ecs::resources::tests 2>&1 | grep "^test result"
cargo test persist_tests 2>&1 | grep "^test result"
```

Expected: all tests pass.

---

## Task 8: Integration testing

- [ ] **Step 1: Test item pickup updates quest state**

```bash
cargo run -- --ecs
```

1. Use debug commands to spawn quest items
2. Pick up items - verify quest state updates and messages appear
3. Check debug TUI for quest progress

- [ ] **Step 2: Test save/load preserves quest state**

1. Collect some quest items
2. Save game (F5)
3. Load game (F9)
4. Verify quest state is preserved

- [ ] **Step 3: Test Azal gate requirement**

1. Collect 4 statues - verify cannot enter Azal
2. Collect 5th statue - verify can enter Azal
3. Test message appears when gate opens

- [ ] **Step 4: Test win condition**

1. Rescue all 3 princesses
2. Obtain talisman
3. Verify win condition is met

---

## Task 9: Final verification

- [ ] **Step 1: Full build check**

```bash
cargo build 2>&1 | grep "^error"
```

Expected: no errors.

- [ ] **Step 2: Run test suite**

```bash
cargo test 2>&1 | grep "^test result"
```

Expected: all tests pass.

- [ ] **Step 3: Commit changes**

```bash
git add src/game/ecs/resources.rs src/game/ecs/systems/item.rs src/game/persist.rs proto/faery_save.proto
git commit -m "feat(ecs): implement QuestState tracking and persistence

- Add QuestState struct to track princess rescues, statues, key items
- Serialize/deserialize quest state in protobuf save format
- Hook item pickup to update quest progress
- Add quest state helpers (can_enter_azal, has_won, progress_percent)
- Add comprehensive unit tests for quest state logic"
```

---

## Completion check

```bash
cargo build 2>&1 | grep "^error"
cargo test 2>&1 | grep "^test result"
cargo run -- --ecs 2>&1 | head -5
```

All three succeed. Quest state tracks properly, persists across save/load, and integrates with gameplay systems.

---

## Context

### Quest items (inventory slots)
- `stuff[7]` = Sun Stone (Witch vulnerability)
- `stuff[5]` = Golden Lasso (swan flight)
- `stuff[6]` = Shell (summon turtle)
- `stuff[22]` = Talisman (win condition)
- `stuff[25]` (or equivalent) = statue count
- `res.region.princess` = princess rescue counter (already exists)

### Proto file current state
`proto/faery_save.proto` already has `princess` field. Need 8 new fields for quest items.

### Item system hook point
`src/game/ecs/systems/item.rs` handles TakeItem action. On pickup, check `WorldObj.ob_id` and update QuestState.

---

## Dependencies
Plan G (region entities), Plan L (narrative for rescue sequences). Plans A-D.

---

## Spec references
- `docs/spec/world-structure.md` §2.7 — quest item locations
- `docs/spec/death-revival.md` §20.7 — win condition (talisman)
- `docs/spec/save-load.md` §24.1 — proto fields