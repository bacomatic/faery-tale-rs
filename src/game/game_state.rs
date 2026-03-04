use crate::game::actor::Actor;
use crate::game::debug_command::GodModeFlags;

/// Lasso item index in stuff array (from original fmain.h).
pub const ITEM_LASSO: usize = 16;
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
    pub on_raft: bool,
    /// Proximity to raft actor: 0=none, 1=near (within 16px), 2=aboard (within 9px).
    /// Mirrors raftprox from fmain.c (player-107).
    pub raftprox: i16,
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

    // Tick counter (cumulative ticks since start)
    pub tick_counter: u32,

    // Brother liveness (Julian=0, Phillip=1, Kevin=2)
    pub brother_alive: [bool; 3],
}

impl GameState {
    /// Max fatigue before forced sleep (original: 500).
    pub const MAX_FATIGUE: i16 = 500;

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
            on_raft: false,
            raftprox: 0,
            actor_file: 0,
            set_file: 0,

            princess: 0,
            dayperiod: 0,

            current_mood: 0,

            facing: 0,
            gold: 0,

            god_mode: GodModeFlags::empty(),
            light_sticky: false,
            secret_sticky: false,
            freeze_sticky: false,

            tick_counter: 0,
            brother_alive: [true, true, true],
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

    /// Advance game state by `delta` ticks.
    pub fn tick(&mut self, delta: u32) {
        const HUNGER_PERIOD: u32 = 300;
        const HUNGER_MAX: i16 = 300;
        const HEAL_PERIOD: u32 = 600;

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

        // Advance day/night cycle once per tick.
        for _ in 0..delta {
            self.daynight_tick();
        }

        let prev = self.tick_counter;
        self.tick_counter = self.tick_counter.wrapping_add(delta);

        // Hunger: +1 every HUNGER_PERIOD ticks.
        let prev_hunger_phase = prev / HUNGER_PERIOD;
        let next_hunger_phase = self.tick_counter / HUNGER_PERIOD;
        if next_hunger_phase > prev_hunger_phase {
            let increments = (next_hunger_phase - prev_hunger_phase) as i16;
            self.hunger = (self.hunger + increments).min(HUNGER_MAX);
            self.apply_hunger_effects();
        }

        // Healing: +1 vitality every HEAL_PERIOD ticks when out of battle and injured.
        if !self.battleflag && self.vitality > 0 && self.vitality < 100 {
            let prev_heal_phase = prev / HEAL_PERIOD;
            let next_heal_phase = self.tick_counter / HEAL_PERIOD;
            if next_heal_phase > prev_heal_phase {
                let increments = (next_heal_phase - prev_heal_phase) as i16;
                self.vitality = (self.vitality + increments).min(100);
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

    /// Switch to the given brother: mark current as dead, swap inventory context.
    pub fn activate_brother(&mut self, new_idx: usize) {
        self.brother_alive[self.active_brother] = false;
        self.active_brother = new_idx;
        // brother field: 1=Julian, 2=Phillip, 3=Kevin
        self.brother = (new_idx as u8) + 1;
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
    }

    /// Summon turtle using a shell item. Returns true if successful.
    /// Turtle acts like raft for water traversal (on_raft=true) but cannot enter mountains.
    pub fn summon_turtle(&mut self) -> bool {
        if self.stuff()[ITEM_SHELL] > 0 {
            self.stuff_mut()[ITEM_SHELL] -= 1;
            self.active_carrier = CARRIER_TURTLE;
            self.on_raft = true;
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
            true
        } else {
            false
        }
    }

    /// Stop swan flight (land).
    pub fn stop_swan_flight(&mut self) {
        self.flying = 0;
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
        if item_id >= 35 { return false; }
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
        if item_id >= 35 { return false; }
        let stuff = self.stuff_mut();
        if stuff[item_id] > 0 {
            stuff[item_id] -= 1;
            true
        } else {
            false
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

    /// Increment fatigue by 1. At MAX_FATIGUE, resets to 0 (forced sleep).
    /// Returns true if forced sleep occurred.
    pub fn tick_fatigue(&mut self) -> bool {
        self.fatigue = (self.fatigue + 1).min(Self::MAX_FATIGUE);
        if self.fatigue >= Self::MAX_FATIGUE {
            self.fatigue = 0;
            true
        } else {
            false
        }
    }

    /// Per-movement-step fatigue update (player-111).
    /// +1 when moving, -1 when resting. Returns true if forced sleep triggered.
    pub fn fatigue_step(&mut self, moved: bool) -> bool {
        if moved {
            self.fatigue = self.fatigue.saturating_add(1);
        } else {
            self.fatigue = self.fatigue.saturating_sub(1);
        }
        if self.fatigue >= Self::MAX_FATIGUE {
            self.fatigue = 0;
            true
        } else {
            false
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

    /// Update safe spawn point if current terrain is passable (not water).
    /// terrain_type: 0=open, 1=hard block edge; 2+ = water/impassable.
    pub fn update_safe_spawn(&mut self, terrain_type: u8) {
        if terrain_type < 2 {
            self.safe_x = self.hero_x;
            self.safe_y = self.hero_y;
            self.safe_r = self.region_num;
        }
    }

    /// Attempt luck-gated respawn. Returns true if respawned.
    /// Requires luck >= 10; costs 10 luck per use.
    pub fn try_respawn(&mut self) -> bool {
        if self.luck >= 10 {
            self.luck -= 10;
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
    fn test_tick_fatigue_max() {
        let mut s = GameState::new();
        s.fatigue = GameState::MAX_FATIGUE - 1;
        let forced = s.tick_fatigue();
        assert!(forced);
        assert_eq!(s.fatigue, 0);
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
    }

    #[test]
    fn test_try_respawn_no_luck() {
        let mut s = GameState::new();
        s.luck = 5;
        assert!(!s.try_respawn());
    }
}
