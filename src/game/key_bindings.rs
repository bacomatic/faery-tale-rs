use serde::{Deserialize, Serialize};

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

    // Menu / UI
    Pause,
    Inventory,
    Take,
    Look,
    UseItem,
    Give,
    Yell,
    Speak,
    Ask,
    Map,
    Find,
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
}
