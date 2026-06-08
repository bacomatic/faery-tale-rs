//! ClockSystem — advances the day/night cycle, decrements spell timers.
//! Port of GameState::tick() timer section and daynight_tick().
//! See docs/spec/daynight-cycle.md for timing constants.

use hecs::World;
use crate::game::ecs::resources::Resources;
use crate::game::ecs::events::ClockEvent;

/// NTSC daynight cycle length (fmain.c: DAYLEN = 24000).
const DAYLEN: u16 = 24000;
/// Number of time periods per day.
const PERIODS_PER_DAY: u8 = 12;

pub fn run(world: &mut World, res: &mut Resources) {
    let clock = &mut res.clock;

    // Advance day/night counter.
    let prev_daynight = clock.daynight;
    clock.daynight = clock.daynight.wrapping_add(1);
    if clock.daynight >= DAYLEN {
        clock.daynight = 0;
        clock.game_days = clock.game_days.wrapping_add(1);
    }

    // Compute lightlevel: triangle wave 0→300→0 over DAYLEN ticks.
    let half = DAYLEN / 2;
    clock.lightlevel = if clock.daynight < half {
        (clock.daynight as u32 * 300 / half as u32) as u16
    } else {
        ((DAYLEN - clock.daynight) as u32 * 300 / half as u32) as u16
    };

    // Day period: 12 buckets, each DAYLEN/12 ticks wide.
    let new_period = (clock.daynight as u32 * PERIODS_PER_DAY as u32 / DAYLEN as u32) as u8;
    let prev_period = (prev_daynight as u32 * PERIODS_PER_DAY as u32 / DAYLEN as u32) as u8;
    if new_period != prev_period {
        res.events.clock.push(ClockEvent::NewPeriod { period: new_period });
        res.region.dayperiod = new_period;
    }

    // Tick cycle and flasher counters.
    let clock = &mut res.clock;
    clock.cycle = clock.cycle.wrapping_add(1);
    clock.flasher = clock.flasher.wrapping_add(1);
    clock.tick_counter = clock.tick_counter.wrapping_add(1);

    // Decrement spell timers (sticky mode holds at 1).
    if clock.light_timer > 0 {
        if clock.light_sticky { clock.light_timer = 1; }
        else { clock.light_timer -= 1; }
    }
    if clock.secret_timer > 0 {
        if clock.secret_sticky { clock.secret_timer = 1; }
        else { clock.secret_timer -= 1; }
    }
    if clock.freeze_timer > 0 {
        if clock.freeze_sticky { clock.freeze_timer = 1; }
        else { clock.freeze_timer -= 1; }
    }
}

#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::resources::Resources;
    use super::run;

    fn make_resources(world: &mut World) -> Resources {
        let hero = world.spawn((crate::game::ecs::components::Hero,));
        Resources::new(hero)
    }

    #[test]
    fn daynight_increments() {
        let mut world = World::new();
        let mut res = make_resources(&mut world);
        res.clock.daynight = 100;
        run(&mut world, &mut res);
        assert!(res.clock.daynight > 100 || res.clock.game_days > 0);
    }

    #[test]
    fn freeze_timer_decrements() {
        let mut world = World::new();
        let mut res = make_resources(&mut world);
        res.clock.freeze_timer = 5;
        run(&mut world, &mut res);
        assert_eq!(res.clock.freeze_timer, 4);
    }

    #[test]
    fn freeze_sticky_holds_timer() {
        let mut world = World::new();
        let mut res = make_resources(&mut world);
        res.clock.freeze_timer = 1;
        res.clock.freeze_sticky = true;
        run(&mut world, &mut res);
        assert!(res.clock.freeze_timer > 0);
    }
}