//! NarrativeSystem — advances the narrative queue (placard sequences, speech).
//! Port of tick_narrative_sequence() and execute_active_narrative_step()
//! from gameplay_scene/narrative.rs.
//! See docs/spec/intro-narrative.md.
//!
//! TODO(Plan D): Full implementation requires NarrativeQueue and GameLibrary
//! to be accessible through Resources. Currently a structural stub.

use hecs::World;
use crate::game::ecs::resources::Resources;

pub fn run(_world: &mut World, _res: &mut Resources) {
    // TODO(Plan D): Tick the narrative sequence queue here.
    //
    // When NarrativeQueue is migrated to Resources (Plan D), this system will:
    // 1. Check if a narrative sequence is active (res.narrative.active_sequence).
    // 2. Decrement the wait timer if paused between steps.
    // 3. When timer expires, call execute_active_step() to advance.
    // 4. Handle placard display, speech bubbles, fade transitions.
    //
    // Source logic: gameplay_scene/narrative.rs tick_narrative_sequence()
    //              and execute_active_narrative_step().
}

#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::components::Hero;
    use crate::game::ecs::resources::Resources;
    use super::run;

    fn make_resources(world: &mut World) -> Resources {
        let hero = world.spawn((Hero,));
        Resources::new(hero)
    }

    #[test]
    fn narrative_no_panic_empty() {
        let mut world = World::new();
        let mut res = make_resources(&mut world);
        run(&mut world, &mut res);
    }

    #[test]
    fn narrative_idempotent() {
        let mut world = World::new();
        let mut res = make_resources(&mut world);
        // Running multiple times should be safe.
        run(&mut world, &mut res);
        run(&mut world, &mut res);
        run(&mut world, &mut res);
    }
}
