use crate::game::actor::Actor;
use crate::game::debug_command::GodModeFlags;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum DayPhase {
    #[default]
    Midnight = 0,
    Morning = 4,
    Midday = 6,
    Evening = 9,
}

/// Lasso item index in stuff array (from original fmain.h).
pub const ITEM_LASSO: usize = 5;
/// Swan carrier type ID (from original cfile/carrier tables).
pub const CARRIER_SWAN: i16 = 1;
/// Raft carrier type ID.
pub const CARRIER_RAFT: i16 = 5;
/// Turtle carrier type ID.
pub const CARRIER_TURTLE: i16 = 6;
/// Max hunger before starvation effects begin (original: 300).
pub const MAX_HUNGER: i16 = 300;
/// Item index for food in stuff[].
pub const ITEM_FOOD: usize = 0;
/// Bow item index in stuff[] (inv_list[3] from fmain.c).
pub const ITEM_BOW: usize = 3;
/// Arrow item index in stuff[] (inv_list[8] from fmain.c; stored separately from ARROWBASE).
pub const ITEM_ARROWS: usize = 8;
/// Sea Shell item index in stuff[] (original: stuff[6]; used to summon turtle).
pub const ITEM_SHELL: usize = 6;
/// Turtle nest coordinates (world-space, placeholder values).
pub const TURTLE_NEST_X: u16 = 0x2000;
pub const TURTLE_NEST_Y: u16 = 0x4000;

/// An item lying on the ground in the world.
#[derive(Debug, Clone)]
pub struct WorldObject {
    /// ob_id: original obytes enum value (sprite frame index for rendering).
    pub ob_id: u8,
    /// ob_stat: 1 = ground item, 3 = setfig NPC, 5 = hidden item.
    pub ob_stat: u8,
    pub region: u8,
    pub x: u16,
    pub y: u16,
    pub visible: bool,
    /// goal: index of this object within its ob_listX (set at region load; 0 for dynamic items).
    /// Used by setfig NPCs (wizard, ranger, beggar) to select variant dialogue (SPEC §13.1).
    pub goal: u8,
}

/// Convert daynight counter to dayperiod value per SPEC §17.3.
/// Returns discrete values {0, 4, 6, 9} for midnight/morning/midday/evening.
pub fn dayperiod_from_daynight(daynight: u16) -> u8 {
    (daynight / 2000) as u8
}
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

    // Swan velocity (signed i16, clamped to ±32 horizontal, ±40 vertical)
    pub swan_vx: i16,
    pub swan_vy: i16,

    // Timers
    pub light_timer: i16,
    pub secret_timer: i16,
    pub freeze_timer: i16,

    // Cycle counters
    /// 0–24000 wrapping
    pub daynight: u16,
    /// Number of full day cycles completed
    pub game_days: u32,
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

    // Per-brother inventory (ARROWBASE = 35; array length 36 so index 35 is valid)
    pub julstuff: [u8; 36],
    pub philstuff: [u8; 36],
    pub kevstuff: [u8; 36],
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
    pub on_raft: bool,
    /// Proximity to raft actor: 0=none, 1=near (within 16px), 2=aboard (within 9px).
    /// Mirrors raftprox from fmain.c (player-107).
    pub raftprox: i16,
    /// Actor slot index (1..3) of the active carrier; 0 = no carrier.
    /// Used to gate raft/turtle boarding checks (SPEC §21.2, §21.3).
    pub wcarry: u8,
    pub actor_file: i16,
    pub set_file: i16,

    // Princess/quest
    pub princess: u8,
    pub dayperiod: u8,

    // Hero facing direction (0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW)
    pub facing: u8,
    // Gold carried by the active brother
    pub gold: i32,

    // Music
    pub current_mood: u8,

    // God mode / debug sticky timers
    pub god_mode: GodModeFlags,
    pub light_sticky: bool,
    pub secret_sticky: bool,
    pub freeze_sticky: bool,

    // Cheat flag (persisted in save file per SPEC §25.9)
    pub cheat1: bool,

    // Tick counter (cumulative ticks since start)
    pub tick_counter: u32,

    // Brother liveness (Julian=0, Phillip=1, Kevin=2)
    pub brother_alive: [bool; 3],

    // Items dropped on the ground
    pub world_objects: Vec<WorldObject>,
}

impl GameState {
    /// Max fatigue (used only as a guard; original forced-sleep threshold is 171).
    pub const MAX_FATIGUE: i16 = 200;

    /// Initialize with Julian's default starting values (blist[0] from fmain.c).
    /// The game should call `init_first_brother()` after construction to apply
    /// config-driven stats from faery.toml.  Tests use the defaults directly.
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

            vitality: 23, // 15 + brave(35)/4
            brave: 35,
            luck: 20,
            kind: 15,
            wealth: 20,
            hunger: 0,
            fatigue: 0,
            brother: 1,
            riding: 0,
            flying: 0,
            swan_vx: 0,
            swan_vy: 0,

            light_timer: 0,
            secret_timer: 0,
            freeze_timer: 0,

            daynight: 8000,  // start at full brightness (noon); original pre-initializes here
            game_days: 0,
            lightlevel: 300, // full brightness at startup (original: explicit init)
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

            safe_x: 19036,
            safe_y: 15755,
            safe_r: 3,

            region_num: 3,
            new_region: 0,

            julstuff: [0u8; 36],
            philstuff: [0u8; 36],
            kevstuff: [0u8; 36],
            active_brother: 0,

            actors,
            anix: 0,
            anix2: 0,

            xtype: 0,
            encounter_type: 0,
            encounter_number: 0,

            active_carrier: 0,
            on_raft: false,
            raftprox: 0,
            wcarry: 0,
            actor_file: 0,
            set_file: 0,

            princess: 0,
            dayperiod: 4,    // morning period (0=midnight, 4=morning, 6=midday, 9=evening)

            current_mood: 0,

            facing: 0,
            gold: 0,

            god_mode: GodModeFlags::empty(),
            light_sticky: false,
            secret_sticky: false,
            freeze_sticky: false,
            cheat1: false,

            tick_counter: 0,
            brother_alive: [true, true, true],
            world_objects: Vec::new(),
        }
    }

    /// Returns a reference to the active brother's inventory array.
    pub fn stuff(&self) -> &[u8; 36] {
        match self.active_brother {
            0 => &self.julstuff,
            1 => &self.philstuff,
            _ => &self.kevstuff,
        }
    }

    /// Returns a mutable reference to the active brother's inventory array.
    pub fn stuff_mut(&mut self) -> &mut [u8; 36] {
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
            self.game_days += 1;
        }

        // Recompute lightlevel as a brightness triangle wave (fmain.c:2372-2374).
        // 0 = midnight (darkest), 300 = noon (brightest).
        // lightlevel = daynight / 40; if >= 300: lightlevel = 600 - lightlevel.
        let raw = self.daynight / 40;
        self.lightlevel = if raw >= 300 { 600 - raw } else { raw };

        // Detect period boundary crossing (boundaries at 0, 6000, 12000, 18000).
        const BOUNDARIES: [u16; 4] = [0, 8000, 12000, 18000];
        let crossed = BOUNDARIES
            .iter()
            .any(|&b| (prev < b && self.daynight >= b) || (prev > self.daynight && b == 0));
        if crossed {
            self.dayperiod = dayperiod_from_daynight(self.daynight);
        }
        crossed
    }

    /// Derive (day, hour, minute) from the authoritative `daynight` counter.
    pub fn daynight_to_wall_clock(&self) -> (u32, u32, u32) {
        let hour = (self.daynight / 1000) as u32;
        let remainder = (self.daynight % 1000) as u32;
        let minute = remainder * 60 / 1000;
        (self.game_days, hour, minute)
    }

    /// Get the current day phase from dayperiod.
    pub fn get_day_phase(&self) -> DayPhase {
        match self.dayperiod {
            0 => DayPhase::Midnight,
            4 => DayPhase::Morning,
            6 => DayPhase::Midday,
            9 => DayPhase::Evening,
            _ => DayPhase::Midnight,
        }
    }

    /// Advance game state by `delta` ticks.
    ///
    /// Returns a list of event IDs (matching `events::EVENT_MESSAGES` indices) that were
    /// triggered this update — hunger/fatigue warnings, time-of-day announcements, etc.
    /// The caller (gameplay_scene) is responsible for displaying the corresponding messages.
    pub fn tick(&mut self, delta: u32) -> Vec<u8> {
        let mut events: Vec<u8> = Vec::new();

        // Decrement magic timers (clamped, unless sticky).
        if !self.light_sticky {
            self.light_timer = self.light_timer.saturating_sub(delta as i16).max(0);
        }
        if !self.secret_sticky {
            self.secret_timer = self.secret_timer.saturating_sub(delta as i16).max(0);
        }
        if !self.freeze_sticky {
            self.freeze_timer = self.freeze_timer.saturating_sub(delta as i16).max(0);
        }

        // Advance day/night cycle once per tick, checking hunger/fatigue on each step.
        let cap = crate::game::magic::heal_cap(self.brave);
        for _ in 0..delta {
            let period_crossed = self.daynight_tick();

            // Hunger + fatigue: fire every 128 daynight ticks, matching original fmain.c:2623.
            // `(daynight & 127) == 0` triggers once per 128-tick window.
            if (self.daynight & 127) == 0 && self.vitality > 0 {
                self.hunger_fatigue_step(&mut events);
            }

            // Natural healing (SPEC §18.6): +1 HP every 1024 daynight ticks.
            // Tying the check to daynight means sleep's 64× time acceleration
            // automatically produces ≈63× faster healing (63 extra steps per frame).
            if !self.battleflag && self.vitality > 0 && self.vitality < cap
                && (self.daynight & 0x3FF) == 0
            {
                self.vitality = (self.vitality + 1).min(cap);
            }

            // Time-of-day announcements when dayperiod boundary is crossed (events 28-31).
            if period_crossed {
                let ev = match self.dayperiod {
                    0 => 28u8, // midnight
                    4 => 29,   // morning
                    6 => 30,   // midday
                    9 => 31,   // evening
                    _ => u8::MAX,
                };
                if ev != u8::MAX {
                    events.push(ev);
                }
            }
        }

        self.tick_counter = self.tick_counter.wrapping_add(delta);

        events
    }

    /// Advance the day/night clock by 63 sub-steps for one sleep frame
    /// (SPEC §18.4: "daynight += 63" each frame while sleeping; the remaining
    /// +1 is contributed by the normal `tick()` call that precedes this).
    ///
    /// For each sub-step the natural-healing check fires when
    /// `(daynight & 0x3FF) == 0` (SPEC §18.6), so healing accumulates at
    /// ≈63× the awake rate over the same real-time interval.
    ///
    /// Also decrements fatigue (clamped at 0) and refreshes lightlevel.
    ///
    /// Returns `true` when the wake condition is met:
    ///   `fatigue == 0` OR (`fatigue < 30` AND `daynight ∈ [9000, 10000)`).
    pub fn sleep_advance_daynight(&mut self) -> bool {
        let cap = crate::game::magic::heal_cap(self.brave);
        for _ in 0..63 {
            self.daynight = self.daynight.wrapping_add(1);
            if self.daynight >= 24000 {
                self.daynight = 0;
                self.game_days += 1;
            }
            if !self.battleflag && self.vitality > 0 && self.vitality < cap
                && (self.daynight & 0x3FF) == 0
            {
                self.vitality = (self.vitality + 1).min(cap);
            }
        }

        // Refresh the brightness triangle wave after the daynight jump.
        let raw = self.daynight / 40;
        self.lightlevel = if raw >= 300 { 600u16.saturating_sub(raw) } else { raw };

        self.fatigue = self.fatigue.saturating_sub(1);

        let can_wake_time = self.daynight >= 9000 && self.daynight < 10000;
        self.fatigue == 0 || (self.fatigue < 30 && can_wake_time)
    }

    /// Per-128-daynight-tick hunger and fatigue increment, matching fmain.c:2623-2652.
    /// Pushes triggered event IDs into `events`.
    fn hunger_fatigue_step(&mut self, events: &mut Vec<u8>) {
        // Safe-zone auto-eat per SPEC §18.2 (must check before incrementing hunger)
        if self.try_safe_autoeat() {
            events.push(37); // event(37) per SPEC §18.2
        }

        self.hunger += 1;
        self.fatigue += 1;

        // Hunger threshold messages.
        if self.hunger == 35 {
            events.push(0); // "was getting rather hungry"
        } else if self.hunger == 60 {
            events.push(1); // "was getting very hungry"
        }

        // Fatigue threshold messages.
        if self.fatigue == 70 {
            events.push(3); // "was getting tired"
        } else if self.fatigue == 90 {
            events.push(4); // "was getting sleepy"
        }

        // Every 8 hunger increments: check for starvation damage and forced sleep.
        if (self.hunger & 7) == 0 {
            if self.vitality > 5 {
                if self.hunger > 100 || self.fatigue > 160 {
                    self.vitality = (self.vitality - 2).max(0);
                }
                if self.hunger > 90 {
                    events.push(2); // "was starving!"
                }
            } else if self.fatigue > 170 {
                // Forced sleep from exhaustion — do NOT zero fatigue; it will decrement
                // naturally via sleep_advance_daynight() so the hero sleeps until rested.
                events.push(12); // "just couldn't stay awake any longer!" → forced sleep
            } else if self.hunger > 140 {
                // Hunger collapse — reset hunger to 130 per SPEC §18.2; leave fatigue
                // intact so sleep_advance_daynight() drives the actual sleep duration.
                events.push(24); // "passed out from hunger!" → forced sleep
                self.hunger = 130;
            }
        }
    }

    /// Returns the index of the next living brother after the current one, or None if all dead.
    pub fn next_brother(&self) -> Option<usize> {
        for offset in 1..=2 {
            let idx = (self.active_brother + offset) % 3;
            if self.brother_alive[idx] {
                return Some(idx);
            }
        }
        None
    }

    /// Switch to the given brother: mark current as dead, load stats from config,
    /// and teleport to spawn location.  Mirrors `revive(TRUE)` from fmain.c.
    ///
    /// If `brother` and `spawn` are None the method falls back to the legacy
    /// behaviour of just swapping the index (for tests / code that doesn't have
    /// the game library handy).
    pub fn activate_brother(&mut self, new_idx: usize) {
        self.brother_alive[self.active_brother] = false;
        self.active_brother = new_idx;
        // brother field: 1=Julian, 2=Phillip, 3=Kevin
        self.brother = (new_idx as u8) + 1;
    }

    /// Full brother activation with config-driven stats and spawn coordinates.
    /// Mirrors fmain.c `revive(TRUE)`: load per-brother attrs, clear inventory,
    /// give a dirk, set vitality = 15 + brave/4, teleport to spawn location,
    /// and reset timers.
    pub fn activate_brother_from_config(
        &mut self,
        new_idx: usize,
        brave: i16,
        luck: i16,
        kind: i16,
        wealth: i16,
        spawn_x: u16,
        spawn_y: u16,
        spawn_region: u8,
    ) {
        self.brother_alive[self.active_brother] = false;
        self.active_brother = new_idx;
        self.brother = (new_idx as u8) + 1;

        // Load per-brother stats (blist[] in original)
        self.brave = brave;
        self.luck = luck;
        self.kind = kind;
        self.wealth = wealth;

        // Vitality formula from original revive(): 15 + brave/4
        self.vitality = 15 + brave / 4;

        // Clear inventory and give a dirk (stuff[0] = 1)
        *self.stuff_mut() = [0u8; 36];
        self.stuff_mut()[0] = 1;

        // Equip dirk (fmain.c:3501: stuff[0] = an->weapon = 1).
        if let Some(player) = self.actors.first_mut() {
            player.weapon = 1;
        }

        // Teleport to spawn location
        self.hero_x = spawn_x;
        self.hero_y = spawn_y;
        self.region_num = spawn_region;
        self.safe_x = spawn_x;
        self.safe_y = spawn_y;
        self.safe_r = spawn_region;

        // Reset timers (mirrors revive clearing these)
        self.light_timer = 0;
        self.secret_timer = 0;
        self.freeze_timer = 0;
        self.hunger = 0;
        self.fatigue = 0;
    }

    /// Initialize this state from the first brother (Julian) using config data.
    /// Called once at game start.  Spawn coordinates come from the named location.
    pub fn init_first_brother(
        &mut self,
        brave: i16,
        luck: i16,
        kind: i16,
        wealth: i16,
        spawn_x: u16,
        spawn_y: u16,
        spawn_region: u8,
    ) {
        self.active_brother = 0;
        self.brother = 1;
        self.brave = brave;
        self.luck = luck;
        self.kind = kind;
        self.wealth = wealth;
        self.vitality = 15 + brave / 4;
        self.hero_x = spawn_x;
        self.hero_y = spawn_y;
        self.region_num = spawn_region;
        self.safe_x = spawn_x;
        self.safe_y = spawn_y;
        self.safe_r = spawn_region;
        // Give a dirk
        self.stuff_mut()[0] = 1;
        // Equip dirk (fmain.c:3501: stuff[0] = an->weapon = 1).
        if let Some(player) = self.actors.first_mut() {
            player.weapon = 1;
        }
    }

    /// Returns true if all three brothers are dead.
    pub fn all_dead(&self) -> bool {
        self.brother_alive.iter().all(|&alive| !alive)
    }

    /// Returns true if the active brother has the lasso in inventory.
    pub fn has_lasso(&self) -> bool {
        self.stuff()[ITEM_LASSO] != 0
    }

    /// Board a raft carrier. Returns true if successful.
    pub fn board_raft(&mut self) -> bool {
        if self.active_carrier == CARRIER_RAFT {
            self.on_raft = true;
            true
        } else {
            false
        }
    }

    /// Disembark from raft.
    pub fn leave_raft(&mut self) {
        self.on_raft = false;
        self.active_carrier = 0;
        self.wcarry = 0;
    }

    /// Summon turtle using a shell item. Returns true if successful.
    /// Turtle acts like raft for water traversal (on_raft=true) but cannot enter mountains.
    pub fn summon_turtle(&mut self) -> bool {
        if self.stuff()[ITEM_SHELL] > 0 {
            self.stuff_mut()[ITEM_SHELL] -= 1;
            self.active_carrier = CARRIER_TURTLE;
            self.on_raft = true;
            self.wcarry = 3;  // SPEC §21.3: turtle is in actor slot 3
            true
        } else {
            false
        }
    }

    /// Attempt to rescue a turtle egg from a dead snake NPC.
    /// Returns true if an egg was found.
    /// Stub: try to rescue a turtle egg based on luck.
    /// In the full game, turtle_eggs is a world-state flag not an inventory item.
    pub fn try_rescue_egg(&mut self) -> bool {
        // turtle_eggs is a world object counter in fmain.c, not an inventory slot.
        // Stub: luck-gated chance; full implementation pending NPC object system.
        self.luck > 50
    }

    /// Return eggs to the turtle nest for shell reward.
    /// Returns number of shells received.
    pub fn return_eggs_to_nest(&mut self, hero_x: u16, hero_y: u16, egg_count: u8) -> u8 {
        let at_nest = hero_x.abs_diff(TURTLE_NEST_X) < 32
            && hero_y.abs_diff(TURTLE_NEST_Y) < 32;
        if !at_nest || egg_count == 0 { return 0; }
        let award = egg_count.min(255 - self.stuff()[ITEM_SHELL]);
        self.stuff_mut()[ITEM_SHELL] += award;
        award
    }

    /// Attempt to start swan flight. Returns true if successful.
    /// Requires: has_lasso AND a swan carrier is nearby (active_carrier == CARRIER_SWAN).
    pub fn start_swan_flight(&mut self) -> bool {
        if self.has_lasso() && self.active_carrier == CARRIER_SWAN {
            self.flying = 1;
            self.swan_vx = 0;
            self.swan_vy = 0;
            true
        } else {
            false
        }
    }

    /// Stop swan flight (land). Resets velocity.
    pub fn stop_swan_flight(&mut self) {
        self.flying = 0;
        self.swan_vx = 0;
        self.swan_vy = 0;
    }

    /// Apply directional input to swan velocity (SPEC §21.4).
    /// xdir/ydir are the directional impulse values from collision::XDIR/YDIR.
    /// Velocity is clamped to ±32 horizontal, ±40 vertical.
    pub fn apply_swan_velocity_impulse(&mut self, xdir: i16, ydir: i16) {
        if self.flying == 0 { return; }
        
        self.swan_vx += xdir;
        self.swan_vy += ydir;
        
        // Clamp horizontal velocity to ±32
        self.swan_vx = self.swan_vx.clamp(-32, 32);
        // Clamp vertical velocity to ±40
        self.swan_vy = self.swan_vy.clamp(-40, 40);
    }

    /// Compute new hero position from swan velocity (SPEC §21.4).
    /// Position updates by vel/4 per frame.
    /// Returns (new_x, new_y, wraps_x, wraps_y) where wraps indicate coordinate wrapping.
    pub fn compute_swan_position(&self) -> (u16, u16) {
        if self.flying == 0 {
            return (self.hero_x, self.hero_y);
        }
        
        let dx = self.swan_vx / 4;
        let dy = self.swan_vy / 4;
        
        // Outdoor world wraps at 0x8000
        let new_x = ((self.hero_x as i32 + dx as i32).rem_euclid(0x8000)) as u16;
        let new_y = if self.region_num < 8 {
            ((self.hero_y as i32 + dy as i32).rem_euclid(0x8000)) as u16
        } else {
            (self.hero_y as i32 + dy as i32) as u16
        };
        
        (new_x, new_y)
    }

    /// Check if swan can dismount at current velocity (SPEC §21.4).
    /// Dismount allowed when |vel_x| < 15 && |vel_y| < 15.
    pub fn can_dismount_swan(&self) -> bool {
        if self.flying == 0 { return false; }
        self.swan_vx.abs() < 15 && self.swan_vy.abs() < 15
    }

    /// Check if raft can be boarded at the given terrain (SPEC §21.2).
    /// Returns true if wcarry == 1 AND terrain is water/shore (codes 3..=5).
    pub fn can_board_raft(&self, terrain: u8) -> bool {
        self.wcarry == 1 && (3..=5).contains(&terrain)
    }

    /// Check if turtle can be boarded (SPEC §21.3).
    /// Returns true if wcarry == 3 (turtle is active).
    pub fn can_board_turtle(&self) -> bool {
        self.wcarry == 3
    }

    /// Check if turtle summon is blocked in the central region (SPEC §21.3).
    /// Returns true if position is inside X ∈ [11194, 21373] AND Y ∈ [10205, 16208].
    pub fn is_turtle_summon_blocked(&self) -> bool {
        (11194..=21373).contains(&self.hero_x)
            && (10205..=16208).contains(&self.hero_y)
    }

    /// Eat food from inventory. Returns true if food was available.
    pub fn eat_food(&mut self) -> bool {
        if self.stuff()[ITEM_FOOD] > 0 {
            self.stuff_mut()[ITEM_FOOD] -= 1;
            self.hunger = (self.hunger - 100).max(0);
            true
        } else {
            false
        }
    }

    /// Reduce hunger by a given amount (used by fruit pickup).
    /// Mirrors fmain.c eat(amount): hunger -= amount, clamped to 0.
    pub fn eat_amount(&mut self, amount: i16) {
        self.hunger = (self.hunger - amount).max(0);
    }

    /// Pick up a fruit. Per SPEC §14.5: if hunger >= 15, eat immediately via eat(30);
    /// otherwise store in inventory (stuff[24]++).
    /// Returns true if fruit was auto-eaten, false if stored.
    pub fn pickup_fruit(&mut self) -> bool {
        const FRUIT_ITEM: usize = 24;
        const HUNGER_THRESHOLD: i16 = 15;
        const EAT_AMOUNT: i16 = 30;

        if self.hunger >= HUNGER_THRESHOLD {
            // Auto-eat immediately
            self.eat_amount(EAT_AMOUNT);
            true
        } else {
            // Store in inventory
            if self.stuff()[FRUIT_ITEM] < 255 {
                self.stuff_mut()[FRUIT_ITEM] += 1;
            }
            false
        }
    }

    /// Safe-zone auto-eat per SPEC §18.2.
    /// In a safe zone, when (daynight & 127) == 0, if hunger > 30 and stuff[24] > 0,
    /// auto-eat fruit (direct hunger subtraction, not via eat()).
    /// Returns true if fruit was consumed.
    pub fn try_safe_autoeat(&mut self) -> bool {
        const FRUIT_ITEM: usize = 24;
        const HUNGER_THRESHOLD: i16 = 30;
        const REDUCE_AMOUNT: i16 = 30;

        if self.safe_flag
            && (self.daynight & 127) == 0
            && self.hunger > HUNGER_THRESHOLD
            && self.stuff()[FRUIT_ITEM] > 0
        {
            self.stuff_mut()[FRUIT_ITEM] -= 1;
            self.hunger = (self.hunger - REDUCE_AMOUNT).max(0);
            true
        } else {
            false
        }
    }

    /// Apply hunger effects: if hunger >= MAX_HUNGER, drain vitality by 1.
    /// Call this from tick() after hunger is incremented.
    pub fn apply_hunger_effects(&mut self) {
        if self.hunger >= MAX_HUNGER {
            self.vitality = (self.vitality - 1).max(0);
        }
    }

    /// Pick up an item (port of itrans[] logic from fmain.c).
    /// item_id: item type (0-34). Returns true if picked up.
    pub fn pickup_item(&mut self, item_id: usize) -> bool {
        if item_id >= 36 { return false; }
        let stuff = self.stuff_mut();
        if stuff[item_id] < 255 {
            stuff[item_id] += 1;
            true
        } else {
            false
        }
    }

    /// Drop an item.
    pub fn drop_item(&mut self, item_id: usize) -> bool {
        if item_id >= 36 { return false; }
        let stuff = self.stuff_mut();
        if stuff[item_id] > 0 {
            stuff[item_id] -= 1;
            true
        } else {
            false
        }
    }

    /// Drop an item and place it on the ground at the given world location.
    pub fn drop_item_to_world(&mut self, item_id: usize, region: u8, x: u16, y: u16) -> bool {
        if self.drop_item(item_id) {
            self.world_objects.push(WorldObject {
                ob_id: item_id as u8,
                ob_stat: 1,
                region, x, y,
                visible: true,
                goal: 0,
            });
            true
        } else {
            false
        }
    }

    /// Find the nearest visible ground item within range using calc_dist.
    /// Returns (world_objects index, ob_id) of the nearest item, or None.
    /// Does NOT modify state — caller decides what to do with the item.
    pub fn find_nearest_item(&self, region: u8, hero_x: u16, hero_y: u16, max_range: i32) -> Option<(usize, u8)> {
        use crate::game::collision::calc_dist;

        let hx = hero_x as i32;
        let hy = hero_y as i32;
        let mut best_idx = None;
        let mut best_dist = max_range;

        for (i, obj) in self.world_objects.iter().enumerate() {
            if obj.ob_stat == 3 { continue; } // setfigs not pickable
            if !obj.visible { continue; }
            if obj.region != region { continue; }
            if obj.ob_id == 0x1d { continue; } // empty chest (skip per original)

            let d = calc_dist(hx, hy, obj.x as i32, obj.y as i32);
            if d < best_dist {
                best_dist = d;
                best_idx = Some((i, obj.ob_id));
            }
        }
        best_idx
    }

    /// Mark a world object as picked up (ob_stat → hidden).
    pub fn mark_object_taken(&mut self, world_idx: usize) {
        if let Some(obj) = self.world_objects.get_mut(world_idx) {
            obj.visible = false;
        }
    }

    /// Pick up the nearest visible world object within `range` pixels.
    /// Translates ob_id → stuff[] index via itrans. Special handling:
    /// - MONEY (13): adds 50 gold
    /// - FOOTSTOOL (31), TURTLE (102): not pickable
    /// - Containers (URN 14, CHEST 15, SACKS 16): not yet implemented (skip)
    pub fn pickup_world_object(&mut self, region: u8, hero_x: u16, hero_y: u16, range: u16) -> Option<u8> {
        use crate::game::world_objects::ob_id_to_stuff_index;

        let mut found_idx = None;
        for (i, obj) in self.world_objects.iter().enumerate() {
            if obj.ob_stat == 3 { continue; } // setfig NPCs are not pickable
            if obj.visible && obj.region == region
                && hero_x.abs_diff(obj.x) < range
                && hero_y.abs_diff(obj.y) < range
            {
                found_idx = Some(i);
                break;
            }
        }
        let idx = found_idx?;
        let ob_id = self.world_objects[idx].ob_id;

        // Non-pickable objects.
        match ob_id {
            31 | 102 => return None,     // FOOTSTOOL, TURTLE
            14 | 15 | 16 => return None, // URN, CHEST, SACKS (containers — TODO)
            _ => {}
        }

        // MONEY: +50 gold, no inventory slot.
        if ob_id == 13 {
            self.gold += 50;
            self.world_objects[idx].visible = false;
            return Some(ob_id);
        }

        // Translate ob_id → stuff[] index.
        if let Some(stuff_idx) = ob_id_to_stuff_index(ob_id) {
            if self.pickup_item(stuff_idx) {
                self.world_objects[idx].visible = false;
                return Some(ob_id);
            }
        }
        None
    }

    /// Load static world objects for the given region from the game library.
    /// Only ground items (ob_stat 1) and hidden items (ob_stat 5) are loaded.
    /// SetFig NPCs (ob_stat 3/4) are handled by the NPC system.
    pub fn populate_region_objects(&mut self, region: u8, game_lib: &crate::game::game_library::GameLibrary) {
        self.world_objects.clear();

        // Track per-list indices separately (mirrors original ob_listX vs ob_listg).
        let mut region_goal: u8 = 0;
        let mut global_goal: u8 = 0;

        for obj_cfg in &game_lib.objects {
            // Include objects for this region + global objects (region 255)
            if obj_cfg.region != region && obj_cfg.region != 255 {
                continue;
            }
            // goal = index in the respective ob_listX, counting all entries (SPEC §13.1).
            let is_global = obj_cfg.region == 255;
            let goal = if is_global {
                let g = global_goal;
                global_goal = global_goal.wrapping_add(1);
                g
            } else {
                let g = region_goal;
                region_goal = region_goal.wrapping_add(1);
                g
            };

            // Ground items (1), setfig NPCs (3), and hidden items (5)
            if obj_cfg.ob_stat == 1 || obj_cfg.ob_stat == 3 || obj_cfg.ob_stat == 5 {
                self.world_objects.push(WorldObject {
                    ob_id: obj_cfg.ob_id,
                    ob_stat: obj_cfg.ob_stat,
                    region,  // tag with current region so render filter passes
                    x: obj_cfg.x,
                    y: obj_cfg.y,
                    visible: obj_cfg.ob_stat != 5, // ob_stat 1 and 3 are visible
                    goal,
                });
            }
        }
    }

    /// Returns a string description of the inventory for display.
    pub fn inventory_summary(&self) -> String {
        let stuff = self.stuff();
        let mut items = Vec::new();
        for (i, &count) in stuff.iter().enumerate() {
            if count > 0 {
                items.push(format!("slot{}: {}", i, count));
            }
        }
        if items.is_empty() {
            "Empty pack".to_string()
        } else {
            items.join(", ")
        }
    }

    /// Award a sea shell when a snake guarding turtle eggs is defeated (player-108).
    /// Stubs the full turtle egg rescue quest from fmain.c (race==4 + turtle_eggs flag).
    /// Returns true if a shell was awarded.
    pub fn check_turtle_eggs(&mut self, is_snake: bool) -> bool {
        if is_snake && self.stuff()[ITEM_SHELL] < 255 {
            self.stuff_mut()[ITEM_SHELL] += 1;
            true
        } else {
            false
        }
    }

    /// Update safe spawn point if conditions match original fmain.c:
    /// outdoors (region < 8), not in battle, and passable non-water terrain.
    pub fn update_safe_spawn(&mut self, terrain_type: u8) {
        if self.region_num < 8 && !self.battleflag && terrain_type < 2 {
            self.safe_x = self.hero_x;
            self.safe_y = self.hero_y;
            self.safe_r = self.region_num;
        }
    }

    /// Attempt luck-gated respawn. Returns true if respawned.
    /// Requires luck >= 10; costs 10 luck per use.
    pub fn try_respawn(&mut self) -> bool {
        if self.luck >= 1 {
            self.luck = (self.luck - 5).max(0);
            self.hero_x = self.safe_x;
            self.hero_y = self.safe_y;
            self.region_num = self.safe_r;
            self.vitality = 10;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_lasso_false_by_default() {
        let state = GameState::new();
        assert!(!state.has_lasso());
    }

    #[test]
    fn test_pickup_and_drop() {
        let mut state = GameState::new();
        assert!(state.pickup_item(5));
        assert_eq!(state.stuff()[5], 1);
        assert!(state.drop_item(5));
        assert_eq!(state.stuff()[5], 0);
    }

    #[test]
    fn test_hunger_effects() {
        let mut state = GameState::new();
        state.hunger = MAX_HUNGER;
        let vit_before = state.vitality;
        state.apply_hunger_effects();
        assert_eq!(state.vitality, vit_before - 1);
    }

    #[test]
    fn test_update_safe_spawn() {
        let mut s = GameState::new();
        s.hero_x = 100; s.hero_y = 200; s.region_num = 3;
        s.update_safe_spawn(0);
        assert_eq!(s.safe_x, 100);
        s.hero_x = 999;
        s.update_safe_spawn(3); // water — should not update
        assert_eq!(s.safe_x, 100);
        // Indoor regions should not update safe spawn.
        s.hero_x = 500; s.region_num = 8;
        s.update_safe_spawn(0);
        assert_eq!(s.safe_x, 100, "indoor region must not update safe spawn");
        // Battle should not update safe spawn.
        s.region_num = 3; s.battleflag = true;
        s.update_safe_spawn(0);
        assert_eq!(s.safe_x, 100, "battleflag must prevent safe spawn update");
    }

    #[test]
    fn test_try_respawn_no_luck() {
        let mut s = GameState::new();
        s.luck = 0;
        assert!(!s.try_respawn());
    }

    #[test]
    fn test_hunger_fatigue_step_increments_both() {
        let mut s = GameState::new();
        s.fatigue = 10;
        s.hunger = 5;
        let mut events = Vec::new();
        s.hunger_fatigue_step(&mut events);
        assert_eq!(s.hunger, 6, "hunger should increment by 1");
        assert_eq!(s.fatigue, 11, "fatigue should increment by 1");
    }

    // T3-DEATH-SLEEP-COLLAPSE: SPEC §18.2/§18.3 — forced sleep must not zero fatigue

    #[test]
    fn test_event24_hunger_collapse_preserves_fatigue() {
        // SPEC §18.2: hunger > 140 with (hunger & 7) == 0 and vitality ≤ 5 fires event(24).
        // Fatigue must NOT be zeroed — it drives the actual sleep duration via
        // sleep_advance_daynight(), so zeroing it would cause immediate wake.
        let mut s = GameState::new();
        s.vitality = 3;   // ≤ 5: else branch in the 8-tick check
        s.fatigue = 80;   // < 170: event 12 won't take priority
        s.hunger = 143;   // +1 → 144: 144 > 140, (144 & 7) == 0
        let mut events = Vec::new();
        s.hunger_fatigue_step(&mut events);
        assert!(events.contains(&24), "should emit event 24 at hunger collapse");
        assert_eq!(s.hunger, 130, "hunger must be reset to 130 on collapse");
        assert_eq!(s.fatigue, 81, "fatigue must only increment (+1), not be zeroed");
    }

    #[test]
    fn test_event12_fatigue_collapse_preserves_fatigue() {
        // SPEC §18.3: fatigue > 170 with vitality ≤ 5 fires event(12) (exhaustion collapse).
        // Fatigue must NOT be zeroed — the hero should sleep until it naturally
        // decrements to 0 via sleep_advance_daynight().
        let mut s = GameState::new();
        s.vitality = 3;   // ≤ 5: else branch
        s.fatigue = 175;  // +1 → 176 > 170: event 12 takes priority over event 24
        s.hunger = 7;     // +1 → 8: (8 & 7) == 0, not > 140 so event 24 won't fire
        let mut events = Vec::new();
        s.hunger_fatigue_step(&mut events);
        assert!(events.contains(&12), "should emit event 12 at fatigue > 170");
        assert_eq!(s.fatigue, 176, "fatigue must only increment (+1), not be zeroed");
    }

    #[test]
    fn test_new_starts_at_full_brightness() {
        let s = GameState::new();
        assert_eq!(s.lightlevel, 300, "game must start at full brightness");
        assert_eq!(s.daynight, 8000, "daynight starts at 8000 per original");
    }

    #[test]
    fn test_daynight_to_wall_clock_midnight() {
        let mut s = GameState::new();
        s.daynight = 0;
        s.game_days = 0;
        let (day, hour, minute) = s.daynight_to_wall_clock();
        assert_eq!((day, hour, minute), (0, 0, 0));
    }

    #[test]
    fn test_daynight_to_wall_clock_start() {
        let s = GameState::new();
        let (day, hour, minute) = s.daynight_to_wall_clock();
        assert_eq!((day, hour, minute), (0, 8, 0), "game starts at 08:00");
    }

    #[test]
    fn test_daynight_to_wall_clock_noon() {
        let mut s = GameState::new();
        s.daynight = 12000;
        let (day, hour, minute) = s.daynight_to_wall_clock();
        assert_eq!((day, hour, minute), (0, 12, 0));
    }

    #[test]
    fn test_daynight_to_wall_clock_2330() {
        let mut s = GameState::new();
        s.daynight = 23500;
        let (day, hour, minute) = s.daynight_to_wall_clock();
        assert_eq!((day, hour, minute), (0, 23, 30));
    }

    #[test]
    fn test_game_days_increments_on_wrap() {
        let mut s = GameState::new();
        s.daynight = 23999;
        s.game_days = 0;
        s.daynight_tick();
        assert_eq!(s.daynight, 0);
        assert_eq!(s.game_days, 1);
    }

    #[test]
    fn test_pickup_translates_ob_id() {
        let mut s = GameState::new();
        s.region_num = 3;
        s.hero_x = 100;
        s.hero_y = 100;
        // Gold Key: ob_id 25 → stuff index 16
        s.world_objects.push(WorldObject {
            ob_id: 25, ob_stat: 1, region: 3, x: 100, y: 100, visible: true, goal: 0,
        });
        let result = s.pickup_world_object(3, 100, 100, 24);
        assert!(result.is_some());
        assert_eq!(s.stuff()[16], 1, "Gold Key should be in stuff[16]");
        assert!(!s.world_objects[0].visible);
    }

    #[test]
    fn test_pickup_money_adds_gold() {
        let mut s = GameState::new();
        s.region_num = 3;
        s.hero_x = 100;
        s.hero_y = 100;
        s.gold = 10;
        s.world_objects.push(WorldObject {
            ob_id: 13, ob_stat: 1, region: 3, x: 100, y: 100, visible: true, goal: 0,
        });
        let result = s.pickup_world_object(3, 100, 100, 24);
        assert!(result.is_some());
        assert_eq!(s.gold, 60, "MONEY should add 50 gold");
    }

    #[test]
    fn test_pickup_footstool_blocked() {
        let mut s = GameState::new();
        s.region_num = 8;
        s.hero_x = 100;
        s.hero_y = 100;
        s.world_objects.push(WorldObject {
            ob_id: 31, ob_stat: 1, region: 8, x: 100, y: 100, visible: true, goal: 0,
        });
        let result = s.pickup_world_object(8, 100, 100, 24);
        assert!(result.is_none());
        assert!(s.world_objects[0].visible);
    }

    #[test]
    fn test_populate_region_objects() {
        let lib = crate::game::game_library::load_game_library(std::path::Path::new("faery.toml"))
            .expect("should load faery.toml");
        let mut s = GameState::new();
        s.populate_region_objects(3, &lib);
        let ground_items: Vec<_> = s.world_objects.iter()
            .filter(|o| o.visible && o.ob_stat == 1)
            .collect();
        assert!(ground_items.len() >= 9, "region 3 should have at least 9 visible items, got {}", ground_items.len());
        let chest = ground_items.iter().find(|o| o.ob_id == 15 && o.x == 19298);
        assert!(chest.is_some(), "should have the starting chest");
        // SetFigs (ob_stat 3) should also be loaded and visible.
        let setfigs: Vec<_> = s.world_objects.iter()
            .filter(|o| o.ob_stat == 3)
            .collect();
        assert!(!setfigs.is_empty(), "region 3 should have at least one setfig");
        assert!(setfigs.iter().all(|o| o.visible), "setfigs should be visible");
    }

    #[test]
    fn test_init_first_brother_equips_dirk() {
        let mut state = GameState::new();
        state.actors.push(crate::game::actor::Actor::default());
        state.init_first_brother(10, 10, 10, 100, 1000, 2000, 3);
        assert_eq!(state.stuff()[0], 1, "dirk should be in inventory");
        assert_eq!(state.actors[0].weapon, 1, "dirk should be equipped");
    }

    #[test]
    #[test]
    fn test_can_board_raft_gating() {
        let mut s = GameState::new();
        // Not allowed when wcarry != 1.
        s.wcarry = 0;
        assert!(!s.can_board_raft(3), "cannot board raft when wcarry == 0");
        assert!(!s.can_board_raft(4), "cannot board raft when wcarry == 0");
        assert!(!s.can_board_raft(5), "cannot board raft when wcarry == 0");

        // Not allowed when terrain is not water/shore.
        s.wcarry = 1;
        assert!(!s.can_board_raft(0), "cannot board raft on terrain 0");
        assert!(!s.can_board_raft(1), "cannot board raft on terrain 1");
        assert!(!s.can_board_raft(2), "cannot board raft on terrain 2");
        assert!(!s.can_board_raft(6), "cannot board raft on terrain 6");

        // Allowed only when wcarry == 1 AND terrain in 3..=5.
        assert!(s.can_board_raft(3), "can board raft on terrain 3 (water)");
        assert!(s.can_board_raft(4), "can board raft on terrain 4 (shore)");
        assert!(s.can_board_raft(5), "can board raft on terrain 5 (water)");
    }

    #[test]
    fn test_can_board_turtle_gating() {
        let mut s = GameState::new();
        s.wcarry = 0;
        assert!(!s.can_board_turtle(), "cannot board turtle when wcarry == 0");
        s.wcarry = 1;
        assert!(!s.can_board_turtle(), "cannot board turtle when wcarry == 1 (raft slot)");
        s.wcarry = 3;
        assert!(s.can_board_turtle(), "can board turtle when wcarry == 3");
    }

    #[test]
    fn test_turtle_summon_region_blocking() {
        let mut s = GameState::new();
        // Outside the forbidden region.
        s.hero_x = 11193;
        s.hero_y = 10205;
        assert!(!s.is_turtle_summon_blocked(), "X below lower bound");

        s.hero_x = 21374;
        s.hero_y = 10205;
        assert!(!s.is_turtle_summon_blocked(), "X above upper bound");

        s.hero_x = 15000;
        s.hero_y = 10204;
        assert!(!s.is_turtle_summon_blocked(), "Y below lower bound");

        s.hero_y = 16209;
        assert!(!s.is_turtle_summon_blocked(), "Y above upper bound");

        // Inside the forbidden region.
        s.hero_x = 11194;
        s.hero_y = 10205;
        assert!(s.is_turtle_summon_blocked(), "at lower corner");

        s.hero_x = 21373;
        s.hero_y = 16208;
        assert!(s.is_turtle_summon_blocked(), "at upper corner");

        s.hero_x = 16000;
        s.hero_y = 13000;
        assert!(s.is_turtle_summon_blocked(), "in the middle");
    }

    #[test]
    fn test_swan_start_clears_velocity() {
        let mut state = GameState::new();
        state.swan_vx = 20;
        state.swan_vy = 30;
        state.active_carrier = CARRIER_SWAN;
        state.stuff_mut()[ITEM_LASSO] = 1; // has lasso at index 5
        assert!(state.start_swan_flight());
        assert_eq!(state.flying, 1);
        assert_eq!(state.swan_vx, 0);
        assert_eq!(state.swan_vy, 0);
    }

    #[test]
    fn test_swan_stop_clears_velocity() {
        let mut state = GameState::new();
        state.flying = 1;
        state.swan_vx = 20;
        state.swan_vy = 30;
        state.stop_swan_flight();
        assert_eq!(state.flying, 0);
        assert_eq!(state.swan_vx, 0);
        assert_eq!(state.swan_vy, 0);
    }

    #[test]
    fn test_swan_velocity_accumulates() {
        let mut state = GameState::new();
        state.flying = 1;
        // North: xdir=0, ydir=-3
        state.apply_swan_velocity_impulse(0, -3);
        assert_eq!(state.swan_vx, 0);
        assert_eq!(state.swan_vy, -3);
        // Another north impulse
        state.apply_swan_velocity_impulse(0, -3);
        assert_eq!(state.swan_vy, -6);
    }

    #[test]
    fn test_swan_velocity_horizontal_cap() {
        let mut state = GameState::new();
        state.flying = 1;
        // East: xdir=3, ydir=0
        for _ in 0..20 {
            state.apply_swan_velocity_impulse(3, 0);
        }
        assert_eq!(state.swan_vx, 32, "horizontal velocity capped at 32");
        assert_eq!(state.swan_vy, 0);
    }

    #[test]
    fn test_swan_velocity_vertical_cap() {
        let mut state = GameState::new();
        state.flying = 1;
        // South: xdir=0, ydir=3
        for _ in 0..20 {
            state.apply_swan_velocity_impulse(0, 3);
        }
        assert_eq!(state.swan_vx, 0);
        assert_eq!(state.swan_vy, 40, "vertical velocity capped at 40");
    }

    #[test]
    fn test_swan_velocity_negative_cap() {
        let mut state = GameState::new();
        state.flying = 1;
        // West: xdir=-3, ydir=0
        for _ in 0..20 {
            state.apply_swan_velocity_impulse(-3, 0);
        }
        assert_eq!(state.swan_vx, -32, "horizontal velocity capped at -32");
        // North: xdir=0, ydir=-3
        state.swan_vx = 0;
        for _ in 0..20 {
            state.apply_swan_velocity_impulse(0, -3);
        }
        assert_eq!(state.swan_vy, -40, "vertical velocity capped at -40");
    }

    #[test]
    fn test_swan_position_update_formula() {
        let mut state = GameState::new();
        state.flying = 1;
        state.hero_x = 1000;
        state.hero_y = 2000;
        state.swan_vx = 20;
        state.swan_vy = 32;
        let (new_x, new_y) = state.compute_swan_position();
        // pos += vel/4: dx = 20/4 = 5, dy = 32/4 = 8
        assert_eq!(new_x, 1005);
        assert_eq!(new_y, 2008);
    }

    #[test]
    fn test_swan_position_wraps_outdoor() {
        let mut state = GameState::new();
        state.flying = 1;
        state.region_num = 3; // outdoor
        state.hero_x = 0x7FFE;
        state.hero_y = 0x7FFE;
        state.swan_vx = 32; // max east: 32/4 = 8
        state.swan_vy = 40; // max south: 40/4 = 10
        let (new_x, new_y) = state.compute_swan_position();
        // X: 0x7FFE + 8 = 0x8006, wraps to 0x0006
        // Y: 0x7FFE + 10 = 0x8008, wraps to 0x0008
        assert_eq!(new_x, 0x0006);
        assert_eq!(new_y, 0x0008);
    }

    #[test]
    fn test_swan_position_no_wrap_indoor() {
        let mut state = GameState::new();
        state.flying = 1;
        state.region_num = 8; // indoor
        state.hero_x = 0x7FFE;
        state.hero_y = 0x8500; // indoor Y range
        state.swan_vx = 32;
        state.swan_vy = 40;
        let (new_x, new_y) = state.compute_swan_position();
        // X still wraps (outdoor wrapping), Y does not
        assert_eq!(new_x, 0x0006);
        assert_eq!(new_y, 0x850A); // no wrap
    }

    #[test]
    fn test_swan_dismount_requires_low_velocity() {
        let mut state = GameState::new();
        state.flying = 1;
        state.swan_vx = 14;
        state.swan_vy = 14;
        assert!(state.can_dismount_swan(), "vel < 15 should allow dismount");
        
        state.swan_vx = 15;
        assert!(!state.can_dismount_swan(), "vel >= 15 should block dismount");
        
        state.swan_vx = 14;
        state.swan_vy = 15;
        assert!(!state.can_dismount_swan(), "vy >= 15 should block dismount");
    }

    #[test]
    fn test_swan_dismount_not_flying() {
        let mut state = GameState::new();
        state.flying = 0;
        state.swan_vx = 0;
        state.swan_vy = 0;
        assert!(!state.can_dismount_swan(), "cannot dismount when not flying");
    }

    #[test]
    fn test_swan_velocity_impulse_no_op_when_not_flying() {
        let mut state = GameState::new();
        state.flying = 0;
        state.swan_vx = 0;
        state.swan_vy = 0;
        state.apply_swan_velocity_impulse(3, 3);
        assert_eq!(state.swan_vx, 0, "velocity should not change when not flying");
        assert_eq!(state.swan_vy, 0);
    }

    #[test]
    fn test_inventory_array_len_is_36() {
        // T3-INV-STUFF-36: SPEC §14.1 — ARROWBASE = 35, array must be length 36.
        let state = GameState::new();
        assert_eq!(state.julstuff.len(), 36, "julstuff must have 36 slots (indices 0-35)");
        assert_eq!(state.philstuff.len(), 36);
        assert_eq!(state.kevstuff.len(), 36);
        assert_eq!(state.stuff().len(), 36, "stuff() slice must be length 36");
    }

    #[test]
    fn test_arrowbase_pickup_no_panic() {
        // T3-INV-STUFF-36: pickup_item(35) must not panic and must use slot 35 as accumulator.
        let mut state = GameState::new();
        assert_eq!(state.stuff()[35], 0, "ARROWBASE slot starts at 0");
        let ok = state.pickup_item(35);
        assert!(ok, "pickup_item(35) should succeed");
        assert_eq!(state.stuff()[35], 1, "ARROWBASE accumulator should be incremented");

        // Index 36+ still rejected.
        assert!(!state.pickup_item(36), "pickup_item(36) must return false");
    }

    // -------------------------------------------------------------------------
    // T3-SURV-SLEEP-HEAL: SPEC §18.4 / §18.6 — sleep healing rate tests
    // -------------------------------------------------------------------------

    /// Helper: advance GameState awake by N daynight ticks. Returns heal count.
    fn advance_awake(state: &mut GameState, daynight_ticks: u32) -> i16 {
        let before = state.vitality;
        // tick() advances daynight by delta, healing on each (daynight & 0x3FF) == 0.
        state.tick(daynight_ticks);
        state.vitality - before
    }

    #[test]
    fn test_awake_healing_baseline() {
        // SPEC §18.6: +1 HP every 1024 daynight ticks while awake and injured.
        let mut state = GameState::new();
        state.brave = 0;           // cap = 15
        state.vitality = 1;        // injured
        state.battleflag = false;
        // Align daynight to a known position so the boundary is predictable.
        state.daynight = 0;

        // Advance exactly 1024 daynight ticks (one awake heal period).
        let healed = advance_awake(&mut state, 1024);
        assert_eq!(healed, 1, "awake: should gain exactly 1 HP per 1024 daynight ticks");
    }

    #[test]
    fn test_sleep_healing_scales_with_daynight() {
        // SPEC §18.6: "During sleep, daynight advances by 63 per frame,
        // so healing occurs ≈63× faster."
        //
        // We compare total heal over 1024 real frames:
        //   awake:   1024 daynight ticks → 1 heal
        //   asleep:  1024 × 64 = 65536 daynight ticks → ≈65 heals
        //
        // Use high brave so cap (78) is well above the expected heal count.
        let brave = 252i16; // cap = 15 + 252/4 = 78

        // --- Awake baseline ---
        let mut awake = GameState::new();
        awake.brave = brave;
        awake.vitality = 1;
        awake.battleflag = false;
        awake.daynight = 0;
        // 1024 real-time ticks awake → +1 heal at the 1024-tick boundary
        awake.tick(1024);
        let awake_heal = awake.vitality - 1;
        assert_eq!(awake_heal, 1, "awake: 1 HP per 1024 daynight ticks");

        // --- Sleeping ---
        let mut asleep = GameState::new();
        asleep.brave = brave;
        asleep.vitality = 1;
        asleep.battleflag = false;
        asleep.daynight = 0;
        asleep.fatigue = 255; // prevent waking from fatigue during test

        // 1024 real-time frames: each frame = tick(1) [+1 daynight] +
        //   sleep_advance_daynight() [+63 daynight] = 64 daynight ticks/frame.
        // Over 1024 frames: 65536 daynight ticks → ~65 heals (wrapping at 24000
        // yields 24 heals/cycle; 2 full cycles + 17 heals = 65).
        for _ in 0..1024 {
            asleep.tick(1);
            asleep.sleep_advance_daynight(); // ignore wake return for test
        }
        let sleep_heal = asleep.vitality - 1;

        // Must be substantially more than awake; ≈63× per spec (allow ±15).
        assert!(
            sleep_heal >= 50 && sleep_heal <= 80,
            "sleeping: expected ≈63–65 heals in 1024 frames, got {}",
            sleep_heal
        );
        assert!(
            sleep_heal > awake_heal * 50,
            "sleeping heal ({}) must far exceed awake heal ({})",
            sleep_heal, awake_heal
        );
    }

    #[test]
    fn test_sleep_heal_clamps_at_max_hp() {
        // SPEC §18.6: vitality must not exceed 15 + brave/4.
        let mut state = GameState::new();
        state.brave = 40;           // cap = 25
        state.vitality = 24;       // one below cap
        state.battleflag = false;
        state.daynight = 0;
        state.fatigue = 200;

        // Run many sleep frames — should cap at 25, never exceed it.
        for _ in 0..10_000 {
            state.tick(1);
            state.sleep_advance_daynight();
        }
        assert_eq!(state.vitality, 25, "vitality must not exceed heal cap (15 + brave/4)");
    }

    #[test]
    fn test_awake_heal_no_overshoot() {
        // Healing while awake also must not exceed cap.
        let mut state = GameState::new();
        state.brave = 20;           // cap = 20
        state.vitality = 19;
        state.battleflag = false;
        state.daynight = 0;
        // 5000 ticks — enough for multiple heal events.
        state.tick(5000);
        assert_eq!(state.vitality, 20, "awake: vitality must not exceed heal cap");
    }

    #[test]
    fn test_sleep_no_heal_at_max_hp() {
        // If already at cap, sleeping should not change vitality.
        let mut state = GameState::new();
        state.brave = 0;            // cap = 15
        state.vitality = 15;        // already at cap
        state.battleflag = false;
        state.daynight = 0;
        state.fatigue = 100;

        for _ in 0..1000 {
            state.tick(1);
            state.sleep_advance_daynight();
        }
        assert_eq!(state.vitality, 15, "no heal when already at max HP");
    }
}
