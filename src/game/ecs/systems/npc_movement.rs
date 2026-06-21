//! NpcMovementSystem — executes movement for Walking enemy entities.
//! Port of update_actors() movement pass from gameplay_scene/actors.rs.

use hecs::World;
use crate::game::collision::{apply_update_environ, speed_for_environ, EnvironTransition};
use crate::game::collision::px_to_terrain_type;
use crate::game::ecs::components::{
    ActorMotion, AiState, ArenaDummy, Enemy, EnemyKind, Facing, FrustFlag, Position, Speed,
};
use crate::game::ecs::resources::Resources;
use crate::game::npc::{NpcState, RACE_SNAKE, RACE_WRAITH};

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

    // Collect all active (non-dead, non-dummy) enemies for the environ + movement pass.
    let enemies: Vec<hecs::Entity> = world
        .query::<(hecs::Entity, &AiState)>()
        .with::<&Enemy>()
        .without::<&ArenaDummy>()
        .iter()
        .filter(|(_, ai)| !matches!(ai.state, NpcState::Dead))
        .map(|(e, _)| e)
        .collect();

    let world_data = res.map.world.as_ref();
    let mut any_moved = false;

    for entity in enemies {
        let (npc_state, race, old_x, old_y, old_environ) = {
            let mut q = world.query_one::<(&AiState, &EnemyKind, &Position, &ActorMotion)>(entity);
            match q.get() {
                Ok((ai, k, p, m)) => (ai.state.clone(), k.race, p.x, p.y, m.environ),
                Err(_) => continue,
            }
        };

        // update_environ runs every tick for every actor, regardless of movement state
        // (fmain.c actor_tick Phase 9 — walk_step/still_step both end with update_environ).
        // Wraiths (race 2) and snakes (race 4): j is zeroed before update_environ so they
        // are always treated as dry ground (fmain.c:1638). (fmain2.c:280 also skips
        // the proxcheck terrain test for wraiths during movement.)
        let zeroes_terrain = race == RACE_WRAITH || race == RACE_SNAKE;
        let j = if zeroes_terrain || world_data.is_none() {
            0u8
        } else {
            px_to_terrain_type(world_data.unwrap(), old_x as i32, old_y as i32)
        };

        let is_dying  = matches!(npc_state, NpcState::Dying | NpcState::Dead);
        let is_sinking = matches!(npc_state, NpcState::Sinking);
        let (new_k, transition) = apply_update_environ(j, old_environ, is_dying, is_sinking);

        // Write environ and apply state transitions.
        if let Ok(mut motion) = world.get::<&mut ActorMotion>(entity) {
            motion.environ = new_k;
        }
        match transition {
            EnvironTransition::EnterSink => {
                if let Ok(mut ai) = world.get::<&mut AiState>(entity) {
                    ai.state = NpcState::Sinking;
                }
            }
            EnvironTransition::ExitSink | EnvironTransition::Drown => {
                // Drown: k==30, stop moving (vitality damage is a SPEC-GAP, same as hero side).
                if let Ok(mut ai) = world.get::<&mut AiState>(entity) {
                    ai.state = NpcState::Still;
                }
            }
            EnvironTransition::None => {}
        }

        // Only Walking NPCs attempt position updates.
        if !matches!(npc_state, NpcState::Walking) {
            continue;
        }

        let (facing_dir, base_speed) = {
            let mut q = world.query_one::<(&Facing, &Speed)>(entity);
            match q.get() {
                Ok((f, s)) => (f.dir, s.speed),
                Err(_) => continue,
            }
        };

        // Speed from environ, scaled by the NPC's base speed ratio.
        // The base speed stored on the component is the spawn-time value (normally 2).
        // environ overrides it the same way hero_speed_for_env does for the hero.
        // (fmain.c:1599-1602 — speed is derived from k before the position attempt.)
        let env_speed = speed_for_environ(new_k);
        // Negative env_speed (astral reverse-field) → reverse direction; magnitude is the step.
        let (move_dir, step) = if env_speed < 0 {
            (facing_dir.opposite(), (-env_speed) as i32)
        } else {
            (facing_dir, (base_speed as i32 * env_speed as i32) / 2)
        };

        // Build collision list: hero + other enemies.
        let others: Vec<(i32, i32)> = std::iter::once((hero_pos.x as i32, hero_pos.y as i32))
            .chain(
                snapshot.iter()
                    .filter(|(e, _, _)| *e != entity)
                    .map(|(_, x, y)| (*x as i32, *y as i32))
            )
            .collect();

        // walk_step deviation: try primary, then CW+1, then CCW-1 (fmain.c:1603-1626).
        let is_wraith = race == RACE_WRAITH;
        let committed = try_directions(
            move_dir, step, old_x, old_y,
            world_data, &others, is_wraith,
        );

        if let Some((new_x, new_y)) = committed {
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

/// Try primary direction, then CW+1, then CCW-1 (net -1 from original).
/// Returns Some((new_x, new_y)) if any direction was clear, else None.
fn try_directions(
    primary: crate::game::direction::Direction,
    speed: i32,
    x: f32,
    y: f32,
    world_data: Option<&crate::game::world_data::WorldData>,
    others: &[(i32, i32)],
    is_wraith: bool,
) -> Option<(f32, f32)> {
    for dir in [primary, primary.rotate_cw(), primary.rotate_ccw()] {
        let (dx, dy) = dir.push_offset(speed);
        let nx = x + dx as f32;
        let ny = y + dy as f32;
        let terrain_ok = is_wraith || match world_data {
            Some(w) => crate::game::collision::proxcheck(Some(w), nx as i32, ny as i32),
            None => true,
        };
        let actors_ok = !crate::game::collision::actor_collides(nx as i32, ny as i32, others);
        if terrain_ok && actors_ok {
            return Some((nx, ny));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::components::*;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::spawn::*;
    use crate::game::npc::NpcState;
    use crate::game::direction::Direction;
    use super::{run, try_directions};

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

    // ── diagonal slide tests (fmain.c:1603-1626) ─────────────────────────────

    // A blocker that fills one specific position, simulating a wall.
    fn one_blocker(bx: i32, by: i32) -> Vec<(i32, i32)> {
        vec![(bx, by)]
    }

    #[test]
    fn primary_blocked_slides_cw() {
        // Speed 12 gives offsets large enough to escape the actor bounding box
        // (|dx|<11, |dy|<9): E→(112,100), SE→(112,112), NE→(112,88).
        // Blocker at primary E position only; CW slide (SE) should succeed.
        let result = try_directions(
            Direction::E, 12, 100.0, 100.0,
            None, &one_blocker(112, 100), false,
        );
        let (nx, ny) = result.expect("CW slide should find a clear path");
        assert_eq!((nx as i32, ny as i32), (112, 112));
    }

    #[test]
    fn primary_and_cw_blocked_slides_ccw() {
        // Block E and SE; CCW slide (NE) should succeed.
        let blockers = vec![(112, 100), (112, 112)];
        let result = try_directions(
            Direction::E, 12, 100.0, 100.0,
            None, &blockers, false,
        );
        let (nx, ny) = result.expect("CCW slide should find a clear path");
        assert_eq!((nx as i32, ny as i32), (112, 88));
    }

    #[test]
    fn all_three_blocked_returns_none() {
        // Block all three candidate positions; NPC must not move.
        let blockers = vec![(112, 100), (112, 112), (112, 88)];
        let result = try_directions(
            Direction::E, 12, 100.0, 100.0,
            None, &blockers, false,
        );
        assert!(result.is_none(), "Fully blocked NPC should not move");
    }

    // ── wraith terrain-bypass tests (fmain2.c:280) ────────────────────────────

    #[test]
    fn wraith_ignores_terrain_block() {
        // Without a world map, proxcheck defaults to true; with is_wraith=true the
        // terrain check is skipped entirely. Test that a wraith moves even when a
        // non-wraith would be stopped by terrain (simulated by passing None as world
        // data to ensure no false negatives, combined with actor-only blocker).
        // No actor blockers, no terrain (None map) — wraith should always move.
        let result = try_directions(
            Direction::E, 3, 100.0, 100.0,
            None, &[], true,
        );
        assert!(result.is_some(), "Wraith with no actor blockers should always move");
    }

    #[test]
    fn wraith_still_blocked_by_actors() {
        // Wraith ignores terrain but must respect actor bounding boxes.
        // Block all three candidate positions with actors.
        let blockers = vec![(103, 100), (103, 103), (103, 97)];
        let result = try_directions(
            Direction::E, 3, 100.0, 100.0,
            None, &blockers, true,
        );
        assert!(result.is_none(), "Wraith blocked by actors on all three directions should not move");
    }

    #[test]
    fn non_wraith_respects_terrain() {
        // Non-wraith with no map (terrain always passes) and no actor blockers should move.
        let result = try_directions(
            Direction::E, 3, 100.0, 100.0,
            None, &[], false,
        );
        assert!(result.is_some(), "Non-wraith with clear terrain should move");
    }
}
