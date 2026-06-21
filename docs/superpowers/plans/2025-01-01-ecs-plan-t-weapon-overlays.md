---
title: "Plan T — Weapon Overlays"
plan: T
status: draft
depends_on: [N]
touches: [src/game/ecs/scene.rs]
---

# ECS Migration Plan T: Weapon Overlays

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Render weapon sprite overlays on the hero during gameplay, layered either behind or on top of the body depending on facing. Weapons are drawn from cfile 3 (objects sprite sheet, 16 px frame height) and positioned using offsets from the `STATELIST` array (melee weapons and bow during fight) or `BOW_X`/`BOW_Y` tables (bow during walk cycles).

**Architecture:** The weapon overlay is a second blit in the Hero render pass. It is a direct port of `select_atype_inum`'s weapon-pass branch (`reference/logic/sprite-rendering.md`, fmain.c:2420-2446) **plus** the facing-dependent draw order from `resolve_pass_params` (fmain.c:2400-2409). The body frame index already computed by the Hero pass (call it `inum`, 0–86) drives the offset and the OBJECTS-sheet frame:

- **Offset:** bow-while-walking (`weapon == 4 && inum < 32`) uses `BOW_X[inum]`/`BOW_Y[inum]`; every other case uses `STATELIST[inum].wpn_x`/`wpn_y`.
- **Frame:** hand weapons use `STATELIST[inum].wpn_no + k`, where `k` is a per-class base offset (dirk 64, mace 32, sword 48, bow 0); the bow-while-walking case uses a special derived frame (`30`/`0x53`/`0x51` by direction); the wand uses `facing + 103`.
- **Gate:** the overlay is drawn only when `0 < weapon < 8` and the hero is alive (not in the death/sink/sleep states — i.e. body frame `inum < 80`). It is NOT gated on `wpn_no == 0`.
- **Draw order (facing-dependent — do NOT always draw after the body):** the weapon is drawn *behind* the body for some facings and *on top* for others.
  - General case (hand weapons; bow shooting): weapon is behind the body when `(facing - 2) & 4 != 0`, otherwise on top.
  - Bow while walking (`weapon == 4 && inum < 32`): weapon is behind the body when `(facing & 4) == 0`, otherwise on top.
  - The `Direction` enum numbering matches the original `DIR_*` (NW=0,N=1,NE=2,E=3,SE=4,S=5,SW=6,W=7), so these bit tests port verbatim.

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

The death/sink/sleep group (80–86) is **not** uniformly `wpn_no == 0` — only the death frames 80–82 are 0; the sink/oscillate/sleep frames 83–86 carry `wpn_no == 10`. Therefore the overlay must NOT be gated on `wpn_no`. Instead, suppress the overlay whenever the hero is in a death/sink/sleep state, i.e. body frame `inum >= 80` (mirrors the original `needs_weapon_pass` gate: alive `state < STATE_DEAD(15)` paints, dead states do not).

---

## Background: cfile 3 — objects sprite sheet

cfile 3 is the objects sheet, loaded as `SpriteSheets::object_sprites` in `src/game/ecs/resources.rs`. Its frame height is **16 px** — half the character sprite height (32 px). This distinction matters: `blit_sprite_to_framebuf` receives `frame_h` as a parameter, and the weapon pass must pass `obj_sheet.frame_h` (16), not `SPRITE_H` (32).

The OBJECTS-sheet frame for a weapon overlay is **not** a fixed per-weapon index. It is computed per body frame as `STATELIST[inum].wpn_no + k`, where `k` is the per-weapon-class base offset (fmain.c:2440-2444):

| Weapon code | `k` base offset |
|-------------|-----------------|
| 1 Dirk  | 64 |
| 2 Mace  | 32 |
| 3 Sword | 48 |
| 4 Bow (shooting) | 0 |

The bow *while walking* and the wand take separate frame-derivation paths (see below); they do **not** use `wpn_no + k`.

---

## Background: BOW_X / BOW_Y tables

`BOW_X[32]` and `BOW_Y[32]` are defined in `src/game/sprites.rs` (lines 1261–1275). They provide per-frame X/Y pixel offsets for the bow sprite. The index is the **body frame index `inum`** (the same STATELIST index used for the body), gated on `inum < 32` (the walk groups) — NOT `cycle % 32`. (fmain.c:2422-2423 indexes `bow_x[inum]`.)

**Bow-while-walking frame derivation** (fmain.c:2429-2433): when `weapon == 4 && inum < 32`, the OBJECTS frame is derived from the direction group `q = inum / 8`:

| `q` (direction group) | Frame |
|-----------------------|-------|
| `q & 1 != 0` (west=1, east=3) | `30` (bow drawn east-west) |
| else if `q & 2 != 0` (north=2) | `0x53` (bow drawn north) |
| else (south=0) | `0x51` (bow drawn south) |

**When to use BOW_X/BOW_Y vs STATELIST:**

| Condition | Offset source | Frame |
|-----------|---------------|-------|
| `weapon == 4` AND `inum < 32` (walking) | `BOW_X[inum]`, `BOW_Y[inum]` | `30` / `0x53` / `0x51` (see table above) |
| `weapon == 5` (wand) | `STATELIST[inum].wpn_x/.wpn_y` (with NE y−6) | `facing + 103` |
| All other cases (hand weapons; bow shooting `inum >= 32`) | `STATELIST[inum].wpn_x`, `STATELIST[inum].wpn_y` | `STATELIST[inum].wpn_no + k` |

The bow during fight/shoot (`inum >= 32`) uses the `wpn_no + k` path with `k = 0`. `weapon == 4` is the bow's weapon code in `CombatState.weapon`.

---

## Background: weapon value source

`CombatState.weapon: u8` is populated by Plan N from the hero's active inventory slot. Values are weapon *codes* (`characters-animation.md §8.1`), which differ from inventory item indices:

| Value | Weapon |
|-------|--------|
| 0 | No weapon (unarmed) |
| 1 | Dirk |
| 2 | Mace |
| 3 | Sword |
| 4 | Bow |
| 5 | Wand |
| 8 | Touch |

When `weapon == 0`, the Hero pass must skip the overlay blit entirely — `STATELIST[frame].wpn_no` may be non-zero for unarmed walking poses in some entries. The weapon value from `CombatState` is the authoritative gate.

> **Note:** the bow-walk special case below keys on `weapon == 4`, which is still correct under these weapon codes (4 = Bow).

---

## Task 1: Extract weapon state from the Hero pass

**Files:**
- Modify: `src/game/ecs/scene.rs`

The Hero pass already queries `Option<&CombatState>` and computes a body frame index. This task extracts `weapon` and the numeric facing so they are available for the overlay logic added in Task 2. The bow-walk vs. shoot distinction is driven by the body frame index (`inum < 32`), so `is_fighting` is NOT needed.

- [ ] **Step 1: Locate the Hero pass body frame determination**

Open `src/game/ecs/scene.rs`. Find the Hero query pass. After the body frame index (call it `inum: usize` — the STATELIST index already computed from motion state and facing) — approximately line 809 — add:

```rust
let weapon: u8 = combat_opt.map(|c| c.weapon).unwrap_or(0);
// Numeric 8-direction facing (0..=7) used by the wand frame derivation.
let facing_dir: usize = facing.dir as usize;
```

Notes:
- `combat_opt` is the `Option<&CombatState>` already bound earlier in the same loop iteration.
- `facing` is the `Facing` component already bound for body-frame selection; use whatever binding/expression the Hero pass already uses for the direction. The numeric value must match the original facing encoding used by `facing + 103` (wand) — verify against `Direction`/`Facing` in `src/game/ecs/components.rs`.
- `inum` is the body STATELIST index (0–86) already computed; the weapon overlay reuses it. If the pass names it `frame`, use that name consistently below.

- [ ] **Step 2: Compile check**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no new errors from these two lines.

---

## Task 2: Determine weapon frame and offsets

**Files:**
- Modify: `src/game/ecs/scene.rs`

This task computes the weapon overlay parameters **before** the body blit (because, depending on facing, the weapon may need to be drawn *before* the body — see Task 3). Add this block right after `inum`/`weapon`/`facing_dir` are known and **before** the existing body `blit_sprite_to_framebuf(...)` call. It is a direct port of fmain.c:2420-2446 (frame/offset) and fmain.c:2400-2409 (gate + order).

- [ ] **Step 1: Add the offset + frame determination block**

```rust
// Weapon-overlay pass — port of select_atype_inum (fmain.c:2420-2446).
// `inum` is the body STATELIST index; `weapon` the CombatState weapon code.
let entry = STATELIST.get(inum).copied().unwrap_or(STATELIST[0]);

// --- pixel offset (fmain.c:2422-2426) ---
let (wpn_dx, wpn_dy): (i32, i32) = if weapon == 4 && inum < 32 {
    // Bow while walking: per-frame offset table, indexed by the body frame.
    (BOW_X[inum] as i32, BOW_Y[inum] as i32)
} else {
    (entry.wpn_x as i32, entry.wpn_y as i32)
};

// --- OBJECTS-sheet frame (fmain.c:2429-2444) ---
let mut extra_dy = 0i32;
let wpn_frame: usize = if weapon == 4 && inum < 32 {
    // Bow while walking: frame by direction group q = inum / 8.
    let q = inum / 8;
    if q & 1 != 0 { 30 }          // west / east → east-west bow frame
    else if q & 2 != 0 { 0x53 }   // north
    else { 0x51 }                 // south
} else if weapon == 5 {
    // Wand: frame = facing + 103; NE (facing 2) nudges up 6 px.
    if facing_dir == 2 { extra_dy = -6; }
    facing_dir + 103
} else {
    // Hand weapons + bow shooting: wpn_no + per-class base offset k.
    let k = match weapon {
        2 => 32, // mace
        3 => 48, // sword
        1 => 64, // dirk
        4 => 0,  // bow (shooting)
        _ => 0,
    };
    entry.wpn_no as usize + k
};
let wpn_dy = wpn_dy + extra_dy;
```

Notes:
- `inum` is the STATELIST index already computed for the body sprite. The weapon overlay reuses it — both offset and frame derive from it.
- The bow offset table is indexed by `inum` (the body frame), NOT `cycle % 32` (fmain.c:2422-2423).
- `STATELIST`, `BOW_X`, `BOW_Y` are imported in Task 4.

- [ ] **Step 2: Add the gate + draw-order flags**

Immediately after the block above (still **before** the body blit), add:

```rust
// needs_weapon_pass gate (fmain.c:2400-2401): 0 < weapon < 8 (8 = touch,
// monster-only, no overlay) and the hero is alive (inum < 80 = not death/sink/sleep).
let draw_weapon = weapon > 0 && weapon < 8 && inum < 80;

// Facing-dependent draw order (resolve_pass_params, fmain.c:2402-2407).
// `true` = weapon behind the body (drawn first); `false` = weapon on top.
let weapon_behind = if weapon == 4 && inum < 32 {
    (facing_dir & 4) == 0                  // bow while walking
} else {
    ((facing_dir as i32 - 2) & 4) != 0     // hand weapons + bow shooting
};
```

`facing_dir` is the numeric `Direction` (0..=7) extracted in Task 1.

- [ ] **Step 3: Note on the borrow pattern**

The weapon blit is performed by a small free helper so it can be called on either side of the body blit without a borrow conflict (each call scopes its own `&mut framebuf`). Add it at module scope in `scene.rs`:

```rust
fn blit_weapon_overlay(
    sheets: &[Option<SpriteSheet>],
    wpn_frame: usize,
    x: i32,
    y: i32,
    framebuf: &mut [u32],
    fb_w: i32,
    fb_h: i32,
) {
    if let Some(Some(sheet)) = sheets.get(3) {
        if let Some(wp) = sheet.frame_pixels(wpn_frame) {
            // 16 px object-sheet frame height, NOT SPRITE_H (32).
            blit_sprite_to_framebuf(wp, x, y, sheet.frame_h, framebuf, fb_w, fb_h);
        }
    }
}
```

(`SpriteSheet`/`blit_sprite_to_framebuf` are already in scope in `scene.rs`.)

---

## Task 3: Blit the weapon overlay in the correct order

**Files:**
- Modify: `src/game/ecs/scene.rs`

The weapon is drawn **before** the body when `weapon_behind`, otherwise **after** it. This requires placing two guarded calls around the existing body `blit_sprite_to_framebuf(...)`.

- [ ] **Step 1: Draw the weapon behind the body (before the body blit)**

Immediately **before** the existing hero body blit, insert:

```rust
if draw_weapon && weapon_behind {
    blit_weapon_overlay(sheets, wpn_frame, rel_x + wpn_dx, rel_y + wpn_dy, framebuf, fb_w, fb_h);
}
```

- [ ] **Step 2: Draw the weapon on top of the body (after the body blit)**

Immediately **after** the existing hero body blit, insert:

```rust
if draw_weapon && !weapon_behind {
    blit_weapon_overlay(sheets, wpn_frame, rel_x + wpn_dx, rel_y + wpn_dy, framebuf, fb_w, fb_h);
}
```

Notes:
- The gate (`weapon > 0 && weapon < 8 && inum < 80`) mirrors the original `needs_weapon_pass`: unarmed (0) and touch (>= 8) draw nothing, and no overlay is drawn while dead/sinking/sleeping. Do NOT gate on `wpn_no`/`wpn_frame` — frames 83–86 have non-zero `wpn_no` yet must not paint a weapon.
- Exactly one of the two guarded calls fires per frame (they are mutually exclusive on `weapon_behind`). The "behind" call must precede the body blit and the "on top" call must follow it.
- `rel_x + wpn_dx` / `rel_y + wpn_dy` — `rel_x`/`rel_y` are the hero's screen position (in scope at the body blit); the offsets shift the weapon relative to it.
- Frame height (16 px object-sheet, not `SPRITE_H` 32) and the cfile-3 double-`Option` check are handled inside `blit_weapon_overlay` (Task 2 Step 3). Edge clipping is handled inside `blit_sprite_to_framebuf`.

- [ ] **Step 3: Compile check**

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

- [ ] **Hero with sword (weapon == 3)**
  - Walk South, then enter combat.
  - Expected: sword overlay (frame `wpn_no + 48`) appears at correct STATELIST offsets during walk frames, and at fight-frame offsets during combat. Overlay does not "float" or appear at (0, 0) relative to screen origin.

- [ ] **Hero with mace (weapon == 2) and dirk (weapon == 1)**
  - Walk and fight with each.
  - Expected: overlay frame `wpn_no + 32` (mace) / `wpn_no + 64` (dirk) at correct offsets. The dirk and sword are distinct sprites — confirm they are not swapped.

- [ ] **Hero with bow (weapon == 4) — walk cycle**
  - Walk in all four cardinal directions.
  - Expected: bow bobs visibly as the hero walks, driven by `BOW_X`/`BOW_Y`. The bob is subtle — one or two pixels per frame — not a large jump.

- [ ] **Hero with bow (weapon == 4) — fight/shoot**
  - Enter combat with bow equipped (body frame `inum >= 32`).
  - Expected: bow uses STATELIST `wpn_x/wpn_y` offsets and frame `wpn_no + 0`, not the `BOW_X/BOW_Y` walk path and not frames 30/0x51/0x53.

- [ ] **Hero with wand (weapon == 5)**
  - Equip the wand and face each of the 8 directions.
  - Expected: wand overlay frame = `facing + 103`; the NE facing renders 6 px higher than the others.

- [ ] **Facing-dependent draw order (z-order)**
  - With a hand weapon equipped, rotate the hero through all 8 facings (and repeat while walking with the bow).
  - Expected: the weapon is drawn *behind* the body for some facings and *on top* for others — it must NOT always sit on top. Verify there is no facing where the weapon visibly pops to the wrong layer (e.g. a sword that should be hidden behind the body when facing away appears in front, or vice-versa).

- [ ] **Screen edge clipping**
  - Walk the hero to the left and top edges of the viewport so the body sprite is partially clipped.
  - Expected: weapon overlay clips cleanly at the same edges with no artifacts (black bars, garbage pixels, or panics).

- [ ] **Death / sleep frames**
  - Allow the hero to die (god mode off); also observe the sleep pose.
  - Expected: no weapon overlay while dead/sinking/sleeping (body frame `inum >= 80`). Note this is gated on state, NOT on `wpn_no` — frames 83–86 have `wpn_no == 10` but must still draw no weapon.

---

## Completion check

```bash
cargo build 2>&1 | grep "^error"
cargo test 2>&1 | grep "^test result"
```

Both succeed. Manual gameplay confirms weapon overlays are visible, correctly positioned, and well-behaved at screen edges and during all animation states.

---

## Spec references

- `reference/logic/sprite-rendering.md` (research branch) — `resolve_pass_params` (fmain.c:2400-2409, facing-dependent draw order), `needs_weapon_pass` (gate), and `select_atype_inum` (fmain.c:2420-2446, frame/offset: `wpn_no + k`, bow & wand paths)
- `docs/spec/characters-animation.md` §8.1, §8.4 — weapon codes (1=Dirk, 2=Mace, 3=Sword, 4=Bow, 5=Wand, 8=Touch) and STATELIST fields (`wpn_no`, `wpn_x`, `wpn_y`)
- `docs/spec/display-rendering.md` §1.3 — rendering pipeline phase order; §5.2 — sprite format and frame height by cfile

---

## Dependencies

| Plan | Reason |
|------|--------|
| A | ECS component definitions in place |
| B | System infrastructure in place |
| C | System schedule wired |
| D | `EcsScene` and `blit_actors_inner()` exist |
| N | `CombatState.weapon` populated from inventory |
