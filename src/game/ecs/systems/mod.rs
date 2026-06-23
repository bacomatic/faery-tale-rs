//! Gameplay systems. Each module contains one `pub fn run(...)`.
//! Execution order: clock → input → sleep → movement → carrier →
//! collision → door → zone → npc_ai → npc_movement → combat →
//! missile → encounter → proximity → item → narrative → death → region.

pub mod clock;
pub mod input;
pub mod movement;
pub mod carrier;
pub mod collision;
pub mod door;
pub mod zone;
pub mod npc_ai;
pub mod npc_movement;
pub mod combat;
pub mod damage;
pub mod missile;
pub mod encounter;
pub mod proximity;
pub mod item;
pub mod narrative;
pub mod death;
pub mod region;
pub mod render;
