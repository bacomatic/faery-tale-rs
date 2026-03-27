# Bug Fix Plan — faery-tale-rs

**Date:** 2026-03-26
**Scope:** All 25 open bug/fix issues (#41, #97–#119), excluding rollup trackers #58/#59.
**Structure:** Single master spec, five sequential groups A→B→C→D→E.

---

## Overview

Issues are grouped by dependency and shared root cause. Each group must reach a passing test suite before the next begins. Groups A and B have no inter-group dependencies; C depends on B (actor dimming must work before foreground masking is meaningful); D depends on A (fatigue system must be clean) and C (doors need correct region detection); E depends on D (death sequence needs doors/regions) and B (needs `setmood`).

```
A (quick isolates)
B (lighting)        ← independent of A
C (sprite/render)   ← depends on B (actor dimming)
D (world systems)   ← depends on A (fatigue), C (doors need regions)
E (game systems)    ← depends on B (setmood), D (death needs doors/regions)
```

---

## Group A — Quick Isolates

**Issues:** #97, #98, #99, #100, #109, #111, #113, #114, #116

**Fix order within group:** #99 → #97/#98 (fatigue cleanup as a unit) → #100 → #116 → #113 → #114 → #109 → #111

### #99 — Remove dead code `tick_fatigue()`
Delete `GameState::tick_fatigue()` from `game_state.rs`. It is only referenced by its own unit test. Remove the test too.

### #97 + #98 — Fatigue system (fix as a unit)
- **#97:** `hunger_fatigue_step()` must not write to `self.fatigue` — fatigue during time ticks is already handled there; movement fatigue is `fatigue_step()`'s job. Remove the duplicate `self.fatigue += 1` from whichever function is wrong per the original's design (movement increments fatigue, time ticks increment hunger; both share the daynight-128-tick path for the warning messages only).
- **#98:** `fatigue_step()` returns `bool` (forced sleep) but the call site in the movement path discards it. Wire the return value: when `true`, set player state to sleeping (or queue the event — same as events 12/24 from `hunger_fatigue_step`).

### #100 — Safe spawn guards
Tighten `update_safe_spawn()` in `game_state.rs` to match `fmain.c`: only update when `region_num < 8` (outdoors), `battleflag == false`, and the current terrain is passable non-water. Remove any over-eager updates.

### #116 — Vitality cap and heal rate
- Change `HEAL_PERIOD` from 300 to 1024 in `game_state.rs`.
- Replace `.min(100)` with `.min(crate::game::magic::heal_cap(self.brave))` in the rest-healing block.
- Update the rest-heal guard condition to `self.vitality < heal_cap(self.brave)` (not `< 100`).
- Populate `HeroStats.max_vitality` from `heal_cap(state.brave)` wherever the debug snapshot is built in `debug_window.rs`.

### #113 — Magic menu refresh
After `SetInventory`, `AdjustInventory`, and `HeroPack` debug command handlers in `gameplay_scene.rs`, add:
```rust
let wealth = self.state.wealth;
self.menu.set_options(self.state.stuff(), wealth);
```
Also call `set_options` after any future `pickup_item()` / `drop_item()` call sites.

### #114 — Save slot selection
- Add `save_pending: bool` to `MenuState`.
- Change `(MenuMode::SaveX, 5)` to set `save_pending = true` and call `gomenu(MenuMode::File)`; return `MenuAction::None`.
- Change `(MenuMode::File, h)` to emit `MenuAction::SaveGame(h.saturating_sub(5) as u8)` when `save_pending`, else `MenuAction::LoadGame(h.saturating_sub(5) as u8)`; clear `save_pending`.
- Update `MenuAction::SaveGame` and `MenuAction::LoadGame` to carry `u8` slot parameter.
- Update `dispatch_menu_action` to pass slot to `persist::save_game` / `persist::load_game`.

### #109 — Scroll message filter
Audit all `self.messages.push()` call sites in `gameplay_scene.rs`. Any message string not sourced via `event_msg(&self.narr, …)` or a legitimate narr-derived speak call must be moved to `self.dlog()` or removed. Debug status strings ("Game saved.", "Music on.", etc.) are fine to keep as messages since they are user-facing confirmations.

### #111 — Title text 2× height
Add `render_string_hires<T: RenderTarget>(&self, s, canvas, x, y)` to `FontTexture` that sets the destination rect height to `y_size * 2` while keeping source rect at `y_size` (SDL2 scales on blit). Add a matching `draw_offset_hires` to `Placard`. Use it in `IntroPhase::TitleText` and `IntroPhase::TitleFadeOut` rendering.

**Acceptance criteria (Group A):**
- All existing tests pass.
- Passive rest heals to `15 + brave/4`, not 100.
- Saving via Quit→Save presents the A–H slot screen; loading respects the chosen slot.
- Magic menu shows items after `HeroPack` debug command.
- Title text in the intro is visibly taller.

---

## Group B — Lighting

**Issues:** #115, #119

### #115 + #119 — Day/night palette and jewel effect (fix as a unit)

**`game_state.rs`:**
- Init `daynight = 8000` (not 6000).
- Init `lightlevel = 300` explicitly (not computed on first tick).

**`gameplay_scene.rs` — day/night dimming path:**
Replace the `apply_lightlevel_dim()` call with `fade_page()`:
```rust
let light_on  = self.state.light_timer > 0;
let ll_boost  = if light_on { 200i32 } else { 0 };
let lightlevel = self.state.lightlevel as i32;

let r_pct = ((lightlevel - 80 + ll_boost) * 100 / 300).clamp(0, 100) as i16;
let g_pct = ((lightlevel - 61)            * 100 / 300).clamp(0, 100) as i16;
let b_pct = ((lightlevel - 62)            * 100 / 300).clamp(0, 100) as i16;

let faded = crate::game::palette_fader::fade_page(
    &base_palette, r_pct, g_pct, b_pct, true, light_on,
);
```
For indoors (`region_num >= 8`): use `fade_page(&base, 100, 100, 100, true, false)`.

**Atlas rebuild trigger:**
Add `last_light_on: bool` field alongside `last_lightlevel`. Rebuild when either `lightlevel` or `light_on` changes.

**Actor sprite dimming:**
When the atlas rebuild fires, also re-decode actor sprite pixels through the dimmed palette. Store the current dimmed palette and apply it when building sprite frame pixel buffers.

**Acceptance criteria (Group B):**
- Game starts at full brightness.
- At midnight outdoors, world has a visible blue cast (not black).
- With jewel active at midnight, world has a warm amber/reddish cast.
- Actors dim at the same rate as background tiles.
- Indoors always renders at full brightness.

---

## Group C — Sprite/Render

**Issues:** #108, #102, #104, #101, #41

**Fix order:** #108 → #102 → #104 → #101 → #41

### #108 — Equipped weapon sprite
Port `statelist[87]` from `fmain.c` as a Rust const array in `sprites.rs`:
```rust
struct StatEntry { figure: u8, wpn_no: u8, wpn_x: i8, wpn_y: i8 }
const STATELIST: [StatEntry; 87] = [ … ];
```
In the player sprite render pass, after blitting the body frame, look up `STATELIST[anim_index]` and blit the weapon frame from the OBJECTS sheet (cfile 3) at `(sprite_x + wpn_x, sprite_y + wpn_y)`. Apply k-offset per weapon: dirk +64, mace +32, sword +48. For bow, use `bow_x[facing] / bow_y[facing]` arrays during walk frames.

### #102 — NPC render offsets for carriers
In `compose_actors()`, when an actor's kind is `RAFT`, `CARRIER`, or `DRAGON`, apply the vertical offset from the original's carrier offset table before computing the blit destination. Currently the offset is 0 for all actor types.

### #104 — Foreground tile masking
Split each tile at atlas-build time into background and foreground layers based on the tile's terrain bitmask (bit indicating "draws over sprites"). During `compose()`:
1. Blit all background tiles.
2. Blit all actors/sprites.
3. Blit all foreground tiles on top.

Store the foreground/background split in `TileAtlas` and pass two separate layers to `MapRenderer`.

### #101 — Region 9 dungeon palette / `secret_timer`
When building the dimmed palette for region 9, check `state.secret_timer > 0`. If so, substitute the alternate palette entry for hidden-passage tiles (the entry that makes them visible). Add `secret_timer` to the atlas-rebuild trigger condition alongside `last_lightlevel` and `last_light_on`.

### #41 — Placard border clipping
In `PlacardRenderer::draw_segments()`, clamp `dx` and `dy` to the border bounding rect before issuing `canvas.draw_point()` or `canvas.draw_line()`. The left-side tail is caused by segment coordinates escaping the rect on the first block iteration.

**Acceptance criteria (Group C):**
- Player holds correct weapon sprite.
- Carrier-mounted NPCs sit at correct vertical position.
- Player walks behind foreground tiles (trees, walls).
- Hidden dungeon passages appear when Secret orb is active.
- Placard border animates cleanly within its box.

---

## Group D — World Systems

**Issues:** #96, #103, #105, #106, #107, #112, #118

**Fix order:** #96/#103 → #107 → #106 → #112 → #105 → #118

### #96 + #103 — Door/portal system (fix as a unit)
Populate `faery.toml` `[[doors]]` entries from the original's building entrance/exit coordinates. Each entry: `{ src_region, src_x, src_y, src_w, src_h, dst_region, dst_x, dst_y }`. Load via `game_library.rs`. The existing `doorfind()` call in the movement path already handles the transition — it just needs real data.

### #107 — Event zones
Add `[[zones]]` entries to `faery.toml` from the original's `zone_list[]`: `{ region, x, y, w, h, event_id }`. In the per-frame movement tick, check the player's position against the zone table. When entering a new zone, fire `event_msg(&self.narr, event_id, brother_name)` into the scroll area. Track `last_zone: Option<usize>` to avoid re-firing every frame.

### #106 — World items / objects on the ground
Add `WorldObject { item_id: u8, region: u8, x: u16, y: u16, visible: bool }` to game state. During map render, blit each visible object's sprite at its world position using the OBJECTS sheet. Wire the `Take` action: when player is within pickup range of an object, call `state.pickup_item(item_id)`, mark the object invisible, call `set_options()`. Dropping an item creates a new `WorldObject` at the player's position.

### #112 — Hunger/fatigue stagger and forced sleep
Three hooks in `gameplay_scene.rs`:
1. **Stagger:** After resolving movement direction, when `state.hunger > 120`, with 1-in-4 probability (`rand() & 3 == 0`), rotate `state.facing` ±1 in the 8-direction ring.
2. **Forced sleep transition:** In the tick event handler, map events 12 and 24 to `PlayerState::Sleeping` (set a field on the scene, freeze movement input).
3. **Sleep loop:** While sleeping, each frame: `state.daynight = (state.daynight + 63) % 24000`; `state.fatigue = state.fatigue.saturating_sub(1)`; check wake conditions (`fatigue == 0`, or `fatigue < 30 && daynight in 9000..10000`, or `battleflag && rand64() == 0`); on wake, clear `PlayerState::Sleeping`.

### #105 — Water submersion
Add a terrain-type check after movement resolves. When the player's tile is water terrain (type 2 or as defined in terra_mem):
- Apply a downward Y sprite offset (sinking).
- Switch to SINK animation frame set.
- Apply drowning damage on a timer (every N ticks, `vitality -= 1`).
- Skip this check when `state.on_raft` or `state.flying > 0`.

### #118 — Bird totem map overlay
Add `bigdraw(hero_x, hero_y, world) -> Vec<u32>` to `map_view.rs`:
- Iterate 18 columns × 9 rows of sectors centred on the current viewport.
- For each of the 16×8 tiles per sector, read `terra_mem[tile_idx * 4 + 3]` for the color byte; write one RGBA pixel.
- Returns a 288×72 pixel buffer.

In `render_by_viewstatus()` case 1:
- Blit the buffer into the playfield rect (2× scale).
- Draw `"+"` at computed hero map coordinates if in bounds.
- Call `render_hibar()`.
- On any keypress, set `state.viewstatus = 0`.

Add `region_num >= 8` guard in `magic.rs` ITEM_TOTEM handler.

**Acceptance criteria (Group D):**
- Walking into a building doorway transitions to the correct indoor region.
- "You have entered Tambry" and similar location messages fire on entry.
- Items appear on the ground; Take action picks them up and updates menus.
- Starving player staggers at `hunger > 120`; collapses to sleep at `hunger > 140`.
- Walking into water sinks the player and causes drowning; raft bypasses this.
- Bird totem renders a recognisable sector overview and dismisses on keypress.

---

## Group E — Game Systems

**Issues:** #117, #110

**Fix order:** #117 → #110

### #117 — Music and sound wiring

**Music toggle:**
- `ToggleMusic` OFF: call `resources.audio.as_ref().map(|a| a.stop_score())`.
- `ToggleMusic` ON: call `setmood(now=true)` (see below).

**`setmood()` helper** in `gameplay_scene.rs`:
```rust
fn setmood(&mut self, now: bool) {
    let group = if self.state.vitality == 0 { 6 }
        else if /* near Tambry gates */ { 4 }
        else if self.state.battleflag { 1 }
        else if self.state.region_num >= 8 { 5 }
        else if self.state.lightlevel > 120 { 0 }
        else { 2 };
    if let Some(audio) = resources.audio {
        if self.menu.is_music_on() {
            if now { audio.play_group(group, …) } else { audio.set_score(group as u8) }
        } else {
            audio.stop_score();
        }
    }
}
```
Call `setmood(false)` on region change, battleflag change, and lightlevel threshold crossing (120); call `setmood(true)` on music toggle-on and game load.

**Sound effects:**
Gate each `play_sfx()` call with `self.menu.is_sound_on()`. Add callsites for: combat hit (player and enemy), item use (magic, food), door open/close, spell cast.

### #110 — Death sequence
On `vitality <= 0` (and not god mode):
1. Set `PlayerState::Dying`; play death animation frames; call `setmood(true)` → group 6 (death music).
2. Decrement `goodfairy` counter from 255, 1 per frame (~8.5s at 30fps).
3. If `state.luck > 0` when counter reaches 0: animate faery sprite from right edge to player position, call `revive(false)` — teleport to safe spawn, `vitality = heal_cap(brave)`, `luck -= 5`, clear `PlayerState::Dying`, call `setmood(true)`.
4. If `state.luck == 0`: fade to placard, call `revive(true)` — activate next brother with fresh stats. If all three brothers are gone, show game-over screen and return `SceneResult::Done`.

**Acceptance criteria (Group E):**
- Music stops/starts on toggle; restarts from beginning on toggle-on.
- Song switches to battle music when `battleflag` is set and back when cleared.
- Death plays death animation and music; faery resurrects with luck penalty.
- Next brother activates when luck is gone; game over when all brothers are exhausted.

---

## Cross-cutting notes

- **Test suite:** Each group must leave `cargo test` clean before moving to the next.
- **`gameplay_scene.rs` size:** This file is very large and touches most groups. Where a group adds significant new logic (e.g., `setmood`, `bigdraw`, `PlayerState::Sleeping`), prefer extracting to a new module rather than growing the file further.
- **`faery.toml` data:** Groups D requires adding `[[doors]]` and `[[zones]]` entries. These are data work, not code work — can be done in parallel with code changes but must be complete before integration tests pass.
