use crate::game::actor::Actor;

pub struct GameState {
    // Hero position
    pub hero_x: u16,
    pub hero_y: u16,
    pub map_x: u16,
    pub map_y: u16,
    pub hero_sector: u16,
    pub hero_place: u16,

    // Hero stats
    pub vitality: i16,
    pub brave: i16,
    pub luck: i16,
    pub kind: i16,
    pub wealth: i16,
    pub hunger: i16,
    pub fatigue: i16,
    /// 1 = Julian, 2 = Phillip, 3 = Kevin
    pub brother: u8,
    pub riding: i16,
    pub flying: i16,

    // Timers
    pub light_timer: i16,
    pub secret_timer: i16,
    pub freeze_timer: i16,

    // Cycle counters
    /// 0–24000 wrapping
    pub daynight: u16,
    /// Derived triangle wave 0–300–0
    pub lightlevel: u16,
    pub cycle: u32,
    pub flasher: u32,

    // Flags
    pub battleflag: bool,
    pub quitflag: bool,
    pub witchflag: bool,
    pub safe_flag: bool,
    pub actors_on_screen: bool,
    pub actors_loading: bool,

    // View state
    /// 0=normal, 1=map, 2=message, 3=fade-in, 4=inventory, 98/99=redraw
    pub viewstatus: u8,
    pub cmode: u8,

    // Safe respawn
    pub safe_x: u16,
    pub safe_y: u16,
    pub safe_r: u8,

    // Region
    pub region_num: u8,
    pub new_region: u8,

    // Per-brother inventory
    pub julstuff: [u8; 35],
    pub philstuff: [u8; 35],
    pub kevstuff: [u8; 35],
    /// 0 = Julian, 1 = Phillip, 2 = Kevin
    pub active_brother: usize,

    // Actor list
    pub actors: Vec<Actor>,
    /// Active combat actor count
    pub anix: usize,
    /// Total including objects
    pub anix2: usize,

    // Encounter state
    pub xtype: u16,
    pub encounter_type: u16,
    pub encounter_number: u8,

    // Carrier/special
    pub active_carrier: i16,
    pub actor_file: i16,
    pub set_file: i16,

    // Princess/quest
    pub princess: u8,
    pub dayperiod: u8,

    // Music
    pub current_mood: u8,

    // God mode / debug sticky timers
    pub light_sticky: bool,
    pub secret_sticky: bool,
    pub freeze_sticky: bool,
}

impl GameState {
    /// Initialize to Julian's starting state (mirrors `revive(TRUE)` in original).
    pub fn new() -> Self {
        let mut actors = Vec::with_capacity(20);
        for _ in 0..20 {
            actors.push(Actor::default());
        }
        GameState {
            hero_x: 19036,
            hero_y: 15755,
            map_x: 0,
            map_y: 0,
            hero_sector: 0,
            hero_place: 0,

            vitality: 10,
            brave: 30,
            luck: 20,
            kind: 15,
            wealth: 5,
            hunger: 0,
            fatigue: 0,
            brother: 1,
            riding: 0,
            flying: 0,

            light_timer: 0,
            secret_timer: 0,
            freeze_timer: 0,

            daynight: 0,
            lightlevel: 0,
            cycle: 0,
            flasher: 0,

            battleflag: false,
            quitflag: false,
            witchflag: false,
            safe_flag: false,
            actors_on_screen: false,
            actors_loading: false,

            viewstatus: 0,
            cmode: 0,

            safe_x: 0,
            safe_y: 0,
            safe_r: 0,

            region_num: 0,
            new_region: 0,

            julstuff: [0u8; 35],
            philstuff: [0u8; 35],
            kevstuff: [0u8; 35],
            active_brother: 0,

            actors,
            anix: 0,
            anix2: 0,

            xtype: 0,
            encounter_type: 0,
            encounter_number: 0,

            active_carrier: 0,
            actor_file: 0,
            set_file: 0,

            princess: 0,
            dayperiod: 0,

            current_mood: 0,

            light_sticky: false,
            secret_sticky: false,
            freeze_sticky: false,
        }
    }

    /// Returns a reference to the active brother's inventory array.
    pub fn stuff(&self) -> &[u8; 35] {
        match self.active_brother {
            0 => &self.julstuff,
            1 => &self.philstuff,
            _ => &self.kevstuff,
        }
    }

    /// Returns a mutable reference to the active brother's inventory array.
    pub fn stuff_mut(&mut self) -> &mut [u8; 35] {
        match self.active_brother {
            0 => &mut self.julstuff,
            1 => &mut self.philstuff,
            _ => &mut self.kevstuff,
        }
    }

    /// Advance the day/night cycle by one tick.
    ///
    /// - Skipped when `freeze_timer > 0`.
    /// - `daynight` wraps at 24000.
    /// - `lightlevel` is a triangle wave: 0→300 over first 12000 ticks, 300→0 over last 12000.
    /// - Day-period boundaries are at 0, 6000, 12000, 18000.
    /// - Returns `true` if a boundary was crossed this tick.
    pub fn daynight_tick(&mut self) -> bool {
        if self.freeze_timer > 0 {
            return false;
        }

        let prev = self.daynight;
        self.daynight = self.daynight.wrapping_add(1);
        if self.daynight >= 24000 {
            self.daynight = 0;
        }

        // Recompute lightlevel as a triangle wave.
        self.lightlevel = if self.daynight < 12000 {
            (self.daynight as u32 * 300 / 12000) as u16
        } else {
            ((24000u32 - self.daynight as u32) * 300 / 12000) as u16
        };

        // Detect period boundary crossing (boundaries at 0, 6000, 12000, 18000).
        const BOUNDARIES: [u16; 4] = [0, 6000, 12000, 18000];
        let crossed = BOUNDARIES
            .iter()
            .any(|&b| (prev < b && self.daynight >= b) || (prev > self.daynight && b == 0));
        if crossed {
            self.dayperiod = (self.daynight / 6000) as u8;
        }
        crossed
    }
}
