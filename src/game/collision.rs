//! Collision detection stubs.
//! proxcheck and px_to_im return passable/normal terrain until the
//! terrain system (player-102) is fully wired with real map data.

use crate::game::game_state::GameState;

/// Returns true if the tile at absolute position (x, y) blocks movement.
/// Stub: always returns false (passable) until terrain system is wired.
pub fn proxcheck(_state: &GameState, _x: u16, _y: u16) -> bool {
    false
}

/// Returns the terrain type index at absolute position (x, y).
/// Stub: always returns 0 (normal/grassland) until terrain system is wired.
pub fn px_to_im(_state: &GameState, _x: u16, _y: u16) -> u8 {
    0
}
