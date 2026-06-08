//! Handles [`DebugCommand`] values from the debug console in [`EcsScene`].
//!
//! Called once per tick from `EcsScene::run_tick()` after draining the debug
//! console's command queue.

use hecs::World;

use crate::game::debug_command::DebugCommand;
use crate::game::ecs::components::Position;
use crate::game::ecs::resources::Resources;

/// Dispatch a single debug command against the ECS world and resources.
pub fn handle(cmd: DebugCommand, world: &mut World, res: &mut Resources) {
    match cmd {
        DebugCommand::SetGodMode { flags } => {
            res.brother.god_mode = flags;
        }

        DebugCommand::TeleportCoords { x, y } => {
            if let Ok(mut pos) = world.get::<&mut Position>(res.hero_entity) {
                pos.set(x as f32, y as f32);
            }
        }

        // ── Commands not yet wired in Plan D — will be added incrementally ──
        // SetStat, AdjustStat, InstaKill, HeroPack, TeleportSafe, etc.
        // For now they are silently consumed so we don't panic on unknown input.
        _ => {
            // TODO(Plan D/F): implement remaining debug commands
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hecs::World;
    use crate::game::ecs::components::{Hero, Position};
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::spawn::spawn_hero;
    use crate::game::ecs::components::{HeroStats, Inventory};
    use crate::game::debug_command::GodModeFlags;

    fn make_world_and_res() -> (World, Resources) {
        let mut world = World::new();
        let stats = HeroStats {
            vitality: 100,
            brave: 50,
            luck: 50,
            kind: 50,
            wealth: 50,
            hunger: 0,
            fatigue: 0,
            gold: 0,
        };
        let hero = spawn_hero(&mut world, 100.0, 200.0, 0, stats, Inventory::empty());
        let mut res = Resources::new(hero);
        res.hero_entity = hero;
        (world, res)
    }

    #[test]
    fn teleport_coords_moves_hero() {
        let (mut world, mut res) = make_world_and_res();
        handle(
            DebugCommand::TeleportCoords { x: 300, y: 400 },
            &mut world,
            &mut res,
        );
        let pos = world.get::<&Position>(res.hero_entity).unwrap();
        assert_eq!(pos.x, 300.0);
        assert_eq!(pos.y, 400.0);
    }

    #[test]
    fn set_god_mode_updates_resources() {
        let (mut world, mut res) = make_world_and_res();
        let flags = GodModeFlags::INVINCIBLE | GodModeFlags::NOCLIP;
        handle(DebugCommand::SetGodMode { flags }, &mut world, &mut res);
        assert_eq!(res.brother.god_mode, flags);
    }

    #[test]
    fn unknown_command_does_not_panic() {
        let (mut world, mut res) = make_world_and_res();
        // InstaKill is not yet wired; should silently do nothing.
        handle(DebugCommand::InstaKill, &mut world, &mut res);
    }
}
