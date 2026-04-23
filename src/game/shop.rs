//! Shop (bartender) buy-menu dispatch.
//!
//! Direct port of `buy_dispatch` at `fmain.c:3424-3442` and the
//! `TABLE:jtrans` row map at `fmain2.c:850`.  See
//! `reference/logic/shops.md` for the authoritative behavioural spec.
//!
//! Faery Tale Adventure has exactly one vendor NPC class — the bartender
//! (setfig race byte `0x88`).  Every commercial interaction flows through
//! a single seven-slot dispatch; there is no inn, temple, or weapon shop.

use crate::game::game_state::GameState;
use crate::game::npc::{Npc, RACE_SHOPKEEPER};

/// `TABLE:jtrans` (fmain2.c:850) — seven `(inv_list index, gold price)`
/// pairs keyed by buy-menu slot 0..=6 (= `fmain.c` BUY `hit` values 5..=11).
///
/// Slot 0 (Food) uses `inv_list` index `0` as a sentinel: the Dirk is at
/// `inv_list[0]`, but the Food branch fires `eat(50)` + `event(22)`
/// instead of granting `stuff[0]`.
///
/// Slot 1 (Arrow) uses `inv_list` index `8` as a second sentinel: arrows
/// are granted in ten-shot bundles (`stuff[8] += 10`) with `event(23)`.
pub const JTRANS: [(usize, i16); 7] = [
    (0,   3),  // slot 0 — Food   (sentinel → eat(50) + event(22))
    (8,  10),  // slot 1 — Arrow  (sentinel → stuff[8] += 10 + event(23))
    (11, 15),  // slot 2 — Glass Vial
    (1,  30),  // slot 3 — Mace
    (2,  45),  // slot 4 — Sword
    (3,  75),  // slot 5 — Bow
    (13, 20),  // slot 6 — Bird Totem
];

/// Side-effect classification for a successful purchase.  Mirrors the
/// three branches of `buy_dispatch` at `fmain.c:3433-3437`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuyOutcome {
    /// Food sentinel (`i == 0`): caller narrates `event(22)` and the
    /// `eat(50)` side-effect has already been applied.
    Food,
    /// Arrow sentinel (`i == 8`): `stuff[8] += 10` applied; caller
    /// narrates `event(23)`.
    Arrows,
    /// Generic item: `stuff[inv_idx]++` applied; caller narrates
    /// `extract("% bought a ")` + `inv_list[inv_idx].name` + `"."`.
    Item { inv_idx: usize },
}

/// Result of a `buy_slot` attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuyResult {
    /// Silent no-op.  Produced when the nearest person is not a
    /// bartender or when the slot is out of range.  Matches the
    /// `return` arms at `fmain.c:3425-3427`.
    Silent,
    /// `wealth <= j` denial (`fmain.c:3430`).  Caller prints
    /// `"Not enough money!"` per `reference/logic/dialog_system.md:341`.
    NotEnough,
    /// Purchase succeeded; per-branch side-effects already applied.
    Bought(BuyOutcome),
}

/// `buy_dispatch` slot resolver (`fmain.c:3424-3442`).
///
/// `slot` is the BUY-menu hit minus 5 (i.e. `0..=6`).  Follows the
/// original's `hit > 11` short-circuit by returning [`BuyResult::Silent`]
/// for any slot outside that range, and enforces the strict
/// `wealth > price` gate (`fmain.c:3430` checks `wealth <= j`, so a
/// player with exactly `j` gold cannot afford a `j`-gold item — a
/// deliberate 1-gold margin, preserved verbatim).
pub fn buy_slot(state: &mut GameState, slot: usize) -> BuyResult {
    if slot >= JTRANS.len() {
        return BuyResult::Silent;
    }
    let (inv_idx, price) = JTRANS[slot];
    if state.wealth <= price {
        return BuyResult::NotEnough;
    }
    state.wealth -= price;
    match inv_idx {
        0 => BuyResult::Bought(BuyOutcome::Food),
        8 => {
            let s = state.stuff_mut();
            s[8] = s[8].saturating_add(10);
            BuyResult::Bought(BuyOutcome::Arrows)
        }
        _ => {
            let s = state.stuff_mut();
            s[inv_idx] = s[inv_idx].saturating_add(1);
            BuyResult::Bought(BuyOutcome::Item { inv_idx })
        }
    }
}

/// Port-level proximity probe standing in for the original's
/// `nearest_person` global (`fmain.c:3425-3426`).  Returns `true` when a
/// bartender-race NPC is within a 32×32 bounding box of the hero — the
/// coarse approximation the port has used since the shop module was
/// introduced.  The original gates on `anim_list[nearest_person].race
/// == 0x88` after `nearest_fig` has run; the spec is explicit that the
/// BUY menu is *inert* (silent break) in front of any non-bartender.
pub fn has_shopkeeper_nearby(npcs: &[Npc], hero_x: i16, hero_y: i16) -> bool {
    npcs.iter().any(|n| {
        n.active
            && n.race == RACE_SHOPKEEPER
            && (n.x - hero_x).abs() < 32
            && (n.y - hero_y).abs() < 32
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::game_state::GameState;

    #[test]
    fn buy_slot_food_is_sentinel_no_stuff_grant() {
        // Food fires eat(50)+event(22); stuff[0] (Dirk) must not change.
        let mut state = GameState::new();
        state.wealth = 100;
        state.stuff_mut()[0] = 1; // Dirk count
        let r = buy_slot(&mut state, 0);
        assert_eq!(r, BuyResult::Bought(BuyOutcome::Food));
        assert_eq!(state.wealth, 97);
        assert_eq!(state.stuff()[0], 1, "Dirk count must be untouched");
    }

    #[test]
    fn buy_slot_arrow_grants_bundle_of_10() {
        let mut state = GameState::new();
        state.wealth = 100;
        let r = buy_slot(&mut state, 1);
        assert_eq!(r, BuyResult::Bought(BuyOutcome::Arrows));
        assert_eq!(state.wealth, 90);
        assert_eq!(state.stuff()[8], 10, "arrows land in stuff[8]");
        assert_eq!(state.stuff()[1], 0, "stuff[1] (Mace) must be untouched");
    }

    #[test]
    fn buy_slot_vial_lands_in_stuff_11() {
        let mut state = GameState::new();
        state.wealth = 100;
        let r = buy_slot(&mut state, 2);
        assert_eq!(r, BuyResult::Bought(BuyOutcome::Item { inv_idx: 11 }));
        assert_eq!(state.wealth, 85);
        assert_eq!(state.stuff()[11], 1);
    }

    #[test]
    fn buy_slot_mace_lands_in_stuff_1() {
        let mut state = GameState::new();
        state.wealth = 100;
        let r = buy_slot(&mut state, 3);
        assert_eq!(r, BuyResult::Bought(BuyOutcome::Item { inv_idx: 1 }));
        assert_eq!(state.wealth, 70);
        assert_eq!(state.stuff()[1], 1);
    }

    #[test]
    fn buy_slot_sword_lands_in_stuff_2() {
        let mut state = GameState::new();
        state.wealth = 100;
        let r = buy_slot(&mut state, 4);
        assert_eq!(r, BuyResult::Bought(BuyOutcome::Item { inv_idx: 2 }));
        assert_eq!(state.wealth, 55);
        assert_eq!(state.stuff()[2], 1);
    }

    #[test]
    fn buy_slot_bow_lands_in_stuff_3() {
        let mut state = GameState::new();
        state.wealth = 100;
        let r = buy_slot(&mut state, 5);
        assert_eq!(r, BuyResult::Bought(BuyOutcome::Item { inv_idx: 3 }));
        assert_eq!(state.wealth, 25);
        assert_eq!(state.stuff()[3], 1);
    }

    #[test]
    fn buy_slot_totem_lands_in_stuff_13() {
        let mut state = GameState::new();
        state.wealth = 100;
        let r = buy_slot(&mut state, 6);
        assert_eq!(r, BuyResult::Bought(BuyOutcome::Item { inv_idx: 13 }));
        assert_eq!(state.wealth, 80);
        assert_eq!(state.stuff()[13], 1);
    }

    #[test]
    fn buy_slot_strict_gold_margin() {
        // fmain.c:3430 — `wealth <= j` is a denial; exact money still fails.
        let mut state = GameState::new();
        state.wealth = 30; // Mace costs 30
        let r = buy_slot(&mut state, 3);
        assert_eq!(r, BuyResult::NotEnough);
        assert_eq!(state.wealth, 30);
        assert_eq!(state.stuff()[1], 0);
    }

    #[test]
    fn buy_slot_one_over_price_succeeds() {
        let mut state = GameState::new();
        state.wealth = 31;
        let r = buy_slot(&mut state, 3);
        assert_eq!(r, BuyResult::Bought(BuyOutcome::Item { inv_idx: 1 }));
        assert_eq!(state.wealth, 1);
    }

    #[test]
    fn buy_slot_out_of_range_is_silent() {
        let mut state = GameState::new();
        state.wealth = 999;
        let r = buy_slot(&mut state, 7);
        assert_eq!(r, BuyResult::Silent);
        assert_eq!(state.wealth, 999);
    }
}
