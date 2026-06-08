//! InputSystem — processes raw input events into hero action intents.
//! Port of input handling from gameplay_scene/input.rs.
//! TODO(Plan D): requires InputState migration to Resources.

use hecs::World;
use crate::game::ecs::resources::Resources;

pub fn run(_world: &mut World, _res: &mut Resources) {
    // TODO(Plan D): migrate InputState from GameplayScene to Resources.
}

#[cfg(test)]
mod tests {
    #[test]
    fn input_stub_compiles() {}
}
