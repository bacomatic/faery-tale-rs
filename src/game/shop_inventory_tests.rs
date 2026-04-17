//! TDD tests for T2-SHOP-COSTS, T2-INV-LASSO-INDEX, T2-INV-FRUIT-AUTOEAT, T2-INV-AUTOEAT-SAFE

#[cfg(test)]
mod tests {
    use crate::game::game_state::GameState;
    use crate::game::shop::{buy_item, ITEM_COSTS};

    // ============================================================
    // T2-SHOP-COSTS: Shop prices per SPEC §25.5 BUY
    // ============================================================

    #[test]
    fn t2_shop_costs_food_is_3() {
        // SPEC §25.5 BUY: Food costs 3
        assert_eq!(ITEM_COSTS[0], 3, "Food must cost 3 gold");
    }

    #[test]
    fn t2_shop_costs_arrow_is_10() {
        // SPEC §25.5 BUY: Arrow costs 10 for batch of 10
        assert_eq!(ITEM_COSTS[1], 10, "Arrow must cost 10 gold");
    }

    #[test]
    fn t2_shop_costs_vial_is_15() {
        // SPEC §25.5 BUY: Vial costs 15
        assert_eq!(ITEM_COSTS[2], 15, "Vial must cost 15 gold");
    }

    #[test]
    fn t2_shop_costs_mace_is_30() {
        // SPEC §25.5 BUY: Mace costs 30
        assert_eq!(ITEM_COSTS[3], 30, "Mace must cost 30 gold");
    }

    #[test]
    fn t2_shop_costs_sword_is_45() {
        // SPEC §25.5 BUY: Sword costs 45
        assert_eq!(ITEM_COSTS[4], 45, "Sword must cost 45 gold");
    }

    #[test]
    fn t2_shop_costs_bow_is_75() {
        // SPEC §25.5 BUY: Bow costs 75
        assert_eq!(ITEM_COSTS[5], 75, "Bow must cost 75 gold");
    }

    #[test]
    fn t2_shop_costs_totem_is_20() {
        // SPEC §25.5 BUY: Totem costs 20
        assert_eq!(ITEM_COSTS[6], 20, "Totem must cost 20 gold");
    }

    #[test]
    fn t2_shop_buy_food_deducts_3_gold() {
        let mut state = GameState::new();
        state.gold = 100;
        let result = buy_item(&mut state, 0);
        assert!(result.is_ok());
        assert_eq!(state.gold, 97, "Buying food should deduct 3 gold");
    }

    #[test]
    fn t2_shop_buy_arrow_deducts_10_gold() {
        let mut state = GameState::new();
        state.gold = 100;
        let result = buy_item(&mut state, 1);
        assert!(result.is_ok());
        assert_eq!(state.gold, 90, "Buying arrow should deduct 10 gold");
    }

    #[test]
    fn t2_shop_buy_bow_deducts_75_gold() {
        let mut state = GameState::new();
        state.gold = 100;
        let result = buy_item(&mut state, 5);
        assert!(result.is_ok());
        assert_eq!(state.gold, 25, "Buying bow should deduct 75 gold");
    }

    // ============================================================
    // T2-INV-LASSO-INDEX: Golden Lasso = stuff[5], not stuff[16]
    // ============================================================

    #[test]
    fn t2_inv_lasso_index_is_5() {
        // SPEC §14.3: Golden Lasso is at stuff[5]
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
        // SPEC §14.1: stuff[16] is the Gold Key, not the lasso
        let mut state = GameState::new();
        state.stuff_mut()[16] = 1;
        assert!(!state.has_lasso(), "stuff[16] should not trigger has_lasso()");
    }

    // ============================================================
    // T2-INV-FRUIT-AUTOEAT: Pickup auto-eat if hunger >= 15
    // ============================================================

    #[test]
    fn t2_inv_fruit_autoeat_when_hungry() {
        // SPEC §14.5: On fruit pickup, if hunger >= 15, eat immediately (eat(30))
        let mut state = GameState::new();
        state.hunger = 50;
        let before_hunger = state.hunger;
        
        // Pickup fruit (item 24) — should auto-eat via eat_amount(30)
        let ate = state.pickup_fruit();
        
        assert!(ate, "fruit should be auto-eaten when hunger >= 15");
        assert_eq!(state.hunger, before_hunger - 30, "hunger should reduce by 30");
        assert_eq!(state.stuff()[24], 0, "fruit should not be stored");
    }

    #[test]
    fn t2_inv_fruit_store_when_not_hungry() {
        // SPEC §14.5: When hunger < 15, fruit is stored instead
        let mut state = GameState::new();
        state.hunger = 10;
        let before_hunger = state.hunger;
        
        let ate = state.pickup_fruit();
        
        assert!(!ate, "fruit should not be auto-eaten when hunger < 15");
        assert_eq!(state.hunger, before_hunger, "hunger unchanged");
        assert_eq!(state.stuff()[24], 1, "fruit should be stored in inventory");
    }

    #[test]
    fn t2_inv_fruit_autoeat_threshold_exactly_15() {
        let mut state = GameState::new();
        state.hunger = 15;
        
        let ate = state.pickup_fruit();
        
        assert!(ate, "hunger >= 15 should trigger auto-eat");
        assert_eq!(state.hunger, 0, "hunger should be reduced (15 - 30 = 0, clamped)");
    }

    #[test]
    fn t2_inv_fruit_autoeat_threshold_14_stores() {
        let mut state = GameState::new();
        state.hunger = 14;
        
        let ate = state.pickup_fruit();
        
        assert!(!ate, "hunger < 15 should store fruit");
        assert_eq!(state.stuff()[24], 1);
    }

    // ============================================================
    // T2-INV-AUTOEAT-SAFE: Safe-zone auto-eat
    // ============================================================

    #[test]
    fn t2_inv_autoeat_safe_zone_when_hungry() {
        // SPEC §18.2: In safe zone, when (daynight & 127) == 0, if hunger > 30
        // and stuff[24] > 0, auto-eat fruit
        let mut state = GameState::new();
        state.safe_flag = true;
        state.hunger = 50;
        state.stuff_mut()[24] = 2;
        state.daynight = 0; // (0 & 127) == 0
        
        let ate = state.try_safe_autoeat();
        
        assert!(ate, "should auto-eat in safe zone when hungry");
        assert_eq!(state.hunger, 20, "hunger should reduce by 30");
        assert_eq!(state.stuff()[24], 1, "fruit count should decrement");
    }

    #[test]
    fn t2_inv_autoeat_safe_not_in_safe_zone() {
        let mut state = GameState::new();
        state.safe_flag = false;
        state.hunger = 50;
        state.stuff_mut()[24] = 1;
        state.daynight = 0;
        
        let ate = state.try_safe_autoeat();
        
        assert!(!ate, "should not auto-eat outside safe zone");
        assert_eq!(state.hunger, 50);
        assert_eq!(state.stuff()[24], 1);
    }

    #[test]
    fn t2_inv_autoeat_safe_wrong_daynight_phase() {
        let mut state = GameState::new();
        state.safe_flag = true;
        state.hunger = 50;
        state.stuff_mut()[24] = 1;
        state.daynight = 64; // (64 & 127) != 0
        
        let ate = state.try_safe_autoeat();
        
        assert!(!ate, "should not auto-eat on wrong daynight phase");
    }

    #[test]
    fn t2_inv_autoeat_safe_hunger_exactly_30_no_eat() {
        // SPEC §18.2: hunger > 30, not >= 30
        let mut state = GameState::new();
        state.safe_flag = true;
        state.hunger = 30;
        state.stuff_mut()[24] = 1;
        state.daynight = 0;
        
        let ate = state.try_safe_autoeat();
        
        assert!(!ate, "hunger must be > 30, not >= 30");
    }

    #[test]
    fn t2_inv_autoeat_safe_hunger_31_eats() {
        let mut state = GameState::new();
        state.safe_flag = true;
        state.hunger = 31;
        state.stuff_mut()[24] = 1;
        state.daynight = 0;
        
        let ate = state.try_safe_autoeat();
        
        assert!(ate, "hunger > 30 should trigger auto-eat");
        assert_eq!(state.hunger, 1);
    }

    #[test]
    fn t2_inv_autoeat_safe_no_fruit() {
        let mut state = GameState::new();
        state.safe_flag = true;
        state.hunger = 50;
        state.stuff_mut()[24] = 0; // no fruit
        state.daynight = 0;
        
        let ate = state.try_safe_autoeat();
        
        assert!(!ate, "cannot auto-eat without fruit");
    }

    #[test]
    fn t2_inv_autoeat_safe_daynight_128_triggers() {
        let mut state = GameState::new();
        state.safe_flag = true;
        state.hunger = 50;
        state.stuff_mut()[24] = 1;
        state.daynight = 128; // (128 & 127) == 0
        
        let ate = state.try_safe_autoeat();
        
        assert!(ate, "daynight & 127 == 0 should trigger");
    }
}
