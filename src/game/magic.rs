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

/// Simple pseudo-random 0-7 for magic effects (ports rand8() pattern).
/// Uses system time nanos similar to combat.rs melee_rand().
fn rand8() -> i16 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos & 7) as i16
}

/// Stone ring sector coordinates from fmain.c stone_list[].
/// 11 pairs of (x_sector, y_sector) for teleport destinations.
const STONE_RINGS: [(u8, u8); 11] = [
    (54, 43), (71, 77),  (78, 102), (66, 121), (12, 85),
    (79, 40), (107, 38), (73, 21),  (12, 26),  (26, 53), (84, 60),
];

/// Stone ring activation sector (fmain.c: hero_sector == 144).
const STONE_RING_SECTOR: u16 = 144;

/// Find the index of the stone ring the hero is currently standing at,
/// based on their sector coordinates matching a ring in STONE_RINGS.
fn find_current_ring(hero_x: u16, hero_y: u16) -> Option<usize> {
    let sx = (hero_x >> 8) as u8;
    let sy = (hero_y >> 8) as u8;
    STONE_RINGS.iter().position(|&(rx, ry)| rx == sx && ry == sy)
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
            // fmain.c: teleports hero between stone rings when standing on one.
            // Requires hero_sector == 144, hero centered in tile, and a matching ring.
            if state.hero_sector != STONE_RING_SECTOR {
                return Err("You must stand on a stone ring to use this.");
            }
            // Check hero is roughly centered in tile (fmain.c: (hero_x & 255) / 85 == 1)
            let hx_frac = (state.hero_x & 255) / 85;
            let hy_frac = (state.hero_y & 255) / 64;
            if hx_frac != 1 || hy_frac != 1 {
                return Err("Move to the center of the stone ring.");
            }
            if let Some(current) = find_current_ring(state.hero_x, state.hero_y) {
                // Destination = current ring + facing + 1, wrapped mod 11
                let dest = (current + state.facing as usize + 1) % STONE_RINGS.len();
                let (dx, dy) = STONE_RINGS[dest];
                // Preserve sub-sector offset, change sector
                state.hero_x = ((dx as u16) << 8) | (state.hero_x & 255);
                state.hero_y = ((dy as u16) << 8) | (state.hero_y & 255);
                "The stone ring transports you!"
            } else {
                return Err("The stone ring glows but nothing happens here.");
            }
        }
        ITEM_LANTERN => {
            state.light_timer = state.light_timer.saturating_add(LIGHT_TIMER_INCREMENT);
            "A warm light surrounds you."
        }
        ITEM_VIAL => {
            // SPEC §19.2: vitality += rand8() + 4 (yields 4-11), capped at 15 + brave/4.
            let heal = rand8() + 4;
            let cap = heal_cap(state.brave);
            state.vitality = (state.vitality + heal).min(cap);
            "That feels a lot better!"
        }
        ITEM_ORB => {
            state.secret_timer = state.secret_timer.saturating_add(SECRET_TIMER_INCREMENT);
            "You feel unseen."
        }
        ITEM_TOTEM => {
            // SPEC §19.2, §25.9: blocked when region_num > 7 unless cheat1 is set.
            if state.region_num > 7 && !state.cheat1 {
                return Err("The bird totem does not work indoors.");
            }
            state.viewstatus = 1;
            "The bird totem shows the way."
        }
        ITEM_RING => {
            state.freeze_timer = state.freeze_timer.saturating_add(FREEZE_TIMER_INCREMENT);
            "Time slows around you."
        }
        ITEM_SKULL => {
            // SPEC §19.2: Kill spell kills all visible enemies, brave-- per kill.
            // Counterbalances normal combat brave++ per kill.
            let mut killed = 0usize;
            let anix = state.anix;
            for i in 1..anix {
                let a = &mut state.actors[i];
                if a.vitality > 0 && a.kind == ActorKind::Enemy && a.race < 7 {
                    a.vitality = 0;
                    killed += 1;
                }
            }
            // Decrement brave once per enemy killed (cowardice penalty).
            state.brave = state.brave.saturating_sub(killed as i16);
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
    
    #[test]
    fn test_vial_heal_randomness() {
        // SPEC §19.2: heal = rand8() + 4 (yields 4-11), capped at 15 + brave/4.
        let mut state = GameState::new();
        state.vitality = 5;
        state.brave = 40; // cap = 15 + 40/4 = 25
        state.stuff_mut()[ITEM_VIAL] = 10;
        
        // Test multiple uses to verify randomness and capping.
        for _ in 0..10 {
            let before = state.vitality;
            let _ = use_magic(&mut state, ITEM_VIAL);
            let gained = state.vitality - before;
            // Heal should be in range [4, 11] but capped by (25 - before).
            let expected_max = (11).min(25 - before);
            assert!(gained >= 4.min(25 - before) && gained <= expected_max,
                    "Heal gained {gained} outside expected range");
            assert!(state.vitality <= 25, "Vitality {0} exceeded cap 25", state.vitality);
        }
    }
    
    #[test]
    fn test_vial_heal_cap_enforcement() {
        // SPEC §19.2: heal is capped at 15 + brave/4.
        let mut state = GameState::new();
        state.brave = 20; // cap = 15 + 20/4 = 20
        state.vitality = 18;
        state.stuff_mut()[ITEM_VIAL] = 1;
        let _ = use_magic(&mut state, ITEM_VIAL);
        // Even if rand8() + 4 = 11, vitality should not exceed 20.
        assert!(state.vitality <= 20, "Vitality exceeded cap");
    }
    
    #[test]
    fn test_jade_skull_brave_decrement() {
        // SPEC §19.2: brave-- per kill (counterbalances normal combat brave++).
        let mut state = GameState::new();
        state.brave = 50;
        state.stuff_mut()[ITEM_SKULL] = 1;
        
        // Set up 3 killable enemies (race < 7, vitality > 0, kind = Enemy).
        state.anix = 4;
        state.actors[1].vitality = 10;
        state.actors[1].kind = ActorKind::Enemy;
        state.actors[1].race = 3;
        state.actors[2].vitality = 15;
        state.actors[2].kind = ActorKind::Enemy;
        state.actors[2].race = 5;
        state.actors[3].vitality = 20;
        state.actors[3].kind = ActorKind::Enemy;
        state.actors[3].race = 6;
        
        let _ = use_magic(&mut state, ITEM_SKULL);
        
        // All 3 enemies should be dead.
        assert_eq!(state.actors[1].vitality, 0);
        assert_eq!(state.actors[2].vitality, 0);
        assert_eq!(state.actors[3].vitality, 0);
        
        // Brave should have decreased by 3 (one per kill).
        assert_eq!(state.brave, 47, "Brave should decrement by 3 (one per kill)");
    }
    
    #[test]
    fn test_jade_skull_no_brave_change_if_no_kills() {
        // SPEC §19.2: brave-- per kill; if no kills, no brave change.
        let mut state = GameState::new();
        state.brave = 50;
        state.stuff_mut()[ITEM_SKULL] = 1;
        state.anix = 1; // No enemies.
        
        let _ = use_magic(&mut state, ITEM_SKULL);
        assert_eq!(state.brave, 50, "Brave should not change if no enemies killed");
    }
    
    #[test]
    fn test_totem_blocked_underground() {
        // SPEC §19.2, §25.9: blocked when region_num > 7 unless cheat1 is set.
        let mut state = GameState::new();
        state.stuff_mut()[ITEM_TOTEM] = 1;
        state.region_num = 8; // Underground.
        state.cheat1 = false;
        
        let result = use_magic(&mut state, ITEM_TOTEM);
        assert!(result.is_err(), "Totem should be blocked underground");
        assert_eq!(state.stuff()[ITEM_TOTEM], 1, "Charge should not be consumed");
    }
    
    #[test]
    fn test_totem_allowed_overworld() {
        // SPEC §19.2: allowed when region_num <= 7.
        let mut state = GameState::new();
        state.stuff_mut()[ITEM_TOTEM] = 1;
        state.region_num = 7;
        
        let result = use_magic(&mut state, ITEM_TOTEM);
        assert!(result.is_ok(), "Totem should work in overworld");
        assert_eq!(state.viewstatus, 1);
        assert_eq!(state.stuff()[ITEM_TOTEM], 0, "Charge should be consumed");
    }
    
    #[test]
    fn test_totem_cheat1_bypass() {
        // SPEC §25.9: cheat1 bypasses region restriction.
        let mut state = GameState::new();
        state.stuff_mut()[ITEM_TOTEM] = 1;
        state.region_num = 9; // Deep underground.
        state.cheat1 = true;
        
        let result = use_magic(&mut state, ITEM_TOTEM);
        assert!(result.is_ok(), "Totem should work underground when cheat1 is set");
        assert_eq!(state.viewstatus, 1);
        assert_eq!(state.stuff()[ITEM_TOTEM], 0, "Charge should be consumed");
    }
}
