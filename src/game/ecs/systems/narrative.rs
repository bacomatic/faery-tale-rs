//! Narrative system for managing scripted sequences and dialogue overlays.
//! Handles placard display, timed events, and narrative state transitions.

use hecs::World;
use crate::game::ecs::resources::{Resources, NarrEvent};

/// Run the narrative system.
/// Processes one active event at a time, managing timers and side effects.
pub fn run(world: &mut World, res: &mut Resources) {
    // If no active event, try to activate next.
    if res.narrative.active.is_none() {
        if !res.narrative.activate_next() {
            // No events to process, clear placard viewstatus.
            if res.view.viewstatus == 2 {
                res.view.viewstatus = 0;
            }
            return;
        }
    }

    // Tick the active event.
    if res.narrative.tick() {
        // Event expired — execute side effects, then advance to next.
        execute_event(world, res);
        res.narrative.activate_next();
        // Clear placard overlay if nothing more to show.
        if res.narrative.is_idle() && res.view.viewstatus == 2 {
            res.view.viewstatus = 0;
        }
    } else {
        // Event still active — set viewstatus for placards.
        if let Some(NarrEvent::Placard { .. }) = &res.narrative.active {
            res.view.viewstatus = 2;
        }
    }
}

/// Execute side effects for the just-expired active event.
fn execute_event(world: &mut World, res: &mut Resources) {
    if let Some(event) = res.narrative.active.take() {
        match event {
            NarrEvent::Placard { .. } | NarrEvent::WaitTicks(_) => {
                // No side effects beyond display/timing.
            }
            NarrEvent::TeleportHero { x, y, region } => {
                if let Ok(mut pos) = world.get::<&mut crate::game::ecs::components::Position>(res.hero_entity) {
                    pos.set(x, y);
                }
                res.region.new_region = region;
            }
            NarrEvent::SwapObjectId { object_index, new_id } => {
                res.diag_log.push(format!("SwapObjectId not yet implemented: obj={object_index}, id={new_id}"));
            }
            NarrEvent::ApplyRewards => {
                res.diag_log.push("ApplyRewards not yet implemented".to_string());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hecs::World;
    use crate::game::ecs::components::{Hero, Position};
    use crate::game::ecs::resources::{Resources, NarrEvent};

    fn make_resources(world: &mut World) -> Resources {
        let hero = world.spawn((Hero,));
        Resources::new(hero)
    }

    #[test]
    fn narrative_no_panic_empty() {
        let mut world = World::new();
        let mut res = make_resources(&mut world);
        run(&mut world, &mut res);
        assert!(res.narrative.is_idle());
        assert_eq!(res.view.viewstatus, 0);
    }

    #[test]
    fn placard_timer_expires() {
        let mut world = World::new();
        let mut res = make_resources(&mut world);

        res.narrative.push(NarrEvent::Placard {
            text: "Test placard".to_string(),
            hold_ticks: 3,
        });

        // Tick 1: activates, viewstatus set to 2
        run(&mut world, &mut res);
        assert_eq!(res.view.viewstatus, 2);
        assert!(!res.narrative.is_idle());

        // Tick 2: still active
        run(&mut world, &mut res);
        assert_eq!(res.view.viewstatus, 2);
        assert!(!res.narrative.is_idle());

        // Tick 3: expires, viewstatus cleared
        run(&mut world, &mut res);
        assert_eq!(res.view.viewstatus, 0);
        assert!(res.narrative.is_idle());
    }

    #[test]
    fn wait_ticks_advances() {
        let mut world = World::new();
        let mut res = make_resources(&mut world);

        res.narrative.push(NarrEvent::WaitTicks(2));

        // Tick 1: activate, still ticking
        run(&mut world, &mut res);
        assert!(!res.narrative.is_idle());

        // Tick 2: expires
        run(&mut world, &mut res);
        assert!(res.narrative.is_idle());
    }

    #[test]
    fn queue_processes_all_events() {
        let mut world = World::new();
        let mut res = make_resources(&mut world);

        res.narrative.push(NarrEvent::WaitTicks(1));
        res.narrative.push(NarrEvent::Placard {
            text: "Second".to_string(),
            hold_ticks: 2,
        });
        res.narrative.push(NarrEvent::WaitTicks(1));

        // Tick 1: WaitTicks(1) activates and expires; Placard becomes active but
        // viewstatus is not yet set (set on next tick's else branch).
        run(&mut world, &mut res);
        assert_eq!(res.view.viewstatus, 0, "WaitTicks should be active first (FIFO)");

        // Tick 2: Placard still alive (hold_ticks=2, ticked to 1); viewstatus=2
        run(&mut world, &mut res);
        assert_eq!(res.view.viewstatus, 2, "Placard should be second");

        // Tick 3: Placard expires; WaitTicks(1) becomes active (not yet expired)
        run(&mut world, &mut res);

        // Tick 4: WaitTicks(1) expires; queue idle; viewstatus cleared
        run(&mut world, &mut res);
        assert_eq!(res.view.viewstatus, 0);
        assert!(res.narrative.is_idle());
    }

    #[test]
    fn teleport_hero_updates_position() {
        let mut world = World::new();
        let hero = world.spawn((Hero, Position::new(100.0, 100.0)));
        let mut res = Resources::new(hero);

        res.narrative.push(NarrEvent::TeleportHero {
            x: 500.0,
            y: 300.0,
            region: 2,
        });

        run(&mut world, &mut res);

        let pos = world.get::<&Position>(hero).unwrap();
        assert_eq!(pos.x, 500.0);
        assert_eq!(pos.y, 300.0);
        assert_eq!(res.region.new_region, 2);
    }
}
