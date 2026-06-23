//! InputSystem — translates per-tick fire input into hero CombatState changes.

use hecs::World;
use crate::game::ecs::resources::Resources;
use crate::game::ecs::components::CombatState;
use crate::game::actor::ActorState;
use crate::game::combat::{rand4, TRANS_LIST};

pub fn run(world: &mut World, res: &mut Resources) {
    let hero = res.hero_entity;
    let tick = res.clock.tick_counter;

    if let Ok(mut cs) = world.get::<&mut CombatState>(hero) {
        if res.input_fire {
            // Fire held: advance fight substate each tick via trans_list (fmain.c:1712).
            // Entering from a non-fight state starts at substate 0.
            let s = match cs.state {
                ActorState::Fighting(s) => s as usize,
                _ => 0,
            };
            let next = TRANS_LIST[s][rand4(tick)];
            cs.state = ActorState::Fighting(next);
        } else if matches!(cs.state, ActorState::Fighting(_)) {
            // Fire released: exit to Still immediately.
            cs.state = ActorState::Still;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hecs::World;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::components::{Position, Facing, CombatState, HeroStats, Inventory};
    use crate::game::actor::ActorState;

    fn setup() -> (World, Resources) {
        let mut world = World::new();
        let hero = world.spawn((
            Position::new(0.0, 0.0),
            Facing::default(),
            CombatState::default(),
            HeroStats { vitality: 100, brave: 35, luck: 0, kind: 0, wealth: 0, hunger: 0, fatigue: 0, gold: 0 },
            Inventory::empty(),
        ));
        let res = Resources::new(hero);
        (world, res)
    }

    #[test]
    fn fire_held_enters_fighting_state() {
        let (mut world, mut res) = setup();
        res.input_fire = true;
        run(&mut world, &mut res);
        let cs = world.get::<&CombatState>(res.hero_entity).unwrap();
        assert!(matches!(cs.state, ActorState::Fighting(_)));
    }

    #[test]
    fn fire_held_stays_in_fighting() {
        let (mut world, mut res) = setup();
        res.input_fire = true;
        for tick in 0..20u32 {
            res.clock.tick_counter = tick;
            run(&mut world, &mut res);
            let cs = world.get::<&CombatState>(res.hero_entity).unwrap();
            assert!(matches!(cs.state, ActorState::Fighting(_)),
                "tick {tick}: expected Fighting, got {:?}", cs.state);
        }
    }

    #[test]
    fn fire_released_exits_fighting_to_still() {
        let (mut world, mut res) = setup();
        res.input_fire = true;
        run(&mut world, &mut res);
        res.input_fire = false;
        run(&mut world, &mut res);
        let cs = world.get::<&CombatState>(res.hero_entity).unwrap();
        assert_eq!(cs.state, ActorState::Still, "fire released should exit Fighting to Still");
    }
}
