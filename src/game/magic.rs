//! Magic item system: 7 consumable items with timer-based effects.
//! Ports MAGIC menu (fmain.c case MAGIC, MAGICBASE=9) verbatim.
//!
//! Items occupy stuff[9..=15]; timers (light_timer, secret_timer, freeze_timer)
//! live in GameState and are decremented each tick there.

use crate::game::actor::ActorKind;
use crate::game::game_state::GameState;

/// Magic item indices in stuff[] (MAGICBASE = 9 in fmain.c).
/// hit=5..=11 in the MAGIC menu; item = stuff[4 + hit].
pub const ITEM_STONE_RING: usize = 9;   // hit=5: teleport via stone ring
pub const ITEM_LANTERN:    usize = 10;  // hit=6: light_timer += 760
pub const ITEM_VIAL:       usize = 11;  // hit=7: heal (vitality += rand8() + 4)
pub const ITEM_ORB:        usize = 12;  // hit=8: secret_timer += 360
pub const ITEM_TOTEM:      usize = 13;  // hit=9: show world map
pub const ITEM_RING:       usize = 14;  // hit=10: freeze_timer += 100
pub const ITEM_SKULL:      usize = 15;  // hit=11: kill all on-screen enemies

/// Timer increments ported verbatim from fmain.c.
pub const LIGHT_TIMER_INCREMENT:  i16 = 760;
pub const SECRET_TIMER_INCREMENT: i16 = 360;
pub const FREEZE_TIMER_INCREMENT: i16 = 100;

/// Heal vitality cap formula from fmain.c: `15 + brave / 4`.
pub fn heal_cap(brave: i16) -> i16 {
    15 + brave / 4
}

/// Use a magic item from inventory.
///
/// Mirrors fmain.c `case MAGIC` switch, consuming one charge and applying
/// the effect to `state`.  Returns a human-readable message or `Err` when the
/// item is not in stock.
///
/// Notes on partial implementation:
/// - `ITEM_STONE_RING` (stone-ring teleport): requires stone_list data not yet
///   loaded; returns Ok with a stub message.
/// - `ITEM_TOTEM` (world map): sets `viewstatus = 1`; the caller is responsible
///   for rendering the map overlay.
/// - `ITEM_SKULL` (kill enemies): uses the actor list in GameState directly.
pub fn use_magic(state: &mut GameState, item_idx: usize) -> Result<&'static str, &'static str> {
    if item_idx < ITEM_STONE_RING || item_idx > ITEM_SKULL {
        return Err("Not a magic item");
    }
    if state.stuff()[item_idx] == 0 {
        return Err("You have none of that.");
    }

    let msg = match item_idx {
        ITEM_STONE_RING => {
            // fmain.c: teleports hero between stone rings (requires hero_sector == 144).
            // Full stone-list teleport is deferred; consume item and stub.
            "The stone ring glows but nothing happens here."
        }
        ITEM_LANTERN => {
            state.light_timer = state.light_timer.saturating_add(LIGHT_TIMER_INCREMENT);
            "A warm light surrounds you."
        }
        ITEM_VIAL => {
            // fmain.c: vitality += rand8() + 4, capped at 15 + brave/4.
            // Use fixed heal of 8 (midpoint of rand8() range 0..=7, + 4 = ~8).
            let heal: i16 = 8;
            let cap = heal_cap(state.brave);
            state.vitality = (state.vitality + heal).min(cap);
            "That feels a lot better!"
        }
        ITEM_ORB => {
            state.secret_timer = state.secret_timer.saturating_add(SECRET_TIMER_INCREMENT);
            "You feel unseen."
        }
        ITEM_TOTEM => {
            // Show world map overlay (viewstatus = 1 in fmain.c).
            state.viewstatus = 1;
            "The bird totem shows the way."
        }
        ITEM_RING => {
            state.freeze_timer = state.freeze_timer.saturating_add(FREEZE_TIMER_INCREMENT);
            "Time slows around you."
        }
        ITEM_SKULL => {
            // fmain.c: zero vitality of all on-screen ENEMY actors with race < 7.
            let mut killed = 0usize;
            let anix = state.anix;
            for i in 1..anix {
                let a = &mut state.actors[i];
                if a.vitality > 0 && a.kind == ActorKind::Enemy && a.race < 7 {
                    a.vitality = 0;
                    killed += 1;
                }
            }
            if killed > 0 { "Death takes them all!" } else { "No enemies to claim." }
        }
        _ => return Err("Not a magic item"),
    };

    state.stuff_mut()[item_idx] -= 1;
    Ok(msg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::game_state::GameState;

    #[test]
    fn test_lantern_adds_to_light_timer() {
        let mut state = GameState::new();
        state.stuff_mut()[ITEM_LANTERN] = 1;
        let result = use_magic(&mut state, ITEM_LANTERN);
        assert!(result.is_ok());
        assert_eq!(state.light_timer, LIGHT_TIMER_INCREMENT);
        assert_eq!(state.stuff()[ITEM_LANTERN], 0);
    }

    #[test]
    fn test_orb_adds_to_secret_timer() {
        let mut state = GameState::new();
        state.stuff_mut()[ITEM_ORB] = 2;
        let _ = use_magic(&mut state, ITEM_ORB);
        assert_eq!(state.secret_timer, SECRET_TIMER_INCREMENT);
        // Second use stacks.
        let _ = use_magic(&mut state, ITEM_ORB);
        assert_eq!(state.secret_timer, SECRET_TIMER_INCREMENT * 2);
    }

    #[test]
    fn test_ring_adds_to_freeze_timer() {
        let mut state = GameState::new();
        state.stuff_mut()[ITEM_RING] = 1;
        let _ = use_magic(&mut state, ITEM_RING);
        assert_eq!(state.freeze_timer, FREEZE_TIMER_INCREMENT);
    }

    #[test]
    fn test_vial_heals_vitality() {
        let mut state = GameState::new();
        state.vitality = 5;
        state.brave = 40;
        state.stuff_mut()[ITEM_VIAL] = 1;
        let _ = use_magic(&mut state, ITEM_VIAL);
        assert!(state.vitality > 5);
        assert!(state.vitality <= heal_cap(40));
    }

    #[test]
    fn test_use_item_no_stock() {
        let mut state = GameState::new();
        assert!(use_magic(&mut state, ITEM_LANTERN).is_err());
    }

    #[test]
    fn test_heal_cap() {
        assert_eq!(heal_cap(40), 25); // 15 + 40/4 = 25
        assert_eq!(heal_cap(0),  15);
    }

    #[test]
    fn test_timers_decrement_in_tick() {
        let mut state = GameState::new();
        state.light_timer = 5;
        state.secret_timer = 3;
        state.freeze_timer = 1;
        state.tick(1);
        assert_eq!(state.light_timer, 4);
        assert_eq!(state.secret_timer, 2);
        assert_eq!(state.freeze_timer, 0);
    }
}
