//! DoorSystem — detects hero collision with door rectangles and emits region transitions.
//! Port of door bump/walk-through logic from gameplay_scene (input.rs lines 460–511).
//! See docs/spec/doors-buildings.md.

use hecs::World;
use crate::game::ecs::components::Position;
use crate::game::ecs::resources::Resources;

pub fn run(world: &World, res: &mut Resources) {
    let _hero_pos = match world.get::<&Position>(res.hero_entity) {
        Ok(p) => *p,
        Err(_) => return,
    };

    // Door data is loaded from GameLibrary.doors and filtered per-region.
    // The full door logic requires:
    //   1. A door table (Vec<DoorEntry>) in Resources,
    //   2. Terrain collision data to detect door-tile bumps,
    //   3. Key/inventory checks for locked doors.
    //
    // These are currently owned by GameplayScene (self.doors, self.map_world).
    // TODO(Plan D): migrate door table and opened-door set into Resources,
    // then implement doorfind + doorfind_exit checks here.
}

#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::components::*;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::spawn::spawn_hero;
    use super::run;

    fn hero_stats() -> HeroStats {
        HeroStats { vitality: 100, brave: 0, luck: 0, kind: 0,
                    wealth: 0, hunger: 0, fatigue: 0, gold: 0 }
    }

    #[test]
    fn door_system_no_panic_empty_world() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        run(&world, &mut res);
        assert!(res.events.region.is_empty());
    }

    #[test]
    fn door_system_no_transition_without_doors() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 5000.0, 5000.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        run(&world, &mut res);
        assert!(res.events.region.is_empty());
    }
}
