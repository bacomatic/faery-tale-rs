//! Intra-frame event type for messages, sounds, and state transitions.
//! GameEvents are produced by gameplay logic and consumed by GameplayScene.

#[derive(Debug, Clone)]
pub enum GameEvent {
    /// Show a message in the scrolling text viewport.
    Message { text: String },
    /// Trigger a sound effect.
    Sound { sfx_id: u8 },
    /// Hero vitality dropped to 0 — trigger death/revive cycle.
    HeroDied,
    /// All three brothers are dead — game over.
    GameOver,
    /// Transition to a new region.
    RegionTransition { region: u8 },
    /// Enter a building/dungeon (region_num > 7).
    EnterIndoor { door_index: u8 },
    /// Return outdoors from indoor.
    ExitIndoor,
    /// Start a combat encounter.
    StartEncounter { encounter_type: u8, encounter_number: u8 },
    /// Combat encounter has ended (enemies defeated or fled).
    EndEncounter,
}
