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
