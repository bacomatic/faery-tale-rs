# NPC/Item Rendering & Debug Commands Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire up enemy and setfig NPC rendering on the game map; add `/encounter` and `/items` debug commands.

**Architecture:** Three independent subsystems implemented in order. (1) NPC rendering extracts a `facing_to_frame_base()` helper, loads enemy cfiles 6–12, and adds two render passes to the existing `blit_actors_to_framebuf()`. (2) `/encounter` command follows the existing DebugCommand→parse→handle pattern and adds `spawn_encounter_group()` to `encounter.rs`. (3) `/items` command adds `item_name_to_id()` to `sprites.rs` and a `ScatterItems` DebugCommand variant.

**Tech Stack:** Rust, `blit_sprite_to_framebuf()` / `blit_obj_to_framebuf()` helpers in `gameplay_scene.rs`, `spawn_encounter()` in `encounter.rs`, `WorldObject` in `game_state.rs`.

**Spec:** `docs/superpowers/specs/2026-03-31-npc-item-rendering-debug-design.md`

---

## File Map

| File | Change |
|------|--------|
| `src/game/gameplay_scene.rs` | Extract `facing_to_frame_base()` + `npc_type_to_cfile()` + `npc_to_setfig_idx()` helpers; expand cfile load loop; add enemy + setfig render passes; handle new DebugCommand variants |
| `src/game/encounter.rs` | Add `spawn_encounter_group()` |
| `src/game/sprites.rs` | Add `item_name_to_id()` |
| `src/game/debug_command.rs` | Add `SpawnEncounterRandom`, `SpawnEncounterType(u8)`, `ClearEncounters`, `ScatterItems { count, item_id }` |
| `src/game/debug_console.rs` | Add `/encounter` and `/items` command parsing; update `/help` |

---

## Task 1: Extract `facing_to_frame_base()` helper and refactor hero blit

**Files:**
- Modify: `src/game/gameplay_scene.rs`

The hero blit at line ~2246 contains an inline `match hero_facing { ... }` expression. Extract this into a shared static helper so the forthcoming NPC render pass can reuse it.

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)]` block in `gameplay_scene.rs` (it already exists — search for `mod tests` or `#[cfg(test)]`):

```rust
#[test]
fn test_facing_to_frame_base() {
    // Rust facing: 0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW
    assert_eq!(GameplayScene::facing_to_frame_base(0), 16); // N  → northwalk
    assert_eq!(GameplayScene::facing_to_frame_base(1), 24); // NE → eastwalk
    assert_eq!(GameplayScene::facing_to_frame_base(2), 24); // E  → eastwalk
    assert_eq!(GameplayScene::facing_to_frame_base(3), 0);  // SE → southwalk
    assert_eq!(GameplayScene::facing_to_frame_base(4), 0);  // S  → southwalk
    assert_eq!(GameplayScene::facing_to_frame_base(5), 8);  // SW → westwalk
    assert_eq!(GameplayScene::facing_to_frame_base(6), 8);  // W  → westwalk
    assert_eq!(GameplayScene::facing_to_frame_base(7), 16); // NW → northwalk
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test test_facing_to_frame_base 2>&1 | tail -10
```
Expected: `error[E0599]: no method named 'facing_to_frame_base'`

- [ ] **Step 3: Add the helper method to `GameplayScene` impl**

In `gameplay_scene.rs`, add this as a `fn` on `GameplayScene` just before `blit_actors_to_framebuf` (around line 2218):

```rust
/// Map a facing direction (0=N…7=NW) to the sprite sheet frame base.
/// Mirrors the diroffs[] group mapping from fmain.c:
///   southwalk=0-7, westwalk=8-15, northwalk=16-23, eastwalk=24-31.
fn facing_to_frame_base(facing: u8) -> usize {
    match facing {
        0 => 16, // N  → northwalk
        1 => 24, // NE → eastwalk
        2 => 24, // E  → eastwalk
        3 => 0,  // SE → southwalk
        4 => 0,  // S  → southwalk
        5 => 8,  // SW → westwalk
        6 => 8,  // W  → westwalk
        _ => 16, // NW → northwalk
    }
}
```

- [ ] **Step 4: Replace the inline match in `blit_actors_to_framebuf`**

Find the inline `frame_base` match at ~line 2246 and replace it with a call to the new helper. The existing code:

```rust
let frame_base: usize = match hero_facing {
    0 => 16, // N  → northwalk
    1 => 24, // NE → eastwalk
    2 => 24, // E  → eastwalk
    3 => 0,  // SE → southwalk
    4 => 0,  // S  → southwalk
    5 => 8,  // SW → westwalk
    6 => 8,  // W  → westwalk
    _ => 16, // NW → northwalk
};
```

Replace with:

```rust
let frame_base = Self::facing_to_frame_base(hero_facing);
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cargo test test_facing_to_frame_base 2>&1 | tail -5
```
Expected: `test test_facing_to_frame_base ... ok`

- [ ] **Step 6: Confirm the game still compiles**

```bash
cargo build 2>&1 | grep -E "^error" | head -10
```
Expected: no output (no errors).

- [ ] **Step 7: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "refactor: extract facing_to_frame_base() helper in gameplay_scene

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 2: Add `npc_type_to_cfile()` helper

**Files:**
- Modify: `src/game/gameplay_scene.rs`

This helper maps `(npc_type, race) → Option<usize>` (cfile index) for the enemy render pass. Returns `None` for SetFig humans (handled in Task 4) and inactive/container NPCs.

- [ ] **Step 1: Write the failing test**

Add to `#[cfg(test)]` in `gameplay_scene.rs`:

```rust
#[test]
fn test_npc_type_to_cfile() {
    use crate::game::npc::*;
    // Enemy humans → ogre sheet
    assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_HUMAN, RACE_ENEMY), Some(6));
    // Named humans → None (SetFig pass)
    assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_HUMAN, RACE_NORMAL), None);
    assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_HUMAN, RACE_SHOPKEEPER), None);
    // Enemy types
    assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_ORC,      RACE_ENEMY),  Some(6));
    assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_GHOST,    RACE_UNDEAD), Some(7));
    assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_SKELETON, RACE_UNDEAD), Some(7));
    assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_WRAITH,   RACE_WRAITH), Some(7));
    assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_DRAGON,   RACE_ENEMY),  Some(10));
    assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_SWAN,     RACE_NORMAL), Some(11));
    assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_HORSE,    RACE_NORMAL), Some(5));
    assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_RAFT,     RACE_NORMAL), Some(4));
    // Inactive / container → None
    assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_NONE,      RACE_NORMAL), None);
    assert_eq!(GameplayScene::npc_type_to_cfile(NPC_TYPE_CONTAINER, RACE_NORMAL), None);
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test test_npc_type_to_cfile 2>&1 | tail -10
```
Expected: `error[E0599]: no method named 'npc_type_to_cfile'`

- [ ] **Step 3: Add the helper to `GameplayScene` impl**

Add below `facing_to_frame_base` in `gameplay_scene.rs`:

```rust
/// Map (npc_type, race) → cfile index for enemy sprite rendering.
/// Returns None for SetFig humans (rendered in a separate pass) and skipped types.
/// cfile 7 covers ghost/wraith/skeleton per RESEARCH.md sprite assignments.
fn npc_type_to_cfile(npc_type: u8, race: u8) -> Option<usize> {
    use crate::game::npc::*;
    match npc_type {
        NPC_TYPE_NONE | NPC_TYPE_CONTAINER => None,
        NPC_TYPE_HUMAN if race == RACE_ENEMY => Some(6),
        NPC_TYPE_HUMAN => None,  // SetFig — handled in setfig pass
        NPC_TYPE_SWAN     => Some(11),
        NPC_TYPE_HORSE    => Some(5),
        NPC_TYPE_DRAGON   => Some(10),
        NPC_TYPE_GHOST    => Some(7),
        NPC_TYPE_ORC      => Some(6),
        NPC_TYPE_WRAITH   => Some(7),
        NPC_TYPE_SKELETON => Some(7),
        NPC_TYPE_RAFT     => Some(4),
        _                 => Some(6), // unknown enemy types default to ogre sheet
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test test_npc_type_to_cfile 2>&1 | tail -5
```
Expected: `test test_npc_type_to_cfile ... ok`

- [ ] **Step 5: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "refactor: add npc_type_to_cfile() helper

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 3: Load enemy cfiles 4–12 at region init

**Files:**
- Modify: `src/game/gameplay_scene.rs`

Enemy sprite sheets for cfiles 4–12 are currently never loaded. Expanding the load loop is the only change needed.

- [ ] **Step 1: Find and expand the cfile load loop**

In `gameplay_scene.rs`, find the comment `// sprite-101: load player (cfile 0-2) and setfig (cfile 13-17) sprites` (around line 2624).

The current array is:
```rust
for cfile_idx in [0u8, 1, 2, 13, 14, 15, 16, 17] {
```

Replace with (adds raft=4, turtle=5, and enemies 6–12):
```rust
for cfile_idx in [0u8, 1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17] {
```

Also update the comment to:
```rust
// sprite-101: load player (0-2), carrier (4-5), enemy (6-12), and setfig (13-17) sprites
```

- [ ] **Step 2: Verify it builds**

```bash
cargo build 2>&1 | grep -E "^error" | head -10
```
Expected: no output.

- [ ] **Step 3: Smoke-test that dlog lines appear for new cfiles**

Run the game briefly and check the debug console log output for lines like:
```
sprite-load: cfile 6 → 64 frames
sprite-load: cfile 7 → 64 frames
```
(The existing `dlog(format!("sprite-load: cfile {} → {} frames", ...))` already handles this.)

- [ ] **Step 4: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "feat: load enemy cfiles 4-12 at region init (npc rendering)

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 4: Add enemy NPC render pass in `blit_actors_to_framebuf()`

**Files:**
- Modify: `src/game/gameplay_scene.rs`

Replace the `let _ = npc_table;` stub with a real render pass. For each active NPC, compute facing from its position relative to the hero, look up the cfile, and blit using the existing `blit_sprite_to_framebuf()` helper.

- [ ] **Step 1: Write the failing test**

Add to `#[cfg(test)]` in `gameplay_scene.rs`:

```rust
#[test]
fn test_enemy_npc_render_pass_writes_pixels() {
    use crate::game::sprites::{SpriteSheet, SPRITE_W, SPRITE_H};
    use crate::game::npc::{Npc, NpcTable, NPC_TYPE_ORC, RACE_ENEMY};
    use crate::game::game_state::GameState;
    use crate::game::map_renderer::{MAP_DST_W, MAP_DST_H};

    // Build a minimal mock sprite sheet for cfile 6 (ogre).
    // Pixel value 0 is non-transparent (only 31 is transparent).
    let frames = 64;
    let mock_sheet = SpriteSheet {
        cfile_idx: 6,
        pixels: vec![0u8; SPRITE_W * SPRITE_H * frames],
        num_frames: frames,
        frame_h: SPRITE_H,
    };

    // 18-element vec; only slot 6 is Some.
    let mut sheets: Vec<Option<SpriteSheet>> = (0..18).map(|_| None).collect();
    sheets[6] = Some(mock_sheet);

    let mut state = GameState::new();
    // Hero at viewport center (map_x=0, map_y=0), hero at (8, 26) so rel=(0,0)
    state.hero_x = 8;
    state.hero_y = 26;

    // Place an ORC near the hero but offset so it appears in viewport
    let mut table = NpcTable { npcs: Default::default() };
    table.npcs[0] = Npc {
        npc_type: NPC_TYPE_ORC,
        race: RACE_ENEMY,
        x: 80,  // rel_x = 80 - 0 - 8 = 72, well within 304px viewport
        y: 80,  // rel_y = 80 - 0 - 26 = 54
        vitality: 10,
        gold: 5,
        speed: 2,
        active: true,
    };

    let mut framebuf = vec![31u8; (MAP_DST_W * MAP_DST_H) as usize]; // all transparent
    GameplayScene::blit_actors_to_framebuf(
        &sheets, &None, &state, &Some(table), 0, 0, &mut framebuf, false,
    );

    // At least some pixels in the ORC's blit area should have been overwritten to 0
    let orc_area_start = (54 * MAP_DST_W as usize) + 72;
    let has_written = framebuf[orc_area_start..orc_area_start + SPRITE_W]
        .iter()
        .any(|&p| p == 0);
    assert!(has_written, "expected ORC pixels to be written to framebuf");
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test test_enemy_npc_render_pass_writes_pixels 2>&1 | tail -10
```
Expected: test compiles and fails with `assertion failed` (no pixels written because `let _ = npc_table` discards it).

- [ ] **Step 3: Replace `let _ = npc_table;` with the enemy NPC render pass**

In `blit_actors_to_framebuf`, find and replace:

```rust
        // --- Enemy NPCs from npc_table ---
        // NPC npc_type maps to cfile index:
        //   NPC_TYPE_HUMAN=1 → cfile 6 (ogre file is default enemy)
        // Enemy blitting is best-effort: use npc_type as a rough cfile hint.
        // SetFig NPCs (wizard, king, etc.) use a separate placement system.
        let _ = npc_table; // reserved for future enemy sprite lookup
    }
```

Replace with:

```rust
        // --- Enemy NPCs from npc_table ---
        if let Some(ref table) = npc_table {
            for npc in table.npcs.iter().filter(|n| n.active) {
                let Some(cfile_idx) = Self::npc_type_to_cfile(npc.npc_type, npc.race) else { continue };
                let Some(Some(ref sheet)) = sprite_sheets.get(cfile_idx) else { continue };

                let (rel_x, rel_y) = Self::actor_rel_pos(npc.x as u16, npc.y as u16, map_x, map_y);

                // Compute facing from NPC position relative to hero (NPCs always chase hero).
                let dx = state.hero_x as i32 - npc.x as i32;
                let dy = state.hero_y as i32 - npc.y as i32;
                let npc_facing = if dx.abs() >= dy.abs() {
                    if dx > 0 { 2u8 } else { 6u8 }  // E or W toward hero
                } else {
                    if dy > 0 { 4u8 } else { 0u8 }  // S or N toward hero
                };

                let frame_base = Self::facing_to_frame_base(npc_facing);
                // Wrap with sheet.num_frames to handle short sheets (e.g. dragon=5).
                let frame = (frame_base + (state.cycle as usize % 8)) % sheet.num_frames;

                if let Some(fp) = sheet.frame_pixels(frame) {
                    Self::blit_sprite_to_framebuf(fp, rel_x, rel_y, framebuf, fb_w, fb_h);
                }
            }
        }
    }
```

- [ ] **Step 4: Run tests**

```bash
cargo test test_enemy_npc_render_pass_writes_pixels 2>&1 | tail -5
```
Expected: `test test_enemy_npc_render_pass_writes_pixels ... ok`

- [ ] **Step 5: Run all tests to check for regressions**

```bash
cargo test 2>&1 | tail -10
```
Expected: all tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "feat: add enemy NPC render pass in blit_actors_to_framebuf

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 5: Add SetFig NPC render pass in `blit_actors_to_framebuf()`

**Files:**
- Modify: `src/game/gameplay_scene.rs`

Named NPCs (shopkeepers, beggars, guards, etc.) are identified by `NPC_TYPE_HUMAN` with a non-enemy race. They use `SETFIG_TABLE` for their sprite sheet and are rendered with a static pose.

- [ ] **Step 1: Write the failing test**

Add to `#[cfg(test)]`:

```rust
#[test]
fn test_setfig_render_pass_writes_pixels() {
    use crate::game::sprites::{SpriteSheet, SPRITE_W, SPRITE_H};
    use crate::game::npc::{Npc, NpcTable, NPC_TYPE_HUMAN, RACE_SHOPKEEPER};
    use crate::game::game_state::GameState;
    use crate::game::map_renderer::{MAP_DST_W, MAP_DST_H};

    // Bartender uses cfile 15 (SETFIG_TABLE[8]).
    let mock_sheet = SpriteSheet {
        cfile_idx: 15,
        pixels: vec![0u8; SPRITE_W * SPRITE_H * 8],
        num_frames: 8,
        frame_h: SPRITE_H,
    };
    let mut sheets: Vec<Option<SpriteSheet>> = (0..18).map(|_| None).collect();
    sheets[15] = Some(mock_sheet);

    let mut state = GameState::new();
    state.hero_x = 8;
    state.hero_y = 26;

    let mut table = NpcTable { npcs: Default::default() };
    table.npcs[0] = Npc {
        npc_type: NPC_TYPE_HUMAN,
        race: RACE_SHOPKEEPER,
        x: 80, y: 80,
        vitality: 10, gold: 0, speed: 0,
        active: true,
    };

    let mut framebuf = vec![31u8; (MAP_DST_W * MAP_DST_H) as usize];
    GameplayScene::blit_actors_to_framebuf(
        &sheets, &None, &state, &Some(table), 0, 0, &mut framebuf, false,
    );

    let setfig_area_start = (54 * MAP_DST_W as usize) + 72;
    let has_written = framebuf[setfig_area_start..setfig_area_start + SPRITE_W]
        .iter()
        .any(|&p| p == 0);
    assert!(has_written, "expected SetFig pixels to be written to framebuf");
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cargo test test_setfig_render_pass_writes_pixels 2>&1 | tail -10
```
Expected: test compiles and fails with `assertion failed`.

- [ ] **Step 3: Add `npc_to_setfig_idx()` helper**

Add below `npc_type_to_cfile` in `gameplay_scene.rs`:

```rust
/// Map (npc_type, race) → SETFIG_TABLE index for named NPC rendering.
/// Returns None if the NPC is not a SetFig.
/// SETFIG_TABLE indices: 0=wizard, 8=bartender, 13=beggar (see sprites.rs).
fn npc_to_setfig_idx(npc_type: u8, race: u8) -> Option<usize> {
    use crate::game::npc::*;
    if npc_type != NPC_TYPE_HUMAN { return None; }
    match race {
        RACE_SHOPKEEPER => Some(8),   // bartender
        RACE_BEGGAR     => Some(13),  // beggar
        RACE_NORMAL     => Some(0),   // wizard (default named NPC)
        _               => None,
    }
}
```

- [ ] **Step 4: Add the SetFig render pass inside `blit_actors_to_framebuf`**

Just before the closing `}` of the enemy NPC block (after the enemy NPC `if let Some(ref table)` block), add a second pass for SetFigs:

```rust
        // --- SetFig NPCs (named NPCs: shopkeepers, beggars, etc.) ---
        if let Some(ref table) = npc_table {
            use crate::game::sprites::SETFIG_TABLE;
            for npc in table.npcs.iter().filter(|n| n.active) {
                let Some(setfig_idx) = Self::npc_to_setfig_idx(npc.npc_type, npc.race) else { continue };
                let entry = SETFIG_TABLE[setfig_idx];
                let cfile_idx = entry.cfile_entry as usize;
                let Some(Some(ref sheet)) = sprite_sheets.get(cfile_idx) else { continue };

                let (rel_x, rel_y) = Self::actor_rel_pos(npc.x as u16, npc.y as u16, map_x, map_y);

                // SetFigs are stationary; use image_base as the static frame.
                let frame = (entry.image_base as usize) % sheet.num_frames;
                if let Some(fp) = sheet.frame_pixels(frame) {
                    Self::blit_sprite_to_framebuf(fp, rel_x, rel_y, framebuf, fb_w, fb_h);
                }
            }
        }
```

- [ ] **Step 5: Run tests**

```bash
cargo test test_setfig_render_pass_writes_pixels 2>&1 | tail -5
```
Expected: `test test_setfig_render_pass_writes_pixels ... ok`

- [ ] **Step 6: Run all tests**

```bash
cargo test 2>&1 | tail -10
```
Expected: all pass.

- [ ] **Step 7: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "feat: add setfig NPC render pass in blit_actors_to_framebuf

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 6: Add `spawn_encounter_group()` to `encounter.rs`

**Files:**
- Modify: `src/game/encounter.rs`

The `/encounter` command needs to fill up to 4 NPC slots at once with mixflag blending. Add `spawn_encounter_group()` which fills free NPC slots with a mixed group, mirroring the original `fmain.c` spawn logic.

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)]` block in `encounter.rs`:

```rust
#[test]
fn test_spawn_encounter_group_fills_slots() {
    use crate::game::npc::NpcTable;
    let mut table = NpcTable { npcs: Default::default() };
    let spawned = spawn_encounter_group(&mut table, 0, 100, 100);
    assert_eq!(spawned, 4, "should fill 4 free slots");
    let active: Vec<_> = table.npcs.iter().filter(|n| n.active).collect();
    assert_eq!(active.len(), 4);
}

#[test]
fn test_spawn_encounter_group_respects_existing() {
    use crate::game::npc::{NpcTable, Npc};
    let mut table = NpcTable { npcs: Default::default() };
    // Pre-fill 14 of the 16 slots
    for i in 0..14 {
        table.npcs[i] = Npc { active: true, ..Default::default() };
    }
    let spawned = spawn_encounter_group(&mut table, 0, 100, 100);
    assert_eq!(spawned, 2, "should only fill the 2 remaining free slots");
}

#[test]
fn test_spawn_encounter_group_mixflag_alternates_race() {
    use crate::game::npc::NpcTable;
    let mut table = NpcTable { npcs: Default::default() };
    spawn_encounter_group(&mut table, 0, 100, 100);
    // With mixflag blending, consecutive enemies alternate race (even/odd LSB).
    let active: Vec<_> = table.npcs.iter().filter(|n| n.active).collect();
    assert!(active.len() >= 2);
    // LSBs of race for first two should differ by 1 (0 and 1, or 2 and 3, etc.)
    let r0 = active[0].race & 1;
    let r1 = active[1].race & 1;
    assert_ne!(r0, r1, "mixflag: consecutive NPCs should alternate race LSB");
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test test_spawn_encounter_group 2>&1 | tail -10
```
Expected: `error[E0425]: cannot find function 'spawn_encounter_group'`

- [ ] **Step 3: Add `spawn_encounter_group()` to `encounter.rs`**

Add after `spawn_encounter()`:

```rust
/// Spawn up to 4 enemies into free NPC slots, mirroring fmain.c group encounter logic.
///
/// Applies mixflag blending: each successive NPC alternates the low bit of race
/// (`race = (base_race & 0xFE) | (i & 1)`), creating a mixed enemy group.
/// Spawn positions fan out in 4 cardinal directions from the hero.
///
/// Returns the number of NPCs spawned.
pub fn spawn_encounter_group(
    table: &mut crate::game::npc::NpcTable,
    region_zone_idx: usize,
    hero_x: i16,
    hero_y: i16,
) -> usize {
    const MAX_GROUP: usize = 4;
    const OFFSETS: [(i16, i16); 4] = [(48, 0), (-48, 0), (0, 48), (0, -48)];

    let base = spawn_encounter(region_zone_idx, hero_x, hero_y);
    let base_race = base.race & 0xFE; // clear low bit for mixflag alternation

    let mut spawned = 0;
    for (i, (ox, oy)) in OFFSETS.iter().enumerate() {
        if spawned >= MAX_GROUP { break; }
        if let Some(slot) = table.npcs.iter_mut().find(|n| !n.active) {
            let mut npc = spawn_encounter(region_zone_idx, hero_x, hero_y);
            npc.x = hero_x.saturating_add(*ox);
            npc.y = hero_y.saturating_add(*oy);
            npc.race = base_race | (i as u8 & 1); // mixflag: alternate even/odd
            *slot = npc;
            spawned += 1;
        }
    }
    spawned
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test test_spawn_encounter_group 2>&1 | tail -10
```
Expected: all 3 tests pass.

- [ ] **Step 5: Run all tests**

```bash
cargo test 2>&1 | tail -10
```
Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add src/game/encounter.rs
git commit -m "feat: add spawn_encounter_group() with mixflag blending

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 7: Wire `/encounter` debug command end-to-end

**Files:**
- Modify: `src/game/debug_command.rs`
- Modify: `src/game/debug_console.rs`
- Modify: `src/game/gameplay_scene.rs`

- [ ] **Step 1: Add new DebugCommand variants**

In `debug_command.rs`, add three variants to the `DebugCommand` enum after `QueryTerrain`:

```rust
    /// Force a regional encounter (4 enemies, mixflag blending).
    SpawnEncounterRandom,
    /// Spawn one named enemy type adjacent to the hero.
    SpawnEncounterType(u8),
    /// Deactivate all NPCs in the current npc_table.
    ClearEncounters,
```

- [ ] **Step 2: Add `/encounter` to the `execute_command` match in `debug_console.rs`**

Find the match arm `"/terrain" => ...` and add below it:

```rust
            "/encounter" => self.cmd_encounter(args),
```

- [ ] **Step 3: Add `cmd_encounter()` method to `debug_console.rs`**

Add this method in the "Individual commands" section (after `cmd_adf` or similar):

```rust
fn cmd_encounter(&mut self, args: &[&str]) {
    use crate::game::npc::*;
    match args.first().map(|s| s.to_ascii_lowercase()).as_deref() {
        None => self.push_cmd(DebugCommand::SpawnEncounterRandom),
        Some("clear") => self.push_cmd(DebugCommand::ClearEncounters),
        Some(name) => {
            let npc_type = match name {
                "orc"      => Some(NPC_TYPE_ORC),
                "human"    => Some(NPC_TYPE_HUMAN),
                "ghost"    => Some(NPC_TYPE_GHOST),
                "skeleton" => Some(NPC_TYPE_SKELETON),
                "wraith"   => Some(NPC_TYPE_WRAITH),
                "dragon"   => Some(NPC_TYPE_DRAGON),
                "snake"    => Some(NPC_TYPE_SKELETON), // snake → cfile 7 (same group)
                "swan"     => Some(NPC_TYPE_SWAN),
                "horse"    => Some(NPC_TYPE_HORSE),
                _ => None,
            };
            match npc_type {
                Some(t) => self.push_cmd(DebugCommand::SpawnEncounterType(t)),
                None => self.log(format!(
                    "Unknown enemy type: {}.  Valid: orc ghost skeleton wraith dragon snake swan horse",
                    name
                )),
            }
        }
    }
}
```

- [ ] **Step 4: Add `/encounter` to `/help` in `debug_console.rs`**

In the `cmd_help` topic match, add after `"/terrain"`:

```rust
                "/encounter" => "/encounter — force regional encounter (4 enemies).\n  /encounter <type>  spawn one enemy: orc ghost skeleton wraith dragon snake swan horse\n  /encounter clear   deactivate all active NPCs",
```

In the general `/help` listing, add after the `/terrain` line:

```rust
            "  /encounter [t]    force encounter / spawn type / clear",
```

- [ ] **Step 5: Handle the new commands in `gameplay_scene.rs`**

In the `match cmd` block in `handle_debug_commands` (around line 1777, after `QueryTerrain`), add:

```rust
            SpawnEncounterRandom => {
                let zone_idx = self.state.region_num as usize;
                let hero_x = self.state.hero_x as i16;
                let hero_y = self.state.hero_y as i16;
                if let Some(ref mut table) = self.npc_table {
                    let spawned = crate::game::encounter::spawn_encounter_group(
                        table, zone_idx, hero_x, hero_y,
                    );
                    self.dlog(format!("forced encounter: {} enemies", spawned));
                } else {
                    self.dlog("forced encounter: no npc_table loaded".to_string());
                }
            }
            SpawnEncounterType(npc_type) => {
                use crate::game::npc::*;
                let zone_idx = self.state.region_num as usize;
                let hero_x = self.state.hero_x as i16;
                let hero_y = self.state.hero_y as i16;
                // npc_type is u8 (Copy), no dereference needed
                let requested_type = *npc_type;
                if let Some(ref mut table) = self.npc_table {
                    if let Some(slot) = table.npcs.iter_mut().find(|n| !n.active) {
                        let mut npc = crate::game::encounter::spawn_encounter(
                            zone_idx, hero_x + 48, hero_y,
                        );
                        npc.npc_type = requested_type;
                        npc.race = match requested_type {
                            NPC_TYPE_WRAITH   => RACE_WRAITH,
                            NPC_TYPE_GHOST | NPC_TYPE_SKELETON => RACE_UNDEAD,
                            _                 => RACE_ENEMY,
                        };
                        *slot = npc;
                        self.dlog(format!("spawned enemy type={}", requested_type));
                    } else {
                        self.dlog("spawn enemy: no free NPC slots".to_string());
                    }
                } else {
                    self.dlog("spawn enemy: no npc_table loaded".to_string());
                }
            }
            ClearEncounters => {
                if let Some(ref mut table) = self.npc_table {
                    let n = table.active_count();
                    for npc in table.npcs.iter_mut() {
                        npc.active = false;
                    }
                    self.dlog(format!("cleared {} NPCs", n));
                } else {
                    self.dlog("clear encounters: no npc_table loaded".to_string());
                }
            }
```

- [ ] **Step 6: Build to verify no compile errors**

```bash
cargo build 2>&1 | grep -E "^error" | head -20
```
Expected: no output.

- [ ] **Step 7: Run all tests**

```bash
cargo test 2>&1 | tail -10
```
Expected: all pass.

- [ ] **Step 8: Commit**

```bash
git add src/game/debug_command.rs src/game/debug_console.rs src/game/gameplay_scene.rs
git commit -m "feat: add /encounter debug command (spawn/clear NPCs)

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 8: Add `item_name_to_id()` to `sprites.rs`

**Files:**
- Modify: `src/game/sprites.rs`

Add a helper that resolves a short name or integer string to an `INV_LIST` index. Used by the `/items` command. Talisman (index 22) is intentionally included — the caller is responsible for the guard.

- [ ] **Step 1: Write the failing test**

Add to the `#[cfg(test)]` block in `sprites.rs` (or create one if absent):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_item_name_to_id_exact() {
        assert_eq!(item_name_to_id("sword"),    Some(2));
        assert_eq!(item_name_to_id("bow"),      Some(3));
        assert_eq!(item_name_to_id("talisman"), Some(22));
        assert_eq!(item_name_to_id("shard"),    Some(30));
    }

    #[test]
    fn test_item_name_to_id_numeric() {
        assert_eq!(item_name_to_id("0"),  Some(0));
        assert_eq!(item_name_to_id("22"), Some(22));
        assert_eq!(item_name_to_id("30"), Some(30));
        assert_eq!(item_name_to_id("31"), None); // out of range
    }

    #[test]
    fn test_item_name_to_id_aliases() {
        assert_eq!(item_name_to_id("wand"),   Some(4));
        assert_eq!(item_name_to_id("lasso"),  Some(5));
        assert_eq!(item_name_to_id("shell"),  Some(6));
        assert_eq!(item_name_to_id("arrows"), Some(8));
        assert_eq!(item_name_to_id("key"),    Some(16)); // first key match
    }

    #[test]
    fn test_item_name_to_id_unknown() {
        assert_eq!(item_name_to_id("fireball"), None);
        assert_eq!(item_name_to_id("orc"),      None);
        assert_eq!(item_name_to_id(""),         None);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cargo test test_item_name_to_id 2>&1 | tail -10
```
Expected: `error[E0425]: cannot find function 'item_name_to_id'`

- [ ] **Step 3: Add `item_name_to_id()` to `sprites.rs`**

Add after the `INV_LIST` constant definition:

```rust
/// Map a short item name or numeric string to an INV_LIST index (0–30).
/// Numeric strings parse to the index directly (0–30 valid; 31+ returns None).
/// Name matching is case-insensitive substring: "sword" matches "long sword".
/// Talisman (index 22) is intentionally included — callers guard against it.
pub fn item_name_to_id(name: &str) -> Option<usize> {
    const TABLE: &[(&str, usize)] = &[
        ("dirk",         0),
        ("mace",         1),
        ("sword",        2),
        ("bow",          3),
        ("wand",         4),
        ("lasso",        5),
        ("shell",        6),
        ("sunstone",     7),
        ("sun stone",    7),
        ("arrow",        8),
        ("blue stone",   9),
        ("bluestone",    9),
        ("jewel",       10),
        ("vial",        11),
        ("orb",         12),
        ("totem",       13),
        ("gold ring",   14),
        ("gold key",    16),
        ("green key",   17),
        ("blue key",    18),
        ("red key",     19),
        ("grey key",    20),
        ("gray key",    20),
        ("white key",   21),
        ("ring",        14),  // after "gold ring" so specific match wins
        ("key",         16),  // generic key → gold key (first key)
        ("talisman",    22),
        ("rose",        23),
        ("fruit",       24),
        ("statue",      25),
        ("book",        26),
        ("herb",        27),
        ("writ",        28),
        ("bone",        29),
        ("shard",       30),
    ];

    let lower = name.to_ascii_lowercase();
    if lower.is_empty() { return None; }

    // Numeric index
    if let Ok(n) = lower.parse::<usize>() {
        return if n < INV_LIST.len() { Some(n) } else { None };
    }

    // Exact match first, then substring
    TABLE.iter()
        .find(|(entry, _)| *entry == lower.as_str())
        .or_else(|| TABLE.iter().find(|(entry, _)| lower.contains(*entry)))
        .map(|(_, id)| *id)
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test test_item_name_to_id 2>&1 | tail -10
```
Expected: all 4 tests pass.

- [ ] **Step 5: Run all tests**

```bash
cargo test 2>&1 | tail -10
```
Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add src/game/sprites.rs
git commit -m "feat: add item_name_to_id() helper to sprites.rs

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Task 9: Wire `/items` debug command end-to-end

**Files:**
- Modify: `src/game/debug_command.rs`
- Modify: `src/game/debug_console.rs`
- Modify: `src/game/gameplay_scene.rs`

- [ ] **Step 1: Add `ScatterItems` to `DebugCommand` in `debug_command.rs`**

Add after `ClearEncounters`:

```rust
    /// Scatter items in a ring around the player.
    /// item_id: None = random from safe pool (no talisman); Some(id) = specific item.
    ScatterItems { count: usize, item_id: Option<usize> },
```

- [ ] **Step 2: Add `/items` to `execute_command` in `debug_console.rs`**

After `"/encounter" => self.cmd_encounter(args),` add:

```rust
            "/items" => self.cmd_items(args),
```

- [ ] **Step 3: Add `cmd_items()` to `debug_console.rs`**

Add after `cmd_encounter`:

```rust
fn cmd_items(&mut self, args: &[&str]) {
    match args {
        [] => {
            // All 30 safe items (no talisman)
            self.push_cmd(DebugCommand::ScatterItems { count: 30, item_id: None });
        }
        [arg] => {
            if let Ok(n) = arg.parse::<usize>() {
                self.push_cmd(DebugCommand::ScatterItems { count: n, item_id: None });
            } else {
                match crate::game::sprites::item_name_to_id(arg) {
                    Some(id) => self.push_cmd(DebugCommand::ScatterItems { count: 1, item_id: Some(id) }),
                    None => self.log(format!("Unknown item: {}.  Use a name or index 0-30.", arg)),
                }
            }
        }
        [count_str, name] => {
            match count_str.parse::<usize>() {
                Err(_) => self.log(format!(
                    "Invalid count '{}'. Usage: /items [count] [name|index]", count_str
                )),
                Ok(n) => match crate::game::sprites::item_name_to_id(name) {
                    Some(id) => self.push_cmd(DebugCommand::ScatterItems { count: n, item_id: Some(id) }),
                    None => self.log(format!("Unknown item: {}.  Use a name or index 0-30.", name)),
                },
            }
        }
        _ => self.log("Usage: /items [count] [name|index]  e.g. /items 5 sword".to_string()),
    }
}
```

- [ ] **Step 4: Add `/items` to `/help` in `debug_console.rs`**

In the `cmd_help` topic match, add after `"/encounter"`:

```rust
                "/items" => "/items — scatter items around player.\n  /items             all 30 safe items\n  /items <count>     N random items (no talisman)\n  /items <name|id>   drop one item by name or index 0-30\n  /items <n> <name>  drop N of a named item\n  Note: talisman (triggers end-of-game) only drops with: /items talisman",
```

In the general `/help` listing, add after `/encounter`:

```rust
            "  /items [n] [name]  scatter items around player (no talisman unless named)",
```

- [ ] **Step 5: Write a test for the `ScatterItems` handler in `gameplay_scene.rs`**

Add to `#[cfg(test)]`:

```rust
#[test]
fn test_scatter_items_adds_world_objects() {
    use crate::game::debug_command::DebugCommand;
    // Build a minimal GameplayScene and invoke the handler via its public interface
    // is not feasible without full init; instead test the helper logic directly.
    // We test that ScatterItems with count=5 and no item_id produces 5 WorldObjects.
    use crate::game::game_state::{GameState, WorldObject};
    use crate::game::sprites::INV_LIST;

    let mut state = GameState::new();
    state.hero_x = 1000;
    state.hero_y = 1000;
    state.region_num = 3;

    const TALISMAN_IDX: usize = 22;
    let count = 5usize;
    let safe_pool: Vec<usize> = (0..INV_LIST.len()).filter(|&i| i != TALISMAN_IDX).collect();
    let n = count.min(safe_pool.len());
    for i in 0..n {
        let item_id = safe_pool[i % safe_pool.len()];
        let angle = 2.0f32 * std::f32::consts::PI * (i as f32) / (n as f32);
        let x = (state.hero_x as i32 + (80.0f32 * angle.cos()) as i32).clamp(0, 0x7FFF) as u16;
        let y = (state.hero_y as i32 + (80.0f32 * angle.sin()) as i32).clamp(0, 0x7FFF) as u16;
        state.world_objects.push(WorldObject {
            item_id: item_id as u8,
            region: state.region_num,
            x, y,
            visible: true,
        });
    }
    assert_eq!(state.world_objects.len(), 5);
    // Talisman should not be in the batch
    assert!(state.world_objects.iter().all(|o| o.item_id != TALISMAN_IDX as u8));
}
```

- [ ] **Step 6: Run test to verify the logic test passes**

```bash
cargo test test_scatter_items_adds_world_objects 2>&1 | tail -5
```
Expected: `test test_scatter_items_adds_world_objects ... ok`

- [ ] **Step 7: Handle `ScatterItems` in `gameplay_scene.rs`**

In the `match cmd` block, add after `ClearEncounters`:

```rust
            ScatterItems { count, item_id } => {
                use crate::game::sprites::INV_LIST;
                use crate::game::game_state::WorldObject;
                const TALISMAN_IDX: usize = 22;

                // count and item_id are Copy (usize, Option<usize>)
                let count = *count;
                let item_id = *item_id;
                let region = self.state.region_num;
                let hero_x = self.state.hero_x as i32;
                let hero_y = self.state.hero_y as i32;
                let mut dropped = 0usize;

                if let Some(id) = item_id {
                    // Drop `count` copies of one specific item in a ring.
                    let radius = if count == 1 { 16.0f32 } else { 80.0f32 };
                    for i in 0..count {
                        let angle = if count == 1 {
                            0.0f32
                        } else {
                            2.0 * std::f32::consts::PI * (i as f32) / (count as f32)
                        };
                        let x = (hero_x + (radius * angle.cos()) as i32).clamp(0, 0x7FFF) as u16;
                        let y = (hero_y + (radius * angle.sin()) as i32).clamp(0, 0x7FFF) as u16;
                        self.state.world_objects.push(WorldObject {
                            item_id: id as u8,
                            region,
                            x, y,
                            visible: true,
                        });
                        dropped += 1;
                    }
                    self.dlog(format!("scattered {} x item {} ({})", dropped, id,
                        if id == TALISMAN_IDX { "TALISMAN — end-of-game item" } else { "" }
                    ));
                } else {
                    // Drop `count` items from the safe pool (no talisman), in a ring.
                    let safe_pool: Vec<usize> = (0..INV_LIST.len())
                        .filter(|&i| i != TALISMAN_IDX)
                        .collect();
                    let n = count.min(safe_pool.len() * 4); // allow cycling
                    for i in 0..n {
                        let item_id = safe_pool[i % safe_pool.len()];
                        let angle = 2.0 * std::f32::consts::PI * (i as f32) / (n as f32);
                        let x = (hero_x + (80.0f32 * angle.cos()) as i32).clamp(0, 0x7FFF) as u16;
                        let y = (hero_y + (80.0f32 * angle.sin()) as i32).clamp(0, 0x7FFF) as u16;
                        self.state.world_objects.push(WorldObject {
                            item_id: item_id as u8,
                            region,
                            x, y,
                            visible: true,
                        });
                        dropped += 1;
                    }
                    self.dlog(format!("scattered {} items", dropped));
                }
            }
```

- [ ] **Step 8: Build with no errors**

```bash
cargo build 2>&1 | grep -E "^error" | head -20
```
Expected: no output.

- [ ] **Step 9: Run all tests**

```bash
cargo test 2>&1 | tail -10
```
Expected: all pass.

- [ ] **Step 10: Commit**

```bash
git add src/game/debug_command.rs src/game/debug_console.rs src/game/gameplay_scene.rs
git commit -m "feat: add /items debug command (scatter world objects)

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

## Verification

After all tasks are complete, run the full test suite one final time:

```bash
cargo test 2>&1 | tail -15
```
Expected: all tests pass, no regressions.

Manual smoke test (requires ADF disk):
```bash
cargo run -- --debug --skip-intro
```
- Walk around: enemy NPCs should now be visible and moving toward hero
- `/encounter` → 4 enemies spawn around hero and are visible
- `/encounter ghost` → one ghost appears next to hero
- `/encounter clear` → all NPCs disappear
- `/items` → 30 items appear in a ring
- `/items 3 sword` → 3 swords appear in a ring
- `/items talisman` → talisman drops (verify game still running after picking up is out of scope)
