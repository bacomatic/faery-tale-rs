---
title: "Plan T — Weapon Overlays"
plan: T
status: draft
depends_on: [N]
touches: [src/game/ecs/scene.rs]
---

# ECS Migration Plan T: Weapon Overlays

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Render weapon sprite overlays on top of the hero body sprite during gameplay. Weapons are drawn from cfile 3 (objects sprite sheet, 16 px frame height) and positioned using offsets from the `STATELIST` array (melee weapons and bow during fight) or `BOW_X`/`BOW_Y` tables (bow during walk cycles).

**Architecture:** The weapon overlay is a second blit in the Hero render pass, drawn immediately after the hero body sprite. The body frame index already computed by the Hero pass indexes into `STATELIST` to retrieve `wpn_no` (weapon overlay frame) and `wpn_x`/`wpn_y` (pixel offsets). For the bow weapon during non-fight movement, `BOW_X[cycle % 32]`/`BOW_Y[cycle % 32]` replace the STATELIST offsets. If `weapon == 0` or `wpn_no == 0`, no overlay is drawn.

**Prerequisites:** Plans A, B, C, D complete. Plan N complete (CombatState.weapon field populated from inventory).

**Tech Stack:** Rust 2021, `hecs = "0.11"`, SDL3.

---

## File map

| File | Change |
|------|--------|
| `src/game/ecs/scene.rs` | Add weapon extraction, STATELIST lookup, and weapon blit pass to `blit_actors_inner()` |

---

## Background: STATELIST structure

`STATELIST` is defined in `src/game/sprites.rs` (lines 715–1250) as a 87-entry array of `StatEntry`:

```rust
pub struct StatEntry {
    pub figure: u8,   // body sprite frame index
    pub wpn_no: u8,   // weapon overlay frame index (0 = no overlay)
    pub wpn_x:  i8,   // weapon sprite X offset in pixels
    pub wpn_y:  i8,   // weapon sprite Y offset in pixels
}
```

The `figure` field is used by the Hero pass to select the body frame. The weapon overlay fields (`wpn_no`, `wpn_x`, `wpn_y`) are added by this plan.

Index groups within STATELIST:

| Indices | Animation group |
|---------|----------------|
| 0–7 | South walk |
| 8–15 | West walk |
| 16–23 | North walk |
| 24–31 | East walk |
| 32–43 | South fight |
| 44–55 | West fight |
| 56–67 | North fight |
| 68–79 | East fight |
| 80–86 | Death / sink / sleep |

All 87 entries that correspond to active fight or walk frames carry non-zero `wpn_no` when a weapon is equipped. The death/sink/sleep group (80–86) carries `wpn_no == 0` for all entries — no overlay is drawn during those states.

---

## Background: cfile 3 — objects sprite sheet

cfile 3 is the objects sheet, loaded as `SpriteSheets::object_sprites` in `src/game/ecs/resources.rs`. Its frame height is **16 px** — half the character sprite height (32 px). This distinction matters: `blit_sprite_to_framebuf` receives `frame_h` as a parameter, and the weapon pass must pass `obj_sheet.frame_h` (16), not `SPRITE_H` (32).

Relevant frame indices within cfile 3:

| Frame | Weapon |
|-------|--------|
| 8 | Sword |
| 9 | Mace |
| 10 | Dirk |
| 11 | Bow (walk cycle frame) |
| 12–15 | Fight frames (sword/mace/dirk/bow fight overlays) |

The `wpn_no` values in STATELIST already reference these indices directly.

---

## Background: BOW_X / BOW_Y tables

`BOW_X[32]` and `BOW_Y[32]` are defined in `src/game/sprites.rs` (lines 1261–1275). They provide per-frame X/Y pixel offsets for the bow sprite during walk cycles. The index is `cycle % 32`, where `cycle` is the hero's current walk animation cycle counter from `ActorMotion`.

**When to use BOW_X/BOW_Y vs STATELIST:**

| Condition | Offset source | Frame |
|-----------|---------------|-------|
| `weapon == 4` AND `!is_fighting` | `BOW_X[cycle % 32]`, `BOW_Y[cycle % 32]` | 11 |
| All other cases | `STATELIST[frame].wpn_x`, `STATELIST[frame].wpn_y` | `STATELIST[frame].wpn_no` |

The bow during fight uses STATELIST like any other weapon — the special bow table only applies to walk cycles. `weapon == 4` is the bow's item identifier as encoded in `CombatState.weapon`.

---

## Background: weapon value source

`CombatState.weapon: u8` is populated by Plan N from the hero's active inventory slot. Values:

| Value | Weapon |
|-------|--------|
| 0 | No weapon (unarmed) |
| 1 | Sword |
| 2 | Mace |
| 3 | Dirk |
| 4 | Bow |

When `weapon == 0`, the Hero pass must skip the overlay blit entirely — `STATELIST[frame].wpn_no` may be non-zero for unarmed walking poses in some entries. The weapon value from `CombatState` is the authoritative gate.

---

## Task 1: Extract weapon state from the Hero pass

**Files:**
- Modify: `src/game/ecs/scene.rs`

The Hero pass already queries `Option<&CombatState>`. This task extracts `weapon` and `is_fighting` from that optional reference so they are available for the overlay logic added in Task 2.

- [ ] **Step 1: Locate the Hero pass body frame determination**

Open `src/game/ecs/scene.rs`. Find the Hero query pass. After the body frame index (`frame: usize`) is computed from the motion state and facing direction — approximately line 809 — add the following two lines:

```rust
let weapon: u8 = combat_opt.map(|c| c.weapon).unwrap_or(0);
let is_fighting: bool = matches!(combat_state, Some(ActorState::Fighting(_)));
```

Notes:
- `combat_opt` is the `Option<&CombatState>` already bound earlier in the same loop iteration.
- `combat_state` is the `Option<ActorState>` (or equivalent discriminant) already extracted for frame selection — use whatever binding the Hero pass already uses to test the fighting state. If the pass tests `combat_opt.map(|c| c.state)`, mirror that exact pattern.
- `ActorState::Fighting(_)` — verify the exact variant name against `src/game/ecs/components.rs`. If it differs (e.g. `ActorState::Fight`), use the correct name.

- [ ] **Step 2: Compile check**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no new errors from these two lines.

---

## Task 2: Determine weapon frame and offsets

**Files:**
- Modify: `src/game/ecs/scene.rs`

This task adds the STATELIST/BOW lookup immediately after the body blit call in the Hero pass (approximately line 812), before the loop's closing brace.

- [ ] **Step 1: Add the offset determination block**

```rust
let (wpn_frame, wpn_dx, wpn_dy) = if weapon == 4 && !is_fighting {
    // Bow during walk: use per-frame bob tables.
    let cf = cycle % 32;
    (11usize, BOW_X[cf] as i32, BOW_Y[cf] as i32)
} else {
    // All other weapons (including bow during fight): use STATELIST.
    let entry = STATELIST.get(frame).unwrap_or(&STATELIST[0]);
    if entry.wpn_no == 0 {
        (0usize, 0i32, 0i32)
    } else {
        (entry.wpn_no as usize, entry.wpn_x as i32, entry.wpn_y as i32)
    }
};
```

Notes:
- `cycle` is the hero's walk animation cycle counter. It is already available in the Hero pass as part of `ActorMotion` — use the same binding already used for walk frame selection. If `ActorMotion` stores it as `cycle: u8`, cast with `cycle as usize`.
- `frame` is the STATELIST index already computed for the body sprite. The weapon overlay uses the same index — this is intentional.
- `STATELIST` and `BOW_X`/`BOW_Y` are not yet imported in this function — imports are added in Task 4.

- [ ] **Step 2: Note on `unwrap_or(&STATELIST[0])`**

`STATELIST[0]` is the South-walk frame 0 entry. Using it as the fallback is safe because out-of-range frame indices only occur during death/sink/sleep (indices 80–86), and those entries all have `wpn_no == 0`, so the fallback path (returning `(0, 0, 0)`) is reached before the `unwrap_or` matters in practice. The guard is defensive only.

---

## Task 3: Blit the weapon overlay sprite

**Files:**
- Modify: `src/game/ecs/scene.rs`

This task adds the actual blit call, immediately after the block added in Task 2.

- [ ] **Step 1: Add the weapon blit**

```rust
if weapon > 0 && wpn_frame > 0 {
    if let Some(Some(ref obj_sheet)) = sheets.get(3) {
        if let Some(wp) = obj_sheet.frame_pixels(wpn_frame) {
            blit_sprite_to_framebuf(
                wp,
                rel_x + wpn_dx,
                rel_y + wpn_dy,
                obj_sheet.frame_h,
                framebuf,
                fb_w,
                fb_h,
            );
        }
    }
}
```

Notes:
- The double guard (`weapon > 0 && wpn_frame > 0`) ensures no blit occurs for unarmed heroes or STATELIST entries that carry no overlay. Both conditions are checked: `weapon` gates on the inventory state; `wpn_frame` gates on the STATELIST entry.
- `sheets.get(3)` indexes cfile 3. The outer `Option` is the slice bounds check; the inner `Option` is whether that sheet was successfully loaded. Both must be `Some` — the same double-option pattern used by the Enemy pass.
- `obj_sheet.frame_h` is passed as the height parameter (16 px), **not** `SPRITE_H` (32 px). This is the critical distinction from the character sprite blit above it.
- `rel_x + wpn_dx` / `rel_y + wpn_dy` — `rel_x` and `rel_y` are already in scope from the hero body blit. The offsets shift the weapon relative to the hero's screen position.
- No additional bounds check is needed for the weapon overlay: if the hero body was not culled, the weapon (a smaller or equal sprite offset by a few pixels) is close enough to also render. Clipping at the framebuffer edge is handled inside `blit_sprite_to_framebuf`.

- [ ] **Step 2: Compile check**

```bash
cargo check 2>&1 | grep "^error"
```

Expected errors at this stage: `BOW_X`, `BOW_Y`, and `STATELIST` not in scope. These are resolved in Task 4.

---

## Task 4: Extend imports

**Files:**
- Modify: `src/game/ecs/scene.rs`

- [ ] **Step 1: Find the existing sprites import**

Locate the `use crate::game::sprites::` line inside (or near) `blit_actors_inner()`. It currently imports at minimum `SPRITE_H` and `SPRITE_W`. It may already import `STATELIST`.

- [ ] **Step 2: Add missing imports**

Extend the existing import to include `STATELIST`, `BOW_X`, and `BOW_Y`:

```rust
use crate::game::sprites::{SPRITE_H, SPRITE_W, STATELIST, BOW_X, BOW_Y};
```

Do not add a new `use` line if one already exists for this module — edit the existing line in place to add only the missing names.

- [ ] **Step 3: Full compile**

```bash
cargo build 2>&1 | grep "^error"
```

Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/game/ecs/scene.rs
git commit -m "feat(ecs): weapon overlay blit in hero render pass using STATELIST and BOW_X/BOW_Y"
```

---

## Task 5: Manual gameplay verification

Automated unit tests for pixel-level rendering require a framebuffer harness not yet present in the test suite. Verification is manual. Run the game with `--ecs` and exercise each case below.

- [ ] **Hero with no weapon equipped**
  - Walk in all four cardinal directions.
  - Expected: no weapon overlay visible on any frame. Hero body renders normally.

- [ ] **Hero with sword (weapon == 1)**
  - Walk South, then enter combat.
  - Expected: sword overlay appears at correct STATELIST offsets during walk frames. Fight frames show sword at fight-frame offsets. Overlay does not "float" or appear at (0, 0) relative to screen origin.

- [ ] **Hero with mace (weapon == 2) and dirk (weapon == 3)**
  - Walk and fight with each.
  - Expected: correct overlay frame (9 and 10 respectively) at correct offsets.

- [ ] **Hero with bow (weapon == 4) — walk cycle**
  - Walk in all four cardinal directions.
  - Expected: bow bobs visibly as the hero walks, driven by `BOW_X`/`BOW_Y`. The bob is subtle — one or two pixels per frame — not a large jump.

- [ ] **Hero with bow (weapon == 4) — fight**
  - Enter combat with bow equipped.
  - Expected: bow uses STATELIST offsets during fight frames (same as melee weapons), not BOW_X/BOW_Y.

- [ ] **Screen edge clipping**
  - Walk the hero to the left and top edges of the viewport so the body sprite is partially clipped.
  - Expected: weapon overlay clips cleanly at the same edges with no artifacts (black bars, garbage pixels, or panics).

- [ ] **Death frames**
  - Allow the hero to die (god mode off).
  - Expected: no weapon overlay during death animation frames (STATELIST indices 80–86 all have `wpn_no == 0`).

---

## Completion check

```bash
cargo build 2>&1 | grep "^error"
cargo test 2>&1 | grep "^test result"
```

Both succeed. Manual gameplay confirms weapon overlays are visible, correctly positioned, and well-behaved at screen edges and during all animation states.

---

## Spec references

- `docs/spec/characters-animation.md` §8.4 — STATELIST structure and weapon overlay fields (`wpn_no`, `wpn_x`, `wpn_y`)
- `docs/spec/display-rendering.md` §1.3 — rendering pipeline phase order; §5.2 — sprite format and frame height by cfile
- `docs/spec/inventory-items.md` §14.2 — weapon item identifiers (sword=1, mace=2, dirk=3, bow=4)

---

## Dependencies

| Plan | Reason |
|------|--------|
| A | ECS component definitions in place |
| B | System infrastructure in place |
| C | System schedule wired |
| D | `EcsScene` and `blit_actors_inner()` exist |
| N | `CombatState.weapon` populated from inventory |
