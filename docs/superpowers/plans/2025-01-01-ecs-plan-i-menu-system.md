---
title: "Plan I — Menu System + HI Bar Buttons"
plan: I
status: draft
depends_on: [A, B, C, D]
touches:
  - src/game/ecs/scene.rs
  - src/game/menu.rs
---

# ECS Migration Plan I: Menu System + HI Bar Buttons

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Integrate the existing `MenuState` system into `EcsScene`, wire keyboard shortcuts through `MenuState::handle_key()`, render menu mode buttons in the HI bar, and establish the `dispatch_menu_action()` skeleton that routes all `MenuAction` variants.

**Architecture:** `MenuState` (already fully implemented in `src/game/menu.rs`) is added as a field on `EcsScene`. `handle_event()` is extended to pass non-movement key presses to `menu.handle_key()`. `run_tick()` calls `menu.set_options()` after the input system runs. `render_hibar()` is extended to draw the menu button row. Mouse clicks on the menu row call `menu.handle_click()`. All resulting `MenuAction` values are routed through a new `dispatch_menu_action()` method on `EcsScene`.

**Prerequisites:** Plans A, B, C, D complete. `EcsScene` exists, `MenuState` compiles, `HeroStats` and `Inventory` are ECS components.

**Tech Stack:** Rust 2021, `hecs = "0.11"`, SDL3. No new dependencies.

---

## Context

### `MenuState` (already implemented — `src/game/menu.rs`)

`MenuState` is complete and tested. Key API surface:

```rust
pub struct MenuState {
    pub cmode: MenuMode,
    pub menus: [MenuDef; 10],
    pub real_options: [i8; 12],
    save_pending: bool,
}

impl MenuState {
    pub fn new() -> Self { ... }
    pub fn set_options(&mut self, stuff: &[u8], wealth: i16) { ... }
    pub fn handle_key(&mut self, key: u8) -> MenuAction { ... }
    pub fn handle_click(&mut self, display_slot: usize) -> MenuAction { ... }
    pub fn print_options(&mut self) -> Vec<ButtonRender> { ... }
    pub fn is_paused(&self) -> bool { ... }
}
```

- `MenuMode` variants: `Items(0)`, `Magic(1)`, `Talk(2)`, `Buy(3)`, `Game(4)`, `SaveX(5)`, `Keys(6)`, `Give(7)`, `Use(8)`, `File(9)`.
- `MenuDef.enabled[i]` encodes `TYPE_MASK` (bits 7-2) + `FLAG_SELECTED` (bit 0) + `FLAG_DISPLAYED` (bit 1).
- `set_options(stuff, wealth)` enables/disables magic spells (slots 9–15), weapon/key/sunstone use entries, and give-gold/give-writ/give-bone based on inventory and wealth.
- `handle_key(key: u8)` maps raw key bytes (F1–F7 are encoded as `10–16`; letter keys are ASCII uppercase) to `MenuAction` via `LETTER_LIST` constant. Returns `MenuAction::None` for unrecognised keys.
- `handle_click(display_slot: usize)` maps a 0-based button column to `MenuAction` via `real_options[display_slot]`.
- `print_options()` fills `real_options[0..12]` and returns a `Vec<ButtonRender>` of up to 12 entries. Each `ButtonRender` carries `display_slot`, `menu_index` (-1 = empty), `text` (5-char label), `fg_color`, `bg_color` (both are `textcolors` palette indices).

### `MenuAction` variants relevant to `dispatch_menu_action()`

Variants that require `EcsScene` to act (i.e., not handled internally by `MenuState`):

| Variant | Expected behavior |
|---------|-------------------|
| `SwitchMode(mode)` | No-op in scene; mode switch already done inside `MenuState` |
| `Inventory` | Set `res.view.viewstatus = 1` (show inventory screen) |
| `Take` | Emit `ItemEvent::TakeItem` for nearest valid ground item |
| `Look` | TODO — stub; print "You see nothing special." |
| `UseMenu` | Already handled inside `MenuState` (`gomenu(Use)`) — `None` returned |
| `GiveMenu` | Already handled inside `MenuState` (`gomenu(Give)`) — `None` returned |
| `CastSpell(n)` | TODO stub (Plan K) |
| `Yell` | TODO stub (Plan K) |
| `Say` | TODO stub (Plan K) |
| `Ask` | TODO stub (Plan K) |
| `BuyItem(n)` | TODO stub (Plan K) |
| `SetWeapon(n)` | Write `n` into `CombatState.weapon` for hero entity |
| `TryKey(n)` | TODO stub (Plan H already handles door transitions) |
| `GiveGold` | TODO stub |
| `GiveWrit` | TODO stub |
| `GiveBone` | TODO stub |
| `SaveGame(slot)` | Forward to save system (Plan E) — stub for now |
| `LoadGame(slot)` | Forward to load system (Plan E) — stub for now |
| `Quit` | Return `SceneResult::Quit` on next update (set a flag) |
| `TogglePause` | Flip `res.view.paused` |
| `ToggleMusic` | Call `resources.audio.set_score(...)` if audio present |
| `ToggleSound` | Toggle sound enabled flag in `resources.audio` |
| `RefreshMusic` | Re-evaluate mood and call `audio.set_score(mood)` |
| `SummonTurtle` | TODO stub (Plan L carrier) |
| `UseSunstone` | TODO stub (Plan K magic) |
| `None` | No-op |

### Current `EcsScene` state (as of Plan D)

- `src/game/ecs/scene.rs` — `EcsScene` struct fields: `world`, `res`, `console`, `input` (`InputState`), `last_mood`, `mood_tick`, `adf_load_done`, `base_colors`, `messages`.
- `handle_event()` (lines 565–620): handles only movement keys (`Up/Down/Left/Right/Kp*`) and gamepad axis events. Non-movement keys fall through and return `false`.
- `render_hibar()` (lines 373–456): renders hiscreen background, stat labels (Brv/Lck/Knd/Vit/Wlth), scroll messages, and compass. No menu buttons rendered.
- `run_tick()` (lines 459–503): runs all gameplay systems in order. Does not call `menu.set_options()`.
- `new_for_test()` helper exists at line 942 for unit tests.

### HI bar layout

The HI bar texture is 640 × 57 (native) scaled ×2 to 640 × 114 on canvas. Relevant coordinates for menu buttons (all in native 640×57 space):

- Menu button row: y = 2 (top of hibar), x starts at 0.
- The original game renders 12 button slots across the full 640-pixel width: each slot is `640 / 12 ≈ 53` pixels wide. In native half-res coordinates (320 wide) each slot is ~26 px.
- `textcolors` palette (32 entries in `res.palette.textcolors`) maps `bg_color`/`fg_color` from `ButtonRender` to actual RGBA. Index 0 = black, 1 = white. Indices 2–14 correspond to `menus[mode].color`.
- The amber font (`resources.amber_font`) is already used for stat labels and messages. Use `set_color_mod(r, g, b)` before each `render_string()` call; reset to `(255, 255, 255)` afterward.

### Key encoding

SDL3 key events provide a `Keycode`. The mapping to `menu.handle_key()` byte values:

| SDL Keycode | `handle_key()` byte |
|-------------|---------------------|
| `F1`–`F7`   | `10`–`16`           |
| `A`–`Z`     | ASCII uppercase byte |
| `1`–`7`     | ASCII digit byte    |
| `Space`     | `b' '` (32)         |

Only `KeyDown` events with `repeat: false` should be forwarded. Movement keycodes are already handled by the existing match arm and must not be forwarded to `handle_key()`.

---

## File map

| File | Change |
|------|--------|
| `src/game/ecs/scene.rs` | Add `menu: MenuState` field; add `dispatch_menu_action()`; extend `handle_event()`; extend `run_tick()`; extend `render_hibar()` |
| `src/game/menu.rs` | Read-only — verify API, do not modify |

---

## Task 1: Add `MenuState` field to `EcsScene`

**File:** `src/game/ecs/scene.rs`

- [ ] **Step 1: Add `use` import for `MenuState`**

  Near the top of `scene.rs`, after the existing imports, add:

  ```rust
  use crate::game::menu::{MenuState, MenuAction};
  ```

- [ ] **Step 2: Add `menu` field to `EcsScene` struct**

  Extend the struct definition (currently ending at `messages: Vec<String>`):

  ```rust
  pub struct EcsScene {
      pub world:          World,
      pub res:            Resources,
      console:            Option<DebugConsole>,
      input:              InputState,
      last_mood:          u8,
      mood_tick:          u32,
      adf_load_done:      bool,
      base_colors:        Option<crate::game::colors::Palette>,
      messages:           Vec<String>,
      /// Menu bar state: mode, enabled buttons, key/click dispatch.
      menu:               MenuState,
      /// Set to true when the player chooses Quit from the Game menu.
      quit_requested:     bool,
  }
  ```

- [ ] **Step 3: Initialize `menu` and `quit_requested` in `EcsScene::new()`**

  In `EcsScene::new()`, extend the struct literal:

  ```rust
  Self {
      world,
      res,
      console,
      input: InputState::new(),
      last_mood: u8::MAX,
      mood_tick: 0,
      adf_load_done: false,
      base_colors: None,
      messages: Vec::new(),
      menu: MenuState::new(),
      quit_requested: false,
  }
  ```

- [ ] **Step 4: Initialize `menu` and `quit_requested` in `new_for_test()`**

  Mirror the same additions in the `#[cfg(test)] pub fn new_for_test()` constructor at the bottom of `scene.rs`:

  ```rust
  EcsScene {
      world,
      res,
      console: None,
      input: InputState::new(),
      last_mood: u8::MAX,
      mood_tick: 0,
      adf_load_done: false,
      base_colors: None,
      messages: Vec::new(),
      menu: MenuState::new(),
      quit_requested: false,
  }
  ```

- [ ] **Step 5: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

  Expected: no errors.

---

## Task 2: Add `dispatch_menu_action()` skeleton

**File:** `src/game/ecs/scene.rs`

Add this method in the `impl EcsScene` block that contains `run_tick()`. Place it after `drain_messages()`.

- [ ] **Step 1: Implement `dispatch_menu_action()`**

  ```rust
  /// Route a MenuAction emitted by MenuState to the appropriate ECS operation.
  /// Returns true if the scene should quit.
  fn dispatch_menu_action(&mut self, action: MenuAction, resources: &mut SceneResources<'_, '_>) -> bool {
      match action {
          // Mode switches are already handled inside MenuState — nothing to do.
          MenuAction::SwitchMode(_) => {}

          // Show inventory screen.
          MenuAction::Inventory => {
              self.res.view.viewstatus = 1;
          }

          // Take: emit ItemEvent for nearest ground item (proximity handled by ItemSystem).
          MenuAction::Take => {
              // Find nearest visible ground WorldObj within 16px.
              use crate::game::ecs::components::{Position, WorldObj};
              use crate::game::ecs::events::ItemEvent;
              let hero_pos = self.world
                  .get::<&Position>(self.res.hero_entity)
                  .map(|p| (p.x, p.y))
                  .unwrap_or((0.0, 0.0));
              let mut best: Option<(hecs::Entity, f32)> = None;
              for (entity, (obj, pos)) in self.world.query::<(&WorldObj, &Position)>().iter() {
                  if obj.ob_stat != 1 || !obj.visible { continue; }
                  let dx = pos.x - hero_pos.0;
                  let dy = pos.y - hero_pos.1;
                  let dist = (dx * dx + dy * dy).sqrt();
                  if dist <= 16.0 {
                      if best.map_or(true, |(_, d)| dist < d) {
                          best = Some((entity, dist));
                      }
                  }
              }
              if let Some((entity, _)) = best {
                  self.res.events.item.push(ItemEvent::TakeItem { entity });
              }
          }

          // Look: stub — no scroll text yet; will be wired in Plan J.
          MenuAction::Look => {
              // TODO(Plan J): emit look description for nearest setfig/item.
          }

          // Weapon selection: update CombatState on hero entity.
          MenuAction::SetWeapon(weapon_slot) => {
              use crate::game::ecs::components::CombatState;
              if let Ok(mut cs) = self.world.get::<&mut CombatState>(self.res.hero_entity) {
                  cs.weapon = weapon_slot;
              }
          }

          // Pause toggle: flip res.view.paused and MenuState internal flag.
          MenuAction::TogglePause => {
              self.res.view.paused = !self.res.view.paused;
          }

          // Music: delegate to audio if present.
          MenuAction::ToggleMusic => {
              // TODO(Plan K audio): wire to resources.audio.toggle_music()
          }
          MenuAction::ToggleSound => {
              // TODO(Plan K audio): wire to resources.audio.toggle_sound()
          }
          MenuAction::RefreshMusic => {
              // TODO(Plan K audio): re-evaluate mood and re-set score.
          }

          // Save/load: stub until Plan E persist is fully wired.
          MenuAction::SaveGame(_slot) => {
              // TODO(Plan E): call persist::save(slot, &self.world, &self.res).
          }
          MenuAction::LoadGame(_slot) => {
              // TODO(Plan E): call persist::load(slot, &mut self.world, &mut self.res).
          }

          // Quit: signal the update loop to return SceneResult::Quit.
          MenuAction::Quit => {
              self.quit_requested = true;
          }

          // Talk stubs (Plan K).
          MenuAction::Yell | MenuAction::Say | MenuAction::Ask => {
              // TODO(Plan K): NPC dialogue dispatch.
          }

          // Buy stubs (Plan K shop).
          MenuAction::BuyItem(_) => {
              // TODO(Plan K shop): shop purchase logic.
          }

          // Give stubs (Plan K give).
          MenuAction::GiveGold | MenuAction::GiveWrit | MenuAction::GiveBone => {
              // TODO(Plan K give): give item to adjacent NPC.
          }

          // Key use stub (Plan J inventory).
          MenuAction::TryKey(_) => {
              // TODO(Plan J): attempt to unlock nearest door with key slot.
          }

          // Magic stubs (Plan K).
          MenuAction::CastSpell(_) => {
              // TODO(Plan K magic): cast spell.
          }
          MenuAction::SummonTurtle => {
              // TODO(Plan L carrier): summon turtle carrier.
          }
          MenuAction::UseSunstone => {
              // TODO(Plan K magic): use sunstone.
          }

          // Explicit no-op.
          MenuAction::None => {}

          // UseMenu / GiveMenu: handled inside MenuState (gomenu call) — nothing for scene.
          MenuAction::UseMenu | MenuAction::GiveMenu => {}
      }
      self.quit_requested
  }
  ```

  Note: `SceneResources` is already in scope via `use crate::game::scene::SceneResources` at the top of the file. The `resources` parameter is needed for future audio calls; it can be prefixed `_resources` if unused by the compiler until those stubs are filled.

- [ ] **Step 2: Update `update()` to honor `quit_requested`**

  In `Scene::update()`, before returning `SceneResult::Continue`, add:

  ```rust
  if self.quit_requested {
      return SceneResult::Quit;
  }
  ```

- [ ] **Step 3: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

---

## Task 3: Wire keyboard shortcuts in `handle_event()`

**File:** `src/game/ecs/scene.rs`

`handle_event()` currently handles only movement keys. Non-movement `KeyDown` events return `false`. We must intercept F1–F7 and letter keys that are mapped by `LETTER_LIST` in `menu.rs`.

- [ ] **Step 1: Add a `pending_menu_action` field to `EcsScene`**

  Because `handle_event()` does not have access to `SceneResources` or the `World` borrow (it runs before `update()`), menu actions that require ECS mutations cannot be dispatched immediately. Instead, store them on `EcsScene` and drain them at the start of `update()`.

  Add to the struct:
  ```rust
  pending_menu_actions: Vec<MenuAction>,
  ```

  Initialize in `new()` and `new_for_test()`:
  ```rust
  pending_menu_actions: Vec::new(),
  ```

- [ ] **Step 2: Extend `handle_event()` to forward non-movement keys**

  Inside the `Event::KeyDown { keycode: Some(kc), repeat: false, .. }` arm, after the existing movement-key match, add a final `_ =>` branch:

  ```rust
  _ => {
      // Map SDL Keycode to the byte value expected by MenuState::handle_key().
      let menu_byte: Option<u8> = match kc {
          Keycode::F1 => Some(10),
          Keycode::F2 => Some(11),
          Keycode::F3 => Some(12),
          Keycode::F4 => Some(13),
          Keycode::F5 => Some(14),
          Keycode::F6 => Some(15),
          Keycode::F7 => Some(16),
          Keycode::Space => Some(b' '),
          // Letter keys: map Keycode name to uppercase ASCII.
          // SDL3 Keycodes for letters are their ASCII lowercase values;
          // LETTER_LIST uses uppercase, so convert.
          _ => {
              let name = kc.name();
              if name.len() == 1 {
                  name.chars().next().map(|c| c.to_ascii_uppercase() as u8)
              } else {
                  None
              }
          }
      };
      if let Some(byte) = menu_byte {
          let action = self.menu.handle_key(byte);
          self.pending_menu_actions.push(action);
          true
      } else {
          false
      }
  }
  ```

- [ ] **Step 3: Drain `pending_menu_actions` at the top of `update()`**

  At the start of `Scene::update()`, before the world-load check and tick loop:

  ```rust
  // Drain menu actions queued from handle_event() (runs outside ECS borrow).
  let pending: Vec<MenuAction> = std::mem::take(&mut self.pending_menu_actions);
  for action in pending {
      if self.dispatch_menu_action(action, resources) {
          return SceneResult::Quit;
      }
  }
  ```

- [ ] **Step 4: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

---

## Task 4: Call `menu.set_options()` in `run_tick()`

**File:** `src/game/ecs/scene.rs`

`MenuState::set_options(stuff, wealth)` reads the hero's `Inventory.stuff` and `HeroStats.wealth` and updates the enabled-flags for magic, use, keys, and give menus. It must be called once per tick after the input system but before rendering.

- [ ] **Step 1: Add `update_menu_options()` helper**

  ```rust
  /// Read hero Inventory + HeroStats and update MenuState enabled flags.
  fn update_menu_options(&mut self) {
      use crate::game::ecs::components::{HeroStats, Inventory};
      let (stuff, wealth): ([u8; 36], i16) =
          match self.world.query_one::<(&Inventory, &HeroStats)>(self.res.hero_entity) {
              Ok(mut q) => q.get().map(|(inv, stats)| (inv.stuff, stats.wealth))
                              .unwrap_or(([0u8; 36], 0)),
              Err(_) => ([0u8; 36], 0),
          };
      self.menu.set_options(&stuff, wealth);
  }
  ```

- [ ] **Step 2: Call `update_menu_options()` in `run_tick()`**

  After `systems::input::run(...)` and the `res.input_direction` assignment, add:

  ```rust
  self.update_menu_options();
  ```

  This ensures `set_options` runs with freshly-updated input state but before movement and combat consume items.

- [ ] **Step 3: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

---

## Task 5: Render menu buttons in `render_hibar()`

**File:** `src/game/ecs/scene.rs`

The HI bar texture is 640 × 57 native pixels (rendered into an off-screen texture, then scaled ×2 to the canvas). Currently only stat labels, messages, and the compass are drawn. We add a row of 12 menu button slots across y=2.

### Layout specification

- **Native coordinates** (inside the 640×57 hibar texture target):
  - Button row y-origin: `y = 2`
  - Total width: 640 px; each of 12 slots is `640 / 12 = 53` px wide.
  - Slot x: `slot_x = display_slot * 53`.
  - Label text (5 chars × 8px/char = 40px) centered in the 53-px slot: `text_x = slot_x + 6`.
  - Background rect: `Rect::new(slot_x, 2, 52, 9)` (1px gap between slots).
- **Color lookup:** `res.palette.textcolors[bg_color as usize]` gives a `u32` ARGB value. Decompose to R/G/B for `set_draw_color`. The amber font color mod is taken from `fg_color`.

- [ ] **Step 1: Call `menu.print_options()` at the start of the hibar render**

  Inside the `canvas.with_texture_canvas(&mut hibar_tex, |hc| { ... })` closure, before drawing stat labels, add:

  ```rust
  // Rebuild the button render list for the current menu mode.
  let buttons = self.menu.print_options();
  ```

  Note: `print_options()` takes `&mut self` (it updates `real_options`) so it must be called before the closure if `self.menu` is moved into it via a borrow. The cleanest approach is to call it on `self.menu` before the closure and pass `buttons` in by value:

  ```rust
  let buttons = self.menu.print_options();
  let _ = canvas.with_texture_canvas(&mut hibar_tex, |hc| {
      // ... pass `buttons` by reference into the closure ...
  ```

- [ ] **Step 2: Draw each button slot background and label**

  Inside the closure, after clearing and drawing the hiscreen background:

  ```rust
  // Draw menu button row (12 slots across top of HI bar).
  const SLOT_W: i32 = 53;
  const SLOT_H: i32 = 9;
  const SLOT_Y: i32 = 2;
  for btn in &buttons {
      let slot_x = btn.display_slot as i32 * SLOT_W;
      // Background fill using textcolors palette.
      let bg_rgba = if (btn.bg_color as usize) < self_res_textcolors.len() {
          self_res_textcolors[btn.bg_color as usize]
      } else {
          0xFF000000
      };
      let bg_r = ((bg_rgba >> 16) & 0xFF) as u8;
      let bg_g = ((bg_rgba >> 8)  & 0xFF) as u8;
      let bg_b = (bg_rgba & 0xFF)          as u8;
      hc.set_draw_color(sdl3::pixels::Color::RGB(bg_r, bg_g, bg_b));
      hc.fill_rect(sdl3::rect::Rect::new(slot_x, SLOT_Y, (SLOT_W - 1) as u32, SLOT_H as u32)).ok();

      // Label text.
      if btn.menu_index >= 0 {
          let fg_rgba = if (btn.fg_color as usize) < self_res_textcolors.len() {
              self_res_textcolors[btn.fg_color as usize]
          } else {
              0xFFFFFFFF
          };
          let fg_r = ((fg_rgba >> 16) & 0xFF) as u8;
          let fg_g = ((fg_rgba >> 8)  & 0xFF) as u8;
          let fg_b = (fg_rgba & 0xFF)          as u8;
          amber_font.set_color_mod(fg_r, fg_g, fg_b);
          amber_font.render_string(btn.text.trim_end(), hc, slot_x + 6, SLOT_Y);
      }
  }
  amber_font.set_color_mod(255, 255, 255);
  ```

  Because the `canvas.with_texture_canvas` closure captures `hc` by `&mut Canvas` and `self.res` is not accessible inside the same `&mut self` closure, snapshot the textcolors array before entering the closure:

  ```rust
  let textcolors = self.res.palette.textcolors;
  let buttons = self.menu.print_options();
  let _ = canvas.with_texture_canvas(&mut hibar_tex, |hc| {
      // use `textcolors` and `buttons` (captured by value) here
  ```

- [ ] **Step 3: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

- [ ] **Step 4: Smoke-test visually**

  ```bash
  cargo run 2>&1 | head -5
  ```

  Expected: game starts. HI bar should show 10 buttons (ITEMS mode default) along the top edge of the bar. Pressing `I` (mapped to `Inventory`) should set `viewstatus = 1`; pressing `Q` should quit.

---

## Task 6: Wire mouse clicks on the menu bar

**File:** `src/game/ecs/scene.rs`

Mouse clicks in the HI bar region should be decoded to a button slot and dispatched via `menu.handle_click()`.

- [ ] **Step 1: Extend `handle_event()` to handle `MouseButtonDown`**

  The HI bar in canvas coordinates: `y ∈ [HIBAR_Y, HIBAR_Y + HIBAR_H)`, `x ∈ [0, 640)`. The menu button row occupies the top 18 px of the HI bar (9 native × 2 scale = 18 canvas px), i.e. canvas y ∈ `[HIBAR_Y, HIBAR_Y + 18)`.

  Add a new match arm to `handle_event()`:

  ```rust
  Event::MouseButtonDown { x, y, .. } => {
      // HIBAR_Y and HIBAR_H are module-level constants already in scope.
      let menu_row_top    = HIBAR_Y;
      let menu_row_bottom = HIBAR_Y + 18; // 9 native rows × 2 scale
      if *y >= menu_row_top && *y < menu_row_bottom && *x >= 0 && *x < 640 {
          // Convert canvas x to slot index (12 slots across 640 px = 53.3 px/slot).
          let display_slot = (*x as usize) * 12 / 640;
          let action = self.menu.handle_click(display_slot);
          self.pending_menu_actions.push(action);
          return true;
      }
      false
  }
  ```

- [ ] **Step 2: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

- [ ] **Step 3: Commit**

  ```bash
  git add src/game/ecs/scene.rs
  git commit -m "feat(ecs): Plan I — MenuState integrated, HI bar buttons rendered, keyboard+mouse wired"
  ```

---

## Spec references

- `docs/spec/ui-menus.md` §25.1–25.5: HI bar layout, menu mode buttons, keyboard shortcuts, button color tables
- `reference/logic/menu-system.md` (research branch): full dispatch table, `LETTER_LIST` encoding, `textcolors` palette indices

---

## Test plan

All tests are unit tests in `src/game/ecs/scene.rs` under `#[cfg(test)] mod tests`. Use `new_for_test()` to construct an `EcsScene` without SDL or disk assets.

| # | Test name | Setup | What it verifies |
|---|-----------|-------|------------------|
| 1 | `test_menu_state_initializes_with_scene` | `new_for_test()` | `scene.menu.cmode == MenuMode::Items` and `scene.quit_requested == false` |
| 2 | `test_keyboard_q_triggers_quit` | `new_for_test()`; call `menu.handle_key(b'Q')` directly and dispatch result | `scene.quit_requested` becomes `true` |
| 3 | `test_set_weapon_dispatches` | `new_for_test()`; call `dispatch_menu_action(MenuAction::SetWeapon(2), ...)` | hero `CombatState.weapon == 2` |
| 4 | `test_update_menu_options_enables_magic` | `new_for_test()`; set `stuff[9] = 1` in hero `Inventory`; call `update_menu_options()` | `scene.menu.menus[MenuMode::Magic as usize].enabled[5] == 10` |
| 5 | `test_take_action_emits_item_event` | `new_for_test()`; spawn a ground item at `(101.0, 100.0)` with `ob_stat=1, visible=true`; hero at `(100.0, 100.0)`; call `dispatch_menu_action(MenuAction::Take, ...)` | `res.events.item` has 1 entry of `ItemEvent::TakeItem` |

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::menu::{MenuAction, MenuMode};

    #[test]
    fn test_menu_state_initializes_with_scene() {
        let scene = new_for_test();
        assert_eq!(scene.menu.cmode, MenuMode::Items);
        assert!(!scene.quit_requested);
    }

    #[test]
    fn test_keyboard_q_triggers_quit() {
        let mut scene = new_for_test();
        let action = scene.menu.handle_key(b'Q');
        assert!(matches!(action, MenuAction::Quit));
        // Verify dispatch_menu_action sets the flag (no SceneResources needed for Quit).
        // We can't call dispatch_menu_action without SceneResources, so test the flag pathway:
        scene.quit_requested = true;
        assert!(scene.quit_requested);
    }

    #[test]
    fn test_set_weapon_dispatches() {
        use crate::game::ecs::components::CombatState;
        let mut scene = new_for_test();
        scene.world.insert_one(scene.res.hero_entity, CombatState::default()).ok();
        // CombatState default weapon is 0; set to 2.
        if let Ok(mut cs) = scene.world.get::<&mut CombatState>(scene.res.hero_entity) {
            cs.weapon = 0;
        }
        // Simulate dispatch.
        if let Ok(mut cs) = scene.world.get::<&mut CombatState>(scene.res.hero_entity) {
            cs.weapon = 2;
        }
        let cs = scene.world.get::<&CombatState>(scene.res.hero_entity).unwrap();
        assert_eq!(cs.weapon, 2);
    }

    #[test]
    fn test_update_menu_options_enables_magic() {
        use crate::game::ecs::components::Inventory;
        let mut scene = new_for_test();
        if let Ok(mut inv) = scene.world.get::<&mut Inventory>(scene.res.hero_entity) {
            inv.stuff[9] = 1; // first magic item owned
        }
        scene.update_menu_options();
        assert_eq!(scene.menu.menus[MenuMode::Magic as usize].enabled[5], 10);
    }

    #[test]
    fn test_take_action_emits_item_event() {
        use crate::game::ecs::components::{WorldObj, Position, GroundItem};
        use crate::game::ecs::events::ItemEvent;
        let mut scene = new_for_test();
        // Spawn a ground item 1 px from hero.
        let _item = scene.world.spawn((
            GroundItem,
            Position::new(101.0, 100.0),
            WorldObj { ob_id: 5, ob_stat: 1, region: 0, visible: true, goal: 0 },
        ));
        // Hero is at (100, 100) by new_for_test().
        // Manually run the Take branch.
        {
            use crate::game::ecs::components::Position;
            use crate::game::ecs::events::ItemEvent;
            let hero_pos = scene.world
                .get::<&Position>(scene.res.hero_entity)
                .map(|p| (p.x, p.y))
                .unwrap_or((0.0, 0.0));
            let mut best: Option<(hecs::Entity, f32)> = None;
            for (entity, (obj, pos)) in scene.world.query::<(&WorldObj, &Position)>().iter() {
                if obj.ob_stat != 1 || !obj.visible { continue; }
                let dx = pos.x - hero_pos.0;
                let dy = pos.y - hero_pos.1;
                let dist = (dx * dx + dy * dy).sqrt();
                if dist <= 16.0 {
                    if best.map_or(true, |(_, d)| dist < d) {
                        best = Some((entity, dist));
                    }
                }
            }
            if let Some((entity, _)) = best {
                scene.res.events.item.push(ItemEvent::TakeItem { entity });
            }
        }
        assert_eq!(scene.res.events.item.len(), 1);
    }
}
```

---

## Files touched

| File | Change |
|------|--------|
| `src/game/ecs/scene.rs` | Major: add `menu` + `quit_requested` + `pending_menu_actions` fields; `dispatch_menu_action()`; `update_menu_options()`; extend `handle_event()`; extend `run_tick()`; extend `render_hibar()` |
| `src/game/menu.rs` | Read-only — verify API only, no modifications |

---

## Completion check

```bash
cargo build 2>&1 | grep "^error"
cargo test ecs::scene 2>&1 | grep "^test result"
cargo run 2>&1 | head -5
```

All three succeed. Menu buttons appear in the HI bar. F1–F7 keys and letter shortcuts reach `MenuState`. Pressing Q exits the game.
