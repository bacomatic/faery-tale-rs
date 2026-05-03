use sdl2::controller::Button;
use sdl2::keyboard::Keycode;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Every action that can be triggered by a key binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GameAction {
    // Movement (8 directions)
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    MoveUpLeft,
    MoveUpRight,
    MoveDownLeft,
    MoveDownRight,

    // Combat
    Fight,
    Attack,
    Shoot,

    // Menu / UI
    Pause,
    Inventory,
    Take,
    Look,
    UseItem,
    Give,
    GetItem,
    DropItem,
    Yell,
    Speak,
    Talk,
    Ask,
    Map,
    Find,
    Sleep,
    Quit,
    LoadGame,
    SaveGame,
    ExitMenu,

    // Magic spells (7 slots)
    CastSpell1,
    CastSpell2,
    CastSpell3,
    CastSpell4,
    CastSpell5,
    CastSpell6,
    CastSpell7,

    // Inventory use slots (7 slots)
    UseSlot1,
    UseSlot2,
    UseSlot3,
    UseSlot4,
    UseSlot5,
    UseSlot6,
    UseSlot7,

    // Special
    UseSpecial,
    Board,
    SummonTurtle,

    // Shop actions
    BuyFood,
    BuyArrow,
    BuyVial,
    BuyMace,
    BuySword,
    BuyBow,
    BuyTotem,

    // Key selection (6 key types)
    SelectKey1,
    SelectKey2,
    SelectKey3,
    SelectKey4,
    SelectKey5,
    SelectKey6,

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

    // UI / meta
    Confirm,
    Cancel,
    Menu,
    Rebind,
}

/// Controller input mode — determines which binding map is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ControllerMode {
    Gameplay,
    Menu,
}

impl GameAction {
    /// All variants in display order.
    pub fn all_actions() -> &'static [GameAction] {
        use GameAction::*;
        &[
            MoveUp,
            MoveDown,
            MoveLeft,
            MoveRight,
            MoveUpLeft,
            MoveUpRight,
            MoveDownLeft,
            MoveDownRight,
            Fight,
            Attack,
            Shoot,
            Pause,
            Inventory,
            Take,
            Look,
            UseItem,
            Give,
            GetItem,
            DropItem,
            Yell,
            Speak,
            Talk,
            Ask,
            Map,
            Find,
            Sleep,
            Quit,
            LoadGame,
            SaveGame,
            ExitMenu,
            CastSpell1,
            CastSpell2,
            CastSpell3,
            CastSpell4,
            CastSpell5,
            CastSpell6,
            CastSpell7,
            UseSlot1,
            UseSlot2,
            UseSlot3,
            UseSlot4,
            UseSlot5,
            UseSlot6,
            UseSlot7,
            UseSpecial,
            Board,
            SummonTurtle,
            BuyFood,
            BuyArrow,
            BuyVial,
            BuyMace,
            BuySword,
            BuyBow,
            BuyTotem,
            SelectKey1,
            SelectKey2,
            SelectKey3,
            SelectKey4,
            SelectKey5,
            SelectKey6,
            MenuUp,
            MenuDown,
            MenuLeft,
            MenuRight,
            MenuConfirm,
            MenuCancel,
            WeaponPrev,
            WeaponNext,
            UseCrystalVial,
            UseOrb,
            UseTotem,
            UseSkull,
            ToggleMenuMode,
            Confirm,
            Cancel,
            Menu,
            Rebind,
        ]
    }

    pub fn display_name(self) -> &'static str {
        use GameAction::*;
        match self {
            MoveUp => "Move Up",
            MoveDown => "Move Down",
            MoveLeft => "Move Left",
            MoveRight => "Move Right",
            MoveUpLeft => "Move Up-Left",
            MoveUpRight => "Move Up-Right",
            MoveDownLeft => "Move Down-Left",
            MoveDownRight => "Move Down-Right",
            Fight => "Fight",
            Attack => "Attack",
            Shoot => "Shoot",
            Pause => "Pause",
            Inventory => "Inventory",
            Take => "Take",
            Look => "Look",
            UseItem => "Use Item",
            Give => "Give",
            GetItem => "Get Item",
            DropItem => "Drop Item",
            Yell => "Yell",
            Speak => "Speak",
            Talk => "Talk",
            Ask => "Ask",
            Map => "Map",
            Find => "Find",
            Sleep => "Sleep",
            Quit => "Quit",
            LoadGame => "Load Game",
            SaveGame => "Save Game",
            ExitMenu => "Exit Menu",
            CastSpell1 => "Cast Spell 1",
            CastSpell2 => "Cast Spell 2",
            CastSpell3 => "Cast Spell 3",
            CastSpell4 => "Cast Spell 4",
            CastSpell5 => "Cast Spell 5",
            CastSpell6 => "Cast Spell 6",
            CastSpell7 => "Cast Spell 7",
            UseSlot1 => "Use Slot 1",
            UseSlot2 => "Use Slot 2",
            UseSlot3 => "Use Slot 3",
            UseSlot4 => "Use Slot 4",
            UseSlot5 => "Use Slot 5",
            UseSlot6 => "Use Slot 6",
            UseSlot7 => "Use Slot 7",
            UseSpecial => "Use Special",
            Board => "Board",
            SummonTurtle => "Summon Turtle",
            BuyFood => "Buy Food",
            BuyArrow => "Buy Arrow",
            BuyVial => "Buy Vial",
            BuyMace => "Buy Mace",
            BuySword => "Buy Sword",
            BuyBow => "Buy Bow",
            BuyTotem => "Buy Totem",
            SelectKey1 => "Select Key 1",
            SelectKey2 => "Select Key 2",
            SelectKey3 => "Select Key 3",
            SelectKey4 => "Select Key 4",
            SelectKey5 => "Select Key 5",
            SelectKey6 => "Select Key 6",
            MenuUp => "Menu Up",
            MenuDown => "Menu Down",
            MenuLeft => "Menu Left",
            MenuRight => "Menu Right",
            MenuConfirm => "Menu Confirm",
            MenuCancel => "Menu Cancel",
            WeaponPrev => "Prev Weapon",
            WeaponNext => "Next Weapon",
            UseCrystalVial => "Crystal Vial",
            UseOrb => "Jewel",
            UseTotem => "Totem",
            UseSkull => "Skull",
            ToggleMenuMode => "Menu Mode",
            Confirm => "Confirm",
            Cancel => "Cancel",
            Menu => "Menu",
            Rebind => "Rebind",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyBindings {
    #[serde(skip)]
    bindings: HashMap<GameAction, Vec<Keycode>>,
}

impl KeyBindings {
    pub fn default_bindings() -> Self {
        let mut b: HashMap<GameAction, Vec<Keycode>> = HashMap::new();
        // Movement: numpad 1-9 (original) + arrow keys (modern convenience).
        // Original did not use WASD; those letters have command meanings.
        b.insert(GameAction::MoveUp, vec![Keycode::Up, Keycode::Kp8]);
        b.insert(GameAction::MoveDown, vec![Keycode::Down, Keycode::Kp2]);
        b.insert(GameAction::MoveLeft, vec![Keycode::Left, Keycode::Kp4]);
        b.insert(GameAction::MoveRight, vec![Keycode::Right, Keycode::Kp6]);
        b.insert(GameAction::MoveUpLeft, vec![Keycode::Kp7]);
        b.insert(GameAction::MoveUpRight, vec![Keycode::Kp9]);
        b.insert(GameAction::MoveDownLeft, vec![Keycode::Kp1]);
        b.insert(GameAction::MoveDownRight, vec![Keycode::Kp3]);
        // Combat: numpad 0 (original), keep Space as alternate
        b.insert(GameAction::Fight, vec![Keycode::Kp0]);
        // Pause: Space (original)
        b.insert(GameAction::Pause, vec![Keycode::Space]);
        // Items menu — original letter keys from manual
        b.insert(GameAction::Inventory, vec![Keycode::L]); // List
        b.insert(GameAction::Take, vec![Keycode::T]);
        b.insert(GameAction::Look, vec![Keycode::Slash]); // '?' (Shift+/ on US)
        b.insert(GameAction::UseItem, vec![Keycode::U]);
        b.insert(GameAction::Give, vec![Keycode::G]);
        // Talk menu
        b.insert(GameAction::Yell, vec![Keycode::Y]);
        b.insert(GameAction::Speak, vec![Keycode::S]); // Say
        b.insert(GameAction::Ask, vec![Keycode::A]);
        // Game menu
        b.insert(GameAction::Quit, vec![Keycode::Q, Keycode::Escape]);
        b.insert(GameAction::SaveGame, vec![Keycode::F8]);
        b.insert(GameAction::LoadGame, vec![Keycode::F9]);
        // Magic: F1-F7 (original function keys)
        b.insert(GameAction::CastSpell1, vec![Keycode::F1]); // Stone
        b.insert(GameAction::CastSpell2, vec![Keycode::F2]); // Jewel
        b.insert(GameAction::CastSpell3, vec![Keycode::F3]); // Vial
        b.insert(GameAction::CastSpell4, vec![Keycode::F4]); // Orb
        b.insert(GameAction::CastSpell5, vec![Keycode::F5]); // Totem
        b.insert(GameAction::CastSpell6, vec![Keycode::F6]); // Ring
        b.insert(GameAction::CastSpell7, vec![Keycode::F7]); // Skull
                                                             // Weapon selection (Use sub-menu): number keys 1-5
        b.insert(GameAction::UseSlot1, vec![Keycode::Num1]); // Dirk
        b.insert(GameAction::UseSlot2, vec![Keycode::Num2]); // Mace
        b.insert(GameAction::UseSlot3, vec![Keycode::Num3]); // Sword
        b.insert(GameAction::UseSlot4, vec![Keycode::Num4]); // Bow
        b.insert(GameAction::UseSlot5, vec![Keycode::Num5]); // Wand
                                                             // Key selection: K prefix (K alone opens key sub-menu)
        b.insert(GameAction::SelectKey1, vec![]); // Gold  (K1 modal)
        b.insert(GameAction::SelectKey2, vec![]); // Green (K2 modal)
        b.insert(GameAction::SelectKey3, vec![]); // Blue  (K3 modal)
        b.insert(GameAction::SelectKey4, vec![]); // Red   (K4 modal)
        b.insert(GameAction::SelectKey5, vec![]); // Grey  (K5 modal)
        b.insert(GameAction::SelectKey6, vec![]); // White (K6 modal)
                                                  // Buy menu keys (original)
        b.insert(GameAction::BuyFood, vec![Keycode::O]);
        b.insert(GameAction::BuyArrow, vec![Keycode::R]);
        b.insert(GameAction::BuyVial, vec![Keycode::Num8]);
        b.insert(GameAction::BuyMace, vec![Keycode::C]);
        b.insert(GameAction::BuySword, vec![Keycode::W]);
        b.insert(GameAction::BuyBow, vec![Keycode::B]);
        b.insert(GameAction::BuyTotem, vec![Keycode::E]);
        // Map
        b.insert(GameAction::Map, vec![Keycode::M]);
        KeyBindings { bindings: b }
    }

    /// Look up what action (if any) a keycode is bound to.
    pub fn action_for_key(&self, keycode: Keycode) -> Option<GameAction> {
        for (action, keys) in &self.bindings {
            if keys.contains(&keycode) {
                return Some(*action);
            }
        }
        None
    }

    /// Override a binding.
    pub fn set_binding(&mut self, action: GameAction, keys: Vec<Keycode>) {
        self.bindings.insert(action, keys);
    }

    /// Restore all bindings to defaults.
    pub fn reset_to_defaults(&mut self) {
        *self = Self::default_bindings();
    }

    /// Returns the current bindings map (read-only).
    pub fn bindings(&self) -> &HashMap<GameAction, Vec<Keycode>> {
        &self.bindings
    }

    /// Rebind an action to a single key, replacing all previous bindings for that action.
    /// Also removes the key from any other action that had it.
    pub fn rebind(&mut self, action: GameAction, keycode: Keycode) {
        // Remove keycode from any other action's binding list
        for (a, keys) in self.bindings.iter_mut() {
            if *a != action {
                keys.retain(|&k| k != keycode);
            }
        }
        self.bindings.insert(action, vec![keycode]);
    }
}

impl Default for KeyBindings {
    fn default() -> Self {
        Self::default_bindings()
    }
}

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
        gameplay.insert(A, GameAction::Fight);
        gameplay.insert(B, GameAction::Take);
        gameplay.insert(X, GameAction::BuyFood); // BuyFood doubles as Eat
        gameplay.insert(Y, GameAction::Look);
        // Bumpers — weapon cycling
        gameplay.insert(LeftShoulder, GameAction::WeaponPrev);
        gameplay.insert(RightShoulder, GameAction::WeaponNext);
        // DPad — magic quick-select
        gameplay.insert(DPadUp, GameAction::UseCrystalVial);
        gameplay.insert(DPadDown, GameAction::UseOrb);
        gameplay.insert(DPadLeft, GameAction::UseTotem);
        gameplay.insert(DPadRight, GameAction::UseSkull);
        // Start/Back/Stick clicks
        gameplay.insert(Start, GameAction::ToggleMenuMode);
        gameplay.insert(Back, GameAction::Map);
        gameplay.insert(LeftStick, GameAction::Inventory);

        let mut menu = HashMap::new();
        // Face buttons
        menu.insert(A, GameAction::MenuConfirm);
        menu.insert(B, GameAction::MenuCancel);
        menu.insert(X, GameAction::BuyFood); // Eat still works in menu
        menu.insert(Y, GameAction::Look); // Look still works in menu
                                          // Bumpers — weapon cycling (unchanged)
        menu.insert(LeftShoulder, GameAction::WeaponPrev);
        menu.insert(RightShoulder, GameAction::WeaponNext);
        // DPad — menu navigation
        menu.insert(DPadUp, GameAction::MenuUp);
        menu.insert(DPadDown, GameAction::MenuDown);
        menu.insert(DPadLeft, GameAction::MenuLeft);
        menu.insert(DPadRight, GameAction::MenuRight);
        // Start exits menu mode too
        menu.insert(Start, GameAction::ToggleMenuMode);
        menu.insert(Back, GameAction::Map);
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
    fn default() -> Self {
        Self::default_bindings()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_action_for_key() {
        let kb = KeyBindings::default_bindings();
        assert_eq!(kb.action_for_key(Keycode::Up), Some(GameAction::MoveUp));
        assert_eq!(kb.action_for_key(Keycode::Space), Some(GameAction::Pause));
        assert_eq!(kb.action_for_key(Keycode::Kp0), Some(GameAction::Fight));
        assert_eq!(kb.action_for_key(Keycode::Return), None);
    }
    #[test]
    fn test_numpad_kp8_maps_to_move_up() {
        let kb = KeyBindings::default_bindings();
        assert_eq!(kb.action_for_key(Keycode::Kp8), Some(GameAction::MoveUp));
    }
    #[test]
    fn test_set_binding() {
        let mut kb = KeyBindings::default_bindings();
        kb.set_binding(GameAction::MoveUp, vec![Keycode::Kp8]);
        assert_eq!(kb.action_for_key(Keycode::Kp8), Some(GameAction::MoveUp));
    }

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
}
