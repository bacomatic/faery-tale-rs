---
title: "Plan M — Shop System (ECS)"
plan: M
status: draft
depends_on: [I, J]
touches: [src/game/shop.rs, src/game/ecs/scene.rs]
---

# ECS Migration Plan M: Shop System (ECS)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port the shop system from legacy `src/game/shop.rs` to ECS. Implement `buy_slot_ecs()` that reads hero wealth from `HeroStats`, deducts gold, delivers items to `Inventory.stuff`, and emits appropriate events. Wire the Buy menu action into `EcsScene::dispatch_menu_action()` with a proximity guard to ensure the player is near a bartender NPC.

**Architecture:** `buy_slot_ecs()` lives in the existing `src/game/shop.rs` file alongside the legacy `buy_slot()` implementation. It operates directly on `hecs::World` and `Resources` rather than `GameState`. `EcsScene::dispatch_menu_action()` calls it from the `MenuAction::BuyItem(hit)` arm, guarded by a new `has_shopkeeper_nearby()` helper that queries `SetFig` entities for ob_id == 8. The `BuyResult` / `BuyOutcome` types are already defined in `shop.rs` and are reused unchanged.

**Prerequisites:** Plans I (Buy menu dispatch), J (Inventory component). Plans A-D complete.

**Tech Stack:** Rust 2021, `hecs = "0.11"`, SDL3.

---

## File map

| File | Change |
|------|--------|
| `src/game/shop.rs` | Add `buy_slot_ecs()` + unit tests |
| `src/game/ecs/scene.rs` | Add `has_shopkeeper_nearby()`, wire `BuyItem` dispatch |

---

## Background: shop item table

The legacy shop is driven by a static table mapping menu slot index to inventory index and gold price. This same table is used verbatim by `buy_slot_ecs()`.

```rust
/// Maps (inventory_index, price_in_gold) for each shop menu slot 0–6.
pub const JTRANS: [(usize, i16); 7] = [
    (0,  3),  // slot 0 — Food        (sentinel: eat(50) + event(22), does NOT increment stuff[0])
    (8, 10),  // slot 1 — Arrows      (sentinel: stuff[8] += 10, not 1)
    (11, 15), // slot 2 — Glass Vial
    (1,  30), // slot 3 — Mace
    (2,  45), // slot 4 — Sword
    (3,  75), // slot 5 — Bow
    (13, 20), // slot 6 — Bird Totem
];
```

The menu `hit` value passed by `MenuAction::BuyItem(hit)` is 0–6, matching the JTRANS slot index directly.

## Background: BuyResult / BuyOutcome

`BuyResult` is already defined in `shop.rs`. The three variants drive both message selection and item delivery:

| Variant | Meaning |
|---------|---------|
| `BuyResult::Silent` | Slot index out of JTRANS range — no message |
| `BuyResult::NotEnough` | `wealth <= price` (strict — exact money fails) — show "not enough money" scroll |
| `BuyResult::Bought(BuyOutcome)` | Purchase succeeded; outcome selects item delivery |

`BuyOutcome` variants:

| Variant | Delivery |
|---------|----------|
| `BuyOutcome::Food` | No increment to `stuff[0]`; applies `eat(50)` (hunger −50, clamped at 0) per `reference/logic/shops.md` / fmain.c:3433; caller pushes food narrative. |
| `BuyOutcome::Arrows` | `stuff[8] = stuff[8].saturating_add(10)` |
| `BuyOutcome::Item { inv_idx }` | `stuff[inv_idx] = stuff[inv_idx].saturating_add(1)` |

## Background: proximity guard

The legacy proximity guard queries for the bartender SetFig (ob_id == 8) within 32 pixels. The ECS equivalent queries all entities with both a `Position` component and a `WorldObj` component whose `ob_id` field equals 8, then checks Chebyshev distance from the hero.

```
Chebyshev(hero, npc) = max(|hero.x - npc.x|, |hero.y - npc.y|) < 32
```

If no bartender is within range, `dispatch_menu_action()` returns silently — no error message, no item delivery.

---

## Task 1: Add `buy_slot_ecs()` to `shop.rs`

**Files:**
- Modify: `src/game/shop.rs`

- [ ] **Step 1: Add ECS imports**

  At the top of `src/game/shop.rs`, add alongside existing imports:

  ```rust
  use hecs::World;
  use crate::game::ecs::resources::Resources;
  use crate::game::ecs::components::{HeroStats, Inventory};
  ```

- [ ] **Step 2: Implement `buy_slot_ecs()`**

  Add after the existing `buy_slot()` function:

  ```rust
  /// ECS-backed shop purchase.
  ///
  /// Validates the slot, checks hero wealth, deducts gold, and delivers the item
  /// into `Inventory.stuff`. Uses the same `JTRANS` table and `BuyResult`/`BuyOutcome`
  /// types as the legacy `buy_slot()` implementation.
  ///
  /// Wealth check is strict: `wealth <= price` fails (exact money is not enough).
  pub fn buy_slot_ecs(slot: usize, world: &mut World, res: &mut Resources) -> BuyResult {
      // 1. Validate slot is in JTRANS range.
      if slot >= JTRANS.len() {
          return BuyResult::Silent;
      }

      let (inv_idx, price) = JTRANS[slot];

      // 2. Check wealth — strict: wealth must be *strictly greater than* price.
      {
          let stats = world
              .get::<HeroStats>(res.hero_entity)
              .expect("hero entity must have HeroStats");
          if stats.wealth <= price {
              return BuyResult::NotEnough;
          }
      }

      // 3. Deduct gold from HeroStats.
      {
          let mut stats = world
              .get_mut::<HeroStats>(res.hero_entity)
              .expect("hero entity must have HeroStats");
          stats.wealth -= price;
      }

      // 4. Deliver item.
      match inv_idx {
          // Food: sentinel slot — no inventory grant. Applies eat(50): hunger
          // is reduced by 50, clamped at 0 (fmain.c:3433 → eat(50), fmain2.c:1706).
          0 => {
              let mut stats = world
                  .get_mut::<HeroStats>(res.hero_entity)
                  .expect("hero entity must have HeroStats");
              stats.hunger = (stats.hunger - 50).max(0);
              BuyResult::Bought(BuyOutcome::Food)
          }

          // Arrows: bundle of 10.
          8 => {
              let mut inv = world
                  .get_mut::<Inventory>(res.hero_entity)
                  .expect("hero entity must have Inventory");
              inv.stuff[8] = inv.stuff[8].saturating_add(10);
              BuyResult::Bought(BuyOutcome::Arrows)
          }

          // Generic item: increment by 1.
          _ => {
              let mut inv = world
                  .get_mut::<Inventory>(res.hero_entity)
                  .expect("hero entity must have Inventory");
              inv.stuff[inv_idx] = inv.stuff[inv_idx].saturating_add(1);
              BuyResult::Bought(BuyOutcome::Item { inv_idx })
          }
      }
  }
  ```

- [ ] **Step 3: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

  Expected: no errors in `shop.rs`.

---

## Task 2: Add unit tests in `shop.rs`

**Files:**
- Modify: `src/game/shop.rs`

Five tests cover all `BuyResult` variants and the saturation-safe delivery paths.

- [ ] **Step 1: Add test module**

  ```rust
  #[cfg(test)]
  mod ecs_tests {
      use super::*;
      use hecs::World;
      use crate::game::ecs::resources::Resources;
      use crate::game::ecs::components::{HeroStats, Inventory};

      /// Spawn a minimal hero with the given wealth and return (World, Resources).
      fn setup(wealth: i16) -> (World, Resources) {
          let mut world = World::new();
          let hero = world.spawn((
              HeroStats { wealth, ..HeroStats::default() },
              Inventory::empty(),
          ));
          let res = Resources::new(hero);
          (world, res)
      }

      // --- slot 0: Food ---

      #[test]
      fn food_deducts_wealth_eats_does_not_increment_stuff() {
          let (mut world, mut res) = setup(10);
          // Pre-set hunger so eat(50) has something to reduce.
          world.get_mut::<HeroStats>(res.hero_entity).unwrap().hunger = 80;

          let result = buy_slot_ecs(0, &mut world, &mut res);

          assert!(matches!(result, BuyResult::Bought(BuyOutcome::Food)),
              "expected Bought(Food), got {:?}", result);

          let stats = world.get::<HeroStats>(res.hero_entity).unwrap();
          // wealth was 10, Food costs 3 → 7 remaining
          assert_eq!(stats.wealth, 7);
          // eat(50): hunger 80 → 30
          assert_eq!(stats.hunger, 30, "Food must reduce hunger by 50 (eat(50))");

          // stuff[0] must remain 0 — Food does NOT increment inventory
          let inv = world.get::<Inventory>(res.hero_entity).unwrap();
          assert_eq!(inv.stuff[0], 0, "Food must not increment stuff[0]");
      }

      #[test]
      fn food_hunger_clamps_at_zero() {
          let (mut world, mut res) = setup(10);
          world.get_mut::<HeroStats>(res.hero_entity).unwrap().hunger = 20;

          buy_slot_ecs(0, &mut world, &mut res);

          // eat(50) on hunger 20 clamps at 0 (not negative).
          let stats = world.get::<HeroStats>(res.hero_entity).unwrap();
          assert_eq!(stats.hunger, 0, "hunger must clamp at 0");
      }

      // --- slot 1: Arrows ---

      #[test]
      fn arrows_add_ten_to_stuff_8() {
          let (mut world, mut res) = setup(50);

          // Pre-load 5 arrows to verify saturation-safe addition
          {
              let mut inv = world.get_mut::<Inventory>(res.hero_entity).unwrap();
              inv.stuff[8] = 5;
          }

          let result = buy_slot_ecs(1, &mut world, &mut res);

          assert!(matches!(result, BuyResult::Bought(BuyOutcome::Arrows)),
              "expected Bought(Arrows), got {:?}", result);

          let inv = world.get::<Inventory>(res.hero_entity).unwrap();
          assert_eq!(inv.stuff[8], 15, "arrows must increase by 10");

          let stats = world.get::<HeroStats>(res.hero_entity).unwrap();
          assert_eq!(stats.wealth, 40, "price 10 deducted from 50");
      }

      // --- insufficient wealth ---

      #[test]
      fn exact_wealth_returns_not_enough() {
          // Wealth exactly equals price — strict check means this fails.
          let sword_price = JTRANS[4].1; // Sword costs 45
          let (mut world, mut res) = setup(sword_price);

          let result = buy_slot_ecs(4, &mut world, &mut res);

          assert!(matches!(result, BuyResult::NotEnough),
              "expected NotEnough when wealth == price, got {:?}", result);

          // wealth must be unchanged
          let stats = world.get::<HeroStats>(res.hero_entity).unwrap();
          assert_eq!(stats.wealth, sword_price, "wealth must not change on NotEnough");
      }

      // --- generic item ---

      #[test]
      fn generic_item_increments_inv_idx() {
          let (mut world, mut res) = setup(100);

          // Slot 2 = Glass Vial, inv_idx 11
          let result = buy_slot_ecs(2, &mut world, &mut res);

          assert!(matches!(result, BuyResult::Bought(BuyOutcome::Item { inv_idx: 11 })),
              "expected Bought(Item {{ inv_idx: 11 }}), got {:?}", result);

          let inv = world.get::<Inventory>(res.hero_entity).unwrap();
          assert_eq!(inv.stuff[11], 1);

          let stats = world.get::<HeroStats>(res.hero_entity).unwrap();
          assert_eq!(stats.wealth, 85, "price 15 deducted from 100");
      }

      // --- out of range ---

      #[test]
      fn out_of_range_slot_returns_silent() {
          let (mut world, mut res) = setup(999);

          let result = buy_slot_ecs(7, &mut world, &mut res);

          assert!(matches!(result, BuyResult::Silent),
              "expected Silent for slot >= JTRANS.len(), got {:?}", result);

          // wealth must be untouched
          let stats = world.get::<HeroStats>(res.hero_entity).unwrap();
          assert_eq!(stats.wealth, 999);
      }
  }
  ```

- [ ] **Step 2: Run tests**

  ```bash
  cargo test shop::ecs_tests 2>&1 | grep -E "^test result|FAILED"
  ```

  Expected: `test result: ok. 6 passed`.

- [ ] **Step 3: Commit**

  ```bash
  git add src/game/shop.rs
  git commit -m "feat(ecs): add buy_slot_ecs() to shop.rs with unit tests"
  ```

---

## Task 3: Add `has_shopkeeper_nearby()` to `EcsScene`

**Files:**
- Modify: `src/game/ecs/scene.rs`

- [ ] **Step 1: Add required imports**

  In `src/game/ecs/scene.rs`, add to the existing import block:

  ```rust
  use crate::game::ecs::components::{Position, WorldObj, SetFig};
  ```

- [ ] **Step 2: Implement `has_shopkeeper_nearby()`**

  Add as a private method on `EcsScene`:

  ```rust
  impl EcsScene {
      /// Returns true if a bartender NPC (SetFig with ob_id == 8) is within
      /// Chebyshev distance 32 of the hero.
      ///
      /// Mirrors the legacy `has_shopkeeper_nearby(hero_x, hero_y)` guard from
      /// `shop.rs`. Uses Chebyshev distance to match the original pixel-distance
      /// check: `max(|dx|, |dy|) < 32`.
      fn has_shopkeeper_nearby(&self) -> bool {
          // Get hero position first.
          let hero_pos = match self.world.get::<Position>(self.res.hero_entity) {
              Ok(p) => (p.x, p.y),
              Err(_) => return false,
          };

          // Query all SetFig entities that also have a Position and WorldObj.
          self.world
              .query::<(&Position, &WorldObj)>()
              .with::<&SetFig>()
              .iter()
              .any(|(_, (pos, obj))| {
                  obj.ob_id == 8
                      && (pos.x - hero_pos.0).abs().max((pos.y - hero_pos.1).abs()) < 32.0
              })
      }
  }
  ```

- [ ] **Step 3: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

  Expected: no errors. If `SetFig` or `WorldObj` are not yet component types, stub them as empty marker structs with `pub ob_id: u8` on `WorldObj` and derive the minimum required traits. Do not add functionality beyond what is needed here.

---

## Task 4: Wire `BuyItem` in `dispatch_menu_action()`

**Files:**
- Modify: `src/game/ecs/scene.rs`

- [ ] **Step 1: Add shop imports**

  ```rust
  use crate::game::shop::{buy_slot_ecs, BuyResult, BuyOutcome};
  ```

- [ ] **Step 2: Add `BuyItem` arm**

  In `EcsScene::dispatch_menu_action()`, add the `BuyItem` match arm. The `hit` value is 0–6 and maps directly to a JTRANS slot:

  ```rust
  MenuAction::BuyItem(hit) => {
      // Guard: player must be adjacent to a bartender.
      if !self.has_shopkeeper_nearby() {
          return;
      }

      let name = self.res.brother.active_name.clone();
      let slot = hit as usize;
      let text = match buy_slot_ecs(slot, &mut self.world, &mut self.res) {
          // Slot out of range — silent (fmain.c `hit > 11` returns, no message).
          BuyResult::Silent => None,
          // Hardcoded denial string — sanctioned in dialog_system.md
          // ("Hardcoded scroll messages", fmain.c:3440). NOT a speak()/event id.
          BuyResult::NotEnough => Some("Not enough money!".to_string()),
          // Food → event_msg[22]; Arrows → event_msg[23]
          // (faery.toml [narr].event_msg, via events::event_msg; `%` → hero name).
          BuyResult::Bought(BuyOutcome::Food) =>
              Some(crate::game::events::event_msg(&self.res.narr, 22, &name)),
          BuyResult::Bought(BuyOutcome::Arrows) =>
              Some(crate::game::events::event_msg(&self.res.narr, 23, &name)),
          // Generic item → hardcoded "% bought a {item}." (dialog_system.md,
          // fmain.c:3436-3437). Item name from inv_list[].name.
          BuyResult::Bought(BuyOutcome::Item { inv_idx }) => Some(format!(
              "{name} bought a {}.",
              crate::game::world_objects::stuff_index_name(inv_idx)
          )),
      };
      if let Some(text) = text {
          self.res.events.message.push(crate::game::ecs::events::MessageEvent { text });
      }
  }
  ```

  > **Message sourcing (authoritative — `reference/logic/shops.md`, `dialog_system.md`):**
  > shop messages are NOT `speak()` speeches. Food/Arrows use `event_msg[22]`/`[23]`
  > from `faery.toml [narr].event_msg`; the "Not enough money!" denial and the
  > `"% bought a {item}."` purchase line are hardcoded literals explicitly
  > enumerated in `reference/logic/dialog_system.md` (lines 301–302), so they are
  > permitted. Do not invent any other prose.

- [ ] **Step 3: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

  Expected: no errors.

- [ ] **Step 4: Commit**

  ```bash
  git add src/game/ecs/scene.rs
  git commit -m "feat(ecs): wire BuyItem dispatch in EcsScene with shopkeeper proximity guard"
  ```

---

## Task 5: Integration testing

- [ ] **Step 1: Full build**

  ```bash
  cargo build 2>&1 | grep "^error"
  ```

  Expected: no errors.

- [ ] **Step 2: Run all shop tests**

  ```bash
  cargo test shop 2>&1 | grep -E "^test result|FAILED"
  ```

  Expected: all 6 tests pass.

- [ ] **Step 3: Manual smoke test**

  With the game running under `--ecs`:

  1. Start with Julian (default wealth: check `game_library.rs` for starting value).
  2. Walk to the inn in region 0 and approach the bartender (ob_id 8).
  3. Open the BUY menu (Plan I). Select Food (slot 0). Verify scroll message appears.
  4. Verify hero wealth decreased by 3.
  5. Walk away from the bartender. Open BUY menu again. Select any item. Verify no
     purchase occurs and no error message appears (silent guard).
  6. Drain wealth below the cheapest item price (Food = 3). Attempt purchase.
     Verify "not enough money" scroll appears and wealth is unchanged.

- [ ] **Step 4: Final commit**

  ```bash
  git add -A
  git commit -m "feat(ecs): shop system fully wired — buy_slot_ecs + proximity guard + BuyItem dispatch"
  ```

---

## Completion check

```bash
cargo build 2>&1 | grep "^error"
cargo test shop 2>&1 | grep -E "^test result|FAILED"
```

Both succeed. `buy_slot_ecs()` is implemented and tested. `BuyItem` dispatches through the proximity guard in `EcsScene`.

---

## Spec references

- `docs/spec/ui-menus.md` §25.5 — BUY mode dispatch table
- `reference/logic/shops.md` (research branch) — original `buy_dispatch` implementation, JTRANS table, wealth check, and message behavior (event_msg[22]/[23], hardcoded strings)
- `reference/logic/dialog_system.md` (research branch) — sanctioned hardcoded scroll messages ("Not enough money!", "% bought a {item}.")

## Test plan

- `food_deducts_wealth_eats_does_not_increment_stuff` — wealth −3, hunger −50 (eat), `stuff[0]` unchanged
- `food_hunger_clamps_at_zero` — eat(50) on low hunger clamps at 0
- `arrows_add_ten_to_stuff_8` — `stuff[8]` increases by 10, wealth decrements by 10
- `exact_wealth_returns_not_enough` — `wealth == price` returns `NotEnough`, wealth unchanged
- `generic_item_increments_inv_idx` — `stuff[inv_idx]` increments by 1, wealth decrements by price
- `out_of_range_slot_returns_silent` — slot ≥ 7 returns `Silent`, wealth unchanged

## Files touched

| File | Change |
|------|--------|
| `src/game/shop.rs` | Add `buy_slot_ecs()` + 6 unit tests |
| `src/game/ecs/scene.rs` | Add `has_shopkeeper_nearby()`, wire `BuyItem` arm in `dispatch_menu_action()` |
