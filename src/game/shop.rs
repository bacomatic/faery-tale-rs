//! Shop system: shopkeeper NPC interaction and item purchasing.
//! Ports jtrans[] cost table and BUY action from fmain.c.

use crate::game::game_state::GameState;

/// Item cost table (jtrans[] from original). Index = item slot, value = gold cost.
/// 0 = not for sale.
pub const ITEM_COSTS: &[i32] = &[
    5,   // 0: food
    10,  // 1: torch
    15,  // 2: rope
    20,  // 3: key
    25,  // 4: potion
    30,  // 5: armor (leather)
    50,  // 6: armor (chain)
    100, // 7: armor (plate)
    30,  // 8: weapon (dagger)
    50,  // 9: weapon (short sword)
    80,  // 10: weapon (long sword)
    100, // 11: weapon (axe)
    0,   // 12-34: not for sale (pad with zeros)
];

/// Purchase an item from a shopkeeper.
/// Returns Ok(gold_spent) or Err("reason").
pub fn buy_item(state: &mut GameState, item_idx: usize) -> Result<i32, &'static str> {
    if item_idx >= 35 { return Err("No such item"); }
    let cost = ITEM_COSTS.get(item_idx).copied().unwrap_or(0);
    if cost == 0 { return Err("Not for sale"); }
    if state.gold < cost { return Err("Not enough gold"); }
    state.gold -= cost;
    state.stuff_mut()[item_idx] += 1;
    Ok(cost)
}

/// Find a shopkeeper NPC near the hero position.
/// Returns true if a shopkeeper is within 32px.
pub fn has_shopkeeper_nearby(npcs: &[crate::game::npc::Npc], hero_x: i16, hero_y: i16) -> bool {
    use crate::game::npc::RACE_SHOPKEEPER;
    npcs.iter().any(|n| {
        n.active && n.race == RACE_SHOPKEEPER
            && (n.x - hero_x).abs() < 32
            && (n.y - hero_y).abs() < 32
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::game_state::GameState;

    #[test]
    fn test_buy_item_success() {
        let mut state = GameState::new();
        state.gold = 100;
        let result = buy_item(&mut state, 0); // food costs 5
        assert!(result.is_ok());
        assert_eq!(state.gold, 95);
        assert_eq!(state.stuff()[0], 1);
    }

    #[test]
    fn test_buy_item_no_gold() {
        let mut state = GameState::new();
        state.gold = 0;
        let result = buy_item(&mut state, 4); // potion costs 25
        assert!(result.is_err());
    }

    #[test]
    fn test_buy_not_for_sale() {
        let mut state = GameState::new();
        state.gold = 999;
        let result = buy_item(&mut state, 12); // not for sale
        assert!(result.is_err());
    }
}
