//! CarrierSystem — manages raft/turtle/swan/dragon mount state.
//! Port of carrier logic from gameplay_scene/carriers.rs.
//! TODO(Plan D): requires carrier mount state in Resources.

use hecs::World;
use crate::game::ecs::resources::Resources;

pub fn run(_world: &mut World, _res: &mut Resources) {
    // TODO(Plan D): port carrier mount/dismount logic from gameplay_scene/carriers.rs.
}

#[cfg(test)]
mod tests {
    #[test]
    fn carrier_stub_compiles() {}
}
