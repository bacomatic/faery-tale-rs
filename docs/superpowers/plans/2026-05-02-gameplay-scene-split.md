# gameplay_scene.rs Module Split Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split `src/game/gameplay_scene.rs` (10,015 lines) into focused submodules so agents read only the relevant ~500–1500 line file per task instead of the full monolith.

**Architecture:** Convert `gameplay_scene.rs` into a `gameplay_scene/` directory with `mod.rs` (struct definition + small helpers) and one `impl GameplayScene` file per logical subsystem. Rust allows multiple `impl` blocks for the same type across modules in the same crate — this is a pure structural refactor with zero API changes. All `pub` interfaces remain identical.

**Tech Stack:** Rust, `cargo test` for verification after each extraction.

---

## Current structure summary

All methods are in one `impl GameplayScene` block (lines 435–5455) plus `impl Scene for GameplayScene` (5457–6450) and ~3,565 lines of `#[cfg(test)]` code (6451–10015).

Key method groups by line range:

| Group | Line range | Description |
|-------|-----------|-------------|
| Struct fields + small helpers | 1–569, 786–861 | GameplayScene struct, new(), init_from_library(), utility fns |
| Narrative | 570–785, 1965–1998 | Placard sequences, princess rescue |
| Spells + direction | 862–934 | try_cast_spell, current_direction |
| Input (large) | 935–1568, 5346–5456 | apply_player_input, compass input, cheat keys |
| Proximity + speech | 1569–1693 | nearest_fig, update_proximity_speech |
| Environment | 1694–1964 | update_environ, goodfairy countdown, fiery death, environ damage |
| NPC interaction | 1999–2224 | handle_setfig_talk, do_buy_slot |
| Combat | 2225–2535 | run_combat_tick, apply_hit |
| Carriers | 2536–2623 | update_turtle_autonomous, facing_toward |
| Actors | 2624–2779 | update_actors |
| Rendering | 2780–3077, 4920–5345 | render_hibar, render_by_viewstatus, blit_* helpers |
| Region / map | 3078–3137, 4765–4919 | on_region_changed, outdoor_map_blocks, palette fns, map math |
| Menu actions | 3138–3876 | dispatch_menu_action, do_option, toggle_menu_mode |
| Items | 3877–4230 | handle_take_item, search_body, mark_npc_looted |
| Events + mood | 4231–4275 | handle_game_event, setmood, stat_field_mut |
| Debug commands | 4276–4764 | apply_command |
| Scene trait impl | 5457–6450 | handle_event, update, as_any* |
| Tests | 6451–10015 | All #[cfg(test)] modules |

---

## Extraction order (lowest → highest coupling)

Tasks 1–5: Low coupling (safe to extract first, minimal caller impact)
Tasks 6–9: Medium coupling
Tasks 10–12: High coupling (extract last)
Task 13: Tests

---

## Task 1: Create the module directory scaffold

**Files:**
- Create: `src/game/gameplay_scene/` (directory)
- Rename: `src/game/gameplay_scene.rs` → `src/game/gameplay_scene/mod.rs`

- [x] **Step 1: Create directory and move file**

```bash
mkdir -p src/game/gameplay_scene
cp src/game/gameplay_scene.rs src/game/gameplay_scene/mod.rs
```

- [x] **Step 2: Verify it builds**

```bash
cargo check 2>&1 | head -30
```

Expected: compiles cleanly. Rust resolves `mod gameplay_scene` to either `gameplay_scene.rs` OR `gameplay_scene/mod.rs` — after the copy both exist, so we must remove the original.

- [x] **Step 3: Remove the original file**

```bash
rm src/game/gameplay_scene.rs
```

- [x] **Step 4: Verify again**

```bash
cargo check 2>&1 | head -30
```

Expected: zero errors.

- [x] **Step 5: Run tests**

```bash
cargo test 2>&1 | tail -20
```

Expected: all tests pass (no behavior change, just file moved).

- [x] **Step 6: Commit**

```bash
git add src/game/gameplay_scene/ src/game/gameplay_scene.rs
git commit -m "refactor: convert gameplay_scene.rs to gameplay_scene/ module directory

Pure file move — no code changes. Prepares for per-subsystem extraction."
```

---

## Task 2: Extract narrative.rs (low coupling)

**Files:**
- Create: `src/game/gameplay_scene/narrative.rs`
- Modify: `src/game/gameplay_scene/mod.rs`

Methods to extract from `mod.rs`:
- `tick_narrative_sequence` (~570–573)
- `execute_active_narrative_step` (~574–648)
- `rescue_placard_key_for_princess_count` (~649–656)
- `enqueue_princess_rescue_sequence` (~657–686)
- `enqueue_succession_placards` (~687–707)
- `execute_princess_rescue` (~1965–1998)
- All `debug_*` narrative helpers (~708–785)

- [x] **Step 1: Create narrative.rs with the impl block**

In `narrative.rs`, write:

```rust
//! Narrative sequence handling — placard queuing, princess rescue, brother succession.
//! See `docs/spec/intro-narrative.md` for specification.

use super::*;

impl GameplayScene {
    // [paste extracted methods here]
}
```

- [x] **Step 2: Add module declaration to mod.rs**

Add near the top of `mod.rs` (after `use` statements, before `impl GameplayScene`):

```rust
mod narrative;
```

- [x] **Step 3: Remove extracted methods from mod.rs**

Delete the method bodies for the methods listed above from `mod.rs`. Keep only the `mod narrative;` declaration.

- [x] **Step 4: cargo check**

```bash
cargo check 2>&1 | head -30
```

Fix any visibility errors (methods called from other modules may need `pub(crate)` or `pub(super)`).

- [x] **Step 5: cargo test**

```bash
cargo test 2>&1 | tail -20
```

Expected: all tests pass.

- [x] **Step 6: Commit**

```bash
git add src/game/gameplay_scene/
git commit -m "refactor: extract narrative methods into gameplay_scene/narrative.rs

Extracts placard sequences, princess rescue, brother succession handlers.
~250 lines moved out of mod.rs."
```

---

## Task 3: Extract proximity.rs (low coupling)

**Files:**
- Create: `src/game/gameplay_scene/proximity.rs`
- Modify: `src/game/gameplay_scene/mod.rs`

Methods to extract:
- `nearest_fig` (~1569–1633)
- `update_proximity_speech` (~1634–1693)

- [x] **Step 1: Create proximity.rs**

```rust
//! NPC proximity detection and auto-speech triggering.
//! See `docs/spec/npcs-dialogue.md` §13 for specification.

use super::*;

impl GameplayScene {
    // [paste extracted methods here]
}
```

- [x] **Step 2: Add `mod proximity;` to mod.rs, remove methods**

- [x] **Step 3: cargo check && cargo test**

```bash
cargo check 2>&1 | head -20 && cargo test 2>&1 | tail -10
```

- [x] **Step 4: Commit**

```bash
git add src/game/gameplay_scene/
git commit -m "refactor: extract proximity methods into gameplay_scene/proximity.rs

Extracts nearest_fig and update_proximity_speech. ~125 lines."
```

---

## Task 4: Extract carriers.rs (low coupling)

**Files:**
- Create: `src/game/gameplay_scene/carriers.rs`
- Modify: `src/game/gameplay_scene/mod.rs`

Methods to extract:
- `update_turtle_autonomous` (~2536–2607)
- `facing_toward` (~2608–2623)

- [x] **Step 1: Create carriers.rs**

```rust
//! Carrier vehicle AI — turtle autonomous movement, facing helpers.
//! See `docs/spec/carriers.md` for specification.

use super::*;

impl GameplayScene {
    // [paste extracted methods here]
}
```

- [x] **Step 2: Add `mod carriers;` to mod.rs, remove methods**

- [x] **Step 3: cargo check && cargo test**

```bash
cargo check 2>&1 | head -20 && cargo test 2>&1 | tail -10
```

- [x] **Step 4: Commit**

```bash
git add src/game/gameplay_scene/
git commit -m "refactor: extract carrier methods into gameplay_scene/carriers.rs

Extracts update_turtle_autonomous and facing_toward. ~90 lines."
```

---

## Task 5: Extract game_event.rs (low coupling)

**Files:**
- Create: `src/game/gameplay_scene/game_event.rs`
- Modify: `src/game/gameplay_scene/mod.rs`

Methods to extract:
- `handle_game_event` (~4231–4249)
- `setmood` (~4250–4275)
- `stat_field_mut` (~4904–4919)

- [x] **Step 1: Create game_event.rs**

```rust
//! Game event dispatch, mood calculation, and stat field helpers.

use super::*;

impl GameplayScene {
    // [paste extracted methods here]
}
```

- [x] **Step 2: Add `mod game_event;` to mod.rs, remove methods**

- [x] **Step 3: cargo check && cargo test**

```bash
cargo check 2>&1 | head -20 && cargo test 2>&1 | tail -10
```

- [x] **Step 4: Commit**

```bash
git add src/game/gameplay_scene/
git commit -m "refactor: extract game_event methods into gameplay_scene/game_event.rs

Extracts handle_game_event, setmood, stat_field_mut. ~80 lines."
```

---

## Task 6: Extract combat_logic.rs (medium coupling)

**Files:**
- Create: `src/game/gameplay_scene/combat_logic.rs`
- Modify: `src/game/gameplay_scene/mod.rs`

Methods to extract:
- `run_combat_tick` (~2225–2323)
- `apply_hit` (~2324–2535)

- [x] **Step 1: Create combat_logic.rs**

```rust
//! Combat tick processing and hit application.
//! See `docs/spec/combat.md` for specification.

use super::*;

impl GameplayScene {
    // [paste extracted methods here]
}
```

- [x] **Step 2: Add `mod combat_logic;` to mod.rs, remove methods**

- [x] **Step 3: cargo check && cargo test**

```bash
cargo check 2>&1 | head -20 && cargo test 2>&1 | tail -10
```

- [x] **Step 4: Commit**

```bash
git add src/game/gameplay_scene/
git commit -m "refactor: extract combat methods into gameplay_scene/combat_logic.rs

Extracts run_combat_tick and apply_hit. ~310 lines."
```

---

## Task 7: Extract region.rs (medium coupling)

**Files:**
- Create: `src/game/gameplay_scene/region.rs`
- Modify: `src/game/gameplay_scene/mod.rs`

Methods to extract:
- `on_region_changed` (~3078–3137)
- `outdoor_map_blocks` (~4765–4780)
- `outdoor_region_from_pos` (~4781–4795)
- `region_palette` (~4796–4814)
- `build_base_colors_palette` (~4815–4843)
- `compute_current_palette` (~4844–4903)
- `snap_camera_to_hero` (~5035–5042)
- `map_adjust` (~5043–5068)
- `actor_rel_pos` (~5069–5073)
- `carrier_rel_pos` (~5074–5077)
- `actor_rel_pos_offset` (~5078–5089)

- [x] **Step 1: Create region.rs**

```rust
//! Region transitions, map coordinate math, and palette computation.
//! See `docs/spec/world-structure.md` and `docs/spec/palettes-daynight-visuals.md`.

use super::*;

impl GameplayScene {
    // [paste extracted methods here]
}
```

- [x] **Step 2: Add `mod region;` to mod.rs, remove methods**

- [x] **Step 3: cargo check && cargo test**

```bash
cargo check 2>&1 | head -20 && cargo test 2>&1 | tail -10
```

- [x] **Step 4: Commit**

```bash
git add src/game/gameplay_scene/
git commit -m "refactor: extract region/map methods into gameplay_scene/region.rs

Extracts region transitions, palette computation, camera and coordinate helpers. ~370 lines."
```

---

## Task 8: Extract items.rs (medium coupling)

**Files:**
- Create: `src/game/gameplay_scene/items.rs`
- Modify: `src/game/gameplay_scene/mod.rs`

Methods to extract:
- `handle_take_item` (~3877–4075)
- `search_body` (~4076–4222)
- `mark_npc_looted` (~4223–4230)

- [x] **Step 1: Create items.rs**

```rust
//! Item pickup, body searching, and loot tracking.
//! See `docs/spec/inventory-items.md` for specification.

use super::*;

impl GameplayScene {
    // [paste extracted methods here]
}
```

- [x] **Step 2: Add `mod items;` to mod.rs, remove methods**

- [x] **Step 3: cargo check && cargo test**

```bash
cargo check 2>&1 | head -20 && cargo test 2>&1 | tail -10
```

- [x] **Step 4: Commit**

```bash
git add src/game/gameplay_scene/
git commit -m "refactor: extract item methods into gameplay_scene/items.rs

Extracts handle_take_item, search_body, mark_npc_looted. ~355 lines."
```

---

## Task 9: Extract environ.rs (medium coupling) ✅

**Files:**
- Create: `src/game/gameplay_scene/environ.rs`
- Modify: `src/game/gameplay_scene/mod.rs`

Methods to extract:
- `update_spectre_visibility` (~846–861)
- `update_environ` (~1694–1765)
- `tick_goodfairy_countdown` (~1766–1913)
- `update_fiery_death` (~1914–1921)
- `apply_environ_damage` (~1922–1964)

- [x] **Step 1: Create environ.rs**

```rust
//! Environmental state updates — spectre visibility, terrain damage, goodfairy countdown.
//! See `docs/spec/survival.md` and `docs/spec/death-revival.md`.

use super::*;

impl GameplayScene {
    // [paste extracted methods here]
}
```

- [x] **Step 2: Add `mod environ;` to mod.rs, remove methods**

- [x] **Step 3: cargo check && cargo test**

```bash
cargo check 2>&1 | head -20 && cargo test 2>&1 | tail -10
```

- [x] **Step 4: Commit**

```bash
git add src/game/gameplay_scene/
git commit -m "refactor: extract environ methods into gameplay_scene/environ.rs

Extracts spectre visibility, environ tick, goodfairy countdown, fiery death. ~300 lines."
```

---

## Task 10: Extract rendering.rs (medium-high coupling) ✅

**Files:**
- Create: `src/game/gameplay_scene/rendering.rs`
- Modify: `src/game/gameplay_scene/mod.rs`

Methods to extract:
- `render_hibar` (~2780–2887)
- `render_by_viewstatus` (~2888–3077)
- `blit_sprite_to_framebuf` (~4920–4944)
- `blit_obj_to_framebuf` (~4945–4974)
- `compute_weapon_blit` (~4975–5034)
- `blit_actors_to_framebuf` (~5226–5345)
- `npc_type_to_cfile` (~5127–5149)
- `npc_to_setfig_idx` (~5150–5169)
- `swan_grounded_override` (~5170–5184)
- `npc_animation_frame` (~5185–5225)
- `facing_to_frame_base` (~5090–5110)
- `facing_to_fight_frame_base` (~5111–5126)

- [x] **Step 1: Create rendering.rs**

```rust
//! Sprite blitting, actor rendering, and animation frame computation.
//! See `docs/spec/display-rendering.md` for specification.

use super::*;

impl GameplayScene {
    // [paste extracted methods here]
}
```

- [x] **Step 2: Add `mod rendering;` to mod.rs, remove methods**

- [x] **Step 3: cargo check — pay attention to visibility**

```bash
cargo check 2>&1 | head -40
```

These are all `fn` (not `pub fn`), so they should be accessible within the same module tree. Fix any `pub(super)` needed.

- [x] **Step 4: cargo test**

```bash
cargo test 2>&1 | tail -20
```

- [x] **Step 5: Commit**

```bash
git add src/game/gameplay_scene/
git commit -m "refactor: extract rendering methods into gameplay_scene/rendering.rs

Extracts render_hibar, render_by_viewstatus, blit_* helpers, animation frame fns. ~720 lines."
```

---

## Task 11: Extract npc_interaction.rs (medium-high coupling) ✅

**Files:**
- Create: `src/game/gameplay_scene/npc_interaction.rs`
- Modify: `src/game/gameplay_scene/mod.rs`

Methods to extract:
- `handle_setfig_talk` (~1999–2149)
- `do_buy_slot` (~2150–2224)

- [x] **Step 1: Create npc_interaction.rs**

```rust
//! NPC conversation dispatch and shop purchase handling.
//! See `docs/spec/npcs-dialogue.md` for specification.

use super::*;

impl GameplayScene {
    // [paste extracted methods here]
}
```

- [x] **Step 2: Add `mod npc_interaction;` to mod.rs, remove methods**

- [x] **Step 3: cargo check && cargo test**

```bash
cargo check 2>&1 | head -20 && cargo test 2>&1 | tail -10
```

- [x] **Step 4: Commit**

```bash
git add src/game/gameplay_scene/
git commit -m "refactor: extract NPC interaction methods into gameplay_scene/npc_interaction.rs

Extracts handle_setfig_talk and do_buy_slot. ~225 lines."
```

---

## Task 12: Extract actors.rs (high coupling) ✅

**Files:**
- Create: `src/game/gameplay_scene/actors.rs`
- Modify: `src/game/gameplay_scene/mod.rs`

Methods to extract:
- `update_actors` (~2624–2779)

- [x] **Step 1: Create actors.rs**

```rust
//! Per-frame actor update loop — NPC AI ticks, animation, position.
//! See `docs/spec/ai-encounters.md` for specification.

use super::*;

impl GameplayScene {
    // [paste extracted methods here]
}
```

- [x] **Step 2: Add `mod actors;` to mod.rs, remove method**

- [x] **Step 3: cargo check && cargo test**

```bash
cargo check 2>&1 | head -20 && cargo test 2>&1 | tail -10
```

- [x] **Step 4: Commit**

```bash
git add src/game/gameplay_scene/
git commit -m "refactor: extract update_actors into gameplay_scene/actors.rs (~155 lines)"
```

---

## Task 13: Extract input.rs (high coupling) ✅

**Files:**
- Create: `src/game/gameplay_scene/input.rs`
- Modify: `src/game/gameplay_scene/mod.rs`

Methods to extract:
- `try_cast_spell` (~862–919)
- `current_direction` (~920–934)
- `apply_player_input` (~935–1568)
- `apply_compass_input_from_canvas` (~5346–5389)
- `handle_cheat1_key` (~5390–5456)

- [x] **Step 1: Create input.rs**

```rust
//! Player input processing, direction mapping, spell casting, cheat key handling.
//! See `docs/spec/movement-input.md` for specification.

use super::*;

impl GameplayScene {
    // [paste extracted methods here]
}
```

- [x] **Step 2: Add `mod input;` to mod.rs, remove methods**

- [x] **Step 3: cargo check && cargo test**

```bash
cargo check 2>&1 | head -20 && cargo test 2>&1 | tail -20
```

Input handling has the most tests — expect several test failures if method visibility isn't correct.

- [x] **Step 4: Fix any visibility issues and re-run tests**

```bash
cargo test 2>&1 | tail -20
```

- [x] **Step 5: Commit**

```bash
git add src/game/gameplay_scene/
git commit -m "refactor: extract input methods into gameplay_scene/input.rs

Extracts apply_player_input, try_cast_spell, compass and cheat key handlers. ~750 lines."
```

---

## Task 14: Extract menu_actions.rs (high coupling) ✅

**Files:**
- Create: `src/game/gameplay_scene/menu_actions.rs`
- Modify: `src/game/gameplay_scene/mod.rs`

Methods to extract:
- `dispatch_menu_action` (~3138–3444)
- `do_option` (~3445–3865)
- `toggle_menu_mode` (~3866–3876)

- [x] **Step 1: Create menu_actions.rs**

```rust
//! Menu action dispatch and option execution.
//! See `docs/spec/ui-menus.md` for specification.

use super::*;

impl GameplayScene {
    // [paste extracted methods here]
}
```

- [x] **Step 2: Add `mod menu_actions;` to mod.rs, remove methods**

- [x] **Step 3: cargo check && cargo test**

```bash
cargo check 2>&1 | head -20 && cargo test 2>&1 | tail -20
```

- [x] **Step 4: Commit**

```bash
git add src/game/gameplay_scene/
git commit -m "refactor: extract menu methods into gameplay_scene/menu_actions.rs

Extracts dispatch_menu_action, do_option, toggle_menu_mode. ~740 lines."
```

---

## Task 15: Extract debug_commands.rs ✅

**Files:**
- Create: `src/game/gameplay_scene/debug_commands.rs`
- Modify: `src/game/gameplay_scene/mod.rs`

Methods to extract:
- `apply_command` (~4276–4764)

- [x] **Step 1: Create debug_commands.rs**

```rust
//! Debug command dispatch (DebugCommand handler).
//! See `docs/DEBUG_SPECIFICATION.md` for specification.

use super::*;

impl GameplayScene {
    // [paste extracted methods here]
}
```

- [x] **Step 2: Add `mod debug_commands;` to mod.rs, remove method**

- [x] **Step 3: cargo check && cargo test**

```bash
cargo check 2>&1 | head -20 && cargo test 2>&1 | tail -10
```

- [x] **Step 4: Commit**

```bash
git add src/game/gameplay_scene/
git commit -m "refactor: extract apply_command into gameplay_scene/debug_commands.rs (~490 lines)"
```

---

## Task 16: Extract scene_impl.rs (the orchestrator)

**Files:**
- Create: `src/game/gameplay_scene/scene_impl.rs`
- Modify: `src/game/gameplay_scene/mod.rs`

Content to extract:
- `impl Scene for GameplayScene` block (~5457–6450): `handle_event`, `update`, `as_any`, `as_any_mut`

- [ ] **Step 1: Create scene_impl.rs**

```rust
//! Scene trait implementation — event handling and per-frame update loop.

use super::*;

impl Scene for GameplayScene {
    // [paste extracted impl here]
}
```

- [ ] **Step 2: Add `mod scene_impl;` to mod.rs, remove the impl block**

- [ ] **Step 3: cargo check && cargo test**

```bash
cargo check 2>&1 | head -20 && cargo test 2>&1 | tail -10
```

- [ ] **Step 4: Commit**

```bash
git add src/game/gameplay_scene/
git commit -m "refactor: extract Scene impl into gameplay_scene/scene_impl.rs (~990 lines)"
```

---

## Task 17: Extract tests.rs

**Files:**
- Create: `src/game/gameplay_scene/tests.rs`
- Modify: `src/game/gameplay_scene/mod.rs`

Content to extract:
- All `#[cfg(test)]` modules (~6451–10015)

- [ ] **Step 1: Create tests.rs**

```rust
// Tests for GameplayScene — extracted from mod.rs for readability.
// All test helpers and test functions live here.

use super::*;
// [paste all #[cfg(test)] content here]
```

- [ ] **Step 2: Add to mod.rs**

```rust
#[cfg(test)]
mod tests;
```

Remove all `#[cfg(test)]` blocks from `mod.rs`.

- [ ] **Step 3: cargo test**

```bash
cargo test 2>&1 | tail -20
```

Expected: all tests pass (same count as before).

- [ ] **Step 4: Check mod.rs final size**

```bash
wc -l src/game/gameplay_scene/mod.rs
```

Expected: ~600–800 lines (struct definition, field declarations, `new()`, `init_from_library()`, small public accessors, and module declarations).

- [ ] **Step 5: Final commit**

```bash
git add src/game/gameplay_scene/
git commit -m "refactor: extract tests into gameplay_scene/tests.rs (~3565 lines)

gameplay_scene/mod.rs now contains only the struct definition, constructor,
and module declarations. Full split complete."
```

---

## Final verification

- [ ] **Run full test suite**

```bash
cargo test 2>&1 | tail -20
```

Expected: same test count as before the split, all passing.

- [ ] **Check module sizes**

```bash
wc -l src/game/gameplay_scene/*.rs | sort -rn
```

Expected: no file exceeds ~1000 lines (except tests.rs which is ~3565).

- [ ] **cargo clippy**

```bash
cargo clippy 2>&1 | grep -E 'warning|error' | head -20
```

Fix any new clippy warnings introduced by visibility changes.
