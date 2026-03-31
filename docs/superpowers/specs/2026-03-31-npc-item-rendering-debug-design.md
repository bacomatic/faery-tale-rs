# NPC/Item Rendering & Debug Commands Design

**Date:** 2026-03-31
**Scope:** Wire up enemy/setfig NPC rendering; add `/encounter` and `/items` debug commands.

---

## Problem Statement

The NPC data-loading, movement, and combat systems are all fully implemented. However, NPCs are **never drawn on screen**: enemy sprite sheets (cfiles 6–12) are not loaded at region init, and `blit_actors_to_framebuf()` discards the NPC table with `let _ = npc_table`. SetFig (named NPC) rendering is also unimplemented. Additionally, the debug console lacks commands to force/clear encounters or scatter items in the world.

---

## 1. Enemy NPC Rendering

### 1a. Load enemy sprite sheets at region init

**File:** `src/game/gameplay_scene.rs` — region reload block (~line 2624)

Expand the cfile load loop from `[0u8, 1, 2, 13, 14, 15, 16, 17]` to include enemy sheets:

```rust
for cfile_idx in [0u8, 1, 2, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17]
```

Enemy cfile mapping:
| cfile | Contents |
|-------|----------|
| 6 | Ogre / default humanoid enemy |
| 7 | Ghost / Wraith / Skeleton |
| 8 | Dark Knight / Spider / Snake |
| 9 | Necromancer / Farmer |
| 10 | Dragon (5 frames) |
| 11 | Bird |
| 12 | Snake / Salamander |

### 1b. NPC render pass in `blit_actors_to_framebuf()`

**File:** `src/game/gameplay_scene.rs` — after hero blit, replacing `let _ = npc_table;`

For each `npc` in `npc_table.npcs` where `npc.active`:

1. Map `npc.npc_type` → `cfile_idx` using a match:
   - `NPC_TYPE_ORC | NPC_TYPE_HUMAN` → 6
   - `NPC_TYPE_GHOST` → 7
   - `NPC_TYPE_SKELETON` → 7
   - `NPC_TYPE_WRAITH` → 7
   - `NPC_TYPE_DRAGON` → 10
   - `NPC_TYPE_SWAN` → 11
   - `NPC_TYPE_HORSE` → 5 (turtle/carrier, already loaded)
   - default → 6

2. Compute screen-relative position from `npc.x/y` using existing `actor_rel_pos()` helper.

3. Pick animation frame: extract the inline `frame_base` match on `hero_facing` (currently at ~line 2246 in `blit_actors_to_framebuf`) into a `facing_to_frame_base(facing: u8) -> usize` helper so both hero and NPC paths share it.
   For enemy sprites, all use the same 8-direction × 8-frame layout as the player sheets.

4. Call `blit_sprite_to_framebuf(fp, rel_x, rel_y, framebuf, fb_w, fb_h)` — bounds clipping is already handled inside this function.

**Depth ordering:** Render NPCs in `npc_table` order (original game does not z-sort within the NPC list).

### 1c. SetFig NPC render pass

SetFig NPCs (shopkeepers, quest givers, guards, etc.) are identified by `npc.race` being one of the named-NPC race codes (`RACE_SHOPKEEPER = 0x88`, `RACE_BEGGAR = 0x8D`, etc.) or by `npc_type == NPC_TYPE_HUMAN` with a non-enemy race.

Use `SETFIG_TABLE: [SetfigEntry; 14]` from `sprites.rs` to look up `cfile_entry` and `image_base` from the NPC's type/subtype. SetFigs are stationary; use idle frame cycling (same 8-frame rate as hero still animation).

---

## 2. Debug Command: `/encounter`

### New `DebugCommand` variants (in `debug_command.rs`)

```rust
SpawnEncounterRandom,         // /encounter
SpawnEncounterType(u8),       // /encounter <type>
ClearEncounters,              // /encounter clear
```

### Console syntax

```
/encounter              → force a full regional encounter (4 enemies, mixflag)
/encounter <type>       → spawn one named enemy type next to player
/encounter clear        → deactivate all NPCs in current npc_table
```

Valid type names: `orc`, `human`, `ghost`, `skeleton`, `wraith`, `dragon`, `snake`, `swan`, `horse`

### `SpawnEncounterRandom` behavior

Mirrors what the game does on a natural encounter trigger:

1. Spawn up to 4 enemies by filling free (`active == false`) NPC slots.
2. Use `spawn_encounter(zone_idx, hero_x, hero_y)` for each slot.
3. Apply mixflag blending: `race = (encounter_type & 0xFFFE) + (i % 2)` to alternate between even/odd encounter IDs in the pair, matching the `mixflag & 2` behavior from `fmain.c`.
4. Offset each spawn position radially around the hero (4 positions fanning out from the hero's facing direction, ~48–80px apart).
5. Log: `"forced encounter: N enemies"` to debug console.

Works regardless of `in_encounter_zone` (debug override).

### `SpawnEncounterType` behavior

Spawns a single NPC of the specified type adjacent to the hero (ahead in facing direction, ~48px). Uses `spawn_encounter()` for difficulty scaling, then overrides `npc_type` and `race` to match the requested type.

### `ClearEncounters` behavior

Sets `active = false` on every entry in the current `npc_table`. Logs `"cleared N NPCs"`.

---

## 3. Debug Command: `/items`

### New `DebugCommand` variant (in `debug_command.rs`)

```rust
ScatterItems { count: usize, item_id: Option<usize> },
```

### Console syntax

```
/items                     → drop all 30 safe items in a ring around player
/items <count>             → drop <count> random items (safe pool)
/items <id_or_name>        → drop one item by index (0–30) or partial name
/items <count> <id_or_name>→ drop <count> copies of named item
```

### Safe item pool

The safe pool is `INV_LIST[0..31]` **excluding** `ITEM_TALISMAN` (index 22).
Talisman can only be dropped with **`/items talisman`** (explicit name required). Any random roll that lands on `ITEM_TALISMAN` is rerolled. This prevents accidental game-ending triggers.

### `ScatterItems` behavior

1. Resolve `item_id` from the command argument:
   - If `None`: scatter `count` items from the safe pool (random selection with replacement, or all 30 if count matches pool size).
   - If explicit name `"talisman"`: drop talisman regardless of guard.
   - If other name/index: look up via a new `item_name_to_id(name: &str) -> Option<usize>` helper in `sprites.rs` that matches on a static `&[(&str, usize)]` table (partial substring match).

2. Place each item as a new `WorldObject` entry in `state.world_objects`:
   ```rust
   WorldObject { item_id, region: state.region_num, x, y, visible: true }
   ```

3. Positions: for multi-item scatters, arrange in a ring at radius ~80px around hero, evenly spaced by angle. For single-item drops, place at hero position + small random offset (~16px).

4. Name resolution table: short-name → `INV_LIST` index. Uses a simple static `&[(&str, usize)]` slice, looked up by substring match. This is the same table that will fix the `fix-item-names` task.

---

## 4. Files Changed

| File | Change |
|------|--------|
| `src/game/gameplay_scene.rs` | Expand cfile load loop; add NPC/setfig render pass in `blit_actors_to_framebuf()`; handle new `DebugCommand` variants |
| `src/game/debug_command.rs` | Add `SpawnEncounterRandom`, `SpawnEncounterType(u8)`, `ClearEncounters`, `ScatterItems { count, item_id }` |
| `src/game/debug_console.rs` | Add `/encounter` and `/items` command parsing; update `/help` output |
| `src/game/sprites.rs` | Add `item_name_to_id(name: &str) -> Option<usize>` helper for debug name resolution |
| `src/game/encounter.rs` | Add `spawn_encounter_group()` for the 4-enemy random encounter with mixflag; keep existing `spawn_encounter()` unchanged |

---

## 5. Out of Scope

- NPC dialogue or interaction beyond what already exists
- Item pick-up animation changes
- Any new NPC types not present in original game data
- Z-sorting of NPC sprites (original doesn't do it)
- Saving debug-spawned items/NPCs to the save file
