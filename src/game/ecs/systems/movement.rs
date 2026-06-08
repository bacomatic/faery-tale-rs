//! MovementSystem — applies hero movement intents to Position.
//! Port of hero movement logic from gameplay_scene/input.rs.
//! TODO(Plan D): requires hero velocity and InputState in Resources.

use hecs::World;
use crate::game::ecs::resources::Resources;

pub fn run(_world: &mut World, _res: &mut Resources) {
    // TODO(Plan D): apply hero ActorMotion velocity to Position component.
}

#[cfg(test)]
mod tests {
    #[test]
    fn movement_stub_compiles() {}
}
