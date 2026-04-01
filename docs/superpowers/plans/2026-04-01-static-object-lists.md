# Static Object Lists Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the original game's static world object lists (`ob_listg`, `ob_list0`–`ob_list9`) so that ground items (chests, sacks, keys, etc.) appear at their canonical positions when entering a region.

**Architecture:** Port the object data tables from `original/fmain2.c:1347–1508` into a new `src/game/world_objects.rs` module. On region change, populate `GameState.world_objects` from the static lists for ground items (ob_stat == 1 and ob_stat == 5). Rename `WorldObject.item_id` to `ob_id` to match the original's sprite-frame-based object type system, and add an `itrans` translation table for ob_id → stuff[] index on pickup.

**Tech Stack:** Rust, existing `GameState`/`WorldObject` types, `gameplay_scene.rs` region transition hooks

**Key references:**
- Original data: `original/fmain2.c:1294–1508` (obytes enum, itrans[], ob_listg[], ob_list0–9)
- Existing rendering: `gameplay_scene.rs:3303–3315` (world object render loop)
- Existing pickup: `game_state.rs:665–685` (pickup_world_object)
- Object types: `RESEARCH.md:962–1007` (ob_id → stuff[] mapping)

---

### Task 1: Rename `WorldObject.item_id` → `ob_id`

**Files:**
- Modify: `src/game/game_state.rs`
- Modify: `src/game/gameplay_scene.rs`

The current `WorldObject.item_id` stores stuff[] indices, but the original uses `ob_id` values (sprite frame indices). This rename aligns with the original and makes the field's purpose unambiguous.

- [ ] **Step 1: Rename the field in the struct definition**

In `src/game/game_state.rs`, change the `WorldObject` struct:

```rust
/// An item lying on the ground in the world.
/// `ob_id` is the original obytes enum value — used as the sprite frame index
/// for rendering and translated via `itrans` to a stuff[] index on pickup.
#[derive(Debug, Clone)]
pub struct WorldObject {
    pub ob_id: u8,
    pub region: u8,
    pub x: u16,
    pub y: u16,
    pub visible: bool,
}
```

- [ ] **Step 2: Fix all references in `game_state.rs`**

In `drop_item_to_world` (around line 654):
```rust
self.world_objects.push(WorldObject {
    ob_id: item_id as u8,
    region, x, y,
    visible: true,
});
```

In `pickup_world_object` (around line 678):
```rust
let ob_id = self.world_objects[idx].ob_id;
```
(This will be updated further in Task 3 to use `itrans`.)

- [ ] **Step 3: Fix all references in `gameplay_scene.rs`**

Search for `item_id` in `WorldObject` contexts. There are ~6 occurrences:
- Debug scatter (lines ~2234, ~2256): `item_id: id as u8` → `ob_id: id as u8`
- Render loop (line ~3309): `let frame = obj.item_id as usize` → `let frame = obj.ob_id as usize`
- Render masking (line ~3338): same change
- Test (line ~3529): `item_id: item_id as u8` → `ob_id: item_id as u8`
- Test assertion (line ~3537): `o.item_id` → `o.ob_id`

- [ ] **Step 4: Build and run tests**

Run: `cargo test 2>&1 | tail -5`
Expected: All tests pass. No functional change — just a rename.

- [ ] **Step 5: Commit**

```bash
git add -u && git commit -m "refactor: rename WorldObject.item_id to ob_id

Aligns field name with the original game's obytes enum. The ob_id value
is the sprite frame index used for rendering, not the stuff[] inventory
index. This prepares for adding the itrans translation table and static
object lists.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 2: Create `world_objects.rs` with static data tables

**Files:**
- Create: `src/game/world_objects.rs`
- Modify: `src/game/mod.rs`

Port `itrans[]`, `ob_listg[]`, `ob_list0–9[]`, `mapobs[]`, and `dstobs[]` initial values from `original/fmain2.c:1294–1516`.

- [ ] **Step 1: Create the module with the `itrans` table**

Create `src/game/world_objects.rs`:

```rust
//! Static world object lists — direct port of ob_listg/ob_list0–9 from fmain2.c.
//!
//! Each entry is (x, y, ob_id, ob_stat):
//!   ob_stat 0 = nonexistent, 1 = ground item, 2 = in inventory,
//!   3 = setfig (NPC), 4 = dead setfig, 5 = hidden (revealed by Look),
//!   6 = cabinet item.

/// Translate an ob_id (obytes enum value) to a stuff[] inventory index.
/// Returns None for ob_ids that don't map to inventory items (containers,
/// money, footstools, turtle eggs, dead brother bones, etc.).
///
/// Direct port of itrans[] from fmain2.c:1325–1332.
pub fn ob_id_to_stuff_index(ob_id: u8) -> Option<usize> {
    // itrans is a flat array of (ob_id, stuff_index) pairs, 0-terminated.
    const ITRANS: &[(u8, u8)] = &[
        (11,  35),  // QUIVER → arrows (×10 in pickup logic)
        (18,   9),  // B_STONE → Blue Stone
        (19,  10),  // G_JEWEL → Green Jewel
        (22,  11),  // VIAL → Glass Vial
        (21,  12),  // C_ORB → Crystal Orb
        (23,  13),  // B_TOTEM → Bird Totem
        (17,  14),  // G_RING → Gold Ring
        (24,  15),  // J_SKULL → Jade Skull
        (145,  4),  // M_WAND → Magic Wand
        (27,   5),  // → Golden Lasso
        (8,    2),  // → Sword
        (9,    1),  // → Mace
        (12,   0),  // → Dirk
        (10,   3),  // → Bow
        (147, 23),  // ROSE → Rose
        (148, 24),  // FRUIT → Fruit
        (149, 25),  // STATUE → Gold Statue
        (150, 26),  // BOOK → Book
        (151,  6),  // SHELL → Sea Shell
        (155,  7),  // → Sun Stone
        (136, 27),  // → Herb
        (137, 28),  // → Writ
        (138, 29),  // → Bone
        (139, 22),  // → Talisman
        (140, 30),  // → Shard
        (25,  16),  // GOLD_KEY → Gold Key
        (153, 17),  // GREEN_KEY → Green Key
        (114, 18),  // BLUE_KEY → Blue Key
        (242, 19),  // RED_KEY → Red Key
        (26,  20),  // GREY_KEY → Grey Key
        (154, 21),  // WHITE_KEY → White Key
    ];
    for &(oid, sidx) in ITRANS {
        if oid == ob_id {
            return Some(sidx as usize);
        }
    }
    None
}
```

- [ ] **Step 2: Add the static object list data**

Append to `src/game/world_objects.rs`:

```rust
/// A raw static object entry: (x, y, ob_id, ob_stat).
/// Direct port of ob_listg[] and ob_list0–9 from fmain2.c:1347–1508.
pub type RawObject = (u16, u16, u8, u8);

/// Global objects (ob_listg) — visible across all regions.
/// Indices 0–4 are special (give-slot, dead brothers, ghost brothers).
/// Index 5 is the spectre (setfig, region-specific).
/// Indices 6–10 are gold statues.
pub const OB_LISTG: &[RawObject] = &[
    (0, 0, 0, 0),              // 0: special item (for 'give')
    (0, 0, 28, 0),             // 1: dead brother 1
    (0, 0, 28, 0),             // 2: dead brother 2
    (19316, 15747, 11, 0),     // 3: ghost brother 1
    (18196, 15735, 11, 0),     // 4: ghost brother 2
    (12439, 36202, 10, 3),     // 5: spectre (setfig)
    (11092, 38526, 149, 1),    // 6: gold statue (seahold)
    (25737, 10662, 149, 1),    // 7: gold statue (ogre den)
    (2910, 39023, 149, 1),     // 8: gold statue (octal room)
    (12025, 37639, 149, 0),    // 9: gold statue (sorceress) — hidden
    (6700, 33766, 149, 0),     // 10: gold statue (priest) — hidden
];

/// Region 0: snow land objects.
pub const OB_LIST0: &[RawObject] = &[
    (3340, 6735, 12, 3),       // ranger west (setfig)
    (9678, 7035, 12, 3),       // ranger east (setfig)
    (4981, 6306, 12, 3),       // ranger north (setfig)
];

/// Region 1: maze forest (north).
pub const OB_LIST1: &[RawObject] = &[
    (23087, 5667, 102, 1),     // TURTLE eggs
];

/// Region 2: swamp land.
pub const OB_LIST2: &[RawObject] = &[
    (13668, 15000, 0, 3),      // wizard (setfig)
    (10627, 13154, 0, 3),      // wizard (setfig)
    (4981, 10056, 12, 3),      // ranger (setfig)
    (13950, 11087, 16, 1),     // SACKS
    (10344, 36171, 151, 1),    // SHELL
];

/// Region 3: maze forest (south), manor, tambry — starting region.
pub const OB_LIST3: &[RawObject] = &[
    (19298, 16128, 15, 1),     // CHEST
    (18310, 15969, 13, 3),     // beggar (setfig)
    (20033, 14401, 0, 3),      // wizard (setfig)
    (24794, 13102, 13, 3),     // beggar (setfig)
    (21626, 15446, 18, 1),     // B_STONE at stone ring
    (21616, 15456, 13, 1),     // MONEY
    (21636, 15456, 17, 1),     // G_RING
    (20117, 14222, 19, 1),     // G_JEWEL
    (24185, 9840, 16, 1),      // SACKS
    (25769, 10617, 13, 1),     // MONEY
    (25678, 10703, 18, 1),     // B_STONE
    (17177, 10599, 20, 1),     // SCRAP
];

/// Region 4: desert.
pub const OB_LIST4: &[RawObject] = &[
    (0, 0, 0, 0),              // dummy
    (0, 0, 0, 0),              // dummy
    (6817, 19693, 13, 3),      // beggar (setfig) — must be 3rd object
];

/// Region 5: farm and city.
pub const OB_LIST5: &[RawObject] = &[
    (22184, 21156, 13, 3),     // beggar (setfig)
    (18734, 17595, 17, 1),     // G_RING
    (21294, 22648, 15, 1),     // CHEST
    (22956, 19955, 0, 3),      // wizard (setfig)
    (28342, 22613, 0, 3),      // wizard (setfig)
];

/// Region 6: lava plain.
pub const OB_LIST6: &[RawObject] = &[
    (24794, 13102, 13, 3),     // dummy (beggar setfig)
];

/// Region 7: southern mountain land.
pub const OB_LIST7: &[RawObject] = &[
    (23297, 5797, 102, 1),     // TURTLE (dummy object)
];

/// Region 8: interiors of buildings (77 static objects).
pub const OB_LIST8: &[RawObject] = &[
    // NPCs (setfig, ob_stat 3)
    (6700, 33756, 1, 3),       // priest in chapel
    (5491, 33780, 5, 3),       // king on throne
    (5592, 33764, 6, 3),       // noble
    (5514, 33668, 2, 3),       // guard
    (5574, 33668, 2, 3),       // guard
    (8878, 38995, 0, 3),       // wizard
    (7776, 34084, 0, 3),       // wizard
    (5514, 33881, 3, 3),       // guard
    (5574, 33881, 3, 3),       // guard
    (10853, 35656, 4, 3),      // princess
    (12037, 37614, 7, 3),      // sorceress
    (11013, 36804, 9, 3),      // witch
    (9631, 38953, 8, 3),       // bartender
    (10191, 38953, 8, 3),      // bartender
    (10649, 38953, 8, 3),      // bartender
    (2966, 33964, 8, 3),       // bartender
    // Ground items (ob_stat 1)
    (9532, 40002, 31, 1),      // FOOTSTOOL
    (6747, 33751, 31, 1),      // FOOTSTOOL
    (11410, 36169, 155, 1),    // sunstone (27+128 = 155)
    (9550, 39964, 23, 1),      // B_TOTEM (cabinet)
    (9552, 39964, 23, 1),      // B_TOTEM
    (9682, 39964, 23, 1),      // B_TOTEM (cabinet)
    (9684, 39964, 23, 1),      // B_TOTEM
    (9532, 40119, 23, 1),      // B_TOTEM (table)
    (9575, 39459, 14, 1),      // URN
    (9590, 39459, 14, 1),      // URN
    (9605, 39459, 14, 1),      // URN
    (9680, 39453, 22, 1),      // VIAL
    (9682, 39453, 22, 1),      // VIAL
    (9784, 39453, 22, 1),      // VIAL
    (9668, 39554, 15, 1),      // CHEST
    (11090, 39462, 13, 1),     // MONEY
    (11108, 39458, 23, 1),     // B_TOTEM
    (11118, 39459, 23, 1),     // B_TOTEM
    (11128, 39459, 23, 1),     // B_TOTEM
    (11138, 39458, 23, 1),     // B_TOTEM
    (11148, 39459, 23, 1),     // B_TOTEM
    (11158, 39459, 23, 1),     // B_TOTEM
    (11855, 36206, 31, 1),     // FOOTSTOOL
    (11909, 36198, 15, 1),     // CHEST
    (11918, 36246, 23, 1),     // B_TOTEM (cabinet)
    (11928, 36246, 23, 1),     // B_TOTEM
    (11938, 36246, 23, 1),     // B_TOTEM
    (12212, 38481, 15, 1),     // CHEST
    (11652, 38481, 242, 1),    // RED_KEY
    (10427, 39977, 31, 1),     // FOOTSTOOL
    (10323, 40071, 14, 1),     // URN
    (10059, 38472, 16, 1),     // SACKS
    (10344, 36171, 151, 1),    // SHELL
    (11936, 36207, 20, 1),     // SCRAP (spectre note)
    (9674, 35687, 14, 1),      // URN
    (5473, 38699, 147, 1),     // ROSE
    (7185, 34342, 148, 1),     // FRUIT
    (7190, 34342, 148, 1),     // FRUIT
    (7195, 34342, 148, 1),     // FRUIT
    (7185, 34347, 148, 1),     // FRUIT
    (7190, 34347, 148, 1),     // FRUIT
    (7195, 34347, 148, 1),     // FRUIT
    (6593, 34085, 148, 1),     // FRUIT
    (6598, 34085, 148, 1),     // FRUIT
    (6593, 34090, 148, 1),     // FRUIT
    (6598, 34090, 148, 1),     // FRUIT
    // Hidden items (ob_stat 5, revealed by Look)
    (3872, 33546, 25, 5),      // GOLD_KEY
    (3887, 33510, 23, 5),      // B_TOTEM
    (4495, 33510, 22, 5),      // VIAL
    (3327, 33383, 24, 5),      // J_SKULL
    (4221, 34119, 11, 5),      // QUIVER
    (7610, 33604, 22, 5),      // VIAL
    (7616, 33522, 13, 5),      // MONEY
    (9570, 35768, 18, 5),      // B_STONE
    (9668, 35769, 11, 5),      // QUIVER
    (9553, 38951, 17, 5),      // G_RING
    (10062, 39005, 24, 5),     // J_SKULL
    (10577, 38951, 22, 5),     // VIAL
    (11062, 39514, 13, 5),     // MONEY
    (8845, 39494, 154, 5),     // WHITE_KEY
    (6542, 39494, 19, 5),      // G_JEWEL
    (7313, 38992, 242, 5),     // RED_KEY
];

/// Region 9: underground areas.
pub const OB_LIST9: &[RawObject] = &[
    (7540, 38528, 145, 1),     // M_WAND
    (9624, 36559, 145, 1),     // M_WAND
    (9624, 37459, 145, 1),     // M_WAND
    (8337, 36719, 145, 1),     // M_WAND
    (8154, 34890, 15, 1),      // CHEST
    (7826, 35741, 15, 1),      // CHEST
    (3460, 37260, 0, 3),       // wizard (setfig)
    (8485, 35725, 13, 1),      // MONEY
    (3723, 39340, 138, 1),     // king's bone (128+10 = 138)
];

/// Per-region static object list lookup.
pub const OB_TABLES: [&[RawObject]; 10] = [
    OB_LIST0, OB_LIST1, OB_LIST2, OB_LIST3, OB_LIST4,
    OB_LIST5, OB_LIST6, OB_LIST7, OB_LIST8, OB_LIST9,
];

/// Initial `dstobs` flags — regions 8 and 9 start pre-distributed
/// (they never get random scatter objects).
pub const DSTOBS_INIT: [bool; 10] = [
    false, false, false, false, false,
    false, false, false, true, true,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_itrans_known_mappings() {
        // CHEST (15), SACKS (16), URN (14) are containers — no stuff[] index.
        assert_eq!(ob_id_to_stuff_index(15), None);
        assert_eq!(ob_id_to_stuff_index(16), None);
        assert_eq!(ob_id_to_stuff_index(14), None);
        // MONEY (13) has no stuff[] index (adds gold directly).
        assert_eq!(ob_id_to_stuff_index(13), None);
        // Known mappings.
        assert_eq!(ob_id_to_stuff_index(25), Some(16));   // GOLD_KEY
        assert_eq!(ob_id_to_stuff_index(242), Some(19));   // RED_KEY
        assert_eq!(ob_id_to_stuff_index(145), Some(4));    // M_WAND
        assert_eq!(ob_id_to_stuff_index(149), Some(25));   // STATUE
        assert_eq!(ob_id_to_stuff_index(151), Some(6));    // SHELL
    }

    #[test]
    fn test_itrans_covers_all_documented_items() {
        // Every ob_id in RESEARCH.md "World object types" table that has a
        // stuff index should be present in itrans.
        let documented: &[(u8, usize)] = &[
            (8, 2), (9, 1), (10, 3), (11, 35), (12, 0),
            (17, 14), (18, 9), (19, 10), (21, 12), (22, 11),
            (23, 13), (24, 15), (25, 16), (26, 20),
            (114, 18), (136, 27), (137, 28), (138, 29), (139, 22), (140, 30),
            (145, 4), (147, 23), (148, 24), (149, 25), (150, 26), (151, 6),
            (153, 17), (154, 21), (155, 7), (242, 19),
        ];
        for &(ob_id, expected) in documented {
            assert_eq!(
                ob_id_to_stuff_index(ob_id), Some(expected),
                "ob_id {} should map to stuff index {}", ob_id, expected
            );
        }
    }

    #[test]
    fn test_ob_list3_has_starting_chest() {
        // The chest near the starting position (19298, 16128) must exist.
        let chest = OB_LIST3.iter().find(|o| o.2 == 15 && o.3 == 1);
        assert!(chest.is_some(), "region 3 should have a ground chest");
        let (x, y, _, _) = chest.unwrap();
        assert_eq!(*x, 19298);
        assert_eq!(*y, 16128);
    }

    #[test]
    fn test_ob_tables_regions_count() {
        assert_eq!(OB_TABLES.len(), 10);
        // Region 8 (interiors) should have the most objects.
        assert!(OB_LIST8.len() > 50);
    }

    #[test]
    fn test_dstobs_init() {
        // Regions 8 and 9 start pre-distributed.
        assert!(!DSTOBS_INIT[0]);
        assert!(!DSTOBS_INIT[3]);
        assert!(DSTOBS_INIT[8]);
        assert!(DSTOBS_INIT[9]);
    }
}
```

- [ ] **Step 3: Register the module**

In `src/game/mod.rs`, add:
```rust
pub mod world_objects;
```

- [ ] **Step 4: Build and run tests**

Run: `cargo test world_objects 2>&1 | tail -10`
Expected: All 5 new tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/game/world_objects.rs src/game/mod.rs
git commit -m "feat: add static world object lists from original game

Ports ob_listg[], ob_list0-9[], itrans[], and dstobs[] initial values
from fmain2.c:1294-1516. Includes all 100+ static objects across 10
regions with their original coordinates, ob_id values, and ob_stat
flags.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 3: Update pickup to use `ob_id_to_stuff_index`

**Files:**
- Modify: `src/game/game_state.rs`

The current `pickup_world_object` treats `ob_id` as a stuff[] index directly. It needs to translate via `itrans`, and handle special cases (MONEY adds gold, containers are not yet implemented, FOOTSTOOL/TURTLE block pickup).

- [ ] **Step 1: Write the failing test**

Add to the `tests` module in `game_state.rs`:

```rust
#[test]
fn test_pickup_world_object_translates_ob_id() {
    use crate::game::world_objects::ob_id_to_stuff_index;
    let mut s = GameState::new();
    s.region_num = 3;
    s.hero_x = 100;
    s.hero_y = 100;
    // Place a Gold Key (ob_id 25 → stuff index 16).
    s.world_objects.push(WorldObject {
        ob_id: 25, region: 3, x: 100, y: 100, visible: true,
    });
    let result = s.pickup_world_object(3, 100, 100, 24);
    assert!(result.is_some());
    assert_eq!(s.stuff()[16], 1, "Gold Key should be in stuff[16]");
    assert!(!s.world_objects[0].visible);
}

#[test]
fn test_pickup_money_adds_gold() {
    let mut s = GameState::new();
    s.region_num = 3;
    s.hero_x = 100;
    s.hero_y = 100;
    s.gold = 10;
    // MONEY ob_id is 13.
    s.world_objects.push(WorldObject {
        ob_id: 13, region: 3, x: 100, y: 100, visible: true,
    });
    let result = s.pickup_world_object(3, 100, 100, 24);
    assert!(result.is_some());
    assert_eq!(s.gold, 60, "MONEY should add 50 gold");
    assert!(!s.world_objects[0].visible);
}

#[test]
fn test_pickup_footstool_blocked() {
    let mut s = GameState::new();
    s.region_num = 8;
    s.hero_x = 100;
    s.hero_y = 100;
    // FOOTSTOOL ob_id is 31 — not pickable.
    s.world_objects.push(WorldObject {
        ob_id: 31, region: 8, x: 100, y: 100, visible: true,
    });
    let result = s.pickup_world_object(8, 100, 100, 24);
    assert!(result.is_none());
    assert!(s.world_objects[0].visible, "footstool should stay visible");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_pickup_world_object_translates 2>&1 | tail -5`
Expected: FAIL — current code uses ob_id as stuff index directly.

- [ ] **Step 3: Rewrite `pickup_world_object` to use itrans**

In `src/game/game_state.rs`, replace the `pickup_world_object` method:

```rust
/// Pick up the nearest visible world object within `range` pixels.
/// Translates ob_id → stuff[] index via itrans. Special handling:
/// - MONEY (13): adds 50 gold
/// - FOOTSTOOL (31), TURTLE (102): not pickable
/// - Containers (URN 14, CHEST 15, SACKS 16): not yet implemented (skip)
pub fn pickup_world_object(&mut self, region: u8, hero_x: u16, hero_y: u16, range: u16) -> Option<u8> {
    use crate::game::world_objects::ob_id_to_stuff_index;

    let mut found_idx = None;
    for (i, obj) in self.world_objects.iter().enumerate() {
        if obj.visible && obj.region == region
            && hero_x.abs_diff(obj.x) < range
            && hero_y.abs_diff(obj.y) < range
        {
            found_idx = Some(i);
            break;
        }
    }
    let idx = found_idx?;
    let ob_id = self.world_objects[idx].ob_id;

    // Non-pickable objects.
    match ob_id {
        31 | 102 => return None,     // FOOTSTOOL, TURTLE
        14 | 15 | 16 => return None, // URN, CHEST, SACKS (containers — TODO)
        _ => {}
    }

    // MONEY: +50 gold, no inventory slot.
    if ob_id == 13 {
        self.gold += 50;
        self.world_objects[idx].visible = false;
        return Some(ob_id);
    }

    // Translate ob_id → stuff[] index.
    if let Some(stuff_idx) = ob_id_to_stuff_index(ob_id) {
        if self.pickup_item(stuff_idx) {
            self.world_objects[idx].visible = false;
            return Some(ob_id);
        }
    }
    None
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test test_pickup 2>&1 | tail -10`
Expected: All pickup tests pass including the 3 new ones.

- [ ] **Step 5: Commit**

```bash
git add -u && git commit -m "feat: pickup translates ob_id via itrans table

pickup_world_object now uses ob_id_to_stuff_index() to map world
object types to inventory slots. MONEY adds 50 gold directly.
FOOTSTOOL and TURTLE block pickup. Containers (URN/CHEST/SACKS)
are skipped for now (will need treasure generation).

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 4: Populate world objects on region entry

**Files:**
- Modify: `src/game/game_state.rs` (add `populate_region_objects` method)
- Modify: `src/game/gameplay_scene.rs` (call on region change)

- [ ] **Step 1: Write the test**

Add to `game_state.rs` tests:

```rust
#[test]
fn test_populate_region_objects_loads_ground_items() {
    let mut s = GameState::new();
    s.region_num = 3;
    s.populate_region_objects(3);
    // Region 3 has ground items: CHEST, B_STONE, MONEY, G_RING, G_JEWEL, SACKS, MONEY, B_STONE, SCRAP
    let ground_items: Vec<_> = s.world_objects.iter()
        .filter(|o| o.visible && o.region == 3)
        .collect();
    assert!(ground_items.len() >= 9, "region 3 should have at least 9 ground items, got {}", ground_items.len());
    // The chest at (19298, 16128) should be present.
    let chest = ground_items.iter().find(|o| o.ob_id == 15 && o.x == 19298);
    assert!(chest.is_some(), "the starting chest should be present");
}

#[test]
fn test_populate_region_objects_includes_global_ground_items() {
    let mut s = GameState::new();
    s.region_num = 8;
    s.populate_region_objects(8);
    // ob_listg has gold statues with ob_stat 1 (indices 6–8 are visible).
    // Region 8 objects should also include the interior ground items.
    let all_items: Vec<_> = s.world_objects.iter()
        .filter(|o| o.visible)
        .collect();
    // Should have global ground items + region 8 ground items.
    assert!(all_items.len() > 30, "region 8 should have many items, got {}", all_items.len());
}

#[test]
fn test_populate_region_objects_skips_setfig_and_dead() {
    let mut s = GameState::new();
    s.region_num = 3;
    s.populate_region_objects(3);
    // Region 3 has setfigs (ob_stat 3): beggar, wizard, beggar.
    // These should NOT appear as world objects.
    // Beggar ob_id is 13 with ob_stat 3, but MONEY is also ob_id 13 with ob_stat 1.
    // Just check that no object has the setfig coordinates.
    let beggar_setfig = s.world_objects.iter()
        .find(|o| o.x == 18310 && o.y == 15969);
    assert!(beggar_setfig.is_none(), "setfig NPCs should not be in world_objects");
}

#[test]
fn test_populate_clears_previous_region_objects() {
    let mut s = GameState::new();
    s.populate_region_objects(3);
    let count_r3 = s.world_objects.len();
    assert!(count_r3 > 0);
    // Switch to region 5.
    s.populate_region_objects(5);
    // Should not accumulate — old region 3 items replaced.
    // But global items are always included.
    let r3_items = s.world_objects.iter().filter(|o| o.region == 3).count();
    assert_eq!(r3_items, 0, "old region 3 items should be cleared");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test test_populate 2>&1 | tail -5`
Expected: FAIL — method doesn't exist.

- [ ] **Step 3: Implement `populate_region_objects`**

Add to `GameState` impl in `src/game/game_state.rs`:

```rust
/// Load static world objects for the given region from the hardcoded
/// object lists (ob_listg + ob_listN). Clears any previous non-global
/// objects.
///
/// Only ground items (ob_stat 1) and hidden items (ob_stat 5) are
/// loaded as WorldObjects. SetFig NPCs (ob_stat 3/4) are handled
/// separately by the NPC system.
pub fn populate_region_objects(&mut self, region: u8) {
    use crate::game::world_objects::{OB_LISTG, OB_TABLES, RawObject};

    // Remove all existing world objects (they'll be rebuilt from the
    // static tables — object state is not persisted across region
    // changes in the original either).
    self.world_objects.clear();

    let load_ground = |list: &[RawObject], region: u8, out: &mut Vec<WorldObject>| {
        for &(x, y, ob_id, ob_stat) in list {
            if ob_stat == 1 || ob_stat == 5 {
                out.push(WorldObject {
                    ob_id,
                    region,
                    x, y,
                    visible: ob_stat == 1,
                });
            }
        }
    };

    // Global objects are visible in every region; tag them with the
    // current region so the render filter passes.
    load_ground(OB_LISTG, region, &mut self.world_objects);

    // Per-region objects.
    if (region as usize) < OB_TABLES.len() {
        load_ground(OB_TABLES[region as usize], region, &mut self.world_objects);
    }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test test_populate 2>&1 | tail -10`
Expected: All 4 new tests pass.

- [ ] **Step 5: Wire into region transition**

In `src/game/gameplay_scene.rs`, in the `on_region_changed` method (around line 1263), add a call to populate objects right after the log line:

```rust
fn on_region_changed(&mut self, region: u8, game_lib: &GameLibrary) {
    self.log_buffer.push(format!("on_region_changed: region changed to {}", region));
    // Reset door interaction state.
    self.opened_doors.clear();
    self.bumped_door = None;
    // Load static world objects for this region.
    self.state.populate_region_objects(region);
    self.log_buffer.push(format!("on_region_changed: loaded {} world objects", self.state.world_objects.len()));
    // ... rest of existing code ...
```

Also add the same call during initial world load (the startup path). Find where the initial `on_region_changed` / world load happens during `WorldLoaded` state (around line 2990) and add:

```rust
self.state.populate_region_objects(region);
```

- [ ] **Step 6: Build and run all tests**

Run: `cargo test 2>&1 | tail -5`
Expected: All tests pass.

- [ ] **Step 7: Commit**

```bash
git add -u && git commit -m "feat: populate static world objects on region entry

Loads ground items (ob_stat 1) and hidden items (ob_stat 5) from the
static object lists into GameState.world_objects when entering a region.
Global objects (ob_listg) are included in every region. SetFig NPCs
(ob_stat 3/4) are excluded — handled by the NPC system.

The starting region (3) now shows the chest at (19298, 16128) and other
canonical items like the blue stone, gold ring, and jewels at the stone
ring near the starting position.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 5: Update debug scatter to use ob_id values

**Files:**
- Modify: `src/game/gameplay_scene.rs`

The `/items` debug command currently pushes WorldObjects with stuff[] indices as `ob_id`. It should use the `INV_LIST[slot].image_number` as the ob_id for correct sprite rendering, or keep using stuff indices since those happen to work for the debug scatter (indices 0–30 are valid sprite frames). However, now that pickup uses `itrans`, the debug items would fail pickup unless they use valid ob_id values.

- [ ] **Step 1: Create an inverse lookup**

Add to `src/game/world_objects.rs`:

```rust
/// Translate a stuff[] inventory index to an ob_id value.
/// Inverse of ob_id_to_stuff_index. Returns None for stuff indices
/// that have no corresponding ob_id (e.g., food slot 0 maps to Dirk ob_id 12).
pub fn stuff_index_to_ob_id(stuff_idx: usize) -> Option<u8> {
    const INVERSE: &[(usize, u8)] = &[
        (0, 12),    // Dirk
        (1, 9),     // Mace
        (2, 8),     // Sword
        (3, 10),    // Bow
        (4, 145),   // Magic Wand
        (5, 27),    // Golden Lasso
        (6, 151),   // Sea Shell
        (7, 155),   // Sun Stone
        (8, 11),    // Arrows (QUIVER)
        (9, 18),    // Blue Stone
        (10, 19),   // Green Jewel
        (11, 22),   // Glass Vial
        (12, 21),   // Crystal Orb
        (13, 23),   // Bird Totem
        (14, 17),   // Gold Ring
        (15, 24),   // Jade Skull
        (16, 25),   // Gold Key
        (17, 153),  // Green Key
        (18, 114),  // Blue Key
        (19, 242),  // Red Key
        (20, 26),   // Grey Key
        (21, 154),  // White Key
        (22, 139),  // Talisman
        (23, 147),  // Rose
        (24, 148),  // Fruit
        (25, 149),  // Gold Statue
        (26, 150),  // Book
        (27, 136),  // Herb
        (28, 137),  // Writ
        (29, 138),  // Bone
        (30, 140),  // Shard
    ];
    for &(si, oid) in INVERSE {
        if si == stuff_idx {
            return Some(oid);
        }
    }
    None
}
```

- [ ] **Step 2: Add a test for the inverse**

Add to the tests module in `world_objects.rs`:

```rust
#[test]
fn test_stuff_to_ob_id_roundtrip() {
    // Every stuff_index_to_ob_id result should roundtrip through ob_id_to_stuff_index.
    for si in 0..=30 {
        if let Some(ob_id) = stuff_index_to_ob_id(si) {
            let back = ob_id_to_stuff_index(ob_id);
            // QUIVER (ob_id 11) maps to stuff index 35 in itrans (arrows ×10),
            // not back to 8. This is intentional — stuff index 8 is the arrows slot.
            if si == 8 {
                assert_eq!(back, Some(35));
            } else {
                assert_eq!(back, Some(si),
                    "stuff_idx {} → ob_id {} → stuff_idx {:?} (expected {})",
                    si, ob_id, back, si);
            }
        }
    }
}
```

- [ ] **Step 3: Update the debug scatter in `gameplay_scene.rs`**

In the `ScatterItems` handler, change the WorldObject creation to use `stuff_index_to_ob_id`:

For the specific-item branch (around line 2234):
```rust
use crate::game::world_objects::stuff_index_to_ob_id;
let ob_id_val = stuff_index_to_ob_id(id).unwrap_or(id as u8);
self.state.world_objects.push(WorldObject {
    ob_id: ob_id_val,
    region,
    x, y,
    visible: true,
});
```

For the pool branch (around line 2256):
```rust
let ob_id_val = stuff_index_to_ob_id(item_id).unwrap_or(item_id as u8);
self.state.world_objects.push(WorldObject {
    ob_id: ob_id_val,
    region,
    x, y,
    visible: true,
});
```

- [ ] **Step 4: Build and run tests**

Run: `cargo test 2>&1 | tail -5`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add -u && git commit -m "fix: debug scatter uses correct ob_id values

The /items command now translates stuff[] indices to ob_id values via
stuff_index_to_ob_id() so that scattered items render with the correct
sprite and can be picked up through the itrans translation path.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 6: Update RESEARCH.md

**Files:**
- Modify: `RESEARCH.md`

- [ ] **Step 1: Add static object list documentation**

In RESEARCH.md, in the "Object distribution" section (after the scatter algorithm), add a new subsection documenting the static object lists:

```markdown
### Static object lists (`fmain2.c:1347–1508`)

Each region has a hardcoded list of objects (`ob_list0`–`ob_list9`) plus a global list (`ob_listg`) shared across all regions. Objects have an `ob_stat` field determining their type:

| ob_stat | Meaning | Handling |
|---------|---------|----------|
| 0 | Nonexistent | Skipped |
| 1 | Ground item | Rendered as object sprite; pickable |
| 2 | In inventory | Skipped |
| 3 | SetFig (NPC) | Rendered as character sprite via setfig_table |
| 4 | Dead SetFig | Rendered as dead NPC |
| 5 | Hidden | Invisible until revealed by Look action |
| 6 | Cabinet item | Special display (not pickable normally) |

**Notable static objects:**
- Region 3 (starting area): chest at (19298, 16128), blue stone + gold ring + money at stone ring near (21626, 15446)
- Region 8 (interiors): 77 objects — NPCs, furniture, food, hidden treasures
- Region 9 (underground): 4 magic wands, 2 chests, king's bone
- Global: 5 gold statues (2 hidden until quest flags set), spectre, ghost/dead brother slots

The 10 scatter objects (see "Object distribution" above) are appended to the **end** of each region's list at runtime, after the static entries. Each region list has 10 blank slots (`TENBLANKS`) reserved for this purpose.
```

- [ ] **Step 2: Verify and commit**

Run: `cargo test 2>&1 | tail -3` to ensure nothing is broken.

```bash
git add RESEARCH.md && git commit -m "research: document static object lists and ob_stat values

Adds documentation for the per-region static object lists (ob_listg,
ob_list0-9) including the ob_stat field meanings, notable objects by
region, and how scatter objects append to the static lists.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 7: Final verification

- [ ] **Step 1: Full build**

Run: `cargo build 2>&1 | tail -5`
Expected: Clean build.

- [ ] **Step 2: Full test suite**

Run: `cargo test 2>&1 | tail -10`
Expected: All tests pass (existing + ~12 new).

- [ ] **Step 3: Manual smoke test**

Run: `cargo run -- --debug --skip-intro 2>&1` and verify:
1. The chest appears on the ground near the starting position
2. Other items at the stone ring area are visible
3. `/items` debug command still works
4. Pressing Take near items shows pickup behavior
