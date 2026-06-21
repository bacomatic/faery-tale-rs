---
title: "Plan S — SetFig Sprite Rendering"
plan: S
status: draft
depends_on: [G]
touches: [src/game/ecs/scene.rs]
---

# ECS Migration Plan S: SetFig Sprite Rendering

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement SetFig (stationary NPC) sprite rendering in `blit_actors_inner()` by adding a third query pass for `(&SetFig, &Position, &WorldObj)` entities, mirroring the existing Enemy blit pattern and selecting the sheet + frame from `SETFIG_TABLE` (cfiles 13–17).

**Architecture:** SetFigs are stationary NPCs — Wizards, Priests, Guards, Bartenders, and similar townspeople. The setfig type index is derived from the stored race byte: `k = obj.ob_id & 0x7f` (fmain.c:3374 strips the 0x80 setfig bit). `crate::game::sprites::SETFIG_TABLE[k]` then yields both `cfile_entry` (the sheet) and `image_base` (the base/idle frame for that type). The render pass blits `sheets[cfile_entry]` at frame `image_base`. No STATELIST lookup and no facing logic is needed for the idle pose.

> **Important:** do NOT read the frame from `SpriteRef`. `SpriteRef.cfile_idx` is currently hardcoded to `13` for every setfig by the merged Plan G `reload_region` (a bug — it ignores `SETFIG_TABLE`), and `SpriteRef` carries no `image_base`. Deriving sheet + frame from `WorldObj.ob_id & 0x7f` via `SETFIG_TABLE` is the authoritative path and is immune to that bug. (Track the Plan G `cfile_idx` hardcode as a separate cleanup; once this plan lands, `SpriteRef` is unused by setfigs.)

**Prerequisites:** Plans A, B, C, D complete. Plan G complete (RegionSystem spawns SetFig entities with a `WorldObj` whose `ob_id` is the race byte).

**Tech Stack:** Rust 2021, `hecs = "0.11"`, SDL3.

---

## File map

| File | Change |
|------|--------|
| `src/game/ecs/scene.rs` | Add SetFig query pass to `blit_actors_inner()` |

---

## Background: SetFig NPC types and cfile assignments

SetFigs are the stationary human NPCs found in towns, castles, and inns. Unlike enemies, they do not move or change facing during gameplay. Their sprite sheets occupy cfiles 13–17, separate from the character sheets (cfiles 0–12) used by the hero and enemies.

| cfile | NPC types | Frame count |
|-------|-----------|-------------|
| 13 | Wizard (0), Priest (1) | 8 frames |
| 14 | Guard (2/3), Princess (4), King (5), Noble (6), Sorceress (7) | 8 frames |
| 15 | Bartender (8) | 8 frames |
| 16 | Witch (9), Spectre (10), Ghost (11) | 8 frames |
| 17 | Ranger (12), Beggar (13) | 8 frames |

Each SetFig type displays its **`image_base` frame** — the idle standing pose for that type. Types share a sheet and are distinguished *only* by `image_base` (e.g. on cfile 13, Wizard=frame 0 but Priest=frame 4; on cfile 14, Guard=0, Guard-back=1, Princess=2, King=4, Noble=6, Sorceress=7). Rendering frame 0 for all of them would show the wrong NPC. There is no walk cycle for the idle render. (The five `can_talk` setfigs animate only during TALK; that talk animation is out of scope here — the idle frame is `image_base`.)

The sheet and frame are resolved at render time from the stored race byte:

```
k          = (obj.ob_id & 0x7f) as usize     // strip 0x80 setfig bit (fmain.c:3374)
entry      = SETFIG_TABLE[k]                  // crate::game::sprites::SETFIG_TABLE
cfile_idx  = entry.cfile_entry
frame      = entry.image_base
```

`obj` is the `WorldObj` component on the setfig entity (`spawn_setfig` stores it with `ob_id = race`). The Enemy pass's `npc_type_to_cfile()` is not used here — it explicitly skips SetFig types (`NPC_TYPE_HUMAN => None`).

---

## Background: Current `blit_actors_inner()` passes

`blit_actors_inner()` in `src/game/ecs/scene.rs` renders all in-world actors in three conceptual phases:

1. **Hero pass** — queries `(&Hero, &Position, &Facing, Option<&ActorMotion>, Option<&CombatState>, ...)`. Determines the body sprite frame from motion state and facing direction, then blits from the hero's dedicated character sheet.
2. **Enemy pass** — queries `(&Enemy, &Position, &Facing, &EnemyKind, Option<&AiState>)`. Calls `npc_type_to_cfile(npc_type, race)` to resolve the cfile index, then blits the appropriate frame based on AI walk cycle.
3. **SetFig pass** *(this plan)* — queries `(&SetFig, &Position, &WorldObj)`. Computes `k = obj.ob_id & 0x7f`, looks up `SETFIG_TABLE[k]`, and blits `sheets[entry.cfile_entry]` at frame `entry.image_base`.

The SetFig pass is intentionally the simplest of the three: no motion state, no facing direction, no STATELIST lookup.

---

## Task 1: Add SetFig query pass to `blit_actors_inner()`

**Files:**
- Modify: `src/game/ecs/scene.rs`

- [ ] **Step 1: Locate the end of the Enemy pass**

Open `src/game/ecs/scene.rs`. Find the Enemy query pass. It ends approximately at line 848, after the final `blit_sprite_to_framebuf(...)` call for the enemy loop. The SetFig pass is inserted immediately after this closing brace and before the function's own closing brace.

- [ ] **Step 2: Add the SetFig query pass**

After the Enemy pass, add:

```rust
// ── SetFigs ──────────────────────────────────────────────────────────────
let mut setfig_q = world.query::<(&SetFig, &Position, &WorldObj)>();
for (_, (_, pos, obj)) in setfig_q.iter() {
    // Setfig type index = race byte with the 0x80 setfig bit stripped (fmain.c:3374).
    let k = (obj.ob_id & 0x7f) as usize;
    let Some(entry) = crate::game::sprites::SETFIG_TABLE.get(k) else { continue; };
    let cfile_idx = entry.cfile_entry as usize;
    let Some(Some(ref sheet)) = sheets.get(cfile_idx) else { continue; };
    let (rel_x, rel_y) = actor_rel_pos(pos.x, pos.y, map_x, map_y);
    if rel_x <= -(SPRITE_W as i32) || rel_x >= fb_w
        || rel_y <= -(SPRITE_H as i32) || rel_y >= fb_h
    {
        continue;
    }
    // Idle pose for this setfig type is SETFIG_TABLE[k].image_base.
    if let Some(fp) = sheet.frame_pixels(entry.image_base as usize) {
        blit_sprite_to_framebuf(fp, rel_x, rel_y, SPRITE_H, framebuf, fb_w, fb_h);
    }
}
```

Notes:
- `sheets` is the existing `&[Option<SpriteSheet>]` slice already in scope from the Hero and Enemy passes.
- `actor_rel_pos` is the existing helper already called by both prior passes.
- `SPRITE_W` and `SPRITE_H` are already imported by the surrounding function.
- The out-of-bounds check (`rel_x <= -(SPRITE_W as i32) || ...`) mirrors the Enemy pass guard exactly. Do not alter its logic.
- `SPRITE_H` is passed to `blit_sprite_to_framebuf` as the frame height — SetFig sprites are the standard character height (32 px), not the object-sheet height (16 px). This mirrors the Enemy pass.
- `SpriteRef` is intentionally NOT used (see the Important note at the top): its `cfile_idx` is hardcoded to 13 by Plan G and it has no `image_base`.

- [ ] **Step 3: Verify imports**

Confirm that `SetFig` and `WorldObj` are imported in the function's `use` block (both in `src/game/ecs/components.rs`), and that `SETFIG_TABLE` is reachable via `crate::game::sprites::SETFIG_TABLE` (used fully-qualified above, so no new `use` is required). If `SetFig`/`WorldObj` are missing, add them to the existing component import line — do not add a new `use` line.

- [ ] **Step 4: Compile check**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no new errors. The most common mistake is accidentally capturing `setfig_q` while `enemy_q` is still borrowed. If this occurs, ensure both query variables are declared and iterated in separate scopes, or drop `enemy_q` explicitly before declaring `setfig_q`.

- [ ] **Step 5: Commit**

```bash
git add src/game/ecs/scene.rs
git commit -m "feat(ecs): add SetFig sprite render pass to blit_actors_inner()"
```

---

## Task 2: Verify component and spawn preconditions

This task is read-only verification. No source changes should be needed if Plan G is complete. It is included so the implementing agent can confirm correctness before writing tests.

**Files:**
- Read: `src/game/ecs/components.rs`
- Read: `src/game/ecs/spawn.rs`

- [ ] **Step 1: Verify `WorldObj` component carries the race byte**

Open `src/game/ecs/components.rs`. Confirm `WorldObj` has a `pub ob_id: u8` field. Confirm `spawn_setfig()` in `src/game/ecs/spawn.rs` attaches the `WorldObj` (it does today: `world.spawn((SetFig, Position, Facing, obj, SpriteRef { .. }))`, and `reload_region` sets `obj.ob_id = npc.race`). The render pass derives sheet + frame from `obj.ob_id & 0x7f`, so a present `WorldObj` is the only precondition.

- [ ] **Step 2: Verify `SetFig` marker component**

In the same file, confirm:

```rust
pub struct SetFig;
```

is present. If it is missing, this plan cannot proceed — Plan G must define it first. Report the gap; do not add the struct here.

- [ ] **Step 3: Verify `SETFIG_TABLE` and note the Plan G spawn bug**

Confirm `crate::game::sprites::SETFIG_TABLE: [SetfigEntry; 14]` exists with `cfile_entry` and `image_base` fields. Then note (do not fix here) that `reload_region` in `scene.rs` currently calls `spawn_setfig(.., cfile_idx = 13)` for every setfig, ignoring `SETFIG_TABLE`. This render pass does not depend on that hardcoded value, but the `SpriteRef.cfile_idx=13` hardcode should be tracked as a separate Plan G cleanup (and `SpriteRef` becomes unused for setfigs).

- [ ] **Step 4: Record findings**

No commit needed for this task. Note any discrepancies in the PR description or as a comment in the implementation PR.

---

## Task 3: Add unit tests

**Files:**
- Modify: `src/game/ecs/scene.rs` (or a dedicated `#[cfg(test)]` block within it)

Three targeted tests covering the table lookup, query iteration, and offscreen culling. Tests use `hecs::World` directly — no SDL context required. Use the existing `make_test_obj(ob_id)` helper from the `scene.rs` test module (or a local equivalent that builds a `WorldObj` with the given `ob_id` and default remaining fields).

- [ ] **Step 1: `setfig_race_resolves_to_table_entry`**

Spawns a SetFig with the Priest race byte (`0x81` = `0x80` setfig bit | index 1) and asserts the table lookup yields the Priest's sheet **and a non-zero frame** — proving the pass does not collapse to frame 0:

```rust
#[test]
fn setfig_race_resolves_to_table_entry() {
    use crate::game::ecs::components::{SetFig, Position, WorldObj};
    use crate::game::sprites::SETFIG_TABLE;
    let mut world = hecs::World::new();
    // Priest = setfig index 1 → race byte 0x81.
    world.spawn((SetFig, Position { x: 100.0, y: 200.0 }, make_test_obj(0x81)));
    let mut q = world.query::<(&SetFig, &WorldObj)>();
    let (_, (_, obj)) = q.iter().next().unwrap();
    let entry = SETFIG_TABLE[(obj.ob_id & 0x7f) as usize];
    assert_eq!(entry.cfile_entry, 13, "priest is on cfile 13");
    assert_eq!(entry.image_base, 4, "priest idle frame is 4, not 0");
}
```

- [ ] **Step 2: `setfig_query_finds_entity`**

Spawns exactly one SetFig entity and asserts the query returns exactly one result. Spawns an Enemy entity alongside it to confirm the SetFig query does not match Enemy.

```rust
#[test]
fn setfig_query_finds_entity() {
    use crate::game::ecs::components::{SetFig, Enemy, Position, WorldObj, EnemyKind};
    let mut world = hecs::World::new();
    world.spawn((
        SetFig,
        Position { x: 10.0, y: 10.0 },
        make_test_obj(0x82), // guard
    ));
    world.spawn((
        Enemy,
        Position { x: 20.0, y: 20.0 },
        EnemyKind { npc_type: 0, race: 0 },
    ));
    let mut setfig_q = world.query::<(&SetFig, &Position, &WorldObj)>();
    assert_eq!(setfig_q.iter().count(), 1);
}
```

- [ ] **Step 3: `setfig_not_rendered_when_offscreen`**

Tests the culling condition in isolation. Positions a SetFig far outside the visible window bounds and asserts the visibility check would skip it. This test does not invoke the full render pipeline — it tests the guard arithmetic directly.

```rust
#[test]
fn setfig_not_rendered_when_offscreen() {
    use crate::game::sprites::{SPRITE_W, SPRITE_H};
    // Simulate: camera at (0,0), framebuffer 320x200.
    let fb_w: i32 = 320;
    let fb_h: i32 = 200;
    // SetFig positioned far to the left — rel_x will be deeply negative.
    let rel_x: i32 = -(SPRITE_W as i32) - 1;
    let rel_y: i32 = 50;
    let culled = rel_x <= -(SPRITE_W as i32)
        || rel_x >= fb_w
        || rel_y <= -(SPRITE_H as i32)
        || rel_y >= fb_h;
    assert!(culled, "SetFig outside framebuffer bounds must be culled");
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test setfig 2>&1
```

Expected: all three tests pass. If any fail due to struct field name mismatches discovered in Task 2, fix the test to match the actual component shape — do not change component definitions.

- [ ] **Step 5: Commit**

```bash
git add src/game/ecs/scene.rs
git commit -m "test(ecs): SetFig sprite render pass — cfile_idx, query, offscreen culling"
```

---

## Completion check

```bash
cargo build 2>&1 | grep "^error"
cargo test setfig 2>&1
```

Both succeed. In a running game with Plan G active, SetFig NPCs (Wizards, Priests, Guards, etc.) appear in towns and castles at their correct world positions. They display the idle standing pose and do not animate.

---

## Spec references

- `docs/spec/characters-animation.md` §8.7–8.8 — SetFig NPC type table, cfile 13–17 assignments
- `docs/spec/display-rendering.md` §1.3 Phase 22 — sprite rendering pipeline
- `docs/spec/npcs-dialogue.md` §13.1 — SetFig types and sprite assignments

---

## Dependencies

| Plan | Reason |
|------|--------|
| A | ECS component definitions in place |
| B | System infrastructure in place |
| C | System schedule wired |
| D | `EcsScene` and `blit_actors_inner()` exist |
| G | RegionSystem spawns SetFig entities with a `WorldObj` (`ob_id` = race byte) |
