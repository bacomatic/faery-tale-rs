//! TDD tests for shop prices / inventory slot helpers.
//!
//! Shop pricing and side-effects are exercised in detail by the unit
//! tests inside `src/game/shop.rs`; this file keeps the
//! inventory-slot helper tests (lasso, fruit auto-eat, safe-zone
//! auto-eat) that grew alongside the original shop rewrite.

#[cfg(test)]
mod tests {
    use crate::game::game_state::GameState;
    use crate::game::shop::{buy_slot, BuyOutcome, BuyResult, JTRANS};

    // ============================================================
    // T2-SHOP-COSTS: jtrans prices per reference/logic/shops.md
    // (fmain2.c:850) — Food=3, Arrow=10, Vial=15, Mace=30, Sword=45,
    // Bow=75, Totem=20.
    // ============================================================

    #[test]
    fn t2_shop_costs_food_is_3() {
        assert_eq!(JTRANS[0].1, 3);
    }
    #[test]
    fn t2_shop_costs_arrow_is_10() {
        assert_eq!(JTRANS[1].1, 10);
    }
    #[test]
    fn t2_shop_costs_vial_is_15() {
        assert_eq!(JTRANS[2].1, 15);
    }
    #[test]
    fn t2_shop_costs_mace_is_30() {
        assert_eq!(JTRANS[3].1, 30);
    }
    #[test]
    fn t2_shop_costs_sword_is_45() {
        assert_eq!(JTRANS[4].1, 45);
    }
    #[test]
    fn t2_shop_costs_bow_is_75() {
        assert_eq!(JTRANS[5].1, 75);
    }
    #[test]
    fn t2_shop_costs_totem_is_20() {
        assert_eq!(JTRANS[6].1, 20);
    }

    #[test]
    fn t2_shop_buy_food_deducts_3_gold_no_dirk_grant() {
        // Food (slot 0) is a sentinel: wealth -= 3, but stuff[0] (Dirk)
        // must NOT be incremented — shop fires eat(50) + event(22).
        let mut state = GameState::new();
        state.wealth = 100;
        state.stuff_mut()[0] = 1;
        let r = buy_slot(&mut state, 0);
        assert_eq!(r, BuyResult::Bought(BuyOutcome::Food));
        assert_eq!(state.wealth, 97);
        assert_eq!(state.stuff()[0], 1, "Dirk must not be granted");
    }

    #[test]
    fn t2_shop_buy_arrow_deducts_10_gold_and_grants_bundle() {
        let mut state = GameState::new();
        state.wealth = 100;
        let r = buy_slot(&mut state, 1);
        assert_eq!(r, BuyResult::Bought(BuyOutcome::Arrows));
        assert_eq!(state.wealth, 90);
        assert_eq!(state.stuff()[8], 10);
    }

    #[test]
    fn t2_shop_buy_bow_deducts_75_gold() {
        let mut state = GameState::new();
        state.wealth = 100;
        let r = buy_slot(&mut state, 5);
        assert_eq!(r, BuyResult::Bought(BuyOutcome::Item { inv_idx: 3 }));
        assert_eq!(state.wealth, 25);
        assert_eq!(state.stuff()[3], 1);
    }

    // ============================================================
    // T2-INV-LASSO-INDEX: Golden Lasso = stuff[5], not stuff[16]
    // ============================================================

    #[test]
    fn t2_inv_lasso_index_is_5() {
        use crate::game::game_state::ITEM_LASSO;
        assert_eq!(ITEM_LASSO, 5, "ITEM_LASSO must be 5, not 16");
    }

    #[test]
    fn t2_inv_lasso_has_lasso_checks_slot_5() {
        let mut state = GameState::new();
        state.stuff_mut()[5] = 1;
        assert!(state.has_lasso(), "has_lasso() should check stuff[5]");
    }

    #[test]
    fn t2_inv_lasso_slot_16_is_gold_key() {
        let mut state = GameState::new();
        state.stuff_mut()[16] = 1;
        assert!(
            !state.has_lasso(),
            "stuff[16] should not trigger has_lasso()"
        );
    }

    // ============================================================
    // T2-INV-FRUIT-AUTOEAT: Pickup auto-eat if hunger >= 15
    // ============================================================

    #[test]
    fn t2_inv_fruit_autoeat_when_hungry() {
        let mut state = GameState::new();
        state.hunger = 50;
        let before_hunger = state.hunger;
        let ate = state.pickup_fruit();
        assert!(ate);
        assert_eq!(state.hunger, before_hunger - 30);
        assert_eq!(state.stuff()[24], 0);
    }

    #[test]
    fn t2_inv_fruit_store_when_not_hungry() {
        let mut state = GameState::new();
        state.hunger = 10;
        let before_hunger = state.hunger;
        let ate = state.pickup_fruit();
        assert!(!ate);
        assert_eq!(state.hunger, before_hunger);
        assert_eq!(state.stuff()[24], 1);
    }

    #[test]
    fn t2_inv_fruit_autoeat_threshold_exactly_15() {
        let mut state = GameState::new();
        state.hunger = 15;
        let ate = state.pickup_fruit();
        assert!(ate);
        assert_eq!(state.hunger, 0);
    }

    #[test]
    fn t2_inv_fruit_autoeat_threshold_14_stores() {
        let mut state = GameState::new();
        state.hunger = 14;
        let ate = state.pickup_fruit();
        assert!(!ate);
        assert_eq!(state.stuff()[24], 1);
    }

    // ============================================================
    // T2-INV-AUTOEAT-SAFE: Safe-zone auto-eat
    // ============================================================

    #[test]
    fn t2_inv_autoeat_safe_zone_when_hungry() {
        let mut state = GameState::new();
        state.safe_flag = true;
        state.hunger = 50;
        state.stuff_mut()[24] = 2;
        state.daynight = 0;
        let ate = state.try_safe_autoeat();
        assert!(ate);
        assert_eq!(state.hunger, 20);
        assert_eq!(state.stuff()[24], 1);
    }

    #[test]
    fn t2_inv_autoeat_safe_not_in_safe_zone() {
        let mut state = GameState::new();
        state.safe_flag = false;
        state.hunger = 50;
        state.stuff_mut()[24] = 1;
        state.daynight = 0;
        let ate = state.try_safe_autoeat();
        assert!(!ate);
        assert_eq!(state.hunger, 50);
        assert_eq!(state.stuff()[24], 1);
    }

    #[test]
    fn t2_inv_autoeat_safe_wrong_daynight_phase() {
        let mut state = GameState::new();
        state.safe_flag = true;
        state.hunger = 50;
        state.stuff_mut()[24] = 1;
        state.daynight = 64;
        let ate = state.try_safe_autoeat();
        assert!(!ate);
    }

    #[test]
    fn t2_inv_autoeat_safe_hunger_exactly_30_no_eat() {
        let mut state = GameState::new();
        state.safe_flag = true;
        state.hunger = 30;
        state.stuff_mut()[24] = 1;
        state.daynight = 0;
        let ate = state.try_safe_autoeat();
        assert!(!ate);
    }

    #[test]
    fn t2_inv_autoeat_safe_hunger_31_eats() {
        let mut state = GameState::new();
        state.safe_flag = true;
        state.hunger = 31;
        state.stuff_mut()[24] = 1;
        state.daynight = 0;
        let ate = state.try_safe_autoeat();
        assert!(ate);
        assert_eq!(state.hunger, 1);
    }

    #[test]
    fn t2_inv_autoeat_safe_no_fruit() {
        let mut state = GameState::new();
        state.safe_flag = true;
        state.hunger = 50;
        state.stuff_mut()[24] = 0;
        state.daynight = 0;
        let ate = state.try_safe_autoeat();
        assert!(!ate);
    }

    #[test]
    fn t2_inv_autoeat_safe_daynight_128_triggers() {
        let mut state = GameState::new();
        state.safe_flag = true;
        state.hunger = 50;
        state.stuff_mut()[24] = 1;
        state.daynight = 128;
        let ate = state.try_safe_autoeat();
        assert!(ate);
    }
}
