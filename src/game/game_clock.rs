
use std::time::Instant;

/**
 * This struct manages the game clock, including launch time and game time.
 *
 * The game wall clock is based on a 24,000 tick day cycle, with specific phases.
 */
#[derive(Debug)]
pub struct GameClock {
    // game clock
    ticker: GameTicker,

    pub total_ticks: u64, // total number of ticks since start
    pub game_ticks: u64, // number of game ticks passed total, resets on death/start
    pub paused: bool,
}

/*
 * Monotonic ticker to track elapsed time in ticks.
 */
#[derive(Debug, PartialEq, Eq)]
struct GameTicker {
    last_update: Instant,
    accumulated_nanos: u128,
}
const NANOS_PER_TICK: u128 = 16_666_667; // nanoseconds per tick (60 ticks per second)

impl GameTicker {
    pub fn new() -> GameTicker {
        GameTicker {
            last_update: Instant::now(),
            accumulated_nanos: 0,
        }
    }

    pub fn update(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_update).as_nanos();
        self.accumulated_nanos += elapsed;
        self.last_update = now;
    }

    pub fn reset(&mut self) {
        self.last_update = Instant::now();
        self.accumulated_nanos = 0;
    }

    pub fn get_elapsed_ticks(&mut self) -> u64 {
        self.update();
        let ticks = (self.accumulated_nanos / NANOS_PER_TICK) as u64;
        self.accumulated_nanos %= NANOS_PER_TICK;
        ticks
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum DayPhase {
    Midnight = 0,   // It was midnight. (00:00 - 5:59)
    Morning = 4,    // It was morning.  (06:00 - 11:59)
    Midday = 6,     // It was midday. (12:00 - 17:59)
    Evening = 9,    // Evening was drawing near. (18:00 - 23:59)
}

/*
    * Original game clock update logic (from fmain.c):

    if (!freeze_timer) /* no time in timestop */
        if ((daynight++) >= 24000)
            daynight = 0;

    lightlevel = daynight / 40;
    if (lightlevel >= 300)
        lightlevel = 600 - lightlevel;
    if (lightlevel < 40)
        ob_listg[5].ob_stat = 3;
    else
        ob_listg[5].ob_stat = 2;

    Day period, two hour segments:
    0 = Midnight (00:00 - 05:59)
    4 = Morning (06:00 - 11:59)
    6 = Midday (12:00 - 17:59)
    9 = Evening (18:00 - 23:59)

    i = (daynight / 2000);
    if (i != dayperiod) {
        switch (dayperiod = i) {
        case 0:
            event(28);
            break;
        case 4:
            event(29);
            break;
        case 6:
            event(30);
            break;
        case 9:
            event(31);
            break;
        }
    }

    Assumption is that this happens every tick (1/60 second), so a full day is 24000 ticks,
    or 400 seconds (6 minutes 40 seconds) of real time. Each hour is 1000 ticks (16.67 seconds).
 */
const TICKS_PER_DAY: u64 = 24000;
const TICKS_PER_HOUR: u64 = 1000;
const TICKS_PER_MINUTE: f64 = TICKS_PER_HOUR as f64 / 60.0; // unfortunately not an integer

impl GameClock {
    pub fn new() -> GameClock {
        GameClock {
            ticker: GameTicker::new(),
            total_ticks: 0,
            game_ticks: 0,
            paused: false,
        }
    }

    /**
     * Update the game clock, calculating elapsed ticks since last update.
     * Call this periodically to keep the clock accurate, generally once per frame.
     */
    pub fn update(&mut self) {
        if self.paused {
            return;
        }
        self.ticker.update();

        let elapsed_ticks = self.ticker.get_elapsed_ticks();
        if elapsed_ticks > 0 {
            self.total_ticks += elapsed_ticks;
            self.game_ticks += elapsed_ticks;
        }
    }

    /**
     * Reset the game ticks to zero (e.g., on player death or new game).
     */
    pub fn reset_game_ticks(&mut self) {
        self.game_ticks = 0;
        self.ticker.reset();
    }

    /**
     * Pause the game clock.
     */
    pub fn pause(&mut self) {
        // make sure we're up to date before pausing
        self.update();
        self.paused = true;
        println!("Game clock paused at {} total ticks, {} game ticks", self.total_ticks, self.game_ticks);
    }

    /**
     * Resume the game clock.
     */
    pub fn resume(&mut self) {
        self.ticker.reset();
        self.paused = false;
        println!("Game clock resumed at {} total ticks, {} game ticks", self.total_ticks, self.game_ticks);
    }

    /**
     * Get the total number of game days passed.
     */
    pub fn get_game_days(&self) -> u64 {
        self.game_ticks / TICKS_PER_DAY
    }

    /**
     * Get the current in-game wall clock time (day, hour, minute).
     */
    pub fn get_game_wall_clock(&self) -> (u32, u32, u32) {
        let day_ticks = self.game_ticks % TICKS_PER_DAY;

        let hour = day_ticks / TICKS_PER_HOUR;
        let minute = (day_ticks % TICKS_PER_HOUR) as f64 / TICKS_PER_MINUTE;
        let day = self.game_ticks / TICKS_PER_DAY;
        (day as u32, hour as u32, minute as u32)
    }

    /**
     * Set the in-game wall clock time (day, hour, minute).
     */
    pub fn set_game_wall_clock(&mut self, day: u32, hour: u32, minute: u32) {
        let total_minutes = (day as f64 * 24.0 * 60.0) + (hour as f64 * 60.0) + (minute as f64);
        let total_ticks = (total_minutes * TICKS_PER_MINUTE) as u64;
        self.game_ticks = total_ticks;
        self.ticker.reset();
    }

    /**
     * Advance the game wall to the specified hour and minute, possibly advancing to the next day.
     */
    pub fn advance_game_wall_clock_to(&mut self, hour: u32, minute: u32) {
        let (mut current_day, current_hour, current_minute) = self.get_game_wall_clock();
        if current_hour > hour || (current_hour == hour && current_minute >= minute) {
            // already at or past target time today, so advance to next day
            current_day += 1;
        }

        self.set_game_wall_clock(current_day, hour, minute);
        self.ticker.reset();
    }

    /**
     * Advance the game wall clock by the specified number of (hours, minutes).
     */
    pub fn advance_game_wall_clock_by(&mut self, hours: u32, minutes: u32) {
        let delta_minutes = (hours as f64 * 60.0) + (minutes as f64);
        let delta_ticks = (delta_minutes * TICKS_PER_MINUTE) as u64;
        self.game_ticks += delta_ticks;
        self.ticker.reset();
    }

    /**
     * Get the current day phase based on the game clock.
     */
    pub fn get_day_phase(&self) -> DayPhase {
        let day_ticks = self.game_ticks % TICKS_PER_DAY;
        let hour = day_ticks / TICKS_PER_HOUR;
        match hour {
            0..=7 => DayPhase::Midnight,
            8..=11 => DayPhase::Morning,
            12..=17 => DayPhase::Midday,
            _ => DayPhase::Evening,
        }
    }
}
