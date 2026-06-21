//! NpcAiSystem — runs AI decision pass for all enemy entities.
//! Port of update_actors() AI pass and tick_npc() from gameplay_scene/actors.rs
//! and game/npc_ai.rs.
//!
//! This system writes AiState and Facing for each enemy.
//! It does NOT write Position (that is NpcMovementSystem's job).

use hecs::{Entity, World};
use crate::game::ecs::components::{
    Enemy, ArenaDummy, Position, Facing, AiState, EnemyKind, Health, HeroStats, Loot,
};
use crate::game::ecs::resources::Resources;
use crate::game::actor::Goal;
use crate::game::npc::NpcState;
use crate::game::npc_ai::tick_npc_ecs;

pub fn run(world: &mut World, res: &mut Resources) {
    let hero_pos = match world.get::<&Position>(res.hero_entity) {
        Ok(p) => *p,
        Err(_) => return,
    };
    let hero_dead = world
        .get::<&HeroStats>(res.hero_entity)
        .map(|s| s.is_dead())
        .unwrap_or(true);

    let freeze = res.clock.is_frozen();
    let tick = res.clock.tick_counter;
    let xtype = res.region.xtype;
    let turtle_eggs = false; // SPEC-GAP: not yet tracked in Resources

    // Snapshot enemy positions for follow/evade targeting (Entity, x, y).
    let positions: Vec<(Entity, f32, f32)> = world
        .query::<(Entity, &Position)>()
        .with::<&Enemy>()
        .iter()
        .map(|(e, p)| (e, p.x, p.y))
        .collect();

    // Find leader entity: first active hostile.
    let leader_entity: Option<Entity> = {
        let mut found = None;
        for (e, ai) in world.query::<(Entity, &AiState)>().with::<&Enemy>().iter() {
            if !matches!(ai.state, NpcState::Dead) &&
               matches!(ai.goal, Goal::Attack1 | Goal::Attack2 |
                                 Goal::Archer1 | Goal::Archer2) {
                found = Some(e);
                break;
            }
        }
        found
    };

    // Collect entities to tick (avoid mid-loop borrow conflicts).
    let enemies: Vec<Entity> = world
        .query::<(Entity, &AiState)>()
        .with::<&Enemy>()
        .without::<&ArenaDummy>()
        .iter()
        .map(|(e, _)| e)
        .collect();

    for entity in enemies {
        // Read race, state, health, and weapon without holding a borrow.
        let (race, state, vitality, weapon) = {
            let mut q = world.query_one::<(&EnemyKind, &AiState, &Health, &Loot)>(entity);
            match q.get() {
                Ok((k, ai, health, loot)) => {
                    (k.race, ai.state.clone(), health.vitality, loot.weapon)
                }
                Err(_) => continue,
            }
        };

        if matches!(state, NpcState::Dead) { continue; }

        // Freeze gate: hostile NPCs (race < 7) skip AI when frozen.
        if freeze && race < 7 { continue; }
        // SETFIG races (>= 0x80) skip the goal FSM entirely.
        if race >= 0x80 { continue; }

        let pos = match world.get::<&Position>(entity) {
            Ok(p) => *p,
            Err(_) => continue,
        };

        // Build position list of other enemies (for flocking/evade).
        let others: Vec<(f32, f32)> = positions
            .iter()
            .filter(|(e, _, _)| *e != entity)
            .map(|(_, x, y)| (*x, *y))
            .collect();

        let is_leader = leader_entity == Some(entity);
        let leader_pos = leader_entity
            .filter(|&le| le != entity)
            .and_then(|le| world.get::<&Position>(le).ok().map(|p| (p.x, p.y)));

        // Get mutable references and run the AI tick.
        if let (Ok(mut ai), Ok(mut facing)) = (
            world.get::<&mut AiState>(entity),
            world.get::<&mut Facing>(entity),
        ) {
            tick_npc_ecs(
                &mut ai,
                &mut facing,
                pos.x, pos.y,
                hero_pos.x, hero_pos.y,
                hero_dead,
                is_leader,
                leader_pos,
                &others,
                tick,
                xtype,
                turtle_eggs,
                freeze,
                vitality,
                weapon,
                race,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::actor::Goal;
    use crate::game::ecs::components::*;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::spawn::*;
    use crate::game::npc::{NpcState, RACE_ENEMY};
    use super::run;

    fn hero_stats() -> HeroStats {
        HeroStats { vitality: 100, brave: 0, luck: 0, kind: 0,
                    wealth: 0, hunger: 0, fatigue: 0, gold: 0 }
    }

    #[test]
    fn frozen_hostile_npc_skips_ai() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 200.0, 200.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        res.clock.freeze_timer = 5; // frozen
        let enemy = spawn_enemy(&mut world, 100.0, 100.0, 1,
            0 /* race < 7 = hostile */, 20, 0, 0, 3, 0, 0);
        world.get::<&mut AiState>(enemy).unwrap().state = NpcState::Still;
        run(&mut world, &mut res);
        // Hostile NPC (race < 7) skips AI when frozen — state unchanged
        let state = world.get::<&AiState>(enemy).unwrap().state.clone();
        assert_eq!(state, NpcState::Still);
    }

    #[test]
    fn ai_runs_when_not_frozen() {
        // Just verify the system runs without panicking when not frozen.
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 200.0, 200.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        res.clock.freeze_timer = 0;
        spawn_enemy(&mut world, 100.0, 100.0, 1, 0, 20, 0, 0, 3, 5, 0);
        run(&mut world, &mut res); // should not panic
    }

    #[test]
    fn healthy_enemy_does_not_flee() {
        // Regression: tick_npc_ecs defaulted vitality to 0, so every healthy
        // enemy got Goal::Flee from select_tactic's low-vitality override.
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 200.0, 200.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        res.clock.freeze_timer = 0;
        res.region.xtype = 3; // normal terrain, not a special zone
        let enemy = spawn_enemy(&mut world, 100.0, 100.0, 1,
            RACE_ENEMY, 20, 1, 0, 2, 0, 0);
        world.get::<&mut AiState>(enemy).unwrap().goal = Goal::Attack1;
        run(&mut world, &mut res);
        let goal = world.get::<&AiState>(enemy).unwrap().goal.clone();
        assert_eq!(goal, Goal::Attack1,
            "healthy armed enemy outside a special zone should keep attacking");
    }
}
