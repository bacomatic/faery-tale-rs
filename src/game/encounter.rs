//! Encounter spawning stubs (npc-104).
use crate::game::npc::Npc;

pub fn should_encounter(_tick: u32) -> bool { false }
pub fn spawn_encounter(_zone: usize, _x: i16, _y: i16) -> Npc { Npc::default() }
