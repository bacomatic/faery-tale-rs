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

    mono_base: Instant,   // time when the game clock was started
    pub mono_ticks: u64,  // total number of ticks since start, monotonic, not affected by pauses
    last_mono_ticks: u64, // mono_ticks at the previous update() call, for computing delta

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
const NANOS_PER_TICK: u128 = 33_333_334; // nanoseconds per tick (30 Hz — NTSC interlaced frame rate)

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

/*
   Original game clock update logic (from fmain.c):

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

   The game loop runs at 30 Hz (NTSC interlaced frame rate). A full day is 24000 ticks,
   or 800 seconds (13 minutes 20 seconds) of real time. Each hour is 1000 ticks (33.3 seconds).
*/

impl GameClock {
    pub fn new() -> GameClock {
        GameClock {
            ticker: GameTicker::new(),
            mono_base: Instant::now(),
            mono_ticks: 0,
            last_mono_ticks: 0,
            game_ticks: 0,
            paused: false,
        }
    }

    /**
     * Update the game clock, calculating elapsed ticks since last update.
     * Call this periodically to keep the clock accurate, generally once per frame.
     * Returns the number of monotonic ticks elapsed since the last call to update().
     */
    pub fn update(&mut self) -> u32 {
        // always update mono ticks, since Instant is monotonic, this is easy
        let mono_duration = Instant::now().duration_since(self.mono_base).as_nanos();
        self.mono_ticks = (mono_duration / NANOS_PER_TICK) as u64;

        let delta = (self.mono_ticks - self.last_mono_ticks) as u32;
        self.last_mono_ticks = self.mono_ticks;

        if self.paused {
            return delta;
        }
        self.ticker.update();

        let elapsed_ticks = self.ticker.get_elapsed_ticks();
        if elapsed_ticks > 0 {
            self.game_ticks += elapsed_ticks;
        }

        delta
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
        println!(
            "Game clock paused at {} total ticks, {} game ticks",
            self.mono_ticks, self.game_ticks
        );
    }

    /**
     * Resume the game clock.
     */
    pub fn resume(&mut self) {
        self.ticker.reset();
        self.paused = false;
        println!(
            "Game clock resumed at {} total ticks, {} game ticks",
            self.mono_ticks, self.game_ticks
        );
    }
}
