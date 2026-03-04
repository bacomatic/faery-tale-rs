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
    LookAround,
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

    // UI / meta
    Confirm,
    Cancel,
    Menu,
    Rebind,
}

impl GameAction {
    /// All variants in display order.
    pub fn all_actions() -> &'static [GameAction] {
        use GameAction::*;
        &[
            MoveUp, MoveDown, MoveLeft, MoveRight,
            MoveUpLeft, MoveUpRight, MoveDownLeft, MoveDownRight,
            Fight, Attack, Shoot,
            Pause, Inventory, Take, Look, LookAround, UseItem, Give, GetItem,
            DropItem, Yell, Speak, Talk, Ask, Map, Find, Sleep, Quit,
            LoadGame, SaveGame, ExitMenu,
            CastSpell1, CastSpell2, CastSpell3, CastSpell4,
            CastSpell5, CastSpell6, CastSpell7,
            UseSlot1, UseSlot2, UseSlot3, UseSlot4,
            UseSlot5, UseSlot6, UseSlot7,
            UseSpecial, Board, SummonTurtle,
            BuyFood, BuyArrow, BuyVial, BuyMace, BuySword, BuyBow, BuyTotem,
            SelectKey1, SelectKey2, SelectKey3, SelectKey4, SelectKey5, SelectKey6,
            Confirm, Cancel, Menu, Rebind,
        ]
    }

    pub fn display_name(self) -> &'static str {
        use GameAction::*;
        match self {
            MoveUp => "Move Up", MoveDown => "Move Down",
            MoveLeft => "Move Left", MoveRight => "Move Right",
            MoveUpLeft => "Move Up-Left", MoveUpRight => "Move Up-Right",
            MoveDownLeft => "Move Down-Left", MoveDownRight => "Move Down-Right",
            Fight => "Fight", Attack => "Attack", Shoot => "Shoot",
            Pause => "Pause", Inventory => "Inventory", Take => "Take",
            Look => "Look", LookAround => "Look Around", UseItem => "Use Item",
            Give => "Give", GetItem => "Get Item", DropItem => "Drop Item",
            Yell => "Yell", Speak => "Speak", Talk => "Talk", Ask => "Ask",
            Map => "Map", Find => "Find", Sleep => "Sleep", Quit => "Quit",
            LoadGame => "Load Game", SaveGame => "Save Game", ExitMenu => "Exit Menu",
            CastSpell1 => "Cast Spell 1", CastSpell2 => "Cast Spell 2",
            CastSpell3 => "Cast Spell 3", CastSpell4 => "Cast Spell 4",
            CastSpell5 => "Cast Spell 5", CastSpell6 => "Cast Spell 6",
            CastSpell7 => "Cast Spell 7",
            UseSlot1 => "Use Slot 1", UseSlot2 => "Use Slot 2",
            UseSlot3 => "Use Slot 3", UseSlot4 => "Use Slot 4",
            UseSlot5 => "Use Slot 5", UseSlot6 => "Use Slot 6",
            UseSlot7 => "Use Slot 7",
            UseSpecial => "Use Special", Board => "Board", SummonTurtle => "Summon Turtle",
            BuyFood => "Buy Food", BuyArrow => "Buy Arrow", BuyVial => "Buy Vial",
            BuyMace => "Buy Mace", BuySword => "Buy Sword", BuyBow => "Buy Bow",
            BuyTotem => "Buy Totem",
            SelectKey1 => "Select Key 1", SelectKey2 => "Select Key 2",
            SelectKey3 => "Select Key 3", SelectKey4 => "Select Key 4",
            SelectKey5 => "Select Key 5", SelectKey6 => "Select Key 6",
            Confirm => "Confirm", Cancel => "Cancel", Menu => "Menu", Rebind => "Rebind",
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
        // Arrow keys for movement
        b.insert(GameAction::MoveUp,        vec![Keycode::Up, Keycode::W, Keycode::Kp8]);
        b.insert(GameAction::MoveDown,      vec![Keycode::Down, Keycode::S, Keycode::Kp2]);
        b.insert(GameAction::MoveLeft,      vec![Keycode::Left, Keycode::A, Keycode::Kp4]);
        b.insert(GameAction::MoveRight,     vec![Keycode::Right, Keycode::D, Keycode::Kp6]);
        b.insert(GameAction::MoveUpLeft,    vec![Keycode::Q, Keycode::Kp7]);
        b.insert(GameAction::MoveUpRight,   vec![Keycode::E, Keycode::Kp9]);
        b.insert(GameAction::MoveDownLeft,  vec![Keycode::Z, Keycode::Kp1]);
        b.insert(GameAction::MoveDownRight, vec![Keycode::C, Keycode::Kp3]);
        // Combat
        b.insert(GameAction::Fight, vec![Keycode::Space, Keycode::F]);
        // Menu/UI — original letter keys from letter_list[]
        b.insert(GameAction::Inventory, vec![Keycode::I]);
        b.insert(GameAction::Take,      vec![Keycode::T]);
        b.insert(GameAction::Look,      vec![Keycode::L]);
        b.insert(GameAction::UseItem,   vec![Keycode::U]);
        b.insert(GameAction::Give,      vec![Keycode::G]);
        b.insert(GameAction::Yell,      vec![Keycode::Y]);
        b.insert(GameAction::Speak,     vec![Keycode::Period]);
        b.insert(GameAction::Ask,       vec![Keycode::Question]);
        b.insert(GameAction::Map,       vec![Keycode::M]);
        b.insert(GameAction::Find,      vec![Keycode::Backslash]);
        b.insert(GameAction::Quit,      vec![Keycode::Escape]);
        b.insert(GameAction::SaveGame,  vec![Keycode::F2]);
        b.insert(GameAction::LoadGame,  vec![Keycode::F3]);
        // Spell keys — original: function keys F5–F9 or number keys
        b.insert(GameAction::CastSpell1, vec![Keycode::Num1]);
        b.insert(GameAction::CastSpell2, vec![Keycode::Num2]);
        b.insert(GameAction::CastSpell3, vec![Keycode::Num3]);
        b.insert(GameAction::CastSpell4, vec![Keycode::Num4]);
        b.insert(GameAction::CastSpell5, vec![Keycode::Num5]);
        b.insert(GameAction::CastSpell6, vec![Keycode::Num6]);
        b.insert(GameAction::CastSpell7, vec![Keycode::Num7]);
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
    fn default() -> Self { Self::default_bindings() }
}

/// Maps SDL2 game controller buttons to game actions.
#[derive(Debug, Clone)]
pub struct ControllerBindings {
    bindings: HashMap<Button, GameAction>,
}

impl ControllerBindings {
    pub fn default_bindings() -> Self {
        let mut m = HashMap::new();
        use Button::*;
        m.insert(DPadUp,    GameAction::MoveUp);
        m.insert(DPadDown,  GameAction::MoveDown);
        m.insert(DPadLeft,  GameAction::MoveLeft);
        m.insert(DPadRight, GameAction::MoveRight);
        m.insert(A,         GameAction::Confirm);
        m.insert(B,         GameAction::Cancel);
        m.insert(Start,     GameAction::Menu);
        m.insert(Back,      GameAction::Inventory);
        ControllerBindings { bindings: m }
    }

    pub fn action_for_button(&self, btn: Button) -> Option<GameAction> {
        self.bindings.get(&btn).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_action_for_key() {
        let kb = KeyBindings::default_bindings();
        assert_eq!(kb.action_for_key(Keycode::Up), Some(GameAction::MoveUp));
        assert_eq!(kb.action_for_key(Keycode::Space), Some(GameAction::Fight));
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
}
