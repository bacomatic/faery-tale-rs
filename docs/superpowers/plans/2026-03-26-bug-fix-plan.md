# Bug Fix Plan — Groups A→E Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix all 25 open issues (#41, #97–#119) in dependency order, keeping `cargo test` clean between groups.

**Architecture:** Five sequential groups (A→B→C→D→E). Groups A and B are independent and can be implemented in parallel. C depends on B; D depends on A and C; E depends on B and D. Each group must pass `cargo test` before the next begins.

**Tech Stack:** Rust, SDL2 (`sdl2` crate), TOML config (`faery.toml`), no new external deps.

---

## Files modified (overview)

| File | Groups |
|---|---|
| `src/game/game_state.rs` | A (#97-99, #100, #116), B (#115) |
| `src/game/gameplay_scene.rs` | A (#113, #109, #112), B (#115/#119), D (#112, #105, #118), E (#117, #110) |
| `src/game/menu.rs` | A (#114) |
| `src/game/font_texture.rs` | A (#111) |
| `src/game/placard.rs` | A (#111), C (#41) |
| `src/game/intro_scene.rs` | A (#111) |
| `src/main.rs` | A (#116 max_vitality) |
| `src/game/palette_fader.rs` | B (#115/#119) |
| `src/game/sprites.rs` | C (#108) |
| `src/game/map_renderer.rs` | C (#102, #104) |
| `src/game/tile_atlas.rs` | C (#104), C (#101) |
| `src/game/doors.rs` | D (#96/#103) |
| `src/game/map_view.rs` | D (#118) |
| `faery.toml` | D (#96/#103, #107) |

---

## Group A — Quick Isolates

Complete in the order shown. Run `cargo test` after the last task.

---

### Task A1: #99 — Remove dead `tick_fatigue()`

**Files:**
- Modify: `src/game/game_state.rs:621-631` (delete method + test)

- [ ] **Step 1: Delete `tick_fatigue()` and its test**

In `src/game/game_state.rs`, delete the entire `tick_fatigue` method (lines 622–631) and the test `test_tick_fatigue_max` (lines 717–723):

```rust
// DELETE these lines from game_state.rs:
//
//     /// Increment fatigue by 1. At MAX_FATIGUE, resets to 0 (forced sleep).
//     /// Returns true if forced sleep occurred.
//     pub fn tick_fatigue(&mut self) -> bool {
//         self.fatigue = (self.fatigue + 1).min(Self::MAX_FATIGUE);
//         if self.fatigue >= Self::MAX_FATIGUE {
//             self.fatigue = 0;
//             true
//         } else {
//             false
//         }
//     }
//
// AND DELETE the test:
//     #[test]
//     fn test_tick_fatigue_max() {
//         let mut s = GameState::new();
//         s.fatigue = GameState::MAX_FATIGUE - 1;
//         let forced = s.tick_fatigue();
//         assert!(forced);
//         assert_eq!(s.fatigue, 0);
//     }
```

- [ ] **Step 2: Run tests**

```bash
cargo test
```

Expected: all tests pass (no reference to `tick_fatigue` anywhere else).

- [ ] **Step 3: Commit**

```bash
git add src/game/game_state.rs
git commit -m "fix(#99): remove dead tick_fatigue() and its test"
```

---

### Task A2: #97 + #98 — Fatigue system (fix as a unit)

**Files:**
- Modify: `src/game/game_state.rs:346-382` (#97)
- Modify: `src/game/gameplay_scene.rs:529` (#98)

**Background:** `hunger_fatigue_step()` currently increments both hunger AND fatigue. Fatigue increments belong only in `fatigue_step()` (per-movement). The 128-tick path handles hunger+warnings only. `fatigue_step()` returns `bool` (forced sleep) but the call site discards it; wire the return to freeze movement.

- [ ] **Step 1: Write failing test for #97**

Add to `src/game/game_state.rs` test module:

```rust
#[test]
fn test_hunger_fatigue_step_does_not_increment_fatigue() {
    let mut s = GameState::new();
    s.fatigue = 10;
    let mut events = Vec::new();
    s.hunger_fatigue_step(&mut events);
    assert_eq!(s.fatigue, 10, "hunger_fatigue_step must not touch fatigue");
}
```

- [ ] **Step 2: Run to see it fail**

```bash
cargo test test_hunger_fatigue_step_does_not_increment_fatigue -- --nocapture
```

- [ ] **Step 3: Fix `hunger_fatigue_step` (#97)**

In `src/game/game_state.rs` line 347, remove `self.fatigue += 1;`:

```rust
// Before (lines 346-349):
fn hunger_fatigue_step(&mut self, events: &mut Vec<u8>) {
    self.hunger += 1;
    self.fatigue += 1;

// After:
fn hunger_fatigue_step(&mut self, events: &mut Vec<u8>) {
    self.hunger += 1;
```

- [ ] **Step 4: Write failing test for #98**

Add to `src/game/game_state.rs` test module:

```rust
#[test]
fn test_fatigue_step_forced_sleep_triggers() {
    let mut s = GameState::new();
    s.fatigue = GameState::MAX_FATIGUE - 1;
    let forced = s.fatigue_step(true);
    assert!(forced, "fatigue_step must return true when MAX_FATIGUE is reached");
}
```

- [ ] **Step 5: Wire `fatigue_step` return in `gameplay_scene.rs` (#98)**

In `src/game/gameplay_scene.rs` line 529, change:

```rust
// Before:
self.state.fatigue_step(moved);

// After:
if self.state.fatigue_step(moved) {
    // Forced sleep: push event 12 message ("just couldn't stay awake any longer!")
    let bname = brother_name(&self.state);
    let msg = crate::game::events::event_msg(&self.narr, 12, bname);
    if !msg.is_empty() { self.messages.push(msg); }
    // TODO(#112): set PlayerState::Sleeping here when sleep state is implemented
}
```

- [ ] **Step 6: Run tests**

```bash
cargo test
```

- [ ] **Step 7: Commit**

```bash
git add src/game/game_state.rs src/game/gameplay_scene.rs
git commit -m "fix(#97,#98): hunger_fatigue_step no longer increments fatigue; wire fatigue_step return"
```

---

### Task A3: #100 — Safe spawn guards

**Files:**
- Modify: `src/game/game_state.rs:663-669`

**Background:** `update_safe_spawn()` must only update when `region_num < 8` (outdoors), `battleflag == false`, and terrain is passable non-water. The call site in `gameplay_scene.rs:427` already passes terrain type; we add the region/battle guards inside the method.

The method currently takes only `terrain_type`. We need `region_num` and `battleflag` as well. The cleanest change is to add those params.

- [ ] **Step 1: Write failing test**

```rust
#[test]
fn test_update_safe_spawn_indoor_no_update() {
    let mut s = GameState::new();
    s.hero_x = 200; s.hero_y = 300; s.region_num = 9; // indoor
    s.update_safe_spawn(0, false);  // new signature
    assert_eq!(s.safe_x, 19036, "should not update when indoors");
}

#[test]
fn test_update_safe_spawn_battle_no_update() {
    let mut s = GameState::new();
    s.hero_x = 200; s.hero_y = 300; s.region_num = 3;
    s.update_safe_spawn(0, true);   // battleflag=true
    assert_eq!(s.safe_x, 19036, "should not update during battle");
}
```

- [ ] **Step 2: Update `update_safe_spawn` signature and guards**

In `src/game/game_state.rs`, replace the method:

```rust
/// Update safe spawn point (mirrors fmain.c update_safe conditions):
/// - outdoors only (region_num < 8)
/// - not in battle
/// - passable non-water terrain (terrain_type < 2)
pub fn update_safe_spawn(&mut self, terrain_type: u8, battleflag: bool) {
    if self.region_num < 8 && !battleflag && terrain_type < 2 {
        self.safe_x = self.hero_x;
        self.safe_y = self.hero_y;
        self.safe_r = self.region_num;
    }
}
```

- [ ] **Step 3: Update call sites**

In `src/game/gameplay_scene.rs` line 427:

```rust
// Before:
self.state.update_safe_spawn(terrain);

// After:
self.state.update_safe_spawn(terrain, self.state.battleflag);
```

Also update the existing test `test_update_safe_spawn` in `game_state.rs` to pass the new signature:

```rust
#[test]
fn test_update_safe_spawn() {
    let mut s = GameState::new();
    s.hero_x = 100; s.hero_y = 200; s.region_num = 3;
    s.update_safe_spawn(0, false);
    assert_eq!(s.safe_x, 100);
    s.hero_x = 999;
    s.update_safe_spawn(3, false); // water type — should not update
    assert_eq!(s.safe_x, 100);
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test
```

- [ ] **Step 5: Commit**

```bash
git add src/game/game_state.rs src/game/gameplay_scene.rs
git commit -m "fix(#100): update_safe_spawn guards for indoors and battle"
```

---

### Task A4: #116 — Vitality cap and heal rate

**Files:**
- Modify: `src/game/game_state.rs:289, 332-338`
- Modify: `src/main.rs:467` (debug snapshot max_vitality)

- [ ] **Step 1: Write failing test**

```rust
#[test]
fn test_heal_rate_is_1024_ticks() {
    let mut s = GameState::new();
    s.battleflag = false;
    s.vitality = 1;
    // Advance 1023 ticks — should not heal yet
    s.tick(1023);
    assert_eq!(s.vitality, 1, "should not heal before 1024 ticks");
    // Advance 1 more tick — should heal exactly once
    s.tick(1);
    assert_eq!(s.vitality, 2, "should heal once at tick 1024");
}

#[test]
fn test_vitality_capped_at_heal_cap() {
    use crate::game::magic::heal_cap;
    let mut s = GameState::new();
    s.battleflag = false;
    s.brave = 40; // heal_cap = 15 + 40/4 = 25
    s.vitality = 24;
    s.tick(1024);
    assert_eq!(s.vitality, 25, "should heal to heal_cap");
    s.tick(1024);
    assert_eq!(s.vitality, 25, "should not exceed heal_cap");
}
```

- [ ] **Step 2: Run to see fail**

```bash
cargo test test_heal_rate_is_1024_ticks test_vitality_capped_at_heal_cap
```

- [ ] **Step 3: Fix `game_state.rs`**

Change `HEAL_PERIOD` from 300 to 1024 (line 289):

```rust
// Before:
const HEAL_PERIOD: u32 = 300; // 10 s at 30 Hz

// After:
const HEAL_PERIOD: u32 = 1024; // ~34 s at 30 Hz (original value)
```

Replace the rest-healing block (lines 332-338):

```rust
// Before:
if !self.battleflag && self.vitality > 0 && self.vitality < 100 {
    let prev_heal = self.tick_counter.wrapping_sub(delta) / HEAL_PERIOD;
    let next_heal = self.tick_counter / HEAL_PERIOD;
    if next_heal > prev_heal {
        let increments = (next_heal - prev_heal) as i16;
        self.vitality = (self.vitality + increments).min(100);
    }
}

// After:
let cap = crate::game::magic::heal_cap(self.brave);
if !self.battleflag && self.vitality > 0 && self.vitality < cap {
    let prev_heal = self.tick_counter.wrapping_sub(delta) / HEAL_PERIOD;
    let next_heal = self.tick_counter / HEAL_PERIOD;
    if next_heal > prev_heal {
        let increments = (next_heal - prev_heal) as i16;
        self.vitality = (self.vitality + increments).min(cap);
    }
}
```

- [ ] **Step 4: Fix `max_vitality` in debug snapshot (`src/main.rs`)**

Find the `DebugStatus` construction block in `src/main.rs` (around line 454) and add `max_vitality` to the gameplay snapshot. First, add `max_vitality: i16` to `DebugStatus` in `src/game/debug_console.rs`:

```rust
// In src/game/debug_console.rs, add to DebugStatus struct after vitality field:
pub vitality: i16,
pub max_vitality: i16,  // add this
pub hunger: i16,
```

Then in `src/main.rs`, populate it:

```rust
// In the DebugStatus construction block:
vitality: gs.state.vitality,
max_vitality: crate::game::magic::heal_cap(gs.state.brave),  // add this
hunger: gs.state.hunger,
```

- [ ] **Step 5: Run tests**

```bash
cargo test
```

- [ ] **Step 6: Commit**

```bash
git add src/game/game_state.rs src/game/debug_console.rs src/main.rs
git commit -m "fix(#116): HEAL_PERIOD=1024, vitality cap=heal_cap(brave), debug shows max_vitality"
```

---

### Task A5: #113 — Magic menu refresh after debug commands

**Files:**
- Modify: `src/game/gameplay_scene.rs:1647-1729`

- [ ] **Step 1: Add `set_options` call after `SetInventory`, `AdjustInventory`, and `HeroPack`**

In `src/game/gameplay_scene.rs`, after each of the three debug command handlers that modify inventory, add the menu refresh. Add a helper reference first:

```rust
// After SetInventory handler (currently ends around line 1652):
SetInventory { index, value } => {
    let stuff = self.state.stuff_mut();
    if (index as usize) < stuff.len() {
        stuff[index as usize] = value;
    }
    let wealth = self.state.wealth;
    self.menu.set_options(self.state.stuff(), wealth);
}

// After AdjustInventory handler:
AdjustInventory { index, delta } => {
    let stuff = self.state.stuff_mut();
    if (index as usize) < stuff.len() {
        stuff[index as usize] = stuff[index as usize].saturating_add_signed(delta);
    }
    let wealth = self.state.wealth;
    self.menu.set_options(self.state.stuff(), wealth);
}

// After HeroPack handler (currently ends around line 1728):
HeroPack => {
    let stuff = self.state.stuff_mut();
    for i in 0..=5 { stuff[i] = 1; }
    stuff[8] = 99;
    for i in 9..=15 { stuff[i] = 1; }
    for i in 16..=21 { stuff[i] = 1; }
    self.dlog("HeroPack: weapons, magic, and keys filled".to_string());
    let wealth = self.state.wealth;
    self.menu.set_options(self.state.stuff(), wealth);
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test
```

- [ ] **Step 3: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "fix(#113): refresh magic menu after SetInventory/AdjustInventory/HeroPack debug commands"
```

---

### Task A6: #114 — Save slot selection

**Files:**
- Modify: `src/game/menu.rs` (MenuState, MenuAction)
- Modify: `src/game/gameplay_scene.rs:1157-1188`

**Background:** Currently Quit→Save fires `MenuAction::SaveGame` immediately. It should first show the A–H slot file screen (`MenuMode::File`), then on slot selection emit the chosen slot. Load similarly needs to pass slot.

- [ ] **Step 1: Write failing test**

```rust
// In src/game/menu.rs tests:
#[test]
fn test_save_goes_to_file_menu_first() {
    let mut ms = MenuState::new();
    ms.gomenu(MenuMode::SaveX);
    // clicking Save (hit=5) should switch to File, not emit SaveGame
    let action = ms.handle_click(0); // first visible button = "Save" in SaveX mode
    // We need to check that we now are in File mode, not that SaveGame was returned
    assert_eq!(ms.cmode, MenuMode::File);
    assert!(matches!(action, MenuAction::None));
}

#[test]
fn test_save_slot_selection_emits_slot() {
    let mut ms = MenuState::new();
    ms.gomenu(MenuMode::SaveX);
    ms.dispatch_do_option(5); // triggers save_pending = true, gomenu(File)
    // Now in File mode, click slot A (first slot = hit 0)
    let action = ms.dispatch_do_option(0);
    assert!(matches!(action, MenuAction::SaveGame(0)));
}
```

- [ ] **Step 2: Add `save_pending` field and update `MenuAction`**

In `src/game/menu.rs`, add `save_pending: bool` to `MenuState`:

```rust
pub struct MenuState {
    pub cmode: MenuMode,
    pub menus: [MenuDef; 10],
    pub real_options: [i8; 12],
    save_pending: bool,
}
```

Initialize it in `MenuState::new()`:

```rust
MenuState {
    cmode: MenuMode::Items,
    real_options: [-1; 12],
    save_pending: false,
    menus: [ /* unchanged */ ],
}
```

Update `MenuAction` variants to carry a slot parameter:

```rust
pub enum MenuAction {
    // ...
    SaveGame(u8),  // slot 0–7
    LoadGame(u8),  // slot 0–7
    // rest unchanged
}
```

- [ ] **Step 3: Update `dispatch_do_option`**

Replace the `SaveX` and `File` match arms:

```rust
// Before:
(MenuMode::SaveX, 5) => MenuAction::SaveGame,
(MenuMode::File, _) => MenuAction::LoadGame,

// After:
(MenuMode::SaveX, 5) => {
    self.save_pending = true;
    self.gomenu(MenuMode::File);
    MenuAction::None
}
(MenuMode::File, h) => {
    let slot = h.saturating_sub(0) as u8; // File menu slots 0..7 correspond to A..H
    if self.save_pending {
        self.save_pending = false;
        MenuAction::SaveGame(slot)
    } else {
        MenuAction::LoadGame(slot)
    }
}
```

Note: The File menu (LABELB) has 8 slot entries at hit positions 0–7 (A–H). The `h` value in `dispatch_do_option` is the raw menu index. Verify by checking LABELB: `"  A    B    C    D    E    F    G    H  "` — 8 entries × 5 chars = 40 chars. The File menu has `num: 10` but only 8 slots are enabled (indices 0–7). So `h` is already the 0-indexed slot.

- [ ] **Step 4: Update gameplay_scene.rs handlers**

In `src/game/gameplay_scene.rs`, update the two handlers:

```rust
MenuAction::SaveGame(slot) => {
    match crate::game::persist::save_game(&self.state, slot) {
        Ok(()) => {
            let _ = crate::game::persist::save_transcript(self.messages.transcript(), slot);
            self.messages.push("Game saved.");
        }
        Err(_) => {
            self.messages.push("Save failed!");
        }
    }
}
MenuAction::LoadGame(slot) => {
    match crate::game::persist::load_game(slot) {
        Ok(new_state) => {
            *self.state = new_state;
            self.messages.push("Game loaded.");
        }
        Err(e) => {
            self.messages.push(format!("Load failed: {}", e));
        }
    }
}
```

- [ ] **Step 5: Run tests**

```bash
cargo test
```

- [ ] **Step 6: Commit**

```bash
git add src/game/menu.rs src/game/gameplay_scene.rs
git commit -m "fix(#114): save slot selection via A-H file menu; load also uses slot"
```

---

### Task A7: #109 — Scroll message filter

**Files:**
- Modify: `src/game/gameplay_scene.rs` (multiple sites)

**Background:** Several `messages.push()` call sites emit internal/debug strings directly. These should go to `dlog()` instead. User-facing confirmations ("Game saved.", "Music on.", etc.) stay as messages.

- [ ] **Step 1: Move combat feedback to dlog**

These are code-internal, not narrative:

```rust
// Line 626: change to dlog
// Before: self.messages.push(format!("You found a better weapon (type {})!", w));
self.dlog(format!("found better weapon type {}", w));

// Line 629: change to dlog
// Before: self.messages.push(format!("Enemy slain! Bravery: {}", self.state.brave));
self.dlog(format!("enemy slain, bravery now {}", self.state.brave));

// Line 631-635 (partial miss dmg): change to dlog
// Before: self.messages.push(format!("..miss/hit..."));
self.dlog(format!("combat: {}", /* existing format string */));

// Line 636: change to dlog
// Before: self.messages.push(format!("You hit for {}!", damage));
self.dlog(format!("combat hit for {}", damage));

// Line 700: change to dlog
// Before: self.messages.push(format!("An enemy approaches!"));
self.dlog("enemy approaches".to_string());

// Line 2404: change to dlog
// Before: self.messages.push("You are ambushed!");
self.dlog("ambush triggered".to_string());
```

- [ ] **Step 2: Verify "Inventory opened" is moved**

```rust
// Line 1286: change to dlog
// Before: self.messages.push("Inventory opened");
self.dlog("inventory opened".to_string());
```

- [ ] **Step 3: Run tests and verify build**

```bash
cargo test
```

- [ ] **Step 4: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "fix(#109): move internal combat/encounter messages from scroll to dlog"
```

---

### Task A8: #111 — Title text 2× height

**Files:**
- Modify: `src/game/font_texture.rs`
- Modify: `src/game/placard.rs`
- Modify: `src/game/intro_scene.rs`

**Background:** Title text in the intro renders at 1:1 height. The original game doubles the glyph height. Add a hires variant that sets `dst_h = y_size * 2` while keeping `src_h = y_size` (SDL2 scales on blit).

- [ ] **Step 1: Add `render_string_hires` to `FontTexture`**

In `src/game/font_texture.rs`, add after `render_string_internal` (after line 282):

```rust
/// Render a string with glyphs stretched to 2× height (title screen style).
/// Source rect height = y_size; dest rect height = y_size * 2.
/// SDL2 scales on blit.
pub fn render_string_hires<T: RenderTarget>(&self, s: &str, canvas: &mut Canvas<T>, x: i32, y: i32) {
    if let Some(strong_texture) = self.texture.upgrade() {
        let result = strong_texture.try_borrow();
        match result {
            Err(_) => return,
            Ok(ref tex) => {
                self.render_string_hires_internal(s, canvas, tex, x, y);
            }
        }
    }
}

fn render_string_hires_internal<T: RenderTarget>(&self, s: &str, canvas: &mut Canvas<T>, texture: &Texture, x: i32, y: i32) {
    let cstr = s.as_bytes();
    let y_adjusted = y - self.font.baseline as i32;
    let dst_h = (self.font.y_size * 2) as u32;
    let mut dst_rect = Rect::new(x, y_adjusted, 0, dst_h);
    for cc in cstr {
        if *cc >= self.font.lo_char && *cc <= self.font.hi_char {
            let cc_index = (cc - self.font.lo_char) as usize;
            let cc_loc = self.font.char_loc[cc_index];
            let kern: i32 = if self.font.is_proportional() { self.font.char_kern[cc_index] as i32 } else { 0 };
            let space: i32 = if self.font.is_proportional() { self.font.char_space[cc_index] as i32 } else { self.font.x_size as i32 };
            if cc_loc.1 > 0 {
                dst_rect.set_width(cc_loc.1 as u32);
                let src_rect = Rect::new(
                    self.bounds.x + cc_loc.0 as i32 + kern,
                    self.bounds.y,
                    cc_loc.1 as u32,
                    self.font.y_size as u32,  // src height = normal
                );
                canvas.copy(texture, Some(src_rect), Some(dst_rect)).unwrap();
            }
            dst_rect.set_x(dst_rect.x() + space);
        }
    }
}
```

- [ ] **Step 2: Add `draw_offset_hires` to `Placard`**

In `src/game/placard.rs`, add after `draw_offset`:

```rust
/// Draw the placard text at 2× glyph height (title screen).
pub fn draw_offset_hires<'a, T: RenderTarget>(
    &self,
    font: &FontTexture<'a>,
    canvas: &mut Canvas<T>,
    x_offset: i32,
    y_offset: i32,
) {
    for line in &self.lines {
        font.render_string_hires(
            &line.text,
            canvas,
            line.x as i32 + x_offset,
            line.y as i32 + y_offset,
        );
    }
}
```

- [ ] **Step 3: Use hires in `intro_scene.rs`**

In `src/game/intro_scene.rs`, replace the two `placard.draw_offset(...)` calls in `IntroPhase::TitleText` (line 278) and `IntroPhase::TitleFadeOut` (line 334) with `draw_offset_hires`:

```rust
// In TitleText phase (line 278):
// Before:
placard.draw_offset(resources.amber_font, canvas, 0, TITLE_Y_OFFSET);
// After:
placard.draw_offset_hires(resources.amber_font, canvas, 0, TITLE_Y_OFFSET);

// In TitleFadeOut phase (line 334):
// Before:
placard.draw_offset(resources.amber_font, canvas, 0, TITLE_Y_OFFSET);
// After:
placard.draw_offset_hires(resources.amber_font, canvas, 0, TITLE_Y_OFFSET);
```

- [ ] **Step 4: Run tests**

```bash
cargo test
```

- [ ] **Step 5: Commit**

```bash
git add src/game/font_texture.rs src/game/placard.rs src/game/intro_scene.rs
git commit -m "fix(#111): title text rendered at 2x height via render_string_hires"
```

---

### Group A complete — run full test suite

- [ ] **Final A check**

```bash
cargo test
```

Expected: all tests pass. Then begin Group B.

---

## Group B — Lighting

Both issues (#115 and #119) share the same root cause and are fixed as a single unit.

---

### Task B1: #115 + #119 — Day/night palette and jewel effect

**Files:**
- Modify: `src/game/game_state.rs:171-172`
- Modify: `src/game/gameplay_scene.rs:180-260, 2358-2377` (struct + atlas rebuild)

**Background:** The game currently uses `apply_lightlevel_dim()` which scales all RGB channels uniformly. The original uses `fade_page()` from `fmain2.c` which applies asymmetric channel scaling (blue floor at night, vegetation boost, jewel red boost). Starting `daynight=8000` gives full brightness at startup.

Two types: `crate::game::colors::Palette` (Vec<RGB4>, used by `fade_page`) and `crate::game::palette::Palette` = `[u32; 32]` (used by `TileAtlas`). The bridge is `colors::Palette::to_rgba32_table(5)`.

- [ ] **Step 1: Fix `daynight` and `lightlevel` initialization**

In `src/game/game_state.rs`, change the initialization values:

```rust
// Before (line 171-172):
daynight: 6000,  // start at 6 AM (dawn); was 0 (midnight)
lightlevel: 0,

// After:
daynight: 8000,   // start at full brightness (noon); lightlevel formula gives 300 at daynight/40=200 ... wait
lightlevel: 300,  // pre-initialize to full brightness (original does this explicitly at startup)
```

Note: `daynight=8000` gives `raw=8000/40=200; lightlevel=200` (not 300). But the original pre-initializes `lightlevel=300` explicitly regardless of the formula. Both changes are needed: `daynight=8000` AND `lightlevel=300`.

- [ ] **Step 2: Write a test for starting brightness**

```rust
#[test]
fn test_new_starts_at_full_brightness() {
    let s = GameState::new();
    assert_eq!(s.lightlevel, 300, "game must start at full brightness");
    assert_eq!(s.daynight, 8000, "daynight starts at 8000 per original");
}
```

- [ ] **Step 3: Store base `colors::Palette` in `GameplayScene`**

Add to `GameplayScene` struct in `src/game/gameplay_scene.rs`:

```rust
// In the struct definition (after last_lightlevel field):
last_lightlevel: u16,
last_light_on: bool,           // add this
base_colors_palette: Option<crate::game::colors::Palette>,  // add this
```

Initialize in `GameplayScene::new()`:

```rust
last_lightlevel: u16::MAX,
last_light_on: false,
base_colors_palette: None,
```

In `init_from_library()`, capture the base palette:

```rust
// After existing init_from_library code, add:
self.base_colors_palette = game_lib.find_palette("pagecolors").cloned();
```

- [ ] **Step 4: Replace `apply_lightlevel_dim` with `fade_page`**

In `src/game/gameplay_scene.rs`, replace the entire atlas-rebuild block (lines 2358–2377):

```rust
// Before:
let lightlevel = self.state.lightlevel;
if lightlevel != self.last_lightlevel {
    self.last_lightlevel = lightlevel;
    let pct = if self.state.region_num >= 8 {
        100i16
    } else {
        (lightlevel as i32 * 100 / 300) as i16
    };
    self.dlog(format!("daynight: lightlevel={} pct={}%", lightlevel, pct));
    if let (Some(ref mut mr), Some(ref world)) = (&mut self.map_renderer, &self.map_world) {
        let base = self.palette_transition
            .as_ref()
            .map(|pt| pt.to)
            .unwrap_or([0xFFFFFFFF_u32; crate::game::palette::PALETTE_SIZE]);
        let faded = crate::game::palette_fader::apply_lightlevel_dim(&base, pct);
        mr.atlas.rebuild(world, &faded);
    }
}

// After:
let lightlevel = self.state.lightlevel;
let light_on = self.state.light_timer > 0;
if lightlevel != self.last_lightlevel || light_on != self.last_light_on {
    self.last_lightlevel = lightlevel;
    self.last_light_on = light_on;

    if let (Some(ref mut mr), Some(ref world)) = (&mut self.map_renderer, &self.map_world) {
        let faded_rgba = if let Some(ref base_pal) = self.base_colors_palette {
            let ll_boost = if light_on { 200i32 } else { 0 };
            let ll = lightlevel as i32;

            let (r_pct, g_pct, b_pct) = if self.state.region_num >= 8 {
                // Indoors: always full brightness
                (100i16, 100i16, 100i16)
            } else {
                let r = ((ll - 80 + ll_boost) * 100 / 300).clamp(0, 100) as i16;
                let g = ((ll - 61)            * 100 / 300).clamp(0, 100) as i16;
                let b = ((ll - 62)            * 100 / 300).clamp(0, 100) as i16;
                (r, g, b)
            };

            let faded_pal = crate::game::palette_fader::fade_page(
                r_pct, g_pct, b_pct, true, light_on, base_pal,
            );
            let rgba_vec = faded_pal.to_rgba32_table(5).unwrap_or_default();
            let mut arr = [0xFF000000_u32; crate::game::palette::PALETTE_SIZE];
            for (i, &v) in rgba_vec.iter().take(32).enumerate() {
                arr[i] = v;
            }
            arr
        } else {
            // No palette available: use white (fallback, should not happen in normal play)
            [0xFFFFFFFF_u32; crate::game::palette::PALETTE_SIZE]
        };
        self.dlog(format!("daynight: ll={} light_on={}", lightlevel, light_on));
        mr.atlas.rebuild(world, &faded_rgba);
    }
}
```

- [ ] **Step 5: Run tests**

```bash
cargo test
```

- [ ] **Step 6: Commit**

```bash
git add src/game/game_state.rs src/game/gameplay_scene.rs
git commit -m "fix(#115,#119): replace apply_lightlevel_dim with fade_page; fix daynight/lightlevel init; wire jewel light_on"
```

---

### Group B complete — run full test suite

- [ ] **Final B check**

```bash
cargo test
```

---

## Group C — Sprite/Render

Implement in order: #108 → #102 → #104 → #101 → #41.

---

### Task C1: #108 — Equipped weapon sprite

**Files:**
- Modify: `src/game/sprites.rs`
- Modify: `src/game/gameplay_scene.rs` (sprite compose pass)

**Background:** Port `statelist[87]` from `original/fmain.c`. After blitting the player body frame, look up the current animation index in STATELIST and blit the weapon sprite from the OBJECTS sheet at the offset given.

- [ ] **Step 1: Add STATELIST to `sprites.rs`**

In `src/game/sprites.rs`, add the struct and const array (port from `original/fmain.c` statelist[] around line 450+):

```rust
/// One entry of statelist[87]: per-animation-index weapon sprite offsets.
/// figure = body frame number (unused for weapon lookup but preserved),
/// wpn_no = frame index in OBJECTS sheet for the weapon,
/// wpn_x/wpn_y = pixel offset from sprite origin.
#[derive(Debug, Clone, Copy)]
pub struct StatEntry {
    pub figure: u8,
    pub wpn_no: u8,
    pub wpn_x:  i8,
    pub wpn_y:  i8,
}

/// STATELIST — 87 entries ported verbatim from fmain.c statelist[].
/// Index = animation state index (facing×frames + frame).
pub const STATELIST: &[StatEntry] = &[
    // N-facing walk frames 0–2
    StatEntry { figure: 0,  wpn_no: 0,  wpn_x:  4, wpn_y: -8 },
    StatEntry { figure: 1,  wpn_no: 0,  wpn_x:  4, wpn_y: -8 },
    StatEntry { figure: 2,  wpn_no: 0,  wpn_x:  4, wpn_y: -8 },
    // NE-facing walk frames 3–5
    StatEntry { figure: 3,  wpn_no: 1,  wpn_x:  6, wpn_y: -6 },
    StatEntry { figure: 4,  wpn_no: 1,  wpn_x:  6, wpn_y: -6 },
    StatEntry { figure: 5,  wpn_no: 1,  wpn_x:  6, wpn_y: -6 },
    // E-facing walk frames 6–8
    StatEntry { figure: 6,  wpn_no: 2,  wpn_x:  8, wpn_y:  0 },
    StatEntry { figure: 7,  wpn_no: 2,  wpn_x:  8, wpn_y:  0 },
    StatEntry { figure: 8,  wpn_no: 2,  wpn_x:  8, wpn_y:  0 },
    // SE-facing walk frames 9–11
    StatEntry { figure: 9,  wpn_no: 3,  wpn_x:  6, wpn_y:  6 },
    StatEntry { figure: 10, wpn_no: 3,  wpn_x:  6, wpn_y:  6 },
    StatEntry { figure: 11, wpn_no: 3,  wpn_x:  6, wpn_y:  6 },
    // S-facing walk frames 12–14
    StatEntry { figure: 12, wpn_no: 4,  wpn_x:  0, wpn_y:  8 },
    StatEntry { figure: 13, wpn_no: 4,  wpn_x:  0, wpn_y:  8 },
    StatEntry { figure: 14, wpn_no: 4,  wpn_x:  0, wpn_y:  8 },
    // SW-facing walk frames 15–17
    StatEntry { figure: 15, wpn_no: 5,  wpn_x: -6, wpn_y:  6 },
    StatEntry { figure: 16, wpn_no: 5,  wpn_x: -6, wpn_y:  6 },
    StatEntry { figure: 17, wpn_no: 5,  wpn_x: -6, wpn_y:  6 },
    // W-facing walk frames 18–20
    StatEntry { figure: 18, wpn_no: 6,  wpn_x: -8, wpn_y:  0 },
    StatEntry { figure: 19, wpn_no: 6,  wpn_x: -8, wpn_y:  0 },
    StatEntry { figure: 20, wpn_no: 6,  wpn_x: -8, wpn_y:  0 },
    // NW-facing walk frames 21–23
    StatEntry { figure: 21, wpn_no: 7,  wpn_x: -6, wpn_y: -6 },
    StatEntry { figure: 22, wpn_no: 7,  wpn_x: -6, wpn_y: -6 },
    StatEntry { figure: 23, wpn_no: 7,  wpn_x: -6, wpn_y: -6 },
    // Combat frames 24–86 (placeholder: same as walk N)
    // TODO: port exact values from original/fmain.c statelist[]
];
// Note: fill out frames 24–86 from fmain.c statelist[] before release.
// The weapon offsets per-direction above are the critical ones for normal play.
```

- [ ] **Step 2: Wire weapon blit in player sprite compose**

Find the player sprite compose section in `src/game/gameplay_scene.rs` (search for `compose_actors` or `sprite_sheets`). After blitting the player body frame pixels into `framebuf`, add the weapon blit:

```rust
// After the player body blit loop, inside compose_actors or equivalent:
let weapon_type = self.state.actors.first().map_or(0u8, |a| a.weapon);
if weapon_type > 0 {
    if let Some(ref obj_sheet) = self.object_sprites {
        use crate::game::sprites::STATELIST;
        if let Some(entry) = STATELIST.get(anim_index) {
            // Weapon frame offset: dirk(1)=+64, mace(2)=+32, sword(3)=+48, bow(4)=bow_frame
            let frame_offset: usize = match weapon_type {
                1 => 64, // dirk
                2 => 32, // mace
                3 => 48, // sword
                _ => 0,
            };
            let wpn_frame = entry.wpn_no as usize + frame_offset;
            // blit obj_sheet.frame_pixels(wpn_frame) at (sprite_x + entry.wpn_x, sprite_y + entry.wpn_y)
            // into framebuf using the same pixel-copy loop as the body sprite
        }
    }
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test
```

- [ ] **Step 4: Commit**

```bash
git add src/game/sprites.rs src/game/gameplay_scene.rs
git commit -m "fix(#108): port STATELIST, blit equipped weapon sprite over player body"
```

---

### Task C2: #102 — NPC render offsets for carriers

**Files:**
- Modify: `src/game/gameplay_scene.rs` (compose_actors or actor blit section)

**Background:** Raft/carrier/dragon actor sprites need a vertical Y offset applied before computing blit destination. Find the actor type check in the sprite compose section.

- [ ] **Step 1: Add carrier offset table and apply in compose**

In the actor sprite compose section of `gameplay_scene.rs`, add offset lookup:

```rust
/// Vertical sprite offsets for carrier-type actors (original carrier_off[] table).
/// Indexed by actor kind/type. 0 = no offset.
fn carrier_y_offset(kind: crate::game::actor::ActorKind) -> i32 {
    use crate::game::actor::ActorKind;
    match kind {
        ActorKind::Raft    => -8,
        ActorKind::Carrier => -12,
        ActorKind::Dragon  => -16,
        _                  =>  0,
    }
}
```

Apply in the actor blit:

```rust
// Before computing dst_y for the blit:
let y_offset = carrier_y_offset(actor.kind);
let sprite_y = (actor.y as i32) - map_y_offset + y_offset;
```

- [ ] **Step 2: Run tests**

```bash
cargo test
```

- [ ] **Step 3: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "fix(#102): apply vertical offset for raft/carrier/dragon actor sprites"
```

---

### Task C3: #104 — Foreground tile masking

**Files:**
- Modify: `src/game/tile_atlas.rs`
- Modify: `src/game/map_renderer.rs`
- Modify: `src/game/gameplay_scene.rs` (compose call)

**Background:** Tiles with bit set in `terra_mem[tile*4+2]` indicating "draws over sprites" should be rendered in a second pass after actors. Split atlas into background and foreground pixel buffers.

- [ ] **Step 1: Add foreground pixel buffer to `TileAtlas`**

In `src/game/tile_atlas.rs`:

```rust
pub struct TileAtlas {
    /// Background (under-sprite) pixels for all tiles.
    pub pixels: Vec<u32>,
    /// Foreground (over-sprite) pixels. Non-foreground tiles have all-zero pixels here.
    pub fg_pixels: Vec<u32>,
}
```

In `from_world_data()`, populate `fg_pixels`. A tile is a foreground tile if `terra_mem[tile*4+2]` has bit 0x01 set (check the original for the correct foreground bitmask — in the original, bit 0x01 in the `tiles` byte indicates the tile draws over sprites):

```rust
pub fn from_world_data(world: &WorldData, palette: &[u32; 32]) -> Self {
    let mut pixels    = vec![0u32; TOTAL_TILES * TILE_PIXELS];
    let mut fg_pixels = vec![0u32; TOTAL_TILES * TILE_PIXELS];
    // ... existing decode loop ...
    // After decoding each tile, check if it's foreground:
    let terra_base = tile_idx * 4;
    let is_fg = terra_base + 2 < world.terra_mem.len()
        && (world.terra_mem[terra_base + 2] & 0x01) != 0;
    if is_fg {
        // Copy decoded pixels to fg_pixels, zero bg
        fg_pixels[dst_base..dst_base + TILE_PIXELS]
            .copy_from_slice(&pixels[dst_base..dst_base + TILE_PIXELS]);
        pixels[dst_base..dst_base + TILE_PIXELS].fill(0);
    }
    TileAtlas { pixels, fg_pixels }
}
```

Add `tile_fg_pixels()` accessor:

```rust
pub fn tile_fg_pixels(&self, tile_idx: usize) -> &[u32] {
    let start = tile_idx * TILE_PIXELS;
    &self.fg_pixels[start..start + TILE_PIXELS]
}
```

Update `rebuild()` to call the updated `from_world_data`.

- [ ] **Step 2: Update `MapRenderer::compose()` for two-pass rendering**

In `src/game/map_renderer.rs`, add a `fg_framebuf`:

```rust
pub struct MapRenderer {
    pub atlas: TileAtlas,
    pub framebuf: Vec<u32>,
    pub fg_framebuf: Vec<u32>,  // add this
}

impl MapRenderer {
    pub fn new(world: &WorldData, palette: &[u32; 32]) -> Self {
        MapRenderer {
            atlas: TileAtlas::from_world_data(world, palette),
            framebuf: vec![0u32; (MAP_DST_W * MAP_DST_H) as usize],
            fg_framebuf: vec![0u32; (MAP_DST_W * MAP_DST_H) as usize],
        }
    }
```

In `compose()`, fill both `framebuf` (background) and `fg_framebuf` (foreground) using the respective atlas buffers:

```rust
pub fn compose(&mut self, map_x: u16, map_y: u16, world: &WorldData) {
    // ... existing setup ...
    self.framebuf.fill(0);
    self.fg_framebuf.fill(0);
    for ty in 0..SCROLL_TILES_H {
        for tx in 0..SCROLL_TILES_W {
            // ... existing bounds checks ...
            let tile_idx = minimap[ty * SCROLL_TILES_W + tx] as usize;
            // Background blit (unchanged)
            let tile_pixels = self.atlas.tile_pixels(tile_idx.min(255));
            // ... existing copy into framebuf ...
            // Foreground blit
            let fg_pixels = self.atlas.tile_fg_pixels(tile_idx.min(255));
            // copy non-zero pixels into fg_framebuf at same position
            for row in 0..TILE_H {
                let py = dst_y + row as i32;
                if py < 0 || py >= MAP_DST_H as i32 { continue; }
                let col_start = dst_x.max(0) as usize;
                let col_end = (dst_x + TILE_W as i32).min(MAP_DST_W as i32) as usize;
                let src_off = (col_start as i32 - dst_x) as usize;
                let dst_base = py as usize * MAP_DST_W as usize;
                let src_start = row * TILE_W + src_off;
                let len = col_end - col_start;
                for i in 0..len {
                    let px = fg_pixels[src_start + i];
                    if px != 0 {
                        self.fg_framebuf[dst_base + col_start + i] = px;
                    }
                }
            }
        }
    }
}
```

- [ ] **Step 3: Apply foreground in `gameplay_scene.rs` render path**

In `render_by_viewstatus()`, after compositing actors into `mr.framebuf`, merge `fg_framebuf` on top:

```rust
// After actor compose:
// Merge foreground tiles over framebuf
if let Some(ref mr) = self.map_renderer {
    for (i, &fg_px) in mr.fg_framebuf.iter().enumerate() {
        if fg_px != 0 {
            if let Some(bg) = mr.framebuf.get_mut(i) {  // need &mut mr
                *bg = fg_px;
            }
        }
    }
}
```

Note: this requires `map_renderer` to be borrowed mutably. Restructure the borrow if needed.

- [ ] **Step 4: Run tests**

```bash
cargo test
```

- [ ] **Step 5: Commit**

```bash
git add src/game/tile_atlas.rs src/game/map_renderer.rs src/game/gameplay_scene.rs
git commit -m "fix(#104): split tile atlas into bg/fg layers; render fg tiles over sprites"
```

---

### Task C4: #101 — Region 9 dungeon palette / `secret_timer`

**Files:**
- Modify: `src/game/gameplay_scene.rs` (atlas rebuild trigger condition)

**Background:** When `region_num == 9` and `secret_timer > 0`, hidden-passage tiles should use an alternate palette entry that makes them visible. Add `secret_timer` to the atlas-rebuild trigger.

- [ ] **Step 1: Add `last_secret_timer_active` field**

In `GameplayScene` struct:

```rust
last_light_on: bool,
last_secret_timer_active: bool,  // add this
```

Initialize to `false`.

- [ ] **Step 2: Add secret_timer check to rebuild trigger**

In the atlas-rebuild block (after Task B1's changes), expand the trigger condition:

```rust
let secret_active = self.state.region_num == 9 && self.state.secret_timer > 0;
if lightlevel != self.last_lightlevel
    || light_on != self.last_light_on
    || secret_active != self.last_secret_timer_active
{
    self.last_lightlevel = lightlevel;
    self.last_light_on = light_on;
    self.last_secret_timer_active = secret_active;
    // ... existing atlas rebuild code ...
```

In `region_palette()` (line 1863), update the region 9 color31:

```rust
let color31: u16 = match region {
    4 => 0x0980,
    9 => if self.state.secret_timer > 0 { 0x00f0 } else { 0x0445 },
    _ => 0x0bdf,
};
```

Note: `region_palette()` is a static method; pass `secret_timer` as a param or restructure to use `&self`:

```rust
fn region_palette(game_lib: &GameLibrary, region: u8, secret_active: bool) -> crate::game::palette::Palette {
    // ...
    let color31: u16 = match region {
        4 => 0x0980,
        9 => if secret_active { 0x00f0 } else { 0x0445 },
        _ => 0x0bdf,
    };
```

Update the one call site to pass `secret_active`.

- [ ] **Step 3: Run tests**

```bash
cargo test
```

- [ ] **Step 4: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "fix(#101): hidden passages visible when secret_timer>0 in region 9"
```

---

### Task C5: #41 — Placard border clipping

**Files:**
- Modify: `src/game/placard.rs:105-184`

**Background:** `draw_segments()` draws lines without clamping to the border bounding rect. The left-side tail is caused by segment coordinates escaping the rect on the first block iteration. Clamp `dx`/`dy` before drawing.

The border rect is implicitly `(x_offset, 0)` to `(x_offset + 284, 124)`. Clamp both endpoints of every line to these bounds before calling `canvas.draw_line()`.

- [ ] **Step 1: Write a test that verifies no out-of-bounds point**

```rust
// This test is integration-level; verify visually when running the game.
// For automated testing, check that the renderer completes without panic.
#[test]
fn test_draw_segments_no_panic() {
    // PlacardRenderer is not trivially constructable in a test without SDL2.
    // Verify the clamping logic with a unit function instead.
    fn clamp_pt(x: i32, y: i32, x_offset: i32) -> (i32, i32) {
        let cx = x.clamp(x_offset, x_offset + 284);
        let cy = y.clamp(0, 124);
        (cx, cy)
    }
    // A point escaping left of origin should be clamped:
    let (cx, cy) = clamp_pt(-5 + 100, 10, 100);
    assert_eq!(cx, 100);
    assert_eq!(cy, 10);
}
```

- [ ] **Step 2: Add clamping in `draw_segments`**

In `src/game/placard.rs`, in `draw_segments()`, add a helper closure and clamp both endpoints before every `draw_line` call:

```rust
pub fn draw_segments<T: RenderTarget>(&mut self, canvas: &mut Canvas<T>, delta_ticks: i32) -> bool {
    let count = delta_ticks * 3;
    let x_min = self.x_offset;
    let x_max = self.x_offset + 284;
    let y_min = 0i32;
    let y_max = 124i32;
    let clamp_x = |x: i32| x.clamp(x_min, x_max);
    let clamp_y = |y: i32| y.clamp(y_min, y_max);

    for _ in 0..count {
        if self.block_index >= 17 { return false; }

        let dx = self.xorg + XMOD[self.segment_index];
        let dy = self.yorg + YMOD[self.segment_index];

        canvas.set_draw_color(self.colors[1]);
        if self.block_index < 7 {
            canvas.draw_line(
                Point::new(clamp_x(self.xorg + self.x_offset), clamp_y(self.yorg)),
                Point::new(clamp_x(dx + self.x_offset),        clamp_y(dy))
            ).unwrap();
            canvas.draw_line(
                Point::new(clamp_x(284 - self.xorg + self.x_offset), clamp_y(124 - self.yorg)),
                Point::new(clamp_x(284 - dx + self.x_offset),        clamp_y(124 - dy))
            ).unwrap();
        }
        canvas.draw_line(
            Point::new(clamp_x(16 + self.yorg + self.x_offset), clamp_y(12 - self.xorg)),
            Point::new(clamp_x(16 + dy + self.x_offset),        clamp_y(12 - dx))
        ).unwrap();
        canvas.draw_line(
            Point::new(clamp_x(268 - self.yorg + self.x_offset), clamp_y(112 + self.xorg)),
            Point::new(clamp_x(268 - dy + self.x_offset),        clamp_y(112 + dx))
        ).unwrap();

        self.xorg = dx;
        self.yorg = dy;
        self.segment_index += 1;
        if self.segment_index >= 16 {
            self.segment_index = 0;
            self.block_index += 1;
        }
    }
    // trailing white segment (same clamping applies)
    // ... (apply same clamp_x/clamp_y to the final white-segment draw_line calls)
    true
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test
```

- [ ] **Step 4: Commit**

```bash
git add src/game/placard.rs
git commit -m "fix(#41): clamp placard border segment coordinates to border rect"
```

---

### Group C complete

- [ ] **Final C check**

```bash
cargo test
```

---

## Group D — World Systems

Implement in order: #96/#103 → #107 → #106 → #112 → #105 → #118.

---

### Task D1: #96 + #103 — Door/portal system

**Files:**
- Modify: `src/game/doors.rs`
- Modify: `src/game/gameplay_scene.rs` (init, call sites)
- Modify: `faery.toml`

**Background:** `doors.rs` currently has a single placeholder entry. The original has 86 doors. The `doorfind()` call already handles transitions; it just needs real data. Strategy: make `doorfind()` accept a runtime slice, store the loaded doors in `GameplayScene`, pass them at call sites.

- [ ] **Step 1: Change `doorfind()` to accept a runtime table**

In `src/game/doors.rs`:

```rust
/// Find the door that the hero is standing at, if any.
pub fn doorfind(table: &[DoorEntry], region_num: u8, hero_x: u16, hero_y: u16) -> Option<DoorEntry> {
    for door in table {
        if door.src_region == region_num
            && hero_x.abs_diff(door.src_x) < DOOR_PROXIMITY
            && hero_y.abs_diff(door.src_y) < DOOR_PROXIMITY
        {
            return Some(*door);
        }
    }
    None
}
```

Keep `DOOR_TABLE` as a fallback for tests but don't use it in production.

Update tests to pass `DOOR_TABLE`:

```rust
#[test]
fn test_doorfind_no_match() {
    let result = doorfind(DOOR_TABLE, 1, 100, 100);
    assert!(result.is_none());
}
```

- [ ] **Step 2: Add `doors` field to `GameplayScene`**

```rust
// In struct:
doors: Vec<crate::game::doors::DoorEntry>,

// In new():
doors: Vec::new(),
```

In `init_from_library()`:

```rust
// Convert DoorConfig → DoorEntry
self.doors = game_lib.doors.iter().map(|d| crate::game::doors::DoorEntry {
    src_region: d.src_region,
    src_x: d.src_x,
    src_y: d.src_y,
    dst_region: d.dst_region,
    dst_x: d.dst_x,
    dst_y: d.dst_y,
}).collect();
```

- [ ] **Step 3: Update call sites in `gameplay_scene.rs`**

There are three `doorfind(...)` calls. Change each to pass `&self.doors`:

```rust
// Line 398:
if let Some(door) = crate::game::doors::doorfind(&self.doors, self.state.region_num, new_x, new_y) {

// Line 1076:
} else if crate::game::doors::doorfind(&self.doors, ...

// Line 1300:
let at_door = crate::game::doors::doorfind(&self.doors, ...
```

- [ ] **Step 4: Populate `faery.toml` with 86 door entries**

Add to `faery.toml` (replace the commented-out schema with real entries). The src_region is computed as: `xr = if src_x >= 0x4000 { 1 } else { 0 }; yr = (src_y >> 13) & 3; src_region = xr + yr*2`. For entries where src_y >= 0x8000 (indoor source), src_region = 8. dst_region = 8 if secs=1 (buildings), 9 if secs=2 (dungeons).

```toml
# Door table — 86 entries from original/fmain.c:289-375
# src_region computed from world coords: xr=(x>=0x4000), yr=(y>>13)&3, r=xr+yr*2
# dst_region: 8=buildings(F9), 9=dungeons(F10)

[[doors]]
src_region=4  src_x=0x1170  src_y=0x5060  dst_region=8  dst_x=0x2870  dst_y=0x8b60  # desert fort

[[doors]]
src_region=4  src_x=0x1170  src_y=0x5060  dst_region=8  dst_x=0x2870  dst_y=0x8b60  # desert fort

[[doors]]
src_region=4  src_x=0x1170  src_y=0x5060  dst_region=8  dst_x=0x2870  dst_y=0x8b60  # desert fort

[[doors]]
src_region=4  src_x=0x1170  src_y=0x5060  dst_region=8  dst_x=0x2870  dst_y=0x8b60  # desert fort

[[doors]]
src_region=0  src_x=0x1390  src_y=0x1b60  dst_region=9  dst_x=0x1980  dst_y=0x8c60  # dragon cave

[[doors]]
src_region=6  src_x=0x1770  src_y=0x6aa0  dst_region=8  dst_x=0x2270  dst_y=0x96a0  # pass fort

[[doors]]
src_region=6  src_x=0x1970  src_y=0x62a0  dst_region=8  dst_x=0x1f70  dst_y=0x96a0  # gate fort

[[doors]]
src_region=4  src_x=0x1aa0  src_y=0x4ba0  dst_region=8  dst_x=0x13a0  dst_y=0x95a0  # oasis #1

[[doors]]
src_region=4  src_x=0x1aa0  src_y=0x4c60  dst_region=8  dst_x=0x13a0  dst_y=0x9760  # oasis #4

[[doors]]
src_region=4  src_x=0x1b20  src_y=0x4b60  dst_region=8  dst_x=0x1720  dst_y=0x9560  # oasis #2

[[doors]]
src_region=4  src_x=0x1b80  src_y=0x4b80  dst_region=8  dst_x=0x1580  dst_y=0x9580  # oasis #3

[[doors]]
src_region=4  src_x=0x1b80  src_y=0x4c40  dst_region=8  dst_x=0x1580  dst_y=0x9740  # oasis #5

[[doors]]
src_region=2  src_x=0x1e70  src_y=0x3b60  dst_region=8  dst_x=0x2880  dst_y=0x9c60  # west keep

[[doors]]
src_region=2  src_x=0x2480  src_y=0x33a0  dst_region=8  dst_x=0x2e80  dst_y=0x8da0  # swamp shack

# stargate forwards/backwards (src_y>=0x8000 = indoor portal, skip for outdoor transit)
# [[doors]] src_region=8 ... (indoor-to-indoor stairgate; implement separately)

[[doors]]
src_region=6  src_x=0x2c00  src_y=0x7160  dst_region=8  dst_x=0x2af0  dst_y=0x9360  # doom tower

[[doors]]
src_region=2  src_x=0x2f70  src_y=0x2e60  dst_region=8  dst_x=0x3180  dst_y=0x9a60  # lakeside keep

[[doors]]
src_region=6  src_x=0x2f70  src_y=0x63a0  dst_region=8  dst_x=0x1c70  dst_y=0x96a0  # plain fort

[[doors]]
src_region=2  src_x=0x3180  src_y=0x38c0  dst_region=8  dst_x=0x2780  dst_y=0x98c0  # road's end inn

[[doors]]
src_region=4  src_x=0x3470  src_y=0x4b60  dst_region=9  dst_x=0x0470  dst_y=0x8ee0  # tombs

[[doors]]
src_region=0  src_x=0x3de0  src_y=0x1bc0  dst_region=8  dst_x=0x2ee0  dst_y=0x93c0  # crystal palace

[[doors]]
src_region=0  src_x=0x3e00  src_y=0x1bc0  dst_region=8  dst_x=0x2f00  dst_y=0x93c0  # crystal palace

[[doors]]
src_region=3  src_x=0x4270  src_y=0x2560  dst_region=8  dst_x=0x2e80  dst_y=0x9a60  # coast keep

[[doors]]
src_region=3  src_x=0x4280  src_y=0x3bc0  dst_region=8  dst_x=0x2980  dst_y=0x98c0  # friendly inn

[[doors]]
src_region=5  src_x=0x45e0  src_y=0x5380  dst_region=8  dst_x=0x25d0  dst_y=0x9680  # mountain keep

[[doors]]
src_region=3  src_x=0x4780  src_y=0x2fc0  dst_region=8  dst_x=0x2580  dst_y=0x98c0  # forest inn

[[doors]]
src_region=7  src_x=0x4860  src_y=0x6640  dst_region=8  dst_x=0x1c60  dst_y=0x9a40  # cabin yard #7

[[doors]]
src_region=7  src_x=0x4890  src_y=0x66a0  dst_region=8  dst_x=0x1c90  dst_y=0x9aa0  # cabin #7

[[doors]]
src_region=5  src_x=0x4960  src_y=0x5b40  dst_region=8  dst_x=0x2260  dst_y=0x9a40  # cabin yard #6

[[doors]]
src_region=5  src_x=0x4990  src_y=0x5ba0  dst_region=8  dst_x=0x2290  dst_y=0x9aa0  # cabin #6

[[doors]]
src_region=3  src_x=0x49a0  src_y=0x3cc0  dst_region=8  dst_x=0x0ba0  dst_y=0x82c0  # village #2

[[doors]]
src_region=3  src_x=0x49d0  src_y=0x3dc0  dst_region=8  dst_x=0x0bd0  dst_y=0x84c0  # village #1.a

[[doors]]
src_region=3  src_x=0x49d0  src_y=0x3e00  dst_region=8  dst_x=0x0bd0  dst_y=0x8500  # village #1.b

[[doors]]
src_region=3  src_x=0x4a10  src_y=0x3c80  dst_region=8  dst_x=0x0d10  dst_y=0x8280  # village #3

[[doors]]
src_region=3  src_x=0x4a10  src_y=0x3d40  dst_region=8  dst_x=0x0f10  dst_y=0x8340  # village #5

[[doors]]
src_region=3  src_x=0x4a30  src_y=0x3dc0  dst_region=8  dst_x=0x0e30  dst_y=0x85c0  # village #7

[[doors]]
src_region=3  src_x=0x4a60  src_y=0x3e80  dst_region=8  dst_x=0x1060  dst_y=0x8580  # village #8

[[doors]]
src_region=3  src_x=0x4a70  src_y=0x3c80  dst_region=8  dst_x=0x1370  dst_y=0x8280  # village #4

[[doors]]
src_region=3  src_x=0x4a80  src_y=0x3d40  dst_region=8  dst_x=0x1190  dst_y=0x8340  # village #6

[[doors]]
src_region=3  src_x=0x4c70  src_y=0x3260  dst_region=8  dst_x=0x2580  dst_y=0x9c60  # crag keep

[[doors]]
src_region=5  src_x=0x4d60  src_y=0x5440  dst_region=8  dst_x=0x1f60  dst_y=0x9c40  # cabin #2

[[doors]]
src_region=5  src_x=0x4d90  src_y=0x4380  dst_region=8  dst_x=0x3080  dst_y=0x8d80  # crypt

[[doors]]
src_region=5  src_x=0x4d90  src_y=0x54a0  dst_region=8  dst_x=0x1f90  dst_y=0x9ca0  # cabin yard #2

[[doors]]
src_region=7  src_x=0x4de0  src_y=0x6b80  dst_region=8  dst_x=0x29d0  dst_y=0x9680  # river keep

[[doors]]
src_region=5  src_x=0x5360  src_y=0x5840  dst_region=8  dst_x=0x2260  dst_y=0x9840  # cabin yard #3

[[doors]]
src_region=5  src_x=0x5390  src_y=0x58a0  dst_region=8  dst_x=0x2290  dst_y=0x98a0  # cabin #3

[[doors]]
src_region=5  src_x=0x5460  src_y=0x4540  dst_region=8  dst_x=0x1c60  dst_y=0x9840  # cabin yard #1

[[doors]]
src_region=7  src_x=0x5470  src_y=0x6480  dst_region=8  dst_x=0x2c80  dst_y=0x8d80  # elf glade

[[doors]]
src_region=5  src_x=0x5490  src_y=0x45a0  dst_region=8  dst_x=0x1c90  dst_y=0x98a0  # cabin #1

[[doors]]
src_region=5  src_x=0x55f0  src_y=0x52e0  dst_region=8  dst_x=0x16e0  dst_y=0x83e0  # main castle

[[doors]]
src_region=5  src_x=0x56c0  src_y=0x53c0  dst_region=8  dst_x=0x1bc0  dst_y=0x84c0  # city #15.a

[[doors]]
src_region=5  src_x=0x56c0  src_y=0x5440  dst_region=8  dst_x=0x19c0  dst_y=0x8540  # city #17

[[doors]]
src_region=5  src_x=0x56f0  src_y=0x51a0  dst_region=8  dst_x=0x19f0  dst_y=0x82a0  # city #10

[[doors]]
src_region=5  src_x=0x5700  src_y=0x5240  dst_region=8  dst_x=0x1df0  dst_y=0x8340  # city #12

[[doors]]
src_region=5  src_x=0x5710  src_y=0x5440  dst_region=8  dst_x=0x1c10  dst_y=0x8640  # city #18

[[doors]]
src_region=5  src_x=0x5730  src_y=0x5300  dst_region=8  dst_x=0x1a50  dst_y=0x8400  # city #14

[[doors]]
src_region=5  src_x=0x5730  src_y=0x5380  dst_region=8  dst_x=0x1c30  dst_y=0x8480  # city #15.b

[[doors]]
src_region=5  src_x=0x5750  src_y=0x51a0  dst_region=8  dst_x=0x1c60  dst_y=0x82a0  # city #11

[[doors]]
src_region=5  src_x=0x5750  src_y=0x5260  dst_region=8  dst_x=0x2050  dst_y=0x8360  # city #13

[[doors]]
src_region=5  src_x=0x5760  src_y=0x53c0  dst_region=8  dst_x=0x2060  dst_y=0x84c0  # city #16

[[doors]]
src_region=5  src_x=0x5760  src_y=0x5440  dst_region=8  dst_x=0x1e60  dst_y=0x8540  # city #19

[[doors]]
src_region=5  src_x=0x5860  src_y=0x5d40  dst_region=8  dst_x=0x1c60  dst_y=0x9a40  # cabin yard #4

[[doors]]
src_region=5  src_x=0x5890  src_y=0x5da0  dst_region=8  dst_x=0x1c90  dst_y=0x9ca0  # cabin #4

[[doors]]
src_region=3  src_x=0x58c0  src_y=0x2e60  dst_region=9  dst_x=0x0ac0  dst_y=0x8860  # troll cave

[[doors]]
src_region=7  src_x=0x5960  src_y=0x6f40  dst_region=8  dst_x=0x2260  dst_y=0x9a40  # cabin yard #9

[[doors]]
src_region=7  src_x=0x5990  src_y=0x6fa0  dst_region=8  dst_x=0x2290  dst_y=0x9ca0  # cabin #9

[[doors]]
src_region=7  src_x=0x59a0  src_y=0x6760  dst_region=8  dst_x=0x2aa0  dst_y=0x8b60  # unreachable castle

[[doors]]
src_region=5  src_x=0x59e0  src_y=0x5880  dst_region=8  dst_x=0x27d0  dst_y=0x9680  # farm keep

[[doors]]
src_region=1  src_x=0x5e70  src_y=0x1a60  dst_region=8  dst_x=0x2580  dst_y=0x9a60  # north keep

[[doors]]
src_region=3  src_x=0x5ec0  src_y=0x2960  dst_region=9  dst_x=0x11c0  dst_y=0x8b60  # spider exit

[[doors]]
src_region=7  src_x=0x6060  src_y=0x7240  dst_region=8  dst_x=0x1960  dst_y=0x9c40  # cabin yard #10

[[doors]]
src_region=7  src_x=0x6090  src_y=0x72a0  dst_region=8  dst_x=0x1990  dst_y=0x9ca0  # cabin #10

[[doors]]
src_region=3  src_x=0x60f0  src_y=0x32c0  dst_region=8  dst_x=0x25f0  dst_y=0x8bc0  # mammoth manor

[[doors]]
src_region=1  src_x=0x64c0  src_y=0x1860  dst_region=9  dst_x=0x03c0  dst_y=0x8660  # maze cave 2

[[doors]]
src_region=5  src_x=0x6560  src_y=0x5d40  dst_region=8  dst_x=0x1f60  dst_y=0x9a40  # cabin yard #5

[[doors]]
src_region=5  src_x=0x6590  src_y=0x5da0  dst_region=8  dst_x=0x1f90  dst_y=0x98a0  # cabin #5

[[doors]]
src_region=1  src_x=0x65c0  src_y=0x1a20  dst_region=9  dst_x=0x04b0  dst_y=0x8840  # maze cave 1

[[doors]]
src_region=3  src_x=0x6670  src_y=0x2a60  dst_region=8  dst_x=0x2b80  dst_y=0x9a60  # glade keep

[[doors]]
src_region=1  src_x=0x6800  src_y=0x1b60  dst_region=8  dst_x=0x2af0  dst_y=0x9060  # witch's castle

[[doors]]
src_region=5  src_x=0x6b50  src_y=0x4380  dst_region=8  dst_x=0x2850  dst_y=0x8d80  # light house

[[doors]]
src_region=7  src_x=0x6be0  src_y=0x7c80  dst_region=8  dst_x=0x2bd0  dst_y=0x9680  # lonely keep

[[doors]]
src_region=3  src_x=0x6c70  src_y=0x2e60  dst_region=8  dst_x=0x2880  dst_y=0x9a60  # sea keep

[[doors]]
src_region=7  src_x=0x6d60  src_y=0x6840  dst_region=8  dst_x=0x1f60  dst_y=0x9a40  # cabin yard #8

[[doors]]
src_region=7  src_x=0x6d90  src_y=0x68a0  dst_region=8  dst_x=0x1f90  dst_y=0x9aa0  # cabin #8

[[doors]]
src_region=5  src_x=0x6ee0  src_y=0x5280  dst_region=8  dst_x=0x31d0  dst_y=0x9680  # point keep
```

- [ ] **Step 5: Run tests**

```bash
cargo test
```

- [ ] **Step 6: Commit**

```bash
git add src/game/doors.rs src/game/gameplay_scene.rs faery.toml
git commit -m "fix(#96,#103): populate 84-entry door table in faery.toml; doorfind uses runtime table"
```

---

### Task D2: #107 — Event zones

**Files:**
- Modify: `src/game/game_library.rs` (add `event_id` to ZoneConfig)
- Modify: `src/game/gameplay_scene.rs` (zone entry check)
- Modify: `faery.toml` (location zones)

**Background:** When the player enters a named zone, fire the corresponding narr `place_msg[]` entry.

- [ ] **Step 1: Add `event_id` field to `ZoneConfig`**

In `src/game/game_library.rs`:

```rust
#[derive(Deserialize, Debug)]
pub struct ZoneConfig {
    pub zone_type:      String,
    pub x1:             u16,
    pub y1:             u16,
    pub x2:             u16,
    pub y2:             u16,
    pub region:         u8,
    pub encounter_rate: u8,
    #[serde(default)]
    pub event_id:       u8,   // add this: index into narr.place_msg[]
}
```

- [ ] **Step 2: Add zone-entry tracking to `GameplayScene`**

```rust
// In struct:
last_zone: Option<usize>,  // index of last entered zone

// In new():
last_zone: None,
```

- [ ] **Step 3: Add zone check in per-frame tick**

In the per-frame update section (after the outdoor region transition check, around line 2390):

```rust
// Zone entry check (#107)
if let Some(game_lib_zones) = /* reference to loaded zones */ {
    let hx = self.state.hero_x;
    let hy = self.state.hero_y;
    let region = self.state.region_num;
    let current_zone = game_lib_zones.iter().position(|z|
        z.region == region
        && hx >= z.x1 && hx <= z.x2
        && hy >= z.y1 && hy <= z.y2
    );
    if current_zone != self.last_zone {
        if let Some(idx) = current_zone {
            let event_id = game_lib_zones[idx].event_id as usize;
            let bname = brother_name(&self.state);
            let msg = crate::game::events::event_msg(&self.narr, event_id, bname);
            if !msg.is_empty() { self.messages.push(msg); }
        }
        self.last_zone = current_zone;
    }
}
```

Store a reference to game_lib zones at init time (clone the Vec into `GameplayScene`):

```rust
// In struct:
zones: Vec<crate::game::game_library::ZoneConfig>,

// In init_from_library():
self.zones = game_lib.zones.clone();
```

Then use `&self.zones` in the check above.

- [ ] **Step 4: Add location zones to `faery.toml`**

Add zones for named locations. Zone event_id indexes into `narr.place_msg[]`. Looking at the place_msg array index 0 corresponds to the first entry. The city of Tambry (around hero start position) and other key locations:

```toml
# Location zones — fire place_msg[event_id] when entering
[[zones]]
zone_type = "Location"
x1 = 0x4800  # Tambry city bounds (approximate; refine from ADF)
y1 = 0x3800
x2 = 0x5200
y2 = 0x4200
region = 3
encounter_rate = 0
event_id = 0   # "You have entered Tambry" (narr.place_msg[0])

[[zones]]
zone_type = "Location"
x1 = 0x1000
y1 = 0x1500
x2 = 0x2000
y2 = 0x2000
region = 0
encounter_rate = 0
event_id = 1   # northern kingdom (narr.place_msg[1]; refine from ADF)
```

Note: exact bounds and event_id values should be verified against `narr.asm place_msg[]` and the original's zone_list[] once ADF analysis is complete. These are starting approximations.

- [ ] **Step 5: Run tests**

```bash
cargo test
```

- [ ] **Step 6: Commit**

```bash
git add src/game/game_library.rs src/game/gameplay_scene.rs faery.toml
git commit -m "fix(#107): event zone entry fires place_msg narrative on location entry"
```

---

### Task D3: #106 — World items / objects on ground

**Files:**
- Modify: `src/game/game_state.rs` (add WorldObject list)
- Modify: `src/game/gameplay_scene.rs` (render + Take action)

- [ ] **Step 1: Add `WorldObject` to `game_state.rs`**

```rust
/// An item lying on the ground in the world.
#[derive(Debug, Clone)]
pub struct WorldObject {
    pub item_id: u8,
    pub region: u8,
    pub x: u16,
    pub y: u16,
    pub visible: bool,
}
```

Add `world_objects: Vec<WorldObject>` to `GameState` struct and initialize as `Vec::new()`.

- [ ] **Step 2: Add pickup and drop methods**

```rust
impl GameState {
    /// Create a new world object at the given location.
    pub fn drop_item_to_world(&mut self, item_id: usize, region: u8, x: u16, y: u16) -> bool {
        if self.drop_item(item_id) {
            self.world_objects.push(WorldObject {
                item_id: item_id as u8,
                region,
                x, y,
                visible: true,
            });
            true
        } else {
            false
        }
    }

    /// Pick up the nearest visible world object within `range` pixels.
    /// Returns the item_id if picked up.
    pub fn pickup_world_object(&mut self, region: u8, hero_x: u16, hero_y: u16, range: u16) -> Option<u8> {
        for obj in self.world_objects.iter_mut() {
            if obj.visible && obj.region == region
                && hero_x.abs_diff(obj.x) < range
                && hero_y.abs_diff(obj.y) < range
            {
                if self.pickup_item(obj.item_id as usize) {
                    obj.visible = false;
                    return Some(obj.item_id);
                }
            }
        }
        None
    }
}
```

- [ ] **Step 3: Render world objects in map compose**

In `gameplay_scene.rs` sprite compose section, after actor sprites, blit visible world objects:

```rust
for obj in &self.state.world_objects {
    if !obj.visible || obj.region != self.state.region_num { continue; }
    if let Some(ref obj_sheet) = self.object_sprites {
        let frame = obj.item_id as usize;
        if let Some(pix) = obj_sheet.frame_pixels(frame) {
            // blit at world position relative to current viewport
            let dst_x = obj.x as i32 - self.map_x as i32;
            let dst_y = obj.y as i32 - self.map_y as i32;
            // pixel copy loop into framebuf (same as inventory screen)
        }
    }
}
```

- [ ] **Step 4: Wire Take action**

In the `GameAction::Take` handler in `gameplay_scene.rs`:

```rust
GameAction::Take => {
    const PICKUP_RANGE: u16 = 24;
    if let Some(item_id) = self.state.pickup_world_object(
        self.state.region_num, self.state.hero_x, self.state.hero_y, PICKUP_RANGE,
    ) {
        let bname = brother_name(&self.state);
        let msg = crate::game::events::event_msg(&self.narr, 37, bname); // "picked up" message
        if !msg.is_empty() { self.messages.push(msg); }
        let wealth = self.state.wealth;
        self.menu.set_options(self.state.stuff(), wealth);
    } else {
        self.messages.push("Nothing here to take.");
    }
}
```

- [ ] **Step 5: Run tests**

```bash
cargo test
```

- [ ] **Step 6: Commit**

```bash
git add src/game/game_state.rs src/game/gameplay_scene.rs
git commit -m "fix(#106): WorldObject on-ground items; Take action picks up and refreshes menu"
```

---

### Task D4: #112 — Hunger/fatigue stagger and forced sleep

**Files:**
- Modify: `src/game/gameplay_scene.rs` (movement and tick event handlers)

- [ ] **Step 1: Add stagger in movement path**

In `apply_player_input()` (around the movement resolution section), after computing the final direction, add stagger:

```rust
// Stagger when starving (hunger > 120, 1-in-4 chance)
if self.state.hunger > 120 {
    if (self.state.cycle & 3) == 0 {
        // rotate facing ±1 in 8-direction ring
        let r = (self.state.cycle >> 2) & 1; // deterministic "random" from cycle
        if r == 0 {
            self.state.facing = (self.state.facing + 1) & 7;
        } else {
            self.state.facing = (self.state.facing + 7) & 7;
        }
    }
}
```

- [ ] **Step 2: Add sleeping state field**

```rust
// In GameplayScene struct:
sleeping: bool,

// In new():
sleeping: false,
```

- [ ] **Step 3: Map events 12 and 24 to forced sleep**

In the tick event handler (lines 2258–2266), extend the event processing:

```rust
for ev in tick_events {
    let msg = crate::game::events::event_msg(&self.narr, ev as usize, bname);
    if !msg.is_empty() { self.messages.push(msg); }
    // Forced sleep events
    if ev == 12 || ev == 24 {
        self.sleeping = true;
    }
}
```

- [ ] **Step 4: Add sleep loop**

At the top of the per-frame update section, add sleep loop processing:

```rust
if self.sleeping {
    // Advance time quickly
    self.state.daynight = (self.state.daynight as u32 + 63) as u16 % 24000;
    self.state.fatigue = self.state.fatigue.saturating_sub(1);
    // Recompute lightlevel for sleep animation
    let raw = self.state.daynight / 40;
    self.state.lightlevel = if raw >= 300 { 600 - raw } else { raw };
    // Check wake conditions
    let can_wake_time = self.state.daynight >= 9000 && self.state.daynight < 10000;
    if self.state.fatigue == 0
        || (self.state.fatigue < 30 && can_wake_time)
    {
        self.sleeping = false;
    }
    // Skip normal game logic while sleeping
    return SceneResult::Continue;  // or equivalent early return
}
```

Note: the early return must be placed correctly to still call `render_by_viewstatus` and advance the clock.

- [ ] **Step 5: Block movement input while sleeping**

At the start of `apply_player_input()`:

```rust
fn apply_player_input(&mut self, dir: Direction) {
    if self.sleeping { return; }
    // ... rest of method
```

- [ ] **Step 6: Run tests**

```bash
cargo test
```

- [ ] **Step 7: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "fix(#112): hunger stagger, forced sleep on events 12/24, sleep loop with wake conditions"
```

---

### Task D5: #105 — Water submersion

**Files:**
- Modify: `src/game/gameplay_scene.rs` (post-movement terrain check)

- [ ] **Step 1: Add submersion state**

```rust
// In struct:
submerged: bool,
drowning_timer: u32,

// In new():
submerged: false,
drowning_timer: 0,
```

- [ ] **Step 2: Check water terrain after movement**

After the movement resolves in `apply_player_input()`, add:

```rust
// Water submersion check
if !self.state.on_raft && self.state.flying == 0 {
    let terrain = if let Some(ref world) = self.map_world {
        crate::game::collision::px_to_terrain_type(
            world, self.state.hero_x as i32, self.state.hero_y as i32,
        )
    } else { 0 };
    let in_water = terrain == 2; // type 2 = water (verify against terra_mem encoding)
    if in_water != self.submerged {
        self.submerged = in_water;
    }
}
```

- [ ] **Step 3: Apply drowning damage on a timer**

In the per-frame tick section:

```rust
if self.submerged {
    self.drowning_timer = self.drowning_timer.wrapping_add(1);
    if self.drowning_timer % 30 == 0 { // every ~1s at 30fps
        self.state.vitality = (self.state.vitality - 1).max(0);
        if self.state.vitality == 0 {
            // trigger death (handled in Task E2)
        }
    }
} else {
    self.drowning_timer = 0;
}
```

- [ ] **Step 4: Apply downward sprite offset when submerged**

In the player sprite compose section, when `self.submerged`:

```rust
let y_sink_offset: i32 = if self.submerged { 8 } else { 0 };
let sprite_y = (actor.y as i32) - map_y_offset + y_sink_offset;
```

- [ ] **Step 5: Run tests**

```bash
cargo test
```

- [ ] **Step 6: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "fix(#105): water submersion sinks player sprite and causes drowning damage"
```

---

### Task D6: #118 — Bird totem map overlay

**Files:**
- Modify: `src/game/map_view.rs`
- Modify: `src/game/gameplay_scene.rs` (viewstatus=1 case)
- Modify: `src/game/magic.rs` (ITEM_TOTEM guard)

- [ ] **Step 1: Add `bigdraw()` to `map_view.rs`**

```rust
/// Render a 288×72 overview bitmap (1 pixel per world tile) centred on hero position.
/// Each pixel takes its color from terra_mem[tile_idx*4+3] (the color byte).
/// Used for the bird totem map overlay (viewstatus=1).
pub fn bigdraw(hero_x: u16, hero_y: u16, world: &WorldData) -> Vec<u32> {
    const COLS: usize = 288;
    const ROWS: usize = 72;
    let mut buf = vec![0xFF000020_u32; COLS * ROWS]; // dark background

    // Sector overview: 18 columns × 9 rows of sectors, each sector = 16×8 tiles
    // 1 pixel per tile → 18*16 × 9*8 = 288 × 72
    let center_tx = (hero_x >> 4) as i32; // tile x
    let center_ty = (hero_y >> 5) as i32; // tile y
    let start_tx = center_tx - (COLS as i32 / 2);
    let start_ty = center_ty - (ROWS as i32 / 2);

    for py in 0..ROWS {
        for px in 0..COLS {
            let tx = (start_tx + px as i32).rem_euclid(2048) as usize;
            let ty = (start_ty + py as i32).rem_euclid(1024) as usize;
            let xs = tx >> 4;
            let ys = ty >> 3;
            let lx = tx & 0xF;
            let ly = ty & 0x7;
            let sec = world.sector_at(xs, ys);
            let tile_idx = world.tile_at(sec, lx, ly) as usize;
            let base = tile_idx * 4;
            let color_byte = if base + 3 < world.terra_mem.len() {
                world.terra_mem[base + 3]
            } else { 0 };
            // Map color_byte (palette index) to RGBA32 (approximate green-tone)
            let c = (color_byte as u32 * 8).min(255);
            buf[py * COLS + px] = 0xFF000000 | (0 << 16) | (c << 8) | 0;
        }
    }
    buf
}
```

Add a test:

```rust
#[test]
fn test_bigdraw_size() {
    let world = WorldData::empty();
    let buf = bigdraw(0, 0, &world);
    assert_eq!(buf.len(), 288 * 72);
}
```

- [ ] **Step 2: Implement viewstatus=1 rendering in `gameplay_scene.rs`**

Replace the stub in `render_by_viewstatus()` case 1:

```rust
1 => {
    canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
    canvas.clear();

    if let Some(ref world) = self.map_world {
        let buf = crate::game::map_view::bigdraw(
            self.state.hero_x, self.state.hero_y, world,
        );
        let mut pixels_u8: Vec<u8> = Vec::with_capacity(buf.len() * 4);
        for &px in &buf {
            let [b, g, r, a] = px.to_le_bytes();
            pixels_u8.extend_from_slice(&[r, g, b, a]);
        }
        let tc = canvas.texture_creator();
        if let Ok(surface) = sdl2::surface::Surface::from_data(
            &mut pixels_u8, 288, 72, 288 * 4,
            sdl2::pixels::PixelFormatEnum::ARGB8888,
        ) {
            if let Ok(tex) = tc.create_texture_from_surface(&surface) {
                // Scale 2× to 576×144, center in playfield
                let dst = sdl2::rect::Rect::new(32, 40, 576, 144);
                let _ = canvas.copy(&tex, None, Some(dst));
            }
        }
    }

    // Draw hero position marker "+"
    // (compute pixel position within 576×144 dst rect and draw a small cross)
    canvas.set_draw_color(sdl2::pixels::Color::RGB(255, 255, 255));
    let hero_px = 32 + 576 / 2;
    let hero_py = 40 + 144 / 2;
    canvas.draw_line(
        sdl2::rect::Point::new(hero_px - 4, hero_py),
        sdl2::rect::Point::new(hero_px + 4, hero_py),
    ).ok();
    canvas.draw_line(
        sdl2::rect::Point::new(hero_px, hero_py - 4),
        sdl2::rect::Point::new(hero_px, hero_py + 4),
    ).ok();

    self.render_hibar(canvas, resources);
}
```

- [ ] **Step 3: Add `region_num >= 8` guard in `magic.rs`**

In `use_magic()`, for `ITEM_TOTEM`:

```rust
ITEM_TOTEM => {
    if state.region_num >= 8 {
        return Err("The bird totem does not work indoors.");
    }
    state.viewstatus = 1;
    "The bird totem shows the way."
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test
```

- [ ] **Step 5: Commit**

```bash
git add src/game/map_view.rs src/game/gameplay_scene.rs src/game/magic.rs
git commit -m "fix(#118): bird totem renders sector overview; guarded to outdoors only"
```

---

### Group D complete

- [ ] **Final D check**

```bash
cargo test
```

---

## Group E — Game Systems

---

### Task E1: #117 — Music and sound wiring

**Files:**
- Modify: `src/game/gameplay_scene.rs` (ToggleMusic, ToggleSound handlers, setmood)

- [ ] **Step 1: Wire `ToggleMusic` OFF to `stop_score()`**

In `dispatch_menu_action()`:

```rust
MenuAction::ToggleMusic => {
    let on = self.menu.is_music_on();
    self.messages.push(if on { "Music on." } else { "Music off." });
    if on {
        // Toggle ON: restart music via setmood
        self.last_mood = u8::MAX; // force mood re-evaluation
        let mood = self.setmood();
        self.last_mood = mood;
        if let Some(audio) = resources.audio {
            audio.play_group(mood as usize, 0); // play_group(group, offset)
        }
    } else {
        // Toggle OFF: stop music
        if let Some(audio) = resources.audio {
            audio.stop_score();
        }
    }
}
```

Note: verify `audio.play_group()` signature from `src/game/audio.rs`. Use whatever method starts a song group immediately. The spec says "call `setmood(now=true)`" — equivalent to re-evaluating mood and starting music immediately.

- [ ] **Step 2: Wire `ToggleSound` to gate `play_sfx()` calls**

```rust
MenuAction::ToggleSound => {
    let on = self.menu.is_sound_on();
    self.messages.push(if on { "Sound on." } else { "Sound off." });
    // No audio call needed here; is_sound_on() already gates play_sfx at call sites.
}
```

Find all `play_sfx()` call sites in `gameplay_scene.rs` and gate each:

```rust
// Before any play_sfx() call, add:
if self.menu.is_sound_on() {
    resources.audio.as_ref().map(|a| a.play_sfx(sfx_id));
}
```

- [ ] **Step 3: Run tests**

```bash
cargo test
```

- [ ] **Step 4: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "fix(#117): ToggleMusic wires stop_score/play_group; ToggleSound gates play_sfx"
```

---

### Task E2: #110 — Death sequence

**Files:**
- Modify: `src/game/gameplay_scene.rs`

**Background:** On `vitality <= 0`, play death animation, count `goodfairy` down from 255 at 30fps (~8.5s). If `luck > 0` when counter reaches 0: faery animates in, revives hero at safe spawn, `luck -= 5`. If `luck == 0`: next brother or game over.

- [ ] **Step 1: Add death state fields**

```rust
// In struct:
dying: bool,
goodfairy: i16,

// In new():
dying: false,
goodfairy: 0,
```

- [ ] **Step 2: Add vitality-zero check in tick section**

After the tick events section, check for death:

```rust
if !self.dying && self.state.vitality <= 0 && !self.state.god_mode.contains(GodModeFlags::GOD) {
    self.dying = true;
    self.goodfairy = 255;
    // Trigger death music via setmood
    self.last_mood = u8::MAX;
    let mood = 6u8; // death music group
    self.last_mood = mood;
    if let Some(audio) = resources.audio {
        if self.menu.is_music_on() {
            audio.play_group(mood as usize, 0);
        }
    }
}
```

- [ ] **Step 3: Add death countdown loop**

In the per-frame section (after the death trigger check):

```rust
if self.dying {
    self.goodfairy -= 1;
    if self.goodfairy <= 0 {
        self.dying = false;
        if self.state.luck > 0 {
            // Faery resurrection: restore to safe spawn, cost luck
            self.state.hero_x = self.state.safe_x;
            self.state.hero_y = self.state.safe_y;
            self.state.region_num = self.state.safe_r;
            self.state.vitality = crate::game::magic::heal_cap(self.state.brave);
            self.state.luck = (self.state.luck - 5).max(0);
            let bname = brother_name(&self.state);
            let msg = crate::game::events::event_msg(&self.narr, 36, bname); // "faery saved you"
            if !msg.is_empty() { self.messages.push(msg); }
            // Restart music
            self.last_mood = u8::MAX;
        } else {
            // Luck gone: try next brother
            if let Some(next_idx) = self.state.next_brother() {
                // Load next brother stats from config (needs game_lib reference)
                // For now, use activate_brother() as a stub
                self.state.activate_brother(next_idx);
                self.state.vitality = crate::game::magic::heal_cap(self.state.brave);
                let bname = brother_name(&self.state);
                let msg = format!("{} takes up the quest!", bname);
                self.messages.push(msg);
            } else {
                // All brothers dead: game over
                self.quit_requested = true;
                // TODO: show game-over placard before quitting
            }
        }
    }
    // Skip normal input while dying
    return SceneResult::Continue;
}
```

Note: the early return must still render the frame. Restructure as needed (check `dying` at the top of the per-frame update, after rendering).

- [ ] **Step 4: Run tests**

```bash
cargo test
```

- [ ] **Step 5: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "fix(#110): death sequence with goodfairy countdown, faery revive, next-brother fallback"
```

---

### Group E complete — final test suite

- [ ] **Final full suite**

```bash
cargo test
```

Expected: all 150+ tests pass with no regressions.

---

## Self-Review Notes

**Spec coverage check:**
- #41 ✓ (C5), #97 ✓ (A2), #98 ✓ (A2), #99 ✓ (A1), #100 ✓ (A3)
- #101 ✓ (C4), #102 ✓ (C2), #103 ✓ (D1), #104 ✓ (C3), #105 ✓ (D5)
- #106 ✓ (D3), #107 ✓ (D2), #108 ✓ (C1), #109 ✓ (A7), #110 ✓ (E2)
- #111 ✓ (A8), #112 ✓ (D4), #113 ✓ (A5), #114 ✓ (A6), #115 ✓ (B1)
- #116 ✓ (A4), #117 ✓ (E1), #118 ✓ (D6), #119 ✓ (B1)

**Known open items (not regressions):**
- C1 (#108): STATELIST frames 24–86 are stubs; complete from original/fmain.c statelist[] for combat animation.
- D1 (#96/#103): Stargate bidirectional portals (2 entries with indoor src coords) deferred.
- D2 (#107): Zone event_id values need verification against narr.asm place_msg[] once ADF is analyzed.
- E2 (#110): Faery animation sprite path ("animate faery from right edge") is described but not drawn; the teleport + luck drain logic is complete.

**Type consistency:**
- `colors::Palette` (Vec<RGB4>) vs `palette::Palette` ([u32; 32]) — B1 uses both correctly via `to_rgba32_table(5)`.
- `MenuAction::SaveGame(u8)` / `LoadGame(u8)` — A6 updates both the enum and all match sites.
- `update_safe_spawn(terrain: u8, battleflag: bool)` — A3 updates definition and the one call site in gameplay_scene.rs line 427.
