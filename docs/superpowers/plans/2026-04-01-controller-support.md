# Controller Support Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement full game controller support with data-driven modal bindings (gameplay mode + menu mode), weapon cycling, magic quick-select, and a visual menu cursor for the HI bar.

**Architecture:** Extend `ControllerBindings` in `key_bindings.rs` to hold two binding maps (gameplay/menu) keyed by a new `ControllerMode` enum. Add `MenuCursor` state and visual outline to `gameplay_scene.rs`. Add weapon cycling logic. Replace all hardcoded controller match arms with a single data-driven lookup.

**Tech Stack:** Rust, SDL2 (sdl2 crate: controller, keyboard, render), existing `GameAction`/`MenuState` infrastructure.

**Spec:** `docs/superpowers/specs/2026-04-01-controller-support-design.md`

---

## File Structure

| File | Change | Responsibility |
|------|--------|----------------|
| `src/game/key_bindings.rs` | Modify | Add `ControllerMode` enum, new `GameAction` variants, redesign `ControllerBindings` with dual maps |
| `src/game/gameplay_scene.rs` | Modify | Add `MenuCursor` struct, `controller_mode` field, replace hardcoded controller match arms, add cursor rendering, add weapon cycling |
| `src/game/menu.rs` | Read-only | Reference for `MenuState::handle_click()` slot dispatch (no changes needed) |
| `src/game/magic.rs` | Read-only | Reference for `ITEM_*` constants (no changes needed) |

---

### Task 1: Add new GameAction variants and ControllerMode enum

**Files:**
- Modify: `src/game/key_bindings.rs`

- [ ] **Step 1: Add ControllerMode enum**

Add after the `GameAction` enum (after line 92 in `key_bindings.rs`):

```rust
/// Controller input mode — determines which binding map is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControllerMode {
    Gameplay,
    Menu,
}
```

- [ ] **Step 2: Add new GameAction variants**

Add these variants to the `GameAction` enum, before the closing brace (before the `Rebind` variant or after `SelectKey6`):

```rust
    // Controller: menu navigation
    MenuUp,
    MenuDown,
    MenuLeft,
    MenuRight,
    MenuConfirm,
    MenuCancel,

    // Controller: weapon cycling
    WeaponPrev,
    WeaponNext,

    // Controller: magic quick-select (DPad in gameplay mode)
    UseCrystalVial,
    UseOrb,
    UseTotem,
    UseSkull,

    // Controller: toggle menu mode
    ToggleMenuMode,
```

- [ ] **Step 3: Add display names for new variants**

In the `display_name()` method, add arms for all new variants:

```rust
            GameAction::MenuUp         => "Menu Up",
            GameAction::MenuDown       => "Menu Down",
            GameAction::MenuLeft       => "Menu Left",
            GameAction::MenuRight      => "Menu Right",
            GameAction::MenuConfirm    => "Menu Confirm",
            GameAction::MenuCancel     => "Menu Cancel",
            GameAction::WeaponPrev     => "Prev Weapon",
            GameAction::WeaponNext     => "Next Weapon",
            GameAction::UseCrystalVial => "Crystal Vial",
            GameAction::UseOrb         => "Jewel",
            GameAction::UseTotem       => "Totem",
            GameAction::UseSkull       => "Skull",
            GameAction::ToggleMenuMode => "Menu Mode",
```

- [ ] **Step 4: Add new variants to all_actions()**

Add the new variants to the `all_actions()` array.

- [ ] **Step 5: Build check**

Run: `cargo build 2>&1 | head -40`
Expected: Successful build (new variants exist but are unused — no warnings are errors for unused variants behind `#[allow(dead_code)]` or similar).

- [ ] **Step 6: Commit**

```bash
git add src/game/key_bindings.rs
git commit -m "feat(controller): add ControllerMode enum and new GameAction variants"
```

---

### Task 2: Redesign ControllerBindings with dual binding maps

**Files:**
- Modify: `src/game/key_bindings.rs`

- [ ] **Step 1: Write failing tests for dual-map ControllerBindings**

Add to the `#[cfg(test)] mod tests` block in `key_bindings.rs`:

```rust
    #[test]
    fn test_controller_gameplay_mode_dpad_maps_to_magic() {
        let cb = ControllerBindings::default_bindings();
        assert_eq!(
            cb.action_for_button(ControllerMode::Gameplay, Button::DPadUp),
            Some(GameAction::UseCrystalVial)
        );
        assert_eq!(
            cb.action_for_button(ControllerMode::Gameplay, Button::DPadDown),
            Some(GameAction::UseOrb)
        );
        assert_eq!(
            cb.action_for_button(ControllerMode::Gameplay, Button::DPadLeft),
            Some(GameAction::UseTotem)
        );
        assert_eq!(
            cb.action_for_button(ControllerMode::Gameplay, Button::DPadRight),
            Some(GameAction::UseSkull)
        );
    }

    #[test]
    fn test_controller_menu_mode_dpad_maps_to_navigation() {
        let cb = ControllerBindings::default_bindings();
        assert_eq!(
            cb.action_for_button(ControllerMode::Menu, Button::DPadUp),
            Some(GameAction::MenuUp)
        );
        assert_eq!(
            cb.action_for_button(ControllerMode::Menu, Button::DPadDown),
            Some(GameAction::MenuDown)
        );
        assert_eq!(
            cb.action_for_button(ControllerMode::Menu, Button::DPadLeft),
            Some(GameAction::MenuLeft)
        );
        assert_eq!(
            cb.action_for_button(ControllerMode::Menu, Button::DPadRight),
            Some(GameAction::MenuRight)
        );
    }

    #[test]
    fn test_controller_face_buttons_gameplay() {
        let cb = ControllerBindings::default_bindings();
        assert_eq!(
            cb.action_for_button(ControllerMode::Gameplay, Button::A),
            Some(GameAction::Fight)
        );
        assert_eq!(
            cb.action_for_button(ControllerMode::Gameplay, Button::B),
            Some(GameAction::Take)
        );
        assert_eq!(
            cb.action_for_button(ControllerMode::Gameplay, Button::X),
            Some(GameAction::BuyFood) // BuyFood doubles as Eat when not near shop
        );
        assert_eq!(
            cb.action_for_button(ControllerMode::Gameplay, Button::Y),
            Some(GameAction::Look)
        );
    }

    #[test]
    fn test_controller_bumpers_both_modes() {
        let cb = ControllerBindings::default_bindings();
        // Gameplay mode
        assert_eq!(
            cb.action_for_button(ControllerMode::Gameplay, Button::LeftShoulder),
            Some(GameAction::WeaponPrev)
        );
        assert_eq!(
            cb.action_for_button(ControllerMode::Gameplay, Button::RightShoulder),
            Some(GameAction::WeaponNext)
        );
        // Menu mode — same
        assert_eq!(
            cb.action_for_button(ControllerMode::Menu, Button::LeftShoulder),
            Some(GameAction::WeaponPrev)
        );
        assert_eq!(
            cb.action_for_button(ControllerMode::Menu, Button::RightShoulder),
            Some(GameAction::WeaponNext)
        );
    }

    #[test]
    fn test_controller_menu_mode_a_b_are_confirm_cancel() {
        let cb = ControllerBindings::default_bindings();
        assert_eq!(
            cb.action_for_button(ControllerMode::Menu, Button::A),
            Some(GameAction::MenuConfirm)
        );
        assert_eq!(
            cb.action_for_button(ControllerMode::Menu, Button::B),
            Some(GameAction::MenuCancel)
        );
    }

    #[test]
    fn test_controller_unknown_button_returns_none() {
        let cb = ControllerBindings::default_bindings();
        assert_eq!(
            cb.action_for_button(ControllerMode::Gameplay, Button::Guide),
            None
        );
        assert_eq!(
            cb.action_for_button(ControllerMode::Menu, Button::Guide),
            None
        );
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test controller 2>&1 | tail -20`
Expected: FAIL — `action_for_button` doesn't accept `ControllerMode` yet.

- [ ] **Step 3: Implement dual-map ControllerBindings**

Replace the existing `ControllerBindings` struct and `impl` with:

```rust
/// Maps SDL2 game controller buttons to game actions, with separate maps
/// for Gameplay and Menu modes.
#[derive(Debug, Clone)]
pub struct ControllerBindings {
    gameplay: HashMap<Button, GameAction>,
    menu: HashMap<Button, GameAction>,
}

impl ControllerBindings {
    pub fn default_bindings() -> Self {
        use Button::*;

        let mut gameplay = HashMap::new();
        // Face buttons
        gameplay.insert(A,         GameAction::Fight);
        gameplay.insert(B,         GameAction::Take);
        gameplay.insert(X,         GameAction::BuyFood);  // BuyFood doubles as Eat
        gameplay.insert(Y,         GameAction::Look);
        // Bumpers — weapon cycling
        gameplay.insert(LeftShoulder,  GameAction::WeaponPrev);
        gameplay.insert(RightShoulder, GameAction::WeaponNext);
        // DPad — magic quick-select
        gameplay.insert(DPadUp,    GameAction::UseCrystalVial);
        gameplay.insert(DPadDown,  GameAction::UseOrb);
        gameplay.insert(DPadLeft,  GameAction::UseTotem);
        gameplay.insert(DPadRight, GameAction::UseSkull);
        // Start/Back/Stick clicks
        gameplay.insert(Start,     GameAction::ToggleMenuMode);
        gameplay.insert(Back,      GameAction::Map);
        gameplay.insert(LeftStick, GameAction::Inventory);

        let mut menu = HashMap::new();
        // Face buttons
        menu.insert(A,         GameAction::MenuConfirm);
        menu.insert(B,         GameAction::MenuCancel);
        menu.insert(X,         GameAction::BuyFood);  // Eat still works in menu
        menu.insert(Y,         GameAction::Look);     // Look still works in menu
        // Bumpers — weapon cycling (unchanged)
        menu.insert(LeftShoulder,  GameAction::WeaponPrev);
        menu.insert(RightShoulder, GameAction::WeaponNext);
        // DPad — menu navigation
        menu.insert(DPadUp,    GameAction::MenuUp);
        menu.insert(DPadDown,  GameAction::MenuDown);
        menu.insert(DPadLeft,  GameAction::MenuLeft);
        menu.insert(DPadRight, GameAction::MenuRight);
        // Start exits menu mode too
        menu.insert(Start,     GameAction::ToggleMenuMode);
        menu.insert(Back,      GameAction::Map);
        menu.insert(LeftStick, GameAction::Inventory);

        ControllerBindings { gameplay, menu }
    }

    /// Look up the action for a button in the given mode.
    pub fn action_for_button(&self, mode: ControllerMode, btn: Button) -> Option<GameAction> {
        match mode {
            ControllerMode::Gameplay => self.gameplay.get(&btn).copied(),
            ControllerMode::Menu => self.menu.get(&btn).copied(),
        }
    }
}

impl Default for ControllerBindings {
    fn default() -> Self { Self::default_bindings() }
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test controller 2>&1 | tail -20`
Expected: All 6 new controller tests pass.

- [ ] **Step 5: Run all tests to verify nothing broken**

Run: `cargo test 2>&1 | tail -5`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/game/key_bindings.rs
git commit -m "feat(controller): redesign ControllerBindings with dual gameplay/menu maps"
```

---

### Task 3: Add MenuCursor struct and controller mode to GameplayScene

**Files:**
- Modify: `src/game/gameplay_scene.rs`

- [ ] **Step 1: Add MenuCursor struct**

Add near the `InputState` struct definition (around line 195):

```rust
/// Cursor state for controller-driven HI bar menu navigation.
#[derive(Debug, Clone)]
struct MenuCursor {
    col: usize,   // 0 or 1 (HI bar is 2 columns)
    row: usize,   // 0–5 (6 rows)
    active: bool,  // true when in menu mode
}

impl Default for MenuCursor {
    fn default() -> Self {
        MenuCursor { col: 0, row: 0, active: false }
    }
}

impl MenuCursor {
    fn navigate_up(&mut self) {
        self.row = if self.row == 0 { 5 } else { self.row - 1 };
    }

    fn navigate_down(&mut self) {
        self.row = if self.row == 5 { 0 } else { self.row + 1 };
    }

    fn navigate_left(&mut self) {
        self.col = if self.col == 0 { 1 } else { 0 };
    }

    fn navigate_right(&mut self) {
        self.col = if self.col == 1 { 0 } else { 1 };
    }

    /// Returns the display slot index for MenuState::handle_click().
    fn slot(&self) -> usize {
        self.row * 2 + self.col
    }
}
```

- [ ] **Step 2: Add fields to GameplayScene struct**

Add to the `GameplayScene` struct fields (after `local_bindings` around line 220):

```rust
    controller_mode: ControllerMode,
    controller_bindings: ControllerBindings,
    menu_cursor: MenuCursor,
```

Add the necessary import at the top of gameplay_scene.rs:

```rust
use crate::game::key_bindings::{ControllerMode, ControllerBindings};
```

- [ ] **Step 3: Initialize fields in GameplayScene constructor**

In the `GameplayScene::new()` or equivalent constructor, add initialization:

```rust
    controller_mode: ControllerMode::Gameplay,
    controller_bindings: ControllerBindings::default_bindings(),
    menu_cursor: MenuCursor::default(),
```

- [ ] **Step 4: Build check**

Run: `cargo build 2>&1 | head -40`
Expected: Successful build.

- [ ] **Step 5: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "feat(controller): add MenuCursor struct and controller state to GameplayScene"
```

---

### Task 4: Replace hardcoded controller match arms with data-driven dispatch

**Files:**
- Modify: `src/game/gameplay_scene.rs`

- [ ] **Step 1: Replace ControllerButtonDown match arm**

Find the existing `Event::ControllerButtonDown` match arm (around line 3316) and replace the entire block:

```rust
            Event::ControllerButtonDown { button, .. } => {
                use sdl2::controller::Button;
                match button {
                    Button::DPadUp    => { self.input.up    = true; true }
                    Button::DPadDown  => { self.input.down  = true; true }
                    Button::DPadLeft  => { self.input.left  = true; true }
                    Button::DPadRight => { self.input.right = true; true }
                    Button::A         => { self.do_option(GameAction::Fight);     true }
                    Button::X         => { self.do_option(GameAction::Inventory); true }
                    Button::Y         => { self.do_option(GameAction::Look);      true }
                    Button::B         => { self.do_option(GameAction::UseItem);   true }
                    Button::LeftShoulder  => { self.do_option(GameAction::CastSpell1); true }
                    Button::RightShoulder => { self.do_option(GameAction::CastSpell2); true }
                    Button::Start     => { self.do_option(GameAction::Pause);     true }
                    Button::Back      => { self.do_option(GameAction::Map);       true }
                    _ => false,
                }
            }
```

Replace with:

```rust
            Event::ControllerButtonDown { button, .. } => {
                if let Some(action) = self.controller_bindings.action_for_button(
                    self.controller_mode, *button
                ) {
                    self.do_option(action);
                }
                true
            }
```

- [ ] **Step 2: Replace ControllerButtonUp match arm**

Find the existing `Event::ControllerButtonUp` match arm (around line 3335) — DPad directions used to clear movement flags. Since DPad is no longer used for movement (stick handles movement), the DPad release handler is no longer needed for movement clearing. Replace:

```rust
            Event::ControllerButtonUp { button, .. } => {
                use sdl2::controller::Button;
                match button {
                    Button::DPadUp    => { self.input.up    = false; true }
                    Button::DPadDown  => { self.input.down  = false; true }
                    Button::DPadLeft  => { self.input.left  = false; true }
                    Button::DPadRight => { self.input.right = false; true }
                    _ => false,
                }
            }
```

With:

```rust
            Event::ControllerButtonUp { .. } => {
                // DPad no longer controls movement (stick does); button-up is a no-op.
                true
            }
```

- [ ] **Step 3: Build check**

Run: `cargo build 2>&1 | head -40`
Expected: Build succeeds. New `GameAction` variants will hit the `_ => {}` fallthrough in `do_option` — that's expected, we'll wire them in the next tasks.

- [ ] **Step 4: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "feat(controller): replace hardcoded controller match arms with data-driven dispatch"
```

---

### Task 5: Wire new GameAction variants into do_option

**Files:**
- Modify: `src/game/gameplay_scene.rs`

- [ ] **Step 1: Add ToggleMenuMode handler**

Add a `toggle_menu_mode` helper method to `GameplayScene`:

```rust
    fn toggle_menu_mode(&mut self) {
        self.menu_cursor.active = !self.menu_cursor.active;
        self.controller_mode = if self.menu_cursor.active {
            ControllerMode::Menu
        } else {
            ControllerMode::Gameplay
        };
    }
```

- [ ] **Step 2: Add menu navigation and magic quick-select handlers to do_option**

In the `do_option` match block, add handlers for the new variants (before the `_ => {}` fallthrough):

```rust
            GameAction::ToggleMenuMode => {
                self.toggle_menu_mode();
            }
            GameAction::MenuUp => {
                self.menu_cursor.navigate_up();
            }
            GameAction::MenuDown => {
                self.menu_cursor.navigate_down();
            }
            GameAction::MenuLeft => {
                self.menu_cursor.navigate_left();
            }
            GameAction::MenuRight => {
                self.menu_cursor.navigate_right();
            }
            GameAction::MenuConfirm => {
                let slot = self.menu_cursor.slot();
                let action = self.menu.handle_click(slot);
                self.dispatch_menu_action(action);
            }
            GameAction::MenuCancel => {
                self.menu_cursor.active = false;
                self.controller_mode = ControllerMode::Gameplay;
            }
            GameAction::UseCrystalVial => {
                self.do_option(GameAction::CastSpell3); // ITEM_VIAL = stuff[11], spell slot 3
            }
            GameAction::UseOrb => {
                self.do_option(GameAction::CastSpell4); // ITEM_ORB = stuff[12], spell slot 4
            }
            GameAction::UseTotem => {
                self.do_option(GameAction::CastSpell5); // ITEM_TOTEM = stuff[13], spell slot 5
            }
            GameAction::UseSkull => {
                self.do_option(GameAction::CastSpell7); // ITEM_SKULL = stuff[15], spell slot 7
            }
```

- [ ] **Step 3: Build check**

Run: `cargo build 2>&1 | head -40`
Expected: Build succeeds.

- [ ] **Step 4: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "feat(controller): wire menu navigation and magic quick-select into do_option"
```

---

### Task 6: Implement weapon cycling

**Files:**
- Modify: `src/game/gameplay_scene.rs`

- [ ] **Step 1: Write failing test for weapon cycling logic**

Add a test module or add to existing tests. Since `cycle_weapon` will need access to `GameState` internals, write the cycling logic as a standalone function that can be tested:

Add a helper function (can be at module level or as an associated function):

```rust
/// Find the next owned weapon slot in the given direction.
/// `current` is the 1-based weapon index (1=Dirk..5=Wand).
/// `direction` is +1 (next) or -1 (prev).
/// `stuff` is the player's inventory array.
/// Returns `Some(new_1based_idx)` if a different weapon is found, `None` otherwise.
fn cycle_weapon_slot(current: u8, direction: i8, stuff: &[u8; 35]) -> Option<u8> {
    let weapon_count = 5; // slots 1..=5
    let cur_0 = (current as i8 - 1).max(0) as usize; // convert to 0-based
    for offset in 1..weapon_count {
        let idx_0 = ((cur_0 as i8 + direction * offset as i8).rem_euclid(weapon_count as i8)) as usize;
        let stuff_idx = idx_0 + 1; // stuff[1..=5] are weapon slots
        if stuff[stuff_idx] > 0 {
            return Some((idx_0 + 1) as u8); // return 1-based
        }
    }
    None
}
```

Add test:

```rust
#[cfg(test)]
mod controller_tests {
    use super::*;

    #[test]
    fn test_cycle_weapon_next() {
        let mut stuff = [0u8; 35];
        stuff[1] = 1; // Dirk
        stuff[3] = 1; // Sword
        stuff[5] = 1; // Wand
        // From Dirk (1), next should be Sword (3)
        assert_eq!(cycle_weapon_slot(1, 1, &stuff), Some(3));
        // From Sword (3), next should be Wand (5)
        assert_eq!(cycle_weapon_slot(3, 1, &stuff), Some(5));
        // From Wand (5), next should wrap to Dirk (1)
        assert_eq!(cycle_weapon_slot(5, 1, &stuff), Some(1));
    }

    #[test]
    fn test_cycle_weapon_prev() {
        let mut stuff = [0u8; 35];
        stuff[1] = 1; // Dirk
        stuff[3] = 1; // Sword
        stuff[5] = 1; // Wand
        // From Dirk (1), prev should wrap to Wand (5)
        assert_eq!(cycle_weapon_slot(1, -1, &stuff), Some(5));
        // From Sword (3), prev should be Dirk (1)
        assert_eq!(cycle_weapon_slot(3, -1, &stuff), Some(1));
    }

    #[test]
    fn test_cycle_weapon_single_owned() {
        let mut stuff = [0u8; 35];
        stuff[1] = 1; // Only Dirk
        assert_eq!(cycle_weapon_slot(1, 1, &stuff), None);
        assert_eq!(cycle_weapon_slot(1, -1, &stuff), None);
    }

    #[test]
    fn test_cycle_weapon_none_owned() {
        let stuff = [0u8; 35];
        assert_eq!(cycle_weapon_slot(1, 1, &stuff), None);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test cycle_weapon 2>&1 | tail -20`
Expected: FAIL — `cycle_weapon_slot` not found.

- [ ] **Step 3: Implement cycle_weapon_slot and wire into do_option**

Add the `cycle_weapon_slot` function as shown above.

Then add handlers in `do_option`:

```rust
            GameAction::WeaponPrev => {
                let current_weapon = self.state.actors.first()
                    .map(|a| a.weapon).unwrap_or(1);
                if let Some(new_weapon) = cycle_weapon_slot(current_weapon, -1, self.state.stuff()) {
                    if let Some(player) = self.state.actors.first_mut() {
                        player.weapon = new_weapon;
                    }
                    let name = match new_weapon {
                        1 => "Dirk", 2 => "Mace", 3 => "Sword", 4 => "Bow",
                        5 => "Wand", _ => "?",
                    };
                    self.messages.push(format!("{} readied.", name));
                    let wealth = self.state.wealth;
                    self.menu.set_options(self.state.stuff(), wealth);
                }
            }
            GameAction::WeaponNext => {
                let current_weapon = self.state.actors.first()
                    .map(|a| a.weapon).unwrap_or(1);
                if let Some(new_weapon) = cycle_weapon_slot(current_weapon, 1, self.state.stuff()) {
                    if let Some(player) = self.state.actors.first_mut() {
                        player.weapon = new_weapon;
                    }
                    let name = match new_weapon {
                        1 => "Dirk", 2 => "Mace", 3 => "Sword", 4 => "Bow",
                        5 => "Wand", _ => "?",
                    };
                    self.messages.push(format!("{} readied.", name));
                    let wealth = self.state.wealth;
                    self.menu.set_options(self.state.stuff(), wealth);
                }
            }
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test cycle_weapon 2>&1 | tail -20`
Expected: All 4 tests pass.

- [ ] **Step 5: Full test suite**

Run: `cargo test 2>&1 | tail -5`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "feat(controller): implement weapon cycling with LB/RB"
```

---

### Task 7: Add menu cursor outline rendering to the HI bar

**Files:**
- Modify: `src/game/gameplay_scene.rs`

- [ ] **Step 1: Add cursor rendering in render_hibar**

In `render_hibar()`, after the compass rendering block (after the `compass_highlight` copy, around line 1395) and before the final `});` that closes `with_texture_canvas`, add the menu cursor outline:

```rust
                // Controller menu cursor outline
                if self.menu_cursor.active {
                    let cursor_col = self.menu_cursor.col;
                    let cursor_row = self.menu_cursor.row;
                    let cursor_x = if cursor_col == 0 { 430i32 } else { 482i32 };
                    let cursor_y = (cursor_row as i32) * 9 + 8;
                    let cursor_w = 48u32; // button text width
                    let cursor_h = 9u32;  // row height
                    hc.set_draw_color(sdl2::pixels::Color::RGB(255, 255, 255));
                    hc.draw_rect(sdl2::rect::Rect::new(
                        cursor_x - 1, cursor_y - 1, cursor_w + 2, cursor_h + 2
                    )).ok();
                }
```

Note: The outline rect is 1px larger than the button area on each side so the text remains fully visible inside the outline.

- [ ] **Step 2: Build check**

Run: `cargo build 2>&1 | head -40`
Expected: Build succeeds.

- [ ] **Step 3: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "feat(controller): render menu cursor outline on HI bar"
```

---

### Task 8: Add MenuCursor unit tests

**Files:**
- Modify: `src/game/gameplay_scene.rs`

- [ ] **Step 1: Add MenuCursor navigation tests**

Add to the `controller_tests` module (or create one in gameplay_scene.rs):

```rust
    #[test]
    fn test_menu_cursor_navigation_wraps() {
        let mut c = MenuCursor::default();
        assert_eq!(c.row, 0);
        assert_eq!(c.col, 0);

        // Up from row 0 wraps to row 5
        c.navigate_up();
        assert_eq!(c.row, 5);

        // Down from row 5 wraps to row 0
        c.navigate_down();
        assert_eq!(c.row, 0);

        // Down increments normally
        c.navigate_down();
        assert_eq!(c.row, 1);

        // Left from col 0 wraps to col 1
        c.navigate_left();
        assert_eq!(c.col, 1);

        // Right from col 1 wraps to col 0
        c.navigate_right();
        assert_eq!(c.col, 0);
    }

    #[test]
    fn test_menu_cursor_slot_calculation() {
        let mut c = MenuCursor::default();
        assert_eq!(c.slot(), 0); // (0,0) → slot 0

        c.col = 1;
        assert_eq!(c.slot(), 1); // (1,0) → slot 1

        c.row = 2;
        c.col = 0;
        assert_eq!(c.slot(), 4); // (0,2) → slot 4

        c.row = 5;
        c.col = 1;
        assert_eq!(c.slot(), 11); // (1,5) → slot 11
    }

    #[test]
    fn test_menu_cursor_position_persists() {
        let mut c = MenuCursor::default();
        c.navigate_down();
        c.navigate_down();
        c.navigate_right();
        assert_eq!(c.row, 2);
        assert_eq!(c.col, 1);

        // Deactivate and reactivate — position should persist
        c.active = false;
        c.active = true;
        assert_eq!(c.row, 2);
        assert_eq!(c.col, 1);
    }
```

- [ ] **Step 2: Run tests**

Run: `cargo test menu_cursor 2>&1 | tail -20`
Expected: All 3 tests pass.

- [ ] **Step 3: Full test suite**

Run: `cargo test 2>&1 | tail -5`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/game/gameplay_scene.rs
git commit -m "test(controller): add MenuCursor navigation and slot tests"
```

---

### Task 9: Final integration build and cleanup

**Files:**
- Modify: `src/game/gameplay_scene.rs` (if needed for warnings)

- [ ] **Step 1: Full build with warnings**

Run: `cargo build 2>&1`
Expected: No errors. Check for unused warnings on old `ControllerBindings` usage or stale imports. Fix any warnings.

- [ ] **Step 2: Full test suite**

Run: `cargo test 2>&1 | tail -10`
Expected: All tests pass.

- [ ] **Step 3: Run clippy**

Run: `cargo clippy 2>&1 | tail -20`
Expected: No new warnings related to controller code.

- [ ] **Step 4: Remove any dead code**

Check if the old `action_for_button(&self, btn: Button)` (single-argument version) is still referenced anywhere. If not, it was already replaced in Task 2. Also verify the old `ControllerBindings::default_bindings()` test (`test_action_for_key` etc.) still compiles — the old test was for `KeyBindings`, not `ControllerBindings`, so it should be fine.

- [ ] **Step 5: Final commit if any cleanup was needed**

```bash
git add -A
git commit -m "chore(controller): cleanup warnings and dead code"
```
