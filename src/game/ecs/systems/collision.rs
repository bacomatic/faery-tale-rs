//! CollisionSystem — computes battleflag from entity proximity.
//! Does NOT perform movement collision (that's in MovementSystem/NpcMovementSystem).

use hecs::World;
use crate::game::ecs::components::{Enemy, Position, Health};
use crate::game::ecs::resources::Resources;

/// Distance threshold for battleflag (original: 300px each axis, not Euclidean).
const BATTLE_RANGE: f32 = 300.0;

pub fn run(world: &World, res: &mut Resources) {
    let hero_pos = match world.get::<&Position>(res.hero_entity) {
        Ok(p) => *p,
        Err(_) => return,
    };

    let battleflag = world
        .query::<(&Position, &Health)>()
        .with::<&Enemy>()
        .iter()
        .any(|(pos, health)| {
            !health.is_dead()
                && (pos.x - hero_pos.x).abs() < BATTLE_RANGE
                && (pos.y - hero_pos.y).abs() < BATTLE_RANGE
        });

    res.region.battleflag = battleflag;
}

#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::components::*;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::spawn::*;
    use super::run;

    fn hero_stats() -> HeroStats {
        HeroStats { vitality: 100, brave: 0, luck: 0, kind: 0,
                    wealth: 0, hunger: 0, fatigue: 0, gold: 0 }
    }

    #[test]
    fn battleflag_set_when_enemy_nearby() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        // Enemy within 300px
        spawn_enemy(&mut world, 150.0, 150.0, 1, 0, 20, 0, 0, 3, 5, 0);
        run(&world, &mut res);
        assert!(res.region.battleflag);
    }

    #[test]
    fn battleflag_clear_when_no_enemies() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        res.region.battleflag = true;
        run(&world, &mut res);
        assert!(!res.region.battleflag);
    }

    #[test]
    fn battleflag_clear_when_enemy_far() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 0.0, 0.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        // Enemy > 300px away
        spawn_enemy(&mut world, 500.0, 500.0, 1, 0, 20, 0, 0, 3, 5, 0);
        run(&world, &mut res);
        assert!(!res.region.battleflag);
    }
}
