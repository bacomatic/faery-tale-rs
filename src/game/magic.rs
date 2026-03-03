//! Magic item system: 7 consumable items with timer-based effects.
//! Ports MAGIC menu and magic item table from fmain.c.

use crate::game::game_state::GameState;

/// Magic item indices in stuff[].
pub const ITEM_WAND: usize = 20;
pub const ITEM_ORB: usize = 21;
pub const ITEM_POTION: usize = 22;
pub const ITEM_CROWN: usize = 23;
pub const ITEM_AMULET: usize = 24;
pub const ITEM_RING: usize = 25;
pub const ITEM_SCROLL: usize = 26;

/// Active magic effect timers (ticks remaining).
#[derive(Debug, Default, Clone)]
pub struct MagicTimers {
    pub orb_ticks: u16,      // protection remaining
    pub crown_ticks: u16,    // speed boost remaining
    pub amulet_ticks: u16,   // undead protection remaining
    pub ring_ticks: u16,     // luck boost remaining
    pub scroll_ticks: u16,   // map reveal remaining
    pub wand_damage: u8,     // wand charges remaining
}

impl MagicTimers {
    pub fn tick(&mut self) {
        self.orb_ticks = self.orb_ticks.saturating_sub(1);
        self.crown_ticks = self.crown_ticks.saturating_sub(1);
        self.amulet_ticks = self.amulet_ticks.saturating_sub(1);
        self.ring_ticks = self.ring_ticks.saturating_sub(1);
        self.scroll_ticks = self.scroll_ticks.saturating_sub(1);
    }

    pub fn has_protection(&self) -> bool { self.orb_ticks > 0 }
    pub fn has_speed_boost(&self) -> bool { self.crown_ticks > 0 }
    pub fn has_undead_ward(&self) -> bool { self.amulet_ticks > 0 }
    pub fn has_luck_boost(&self) -> bool { self.ring_ticks > 0 }
    pub fn has_map_reveal(&self) -> bool { self.scroll_ticks > 0 }
}

/// Use a magic item from inventory.
/// Returns a description of the effect, or Err if item not available.
pub fn use_magic(state: &mut GameState, timers: &mut MagicTimers, item_idx: usize) -> Result<&'static str, &'static str> {
    if state.stuff()[item_idx] == 0 { return Err("No item"); }
    state.stuff_mut()[item_idx] -= 1;
    match item_idx {
        ITEM_WAND => {
            timers.wand_damage = timers.wand_damage.saturating_add(3);
            Ok("Wand charged with 3 bolts!")
        }
        ITEM_ORB => {
            timers.orb_ticks = 30;
            Ok("Protective orb activated for 30 ticks!")
        }
        ITEM_POTION => {
            state.vitality = (state.vitality + 30).min(255);
            Ok("Vitality restored by 30!")
        }
        ITEM_CROWN => {
            timers.crown_ticks = 20;
            Ok("Speed boosted for 20 ticks!")
        }
        ITEM_AMULET => {
            timers.amulet_ticks = 60;
            Ok("Protected from undead for 60 ticks!")
        }
        ITEM_RING => {
            timers.ring_ticks = 10;
            Ok("Luck enhanced for 10 ticks!")
        }
        ITEM_SCROLL => {
            timers.scroll_ticks = 100;
            Ok("Map revealed for 100 ticks!")
        }
        _ => Err("Not a magic item"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::game_state::GameState;

    #[test]
    fn test_use_potion_restores_vitality() {
        let mut state = GameState::new();
        let mut timers = MagicTimers::default();
        state.vitality = 10;
        state.stuff_mut()[ITEM_POTION] = 1;
        let result = use_magic(&mut state, &mut timers, ITEM_POTION);
        assert!(result.is_ok());
        assert_eq!(state.vitality, 40);
    }

    #[test]
    fn test_use_orb_sets_timer() {
        let mut state = GameState::new();
        let mut timers = MagicTimers::default();
        state.stuff_mut()[ITEM_ORB] = 1;
        let _ = use_magic(&mut state, &mut timers, ITEM_ORB);
        assert!(timers.has_protection());
        assert_eq!(timers.orb_ticks, 30);
    }

    #[test]
    fn test_timers_tick_down() {
        let mut timers = MagicTimers { orb_ticks: 3, ..Default::default() };
        timers.tick();
        assert_eq!(timers.orb_ticks, 2);
        timers.tick(); timers.tick();
        assert!(!timers.has_protection());
    }

    #[test]
    fn test_use_item_no_stock() {
        let mut state = GameState::new();
        let mut timers = MagicTimers::default();
        assert!(use_magic(&mut state, &mut timers, ITEM_WAND).is_err());
    }
}
