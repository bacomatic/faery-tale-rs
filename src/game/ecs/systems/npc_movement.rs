//! NpcMovementSystem — executes movement for Walking enemy entities.
//! Port of update_actors() movement pass from gameplay_scene/actors.rs.

use hecs::World;
use crate::game::ecs::components::{Enemy, ArenaDummy, Position, Facing, AiState, Speed, FrustFlag};
use crate::game::ecs::resources::Resources;
use crate::game::npc::NpcState;

pub fn run(world: &mut World, res: &mut Resources) {
    if res.clock.is_frozen() { return; }

    let hero_pos = match world.get::<&Position>(res.hero_entity) {
        Ok(p) => *p,
        Err(_) => return,
    };

    // Snapshot all enemy positions for collision avoidance.
    let snapshot: Vec<(hecs::Entity, f32, f32)> = world
        .query::<(hecs::Entity, &Position)>()
        .with::<&Enemy>()
        .iter()
        .map(|(e, p)| (e, p.x, p.y))
        .collect();

    // Collect Walking enemies to move.
    let enemies: Vec<hecs::Entity> = world
        .query::<(hecs::Entity, &AiState)>()
        .with::<&Enemy>()
        .without::<&ArenaDummy>()
        .iter()
        .filter(|(_, ai)| matches!(ai.state, NpcState::Walking))
        .map(|(e, _)| e)
        .collect();

    let mut any_moved = false;

    for entity in enemies {
        let (facing_dir, speed, old_x, old_y) = {
            let mut q = world.query_one::<(&Facing, &Speed, &Position)>(entity);
            match q.get() {
                Ok((f, s, p)) => (f.dir, s.speed as i32, p.x, p.y),
                Err(_) => continue,
            }
        };

        let (dx, dy) = facing_dir.push_offset(speed);
        let new_x = old_x + dx as f32;
        let new_y = old_y + dy as f32;

        // Build collision list: hero + other enemies.
        let others: Vec<(i32, i32)> = std::iter::once((hero_pos.x as i32, hero_pos.y as i32))
            .chain(
                snapshot.iter()
                    .filter(|(e, _, _)| *e != entity)
                    .map(|(_, x, y)| (*x as i32, *y as i32))
            )
            .collect();

        let can_move_terrain = if let Some(world_data) = res.map.world.as_ref() {
            crate::game::collision::proxcheck(Some(world_data), new_x as i32, new_y as i32)
        } else {
            true // no map loaded — allow movement (used in tests)
        };

        let can_move_actors = !crate::game::collision::actor_collides(
            new_x as i32, new_y as i32, &others
        );

        if can_move_terrain && can_move_actors {
            if let Ok(mut pos) = world.get::<&mut Position>(entity) {
                pos.x = new_x;
                pos.y = new_y;
                any_moved = true;
            }
        }
    }

    // fmain.c: any NPC's successful walk resets the hero's frustflag.
    if any_moved {
        if let Ok(mut frust) = world.get::<&mut FrustFlag>(res.hero_entity) {
            frust.count = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::components::*;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::spawn::*;
    use crate::game::npc::NpcState;
    use crate::game::direction::Direction;
    use super::run;

    fn hero_stats() -> HeroStats {
        HeroStats { vitality: 100, brave: 0, luck: 0, kind: 0,
                    wealth: 0, hunger: 0, fatigue: 0, gold: 0 }
    }

    #[test]
    fn walking_npc_moves() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 200.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        let start_x = 100.0f32;
        let enemy = spawn_enemy(&mut world, start_x, 100.0, 1, 0, 20, 0, 0, 3, 0, 0);
        world.get::<&mut AiState>(enemy).unwrap().state = NpcState::Walking;
        world.get::<&mut Facing>(enemy).unwrap().dir = Direction::E;
        world.get::<&mut Speed>(enemy).unwrap().speed = 3;
        run(&mut world, &mut res);
        let new_x = world.get::<&Position>(enemy).unwrap().x;
        assert!(new_x > start_x, "NPC should move east: was {start_x}, now {new_x}");
    }

    #[test]
    fn frozen_npc_does_not_move() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 200.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        res.clock.freeze_timer = 5;
        let enemy = spawn_enemy(&mut world, 100.0, 100.0, 1, 0, 20, 0, 0, 3, 0, 0);
        world.get::<&mut AiState>(enemy).unwrap().state = NpcState::Walking;
        run(&mut world, &mut res);
        let x = world.get::<&Position>(enemy).unwrap().x;
        assert_eq!(x, 100.0, "Frozen NPC should not move");
    }

    #[test]
    fn still_npc_does_not_move() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 200.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        let start_x = 100.0f32;
        let enemy = spawn_enemy(&mut world, start_x, 100.0, 1, 0, 20, 0, 0, 3, 0, 0);
        // Leave state as Still (default from spawn_enemy)
        run(&mut world, &mut res);
        let x = world.get::<&Position>(enemy).unwrap().x;
        assert_eq!(x, start_x, "Still NPC should not move");
    }
}
