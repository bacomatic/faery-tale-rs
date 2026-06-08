//! RegionSystem — handles region transitions and reloads world/NPC data.
//! Port of on_region_changed() from gameplay_scene/region.rs.
//! See docs/spec/world-structure.md.
//!
//! TODO(Plan D): Full implementation requires GameLibrary and asset data (ADF)
//! to be accessible via Resources. Currently processes RegionTransitionEvent
//! and updates RegionState.

use hecs::World;
use crate::game::ecs::resources::Resources;
use crate::game::ecs::events::RegionTransitionEvent;

pub fn run(_world: &mut World, res: &mut Resources) {
    let transitions: Vec<RegionTransitionEvent> = std::mem::take(&mut res.events.region);

    for event in transitions {
        // TODO(Plan D): Full region transition — reload WorldData from ADF,
        // rebuild NPC table, apply region palette, reposition camera.
        // See gameplay_scene/region.rs on_region_changed().
        res.region.region_num = event.new_region;
        res.region.new_region = event.new_region;
    }
}

#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::components::Hero;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::events::RegionTransitionEvent;
    use super::run;

    fn make_resources(world: &mut World) -> Resources {
        let hero = world.spawn((Hero,));
        Resources::new(hero)
    }

    #[test]
    fn region_transition_updates_region_num() {
        let mut world = World::new();
        let mut res = make_resources(&mut world);
        res.events.region.push(RegionTransitionEvent {
            new_region: 3,
            dest_x: 100.0,
            dest_y: 200.0,
        });
        run(&mut world, &mut res);
        assert_eq!(res.region.region_num, 3);
        assert_eq!(res.region.new_region, 3);
    }

    #[test]
    fn multiple_transitions_applies_last() {
        let mut world = World::new();
        let mut res = make_resources(&mut world);
        res.events.region.push(RegionTransitionEvent {
            new_region: 2, dest_x: 0.0, dest_y: 0.0,
        });
        res.events.region.push(RegionTransitionEvent {
            new_region: 7, dest_x: 50.0, dest_y: 60.0,
        });
        run(&mut world, &mut res);
        assert_eq!(res.region.region_num, 7);
    }

    #[test]
    fn no_events_no_panic() {
        let mut world = World::new();
        let mut res = make_resources(&mut world);
        run(&mut world, &mut res);
        assert_eq!(res.region.region_num, 0); // unchanged
    }
}
