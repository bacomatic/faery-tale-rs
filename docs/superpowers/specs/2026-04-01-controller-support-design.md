# Controller Support Design

**Date:** 2026-04-01
**Status:** Draft
**Approach:** Data-driven bindings with modal context (Approach 2)

## Overview

Implement full game controller support using a two-tier design:
- **Gameplay mode** (default): face buttons for common actions, DPad for magic quick-select, bumpers for weapon cycling.
- **Menu mode** (Start toggle): DPad navigates the HI bar button grid for less-used commands (Speak, Ask, Give, Yell, Sleep, Board, etc.).

The left stick always controls 8-way movement regardless of mode. The game world continues running in both modes.

## Design Principles

- Controller-only gameplay for all common actions (fight, take, eat, look, weapon/magic, map, inventory).
- Menu mode as fallback for the full HI bar command set.
- Xbox layout as primary reference; PlayStation labels noted for documentation.
- No changes to existing keyboard or mouse input paths.

## Button Mapping

### Gameplay Mode (default)

| Button | Xbox | PS | Action |
|--------|------|----|--------|
| Left Stick | LS | LS | 8-way movement (deadzone threshold 8000/32768 ≈ 0.24) |
| A | A | Cross | Fight (directional melee in facing direction / shoot bow if equipped) |
| B | B | Circle | Take (pick up nearest item within 30px) |
| X | X | Square | Eat food |
| Y | Y | Triangle | Look (describe terrain at hero position) |
| LB | LB | L1 | Cycle weapon prev (skip unowned slots, wrap) |
| RB | RB | R1 | Cycle weapon next (skip unowned slots, wrap) |
| DPad Up | DPad Up | DPad Up | Use Crystal Vial |
| DPad Down | DPad Down | DPad Down | Use Jewel/Orb |
| DPad Left | DPad Left | DPad Left | Use Totem |
| DPad Right | DPad Right | DPad Right | Use Skull |
| Start | Menu | Options | Toggle menu mode |
| Back | View | Touchpad | Map view |
| LS Click | LS Click | L3 | Inventory view |
| RS Click | RS Click | R3 | Unassigned |

### Menu Mode (Start toggled on)

| Button | Action |
|--------|--------|
| Left Stick | Movement (still active) |
| DPad Up/Down/Left/Right | Navigate HI bar button grid cursor |
| A | Activate highlighted button (MenuConfirm) |
| B | Exit menu mode (MenuCancel) |
| Start | Exit menu mode |
| LB/RB | Cycle weapons (unchanged) |
| X/Y | Eat/Look (unchanged — gameplay actions remain active) |

## Architecture

### ControllerMode enum

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControllerMode {
    Gameplay,
    Menu,
}
```

### ControllerBindings redesign

Replace the existing single-map `ControllerBindings` with a dual-map structure:

```rust
pub struct ControllerBindings {
    gameplay: HashMap<Button, GameAction>,
    menu: HashMap<Button, GameAction>,
}

impl ControllerBindings {
    pub fn action_for_button(&self, mode: ControllerMode, button: Button) -> Option<GameAction> {
        match mode {
            ControllerMode::Gameplay => self.gameplay.get(&button).copied(),
            ControllerMode::Menu => self.menu.get(&button).copied(),
        }
    }
}
```

Both maps are populated by `default_bindings()`. Buttons that behave the same in both modes (LB, RB, X, Y, Back, LS Click) appear in both maps. DPad buttons have different actions per mode.

### New GameAction variants

```rust
// Menu navigation (Menu mode only)
GameAction::MenuUp,
GameAction::MenuDown,
GameAction::MenuLeft,
GameAction::MenuRight,
GameAction::MenuConfirm,
GameAction::MenuCancel,

// Weapon cycling (both modes)
GameAction::WeaponPrev,
GameAction::WeaponNext,

// Magic quick-select (Gameplay mode only)
GameAction::UseCrystalVial,
GameAction::UseOrb,
GameAction::UseTotem,
GameAction::UseSkull,
```

### GameplayScene changes

New fields:

```rust
controller_mode: ControllerMode,  // default: Gameplay
controller_bindings: ControllerBindings,
menu_cursor: MenuCursor,
```

The `ControllerButtonDown` match arm simplifies to:

```rust
Event::ControllerButtonDown { button, .. } => {
    // Start toggles mode before action lookup
    if *button == Button::Start {
        self.toggle_menu_mode();
        return true;
    }
    if let Some(action) = self.controller_bindings.action_for_button(
        self.controller_mode, *button
    ) {
        self.do_option(action);
    }
    true
}
```

Start is handled before the binding lookup because it changes which map is active. All other buttons go through the data-driven path.

Left stick axis motion handling remains unchanged (direct `InputState` flag updates).

## Menu Mode Cursor

### MenuCursor struct

```rust
struct MenuCursor {
    col: usize,   // 0 or 1 (HI bar is 2 columns)
    row: usize,   // 0–5 (6 rows)
    active: bool,  // true when in menu mode
}
```

### Navigation

- DPad Up/Down: move `row` with wrapping (0 ↔ 5).
- DPad Left/Right: toggle `col` between 0 and 1.
- A: dispatch action at `slot = row * 2 + col` through existing `MenuState::handle_click(slot)`.
- B or Start: deactivate menu mode, return to Gameplay.

### Position persistence

The cursor remembers its `(col, row)` across mode toggles. First activation defaults to (0, 0). Subsequent activations resume at the last position. This allows quick repeated access to the same menu action (Start → A → Start).

### Visual feedback

When menu mode is active, render an **outlined rectangle** (not filled) around the currently highlighted button in the HI bar. The outline uses the same coordinate system as mouse click detection:

- Column x-bounds: `BTN_X_LEFT` (430) to `BTN_X_RIGHT` (482) for col 0; `BTN_X_RIGHT` to `BTN_X_END` (530) for col 1.
- Row y-position: `HIBAR_Y + row * 9 * 2` (9px native row pitch, doubled for line-doubling).
- Outline color: contrasting color visible against the HI bar background (e.g., white or bright amber).

## Weapon Cycling

LB (WeaponPrev) and RB (WeaponNext) cycle through the player's **owned** weapons in slots `stuff[1..=5]` (Dirk, Mace, Sword, Bow, Wand).

### Behavior

- Skip slots where `stuff[slot] == 0` (not owned).
- Wrap from last owned weapon to first (and vice versa).
- If the player owns only one weapon, bumpers do nothing.
- Selecting a weapon equips it as the active weapon (same effect as pressing number keys 1–5).
- Display a brief message via the existing message queue showing the weapon name (e.g., "Sword").

### Implementation

A `cycle_weapon(direction: i8)` method on `GameplayScene` or `GameState`:

1. Find current weapon index in `stuff[1..=5]`.
2. Scan in `direction` (+1 or -1), skipping empty slots, wrapping at boundaries.
3. If a different owned weapon is found, equip it and show the message.

## Scope exclusions

- **Controller rebinding UI**: not in this iteration. Default bindings only.
- **Rumble/haptic feedback**: not in scope.
- **Right stick**: unassigned. Could be used for camera or fast-scroll in future.
- **Trigger analog values**: LT/RT not mapped. Triggers could be added later if needed.
- **Multiple controllers**: single controller only.
- **PlayStation-specific features**: touchpad, adaptive triggers — not applicable.

## Testing

### Unit tests (key_bindings.rs)

- `ControllerBindings::default_bindings()` returns correct action for each button in each mode.
- Gameplay mode: DPad Up → `UseCrystalVial`, A → `Fight`, LB → `WeaponPrev`.
- Menu mode: DPad Up → `MenuUp`, A → `MenuConfirm`, B → `MenuCancel`, LB → `WeaponPrev`.
- Unknown buttons return `None` in both modes.

### Unit tests (MenuCursor)

- Navigation wraps: row 5+1 → 0, row 0-1 → 5.
- Column toggles: col 0 → 1, col 1 → 0.
- Slot calculation: `row * 2 + col` matches expected slot index.
- Position persists across `active` toggles.

### Unit tests (weapon cycling)

- Cycles through owned weapons, skips empty slots.
- Wraps correctly in both directions.
- No-op when only one weapon owned.
- No-op when no weapons owned.

### Manual testing

- Play-test with physical Xbox/PS controller.
- Verify left stick movement in both modes.
- Verify DPad behavior switches on Start toggle.
- Verify outline renders at correct HI bar grid position.
- Verify weapon cycling message appears.
