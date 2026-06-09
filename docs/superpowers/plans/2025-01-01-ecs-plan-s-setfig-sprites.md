---
title: "Plan S — SetFig Sprite Rendering"
plan: S
status: draft
depends_on: [G]
touches: [src/game/ecs/scene.rs]
---

# ECS Migration Plan S: SetFig Sprite Rendering

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement SetFig (stationary NPC) sprite rendering in `blit_actors_inner()` by adding a third query pass for `(&SetFig, &Position, &SpriteRef)` entities, mirroring the existing Enemy blit pattern and using the SetFig sprite sheets (cfiles 13–17) indexed via `SpriteRef.cfile_idx`.

**Architecture:** SetFigs are stationary NPCs — Wizards, Priests, Guards, Bartenders, and similar townspeople. They carry a `SpriteRef` component populated at spawn time by Plan G's RegionSystem. The render pass reads `SpriteRef.cfile_idx` directly and always blits frame 0 (idle standing pose), since SetFigs are never animated. No STATELIST lookup and no facing logic is needed.

**Prerequisites:** Plans A, B, C, D complete. Plan G complete (RegionSystem spawns SetFig entities with valid SpriteRef).

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

All SetFigs display **frame 0** only — the idle standing pose. There is no walk cycle or animation for stationary NPCs.

The mapping from NPC type to cfile is resolved at spawn time. `spawn_setfig()` receives `cfile_idx: u8` from the RegionSystem and stores it in `SpriteRef { cfile_idx }`. At render time, the pass reads `SpriteRef.cfile_idx` directly without consulting `npc_type_to_cfile()` — that helper is used only by the Enemy pass and explicitly skips SetFig types (the `NPC_TYPE_HUMAN => None // SetFig — skip` branch).

---

## Background: Current `blit_actors_inner()` passes

`blit_actors_inner()` in `src/game/ecs/scene.rs` renders all in-world actors in three conceptual phases:

1. **Hero pass** — queries `(&Hero, &Position, &Facing, Option<&ActorMotion>, Option<&CombatState>, ...)`. Determines the body sprite frame from motion state and facing direction, then blits from the hero's dedicated character sheet.
2. **Enemy pass** — queries `(&Enemy, &Position, &Facing, &EnemyKind, Option<&AiState>)`. Calls `npc_type_to_cfile(npc_type, race)` to resolve the cfile index, then blits the appropriate frame based on AI walk cycle.
3. **SetFig pass** *(this plan)* — queries `(&SetFig, &Position, &SpriteRef)`. Reads `SpriteRef.cfile_idx` to index directly into `sheets[]`, always blits frame 0.

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
let mut setfig_q = world.query::<(&SetFig, &Position, &SpriteRef)>();
for (_, pos, sprite_ref) in setfig_q.iter() {
    let cfile_idx = sprite_ref.cfile_idx as usize;
    let Some(Some(ref sheet)) = sheets.get(cfile_idx) else { continue; };
    let (rel_x, rel_y) = actor_rel_pos(pos.x, pos.y, map_x, map_y);
    if rel_x <= -(SPRITE_W as i32) || rel_x >= fb_w
        || rel_y <= -(SPRITE_H as i32) || rel_y >= fb_h
    {
        continue;
    }
    // SetFigs use frame 0 (idle standing pose — never animated).
    if let Some(fp) = sheet.frame_pixels(0) {
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

- [ ] **Step 3: Verify imports**

Confirm that `SetFig` and `SpriteRef` are already imported in the function's `use` block or at the top of the module. Both are defined in `src/game/ecs/components.rs`. No new imports are needed if the Enemy pass already brings in `src::game::ecs::components::*` or names them explicitly. If they are missing, add them to the existing component import line — do not add a new `use` line.

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

- [ ] **Step 1: Verify `SpriteRef` component structure**

Open `src/game/ecs/components.rs`. Confirm the definition reads:

```rust
pub struct SpriteRef {
    pub cfile_idx: u8,
}
```

If the field name or type differs (e.g. `cfile: usize`), update the render pass in Task 1 Step 2 to match — do not change `components.rs` unless it conflicts with Plan G's documented interface.

- [ ] **Step 2: Verify `SetFig` marker component**

In the same file, confirm:

```rust
pub struct SetFig;
```

is present. If it is missing, this plan cannot proceed — Plan G must define it first. Report the gap; do not add the struct here.

- [ ] **Step 3: Verify `spawn_setfig()` populates `SpriteRef`**

Open `src/game/ecs/spawn.rs`. Find `spawn_setfig()`. Confirm its signature accepts `cfile_idx: u8` and that the entity builder includes:

```rust
.with(SpriteRef { cfile_idx })
```

If `SpriteRef` is missing from the spawned entity, the query pass in Task 1 will produce no results at runtime, but will not panic. Report the gap to Plan G; do not add `SpriteRef` insertion here as that is Plan G's responsibility.

- [ ] **Step 4: Record findings**

No commit needed for this task. Note any discrepancies in the PR description or as a comment in the implementation PR.

---

## Task 3: Add unit tests

**Files:**
- Modify: `src/game/ecs/scene.rs` (or a dedicated `#[cfg(test)]` block within it)

Three targeted tests covering spawn correctness, query iteration, and offscreen culling. Tests use `hecs::World` directly — no SDL context required.

- [ ] **Step 1: `setfig_has_correct_cfile_idx`**

Spawns a SetFig entity with `cfile_idx = 13` and asserts that the component value round-trips correctly:

```rust
#[test]
fn setfig_has_correct_cfile_idx() {
    use crate::game::ecs::components::{SetFig, Position, SpriteRef};
    let mut world = hecs::World::new();
    world.spawn((
        SetFig,
        Position { x: 100.0, y: 200.0 },
        SpriteRef { cfile_idx: 13 },
    ));
    let mut q = world.query::<(&SetFig, &SpriteRef)>();
    let results: Vec<_> = q.iter().collect();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].1.1.cfile_idx, 13);
}
```

- [ ] **Step 2: `setfig_query_finds_entity`**

Spawns exactly one SetFig entity and asserts the query returns exactly one result. Spawns an Enemy entity alongside it to confirm the SetFig query does not match Enemy.

```rust
#[test]
fn setfig_query_finds_entity() {
    use crate::game::ecs::components::{SetFig, Enemy, Position, SpriteRef, EnemyKind};
    let mut world = hecs::World::new();
    world.spawn((
        SetFig,
        Position { x: 10.0, y: 10.0 },
        SpriteRef { cfile_idx: 14 },
    ));
    world.spawn((
        Enemy,
        Position { x: 20.0, y: 20.0 },
        EnemyKind { npc_type: 0, race: 0 },
    ));
    let mut setfig_q = world.query::<(&SetFig, &Position, &SpriteRef)>();
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
| G | RegionSystem spawns SetFig entities with valid `SpriteRef` |
