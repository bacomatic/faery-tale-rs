//! RegionSystem — handles region transitions and reloads world/NPC data.
//! Port of on_region_changed() from gameplay_scene/region.rs.
//! See docs/spec/world-structure.md.

use hecs::World;
use crate::game::ecs::resources::Resources;
use crate::game::ecs::events::RegionTransitionEvent;
use crate::game::game_library::GameLibrary;

/// Drain `RegionTransitionEvent` queue. On a transition, store the last event
/// in `res.pending_transition` so `EcsScene::run_tick` can call
/// `reload_region()` (which needs `self.adf` and `self.base_colors`).
/// Also updates `res.region.region_num` immediately for downstream systems.
pub fn run(_world: &mut World, res: &mut Resources, _game_lib: &GameLibrary) {
    let transitions: Vec<RegionTransitionEvent> = std::mem::take(&mut res.events.region);

    for event in transitions {
        res.region.region_num = event.new_region;
        res.region.new_region = event.new_region;
        res.pending_transition = Some(event);
    }
}

#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::components::Hero;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::events::RegionTransitionEvent;
    use crate::game::game_library::GameLibrary;
    use super::run;

    fn make_resources(world: &mut World) -> Resources {
        let hero = world.spawn((Hero,));
        Resources::new(hero)
    }

    fn make_game_lib() -> GameLibrary {
        let config = std::fs::read_to_string("faery.toml")
            .expect("faery.toml must be present in project root for region tests");
        toml::from_str::<GameLibrary>(&config)
            .expect("faery.toml should deserialize without errors")
    }

    #[test]
    fn region_transition_updates_region_num() {
        let mut world = World::new();
        let mut res = make_resources(&mut world);
        let game_lib = make_game_lib();
        res.events.region.push(RegionTransitionEvent {
            new_region: 3,
            dest_x: 100.0,
            dest_y: 200.0,
        });
        run(&mut world, &mut res, &game_lib);
        assert_eq!(res.region.region_num, 3);
        assert_eq!(res.region.new_region, 3);
    }

    #[test]
    fn multiple_transitions_applies_last() {
        let mut world = World::new();
        let mut res = make_resources(&mut world);
        let game_lib = make_game_lib();
        res.events.region.push(RegionTransitionEvent {
            new_region: 2, dest_x: 0.0, dest_y: 0.0,
        });
        res.events.region.push(RegionTransitionEvent {
            new_region: 7, dest_x: 50.0, dest_y: 60.0,
        });
        run(&mut world, &mut res, &game_lib);
        assert_eq!(res.region.region_num, 7);
    }

    #[test]
    fn no_events_no_panic() {
        let mut world = World::new();
        let mut res = make_resources(&mut world);
        let game_lib = make_game_lib();
        run(&mut world, &mut res, &game_lib);
        assert_eq!(res.region.region_num, 0); // unchanged
    }

    #[test]
    fn transition_sets_pending() {
        let mut world = World::new();
        let mut res = make_resources(&mut world);
        let game_lib = make_game_lib();
        res.events.region.push(RegionTransitionEvent {
            new_region: 5,
            dest_x: 100.0,
            dest_y: 200.0,
        });
        run(&mut world, &mut res, &game_lib);
        let pending = res.pending_transition.take().expect("pending_transition should be set");
        assert_eq!(pending.new_region, 5);
        assert_eq!(pending.dest_x, 100.0);
        assert_eq!(pending.dest_y, 200.0);
    }
}
