---
title: "Plan Z — SetFig Spawn cfile Fix"
plan: Z
status: draft
depends_on: [G]
touches: [src/game/ecs/scene.rs]
---

# ECS Migration Plan Z: SetFig Spawn cfile Fix

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the merged Plan G setfig spawn path so each stationary NPC's `SpriteRef.cfile_idx` is the correct sprite sheet derived from `SETFIG_TABLE`, instead of the hardcoded `13`.

**Architecture:** `reload_region()` in `EcsScene` currently spawns every setfig with `cfile_idx = 13u8`. The authoritative sheet for a setfig is `SETFIG_TABLE[race & 0x7f].cfile_entry` (`fmain.c:3374` strips the `0x80` setfig bit; `SETFIG_TABLE` lives in `src/game/sprites.rs`). This plan adds a small pure helper `setfig_cfile_idx(race)` and uses it at the spawn site. The base frame (`image_base`) is **not** stored on the entity: Plan S's render pass derives both sheet and frame from `WorldObj.ob_id & 0x7f` at draw time, so `WorldObj.ob_id` remains the single source of truth. This plan only makes the redundant `SpriteRef.cfile_idx` value truthful and consistent.

**Prerequisites:** Plan G complete (it created `reload_region` and the setfig spawn branch). Independent of Plan S — they touch different code paths and are mutually consistent.

**Tech Stack:** Rust 2021, `hecs = "0.11"`.

---

## Background

### The bug

`src/game/ecs/scene.rs`, in `reload_region()`:

```rust
if npc.npc_type == NPC_TYPE_HUMAN && npc.race != RACE_ENEMY {
    // SetFig: stationary NPC (shopkeeper, beggar, etc.)
    let cfile_idx = 13u8; // default setfig sprite sheet   <-- HARDCODED, WRONG
    let obj = WorldObj { ob_id: npc.race, ob_stat: 1, region, visible: true, goal: 0 };
    spawn_setfig(&mut self.world, x, y, obj, cfile_idx);
}
```

Every setfig gets sheet `13` (wizard/priest), so a bartender, king, witch, ranger, etc. would carry the wrong `cfile_idx`.

### The authoritative mapping

`SETFIG_TABLE` in `src/game/sprites.rs` is a direct port of the original `setfig_table[14]`:

| setfig index (`race & 0x7f`) | NPC | `cfile_entry` |
|---|---|---|
| 0 wizard, 1 priest | | 13 |
| 2 guard, 3 guard-back, 4 princess, 5 king, 6 noble, 7 sorceress | | 14 |
| 8 bartender | | 15 |
| 9 witch, 10 spectre, 11 ghost | | 16 |
| 12 ranger, 13 beggar | | 17 |

`SetfigEntry` fields: `cfile_entry: u8`, `image_base: u8`, `can_talk: bool`.

The setfig type index is `race & 0x7f` (the race byte with the `0x80` setfig bit stripped, `fmain.c:3374`). Known race bytes: bartender `0x88`→8, princess `0x84`→4, witch `0x89`→9, beggar `0x8d`→13.

### Scope note

- This plan does **not** change rendering. Plan S renders setfigs from `WorldObj.ob_id & 0x7f` via `SETFIG_TABLE` and does not read `SpriteRef`. After this fix, `SpriteRef.cfile_idx` is merely correct/consistent rather than a misleading `13`.
- This plan does **not** add `image_base` to `SpriteRef` (that would create a second source of truth competing with the `WorldObj`-derived render path).
- The `spawn_setfig` signature is unchanged.

---

## File map

| File | Change |
|------|--------|
| `src/game/ecs/scene.rs` | Add `setfig_cfile_idx()` helper; use it in `reload_region()`; add 2 unit tests |

---

## Task 1: Add the `setfig_cfile_idx()` helper

**Files:**
- Modify: `src/game/ecs/scene.rs`

- [ ] **Step 1: Write the failing test**

Add to the existing `#[cfg(test)] mod tests` block at the bottom of `src/game/ecs/scene.rs` (it already uses `use super::*;`):

```rust
#[test]
fn setfig_cfile_idx_maps_race_to_sheet() {
    // race & 0x7f indexes SETFIG_TABLE; cfile_entry is the sprite sheet.
    assert_eq!(setfig_cfile_idx(0x80), 13, "wizard  -> cfile 13");
    assert_eq!(setfig_cfile_idx(0x81), 13, "priest  -> cfile 13");
    assert_eq!(setfig_cfile_idx(0x84), 14, "princess-> cfile 14");
    assert_eq!(setfig_cfile_idx(0x88), 15, "bartender-> cfile 15");
    assert_eq!(setfig_cfile_idx(0x89), 16, "witch   -> cfile 16");
    assert_eq!(setfig_cfile_idx(0x8d), 17, "beggar  -> cfile 17");
}

#[test]
fn setfig_cfile_idx_out_of_range_falls_back_to_13() {
    // race & 0x7f == 14 is past SETFIG_TABLE (14 entries, 0..=13) -> fallback 13.
    assert_eq!(setfig_cfile_idx(0x8e), 13);
}
```

- [ ] **Step 2: Run the test to verify it fails**

```bash
cargo test -p faery-tale-rs ecs::scene::tests::setfig_cfile_idx 2>&1 | grep -E "^error|^test result|cannot find"
```

Expected: a compile error — `cannot find function setfig_cfile_idx in this scope`.

- [ ] **Step 3: Implement the helper**

Add this free function immediately after `npc_type_to_cfile()` in `src/game/ecs/scene.rs` (search for `fn npc_type_to_cfile`):

```rust
/// Map a setfig race byte to its sprite-sheet cfile index.
///
/// The setfig type index is `race & 0x7f` (the 0x80 setfig bit stripped,
/// fmain.c:3374). `SETFIG_TABLE` (src/game/sprites.rs) yields the sheet.
/// Out-of-range indices fall back to cfile 13.
fn setfig_cfile_idx(race: u8) -> u8 {
    let k = (race & 0x7f) as usize;
    crate::game::sprites::SETFIG_TABLE
        .get(k)
        .map(|e| e.cfile_entry)
        .unwrap_or(13)
}
```

- [ ] **Step 4: Run the test to verify it passes**

```bash
cargo test -p faery-tale-rs ecs::scene::tests::setfig_cfile_idx 2>&1 | grep -E "^test result|FAILED"
```

Expected: `test result: ok. 2 passed`.

- [ ] **Step 5: Commit**

```bash
git add src/game/ecs/scene.rs
git commit -m "feat(ecs): add setfig_cfile_idx() mapping setfig race to sprite sheet"
```

---

## Task 2: Use the helper in `reload_region()`

**Files:**
- Modify: `src/game/ecs/scene.rs`

- [ ] **Step 1: Replace the hardcoded cfile_idx**

In `reload_region()`, find the setfig spawn branch:

```rust
            if npc.npc_type == NPC_TYPE_HUMAN && npc.race != RACE_ENEMY {
                // SetFig: stationary NPC (shopkeeper, beggar, etc.)
                let cfile_idx = 13u8; // default setfig sprite sheet
                let obj = WorldObj {
```

Replace the `let cfile_idx = 13u8; // default setfig sprite sheet` line with:

```rust
                // Sprite sheet from SETFIG_TABLE[race & 0x7f] (not hardcoded 13).
                // image_base is derived at render time from WorldObj.ob_id (Plan S),
                // so it is intentionally not stored here.
                let cfile_idx = setfig_cfile_idx(npc.race);
```

The rest of the branch (the `WorldObj { ob_id: npc.race, .. }` construction and the `spawn_setfig(..)` call) is unchanged.

- [ ] **Step 2: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors. (`setfig_cfile_idx` is `u8`, matching `spawn_setfig`'s `cfile_idx: u8` parameter.)

- [ ] **Step 3: Verify no warnings introduced**

```bash
cargo build 2>&1 | grep "^warning"
```

Expected: no new warnings referencing `scene.rs` (`setfig_cfile_idx` is now used, so no dead-code warning).

- [ ] **Step 4: Commit**

```bash
git add src/game/ecs/scene.rs
git commit -m "fix(ecs): derive setfig cfile from SETFIG_TABLE in reload_region

- replace hardcoded cfile_idx=13 with setfig_cfile_idx(race)
- corrects sprite-sheet selection for bartender/king/witch/ranger/etc."
```

---

## Task 3: Full verification

**Files:**
- None (verification only)

- [ ] **Step 1: Run the scene test suite**

```bash
cargo test -p faery-tale-rs ecs::scene 2>&1 | grep -E "^test result|FAILED"
```

Expected: all scene tests pass, including the two new `setfig_cfile_idx_*` tests.

- [ ] **Step 2: Full build**

```bash
cargo build 2>&1 | grep -E "^error|^warning"
```

Expected: no errors, no new warnings.

- [ ] **Step 3 (optional manual, requires Plan S): visual check**

If Plan S (SetFig rendering) has landed, run the game and visit a town/inn with multiple setfig types (bartender, ranger, beggar, king). Each should render from its correct sheet rather than the wizard/priest sheet. If Plan S has not landed, this step is skipped — the unit tests cover the spawn-side correctness.

---

## Spec references

- `reference/logic/npc-dialogue.md` (research branch) — setfig type index `k = race & 0x7f` (fmain.c:3374)
- `src/game/sprites.rs` — `SETFIG_TABLE` / `SetfigEntry` (port of `setfig_table[14]`)
- Plan S (`2025-01-01-ecs-plan-s-setfig-sprites.md`) — the render pass that derives sheet + `image_base` from `WorldObj.ob_id & 0x7f`; consistent with this spawn-side fix

## Test plan

- `setfig_cfile_idx_maps_race_to_sheet` — race bytes 0x80/0x81/0x84/0x88/0x89/0x8d map to cfiles 13/13/14/15/16/17
- `setfig_cfile_idx_out_of_range_falls_back_to_13` — index past `SETFIG_TABLE` falls back to 13

## Files touched

| File | Change |
|------|--------|
| `src/game/ecs/scene.rs` | Add `setfig_cfile_idx()`; use it in `reload_region()`; add 2 unit tests |
