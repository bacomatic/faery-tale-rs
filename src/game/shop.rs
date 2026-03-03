//! Shop system stubs (shop-101).
use crate::game::game_state::GameState;
use crate::game::npc::Npc;

pub fn has_shopkeeper_nearby(_npcs: &[Npc], _x: i16, _y: i16) -> bool { false }
pub fn buy_item(_state: &mut GameState, _item: usize) -> Result<i32, &'static str> {
    Err("No shop here")
}
