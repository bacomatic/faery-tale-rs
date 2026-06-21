---
title: "Plan J2 — Item/Body-Search Text + Remove Fabricated Food"
plan: J2
status: draft
depends_on: [J]
touches:
  - src/game/ecs/resources.rs
  - src/game/ecs/scene.rs
  - src/game/ecs/systems/item.rs
---

# ECS Migration Plan J2: Item/Body-Search Text + Remove Fabricated Food

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix the three live fidelity violations that shipped with Plan J: (1) the invented item-pickup string `"Taken."`, (2) the invented body-search strings `"You search the remains."` / `"Searched."`, and (3) the fabricated 6-slot food system (`FOOD_SATIATION` + `BuyItem(n) → eat slot 22+n`) that corrupts quest items (slot 22 = Talisman win condition).

**Architecture:** Item pickup and body-search messages are replaced with the authoritative composed strings enumerated in `reference/logic/dialog_system.md` ("Hardcoded scroll messages"). The `%`/`{name}` token is the active brother's name, sourced from a new `BrotherRoster.active_name` field (avoids threading `GameLibrary` into the item system, and makes Plan M's existing `res.brother.active_name` reference valid). The fabricated food/eat code is deleted; `MenuAction::BuyItem` reverts to a no-op stub that Plan M will own. The body-search weapon grant is corrected from `stuff[weapon]` to `stuff[weapon-1]` (the dead actor's `weapon` is a 1-based code; the inventory slot is `code-1`, per fmain.c:3256-3257).

**Prerequisites:** Plan J shipped (the code being fixed). Plans A–D.

**Tech Stack:** Rust 2021, `hecs = "0.11"`.

---

## Authoritative message sources (`reference/logic/dialog_system.md`)

These are sanctioned hardcoded scroll literals (composed with `%` = brother name); do NOT invent any other prose.

| Situation | Composed text |
|-----------|---------------|
| Single item pickup (fmain.c:3191-3193) | `"{name} found a {item_name}."` |
| Dead brother bones (fmain.c:3173) | `"{name} found his brother's bones."` |
| Body search open (fmain.c:3251) | `"{name} searched the body and found"` |
| …weapon found, `i>0` (fmain.c:3255-3257) | `" a {weapon_name}"`, grant `stuff[i-1]++`, name `inv_list[i-1]` |
| …bow bonus, `i==4` (fmain.c:3260-3262) | `" and {N} Arrows"`, `N = rand8()+2`, `stuff[8] += N` |
| …nothing (fmain.c:3282) | `" nothing"` |
| …close (fmain.c:3283) | `"."` |

`{item_name}` / `{weapon_name}` come from `crate::game::world_objects::stuff_index_name(slot)` (a port of `inv_list[].name`). `rand8()+2` uses `crate::game::combat::bitrand(7) as u8 + 2` (`bitrand(mask) = rand & mask`, so `bitrand(7)` is 0–7).

---

## File map

| File | Change |
|------|--------|
| `src/game/ecs/resources.rs` | Add `BrotherRoster.active_name: String` |
| `src/game/ecs/scene.rs` | Set `active_name` at spawn + succession; delete `FOOD_SATIATION`; stub `BuyItem` |
| `src/game/ecs/systems/item.rs` | Fix `handle_take` + `handle_search` messages; fix weapon grant slot |

---

## Task 1: Add `active_name` to `BrotherRoster`

**Files:**
- Modify: `src/game/ecs/resources.rs`
- Modify: `src/game/ecs/scene.rs`

- [ ] **Step 1: Add the field**

In `src/game/ecs/resources.rs`, add to `struct BrotherRoster` (after `brother`):

```rust
    /// Display name of the active brother (for "%"-substituted scroll text).
    pub active_name: String,
```

Add `active_name: String::new(),` to wherever `BrotherRoster` is constructed in `Resources::new()` (the `brother:` initializer). If `BrotherRoster` derives `Default`, the `String::new()` default is automatic; otherwise add it explicitly.

- [ ] **Step 2: Set `active_name` at initial hero spawn**

In `EcsScene::new()` (`scene.rs`), right after the hero stats are built from `game_lib.get_brother(0)` and `res` is created, add:

```rust
res.brother.active_name = game_lib
    .get_brother(0)
    .map(|b| b.name.clone())
    .unwrap_or_else(|| "Hero".to_string());
```

- [ ] **Step 3: Set `active_name` on succession**

In `EcsScene::drain_brother_deaths()`, immediately after the existing line
`self.res.brother.active_brother = successor as usize;`, add:

```rust
self.res.brother.active_name = game_lib
    .get_brother(successor as usize)
    .map(|b| b.name.clone())
    .unwrap_or_else(|| "Hero".to_string());
```

- [ ] **Step 4: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors.

---

## Task 2: Remove the fabricated food system

**Files:**
- Modify: `src/game/ecs/scene.rs`

The original has no 6-slot food/satiation system. Food is only Fruit (slot 24), consumed via `eat(30)`/auto-eat/`eat(50)` by the survival/shop paths — never via a menu "eat slot 22+n" action. The current code decrements `stuff[22..=27]` (Talisman, Rose, Fruit, Gold Statue, Book, Herb), corrupting quest state.

- [ ] **Step 1: Delete the `FOOD_SATIATION` constant**

Remove this line (near the top of `scene.rs`):

```rust
/// Satiation amount per food slot (stuff[22..=27]). Source: docs/spec/survival.md §18.3.
const FOOD_SATIATION: [i16; 6] = [25, 35, 45, 55, 65, 80];
```

- [ ] **Step 2: Replace the `BuyItem` arm with a no-op stub**

In `dispatch_menu_action()`, replace the entire `MenuAction::BuyItem(n) => { ... }` arm (the block that reads `FOOD_SATIATION`, computes `food_slot = 22 + n`, and decrements `stuff[food_slot]`) with:

```rust
MenuAction::BuyItem(_) => {
    // Shop purchase is implemented in Plan M (buy_slot_ecs + bartender
    // proximity guard). The previous "eat food in slot 22+n" mechanic was
    // fabricated and corrupted quest items (slot 22 = Talisman); removed.
}
```

- [ ] **Step 3: Verify compile (watch for now-unused imports)**

```bash
cargo check 2>&1 | grep -E "^error|^warning"
```

Expected: no errors and no new warnings. If the removed arm leaves an unused `use crate::game::ecs::components::{HeroStats, Inventory};` inside the arm, it is removed with the block; if any module-level import becomes unused, remove it.

---

## Task 3: Fix the item-pickup message in `handle_take()`

**Files:**
- Modify: `src/game/ecs/systems/item.rs`

- [ ] **Step 1: Replace the `"Taken."` message**

In `handle_take()`, replace:

```rust
    res.events.sfx.push(SfxEvent { sfx_id: 5 });
    res.events.message.push(MessageEvent {
        text: "Taken.".to_string(),
    });
```

with:

```rust
    res.events.sfx.push(SfxEvent { sfx_id: 5 });
    // Authoritative single-item pickup line (dialog_system.md, fmain.c:3191-3193).
    let name = res.brother.active_name.clone();
    let item_name = crate::game::world_objects::stuff_index_name(ob_id as usize);
    res.events.message.push(MessageEvent {
        text: format!("{name} found a {item_name}."),
    });
```

Leave the existing `stuff[ob_id] += 1` increment, the quest-flag updates (slots 22/7/5), and the SFX unchanged.

> **Note (out of scope):** per-type pickup variants — gold ("found 50 gold pieces"), containers (chest/urn/sacks), scrap-of-paper (`event 17`), and Fruit's eat-on-pickup-when-hungry behavior (`eat(30)` / `event 36/37`) — are not modeled here. They were not handled before this fix either (the stub only said "Taken."); generic single-item pickup is the faithful baseline. Track them as a follow-up.

- [ ] **Step 2: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors.

---

## Task 4: Fix the body-search messages and weapon grant in `handle_search()`

**Files:**
- Modify: `src/game/ecs/systems/item.rs`

- [ ] **Step 1: Fix the Bones-search message**

In the `is_bones` branch, replace:

```rust
            res.events.message.push(MessageEvent {
                text: "You search the remains.".to_string(),
            });
```

with:

```rust
            let name = res.brother.active_name.clone();
            res.events.message.push(MessageEvent {
                text: format!("{name} found his brother's bones."),
            });
```

(The whole-inventory merge above it is correct and stays.)

- [ ] **Step 2: Rewrite the dead-enemy branch (weapon slot + composed message)**

Replace the entire `if let Some((weapon, gold)) = loot_data { ... }` block with:

```rust
    if let Some((weapon, gold)) = loot_data {
        // Mark looted first to prevent double-search.
        if let Ok(mut loot) = world.get::<&mut Loot>(entity) {
            loot.looted = true;
        }

        let name = res.brother.active_name.clone();
        let mut msg = format!("{name} searched the body and found");

        if weapon > 0 {
            // `weapon` is a 1-based code (1=Dirk..4=Bow); inventory slot is code-1
            // (fmain.c:3256-3257). The previous code used stuff[weapon] (off by one).
            let slot = (weapon - 1) as usize;
            let weapon_name = crate::game::world_objects::stuff_index_name(slot);
            msg.push_str(&format!(" a {weapon_name}"));
            if let Ok(mut inv) = world.get::<&mut Inventory>(res.hero_entity) {
                if slot < inv.stuff.len() {
                    inv.stuff[slot] = inv.stuff[slot].saturating_add(1);
                }
            }
            // Bow (code 4): also grant a random arrow bundle (rand8()+2 = 2..9).
            if weapon == 4 {
                let arrows = crate::game::combat::bitrand(7) as u8 + 2;
                if let Ok(mut inv) = world.get::<&mut Inventory>(res.hero_entity) {
                    inv.stuff[8] = inv.stuff[8].saturating_add(arrows);
                }
                msg.push_str(&format!(" and {arrows} Arrows"));
            }
        } else {
            msg.push_str(" nothing");
        }
        msg.push('.');

        // Gold is transferred silently (the original body-search line announces
        // weapon/treasure, not gold).
        if gold > 0 {
            if let Ok(mut stats) = world.get::<&mut HeroStats>(res.hero_entity) {
                stats.wealth = stats.wealth.saturating_add(gold);
            }
        }

        res.events.sfx.push(SfxEvent { sfx_id: 5 });
        res.events.message.push(MessageEvent { text: msg });
    }
```

> **Note (out of scope):** the original `search_body` also rolls a random treasure item (`treasure_probs[]`) and auto-equips a better weapon (`i > current weapon`). The ECS `Loot` model carries only `weapon + gold`, so those are not ported here; track as a follow-up.

- [ ] **Step 3: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors.

---

## Task 5: Tests

**Files:**
- Modify: `src/game/ecs/systems/item.rs`

Update the existing `#[cfg(test)]` module. Tests construct `World` + `Resources` directly and must set `res.brother.active_name` (no `GameLibrary` needed).

- [ ] **Step 1: Add/adjust the setup to set a brother name**

In each test that calls `handle_take`/`handle_search` (directly or via `run`), set:

```rust
res.brother.active_name = "Julian".to_string();
```

- [ ] **Step 2: Test item-pickup message**

```rust
#[test]
fn take_item_emits_found_message() {
    use crate::game::ecs::components::{Inventory, WorldObj};
    let mut world = World::new();
    let hero = world.spawn((Inventory::empty(),));
    let mut res = Resources::new(hero);
    res.brother.active_name = "Julian".to_string();
    // Glass Vial = slot 11.
    let item = world.spawn((WorldObj { ob_id: 11, ob_stat: 1, region: 0, visible: true, goal: 0 },));
    res.events.item.push(ItemEvent::TakeItem { entity: item });

    run(&mut world, &mut res);

    let msg = res.events.message.last().expect("a message");
    assert_eq!(msg.text, "Julian found a Glass Vial.");
    assert_eq!(world.get::<&Inventory>(hero).unwrap().stuff[11], 1);
}
```

- [ ] **Step 3: Test body search — sword (slot fix) and nothing**

```rust
#[test]
fn search_body_sword_grants_slot_2_and_message() {
    use crate::game::ecs::components::{Enemy, Inventory, Loot, Position, WorldObj};
    let mut world = World::new();
    let hero = world.spawn((Inventory::empty(),
        crate::game::ecs::components::HeroStats::default()));
    let mut res = Resources::new(hero);
    res.brother.active_name = "Julian".to_string();
    // Sword = weapon code 3 → inventory slot 2.
    let enemy = world.spawn((Enemy, Position::new(0.0, 0.0),
        WorldObj { ob_id: 0, ob_stat: 0, region: 0, visible: false, goal: 0 },
        Loot { weapon: 3, gold: 0, looted: false }));
    res.events.item.push(ItemEvent::SearchBody { entity: enemy });

    run(&mut world, &mut res);

    assert_eq!(world.get::<&Inventory>(hero).unwrap().stuff[2], 1, "Sword goes to slot 2");
    assert_eq!(res.events.message.last().unwrap().text,
        "Julian searched the body and found a Sword.");
}

#[test]
fn search_body_no_weapon_says_nothing() {
    use crate::game::ecs::components::{Enemy, Inventory, Loot, Position, WorldObj};
    let mut world = World::new();
    let hero = world.spawn((Inventory::empty(),
        crate::game::ecs::components::HeroStats::default()));
    let mut res = Resources::new(hero);
    res.brother.active_name = "Julian".to_string();
    let enemy = world.spawn((Enemy, Position::new(0.0, 0.0),
        WorldObj { ob_id: 0, ob_stat: 0, region: 0, visible: false, goal: 0 },
        Loot { weapon: 0, gold: 0, looted: false }));
    res.events.item.push(ItemEvent::SearchBody { entity: enemy });

    run(&mut world, &mut res);

    assert_eq!(res.events.message.last().unwrap().text,
        "Julian searched the body and found nothing.");
}
```

- [ ] **Step 4: Test body search — bow grants arrows**

```rust
#[test]
fn search_body_bow_grants_arrows() {
    use crate::game::ecs::components::{Enemy, Inventory, Loot, Position, WorldObj};
    let mut world = World::new();
    let hero = world.spawn((Inventory::empty(),
        crate::game::ecs::components::HeroStats::default()));
    let mut res = Resources::new(hero);
    res.brother.active_name = "Julian".to_string();
    // Bow = weapon code 4 → slot 3; arrows go to slot 8.
    let enemy = world.spawn((Enemy, Position::new(0.0, 0.0),
        WorldObj { ob_id: 0, ob_stat: 0, region: 0, visible: false, goal: 0 },
        Loot { weapon: 4, gold: 0, looted: false }));
    res.events.item.push(ItemEvent::SearchBody { entity: enemy });

    run(&mut world, &mut res);

    assert_eq!(world.get::<&Inventory>(hero).unwrap().stuff[3], 1, "Bow goes to slot 3");
    let arrows = world.get::<&Inventory>(hero).unwrap().stuff[8];
    assert!((2..=9).contains(&arrows), "arrow bundle is 2..9, got {arrows}");
    assert!(res.events.message.last().unwrap().text.contains("Bow and "));
    assert!(res.events.message.last().unwrap().text.ends_with(" Arrows."));
}
```

- [ ] **Step 5: Test Bones search message**

```rust
#[test]
fn search_bones_emits_brothers_bones_message() {
    use crate::game::ecs::components::{Bones, BrotherKind, Inventory, Position, WorldObj};
    let mut world = World::new();
    let hero = world.spawn((Inventory::empty(),));
    let mut res = Resources::new(hero);
    res.brother.active_name = "Phillip".to_string();
    let mut stuff = [0u8; 36];
    stuff[2] = 1; // dead brother carried a Sword
    let bones = world.spawn((Bones, BrotherKind { id: 0 }, Position::new(0.0, 0.0),
        Inventory { stuff },
        WorldObj { ob_id: 0, ob_stat: 1, region: 0, visible: true, goal: 0 }));
    res.events.item.push(ItemEvent::SearchBody { entity: bones });

    run(&mut world, &mut res);

    assert_eq!(world.get::<&Inventory>(hero).unwrap().stuff[2], 1, "brother's Sword merged");
    assert_eq!(res.events.message.last().unwrap().text,
        "Phillip found his brother's bones.");
}
```

- [ ] **Step 6: Test BuyItem no longer touches quest slots**

```rust
// In src/game/ecs/scene.rs tests (where dispatch_menu_action is reachable):
#[test]
fn buyitem_does_not_corrupt_talisman() {
    let mut scene = new_for_test();
    if let Ok(mut inv) = scene.world.get::<&mut crate::game::ecs::components::Inventory>(scene.res.hero_entity) {
        inv.stuff[22] = 1; // Talisman present
    }
    // BuyItem(0) previously mapped to "eat slot 22" and decremented the Talisman.
    // Use whatever harness reaches dispatch_menu_action; assert the slot is unchanged.
    // (If dispatch_menu_action needs SceneResources/game_lib, adapt to the existing
    //  scene-test pattern; the assertion is the point.)
    assert_eq!(scene.world.get::<&crate::game::ecs::components::Inventory>(scene.res.hero_entity).unwrap().stuff[22], 1);
}
```

> If `dispatch_menu_action` cannot be invoked in a unit test without a full `SceneResources`, drop Step 6's call and keep it as a manual verification: open the BUY menu with a Talisman in inventory and confirm slot 22 is not decremented.

- [ ] **Step 7: Run tests**

```bash
cargo test ecs::systems::item 2>&1 | grep -E "^test result|FAILED"
cargo test ecs::scene 2>&1 | grep -E "^test result|FAILED"
```

Expected: all pass.

- [ ] **Step 8: Commit**

```bash
git add src/game/ecs/resources.rs src/game/ecs/scene.rs src/game/ecs/systems/item.rs
git commit -m "fix(ecs): authoritative item/body-search text; remove fabricated food system

- add BrotherRoster.active_name for %-substituted scroll text
- item pickup: \"% found a {item}.\" (was invented \"Taken.\")
- body search: \"% searched the body and found ...\" (was \"Searched.\"/\"You search the remains.\")
- fix body-search weapon grant slot (code-1, was off by one); bow grants arrows
- remove FOOD_SATIATION + BuyItem eat-food hijack that corrupted quest slots 22-27"
```

---

## Completion check

```bash
cargo build 2>&1 | grep -E "^error|^warning"
cargo test ecs::systems::item 2>&1 | grep -E "^test result|FAILED"
```

Both succeed. No invented scroll strings remain in the item system; quest items are no longer corrupted by the BUY menu.

---

## Spec references

- `reference/logic/dialog_system.md` (research branch) — sanctioned hardcoded scroll messages (TAKE + body-search composition)
- `reference/logic/inventory.md` (research branch) — `search_body` (fmain.c:3251-3285): weapon grant `stuff[i-1]`, bow arrow bonus
- `docs/spec/survival.md` §18.3–18.5 — real food model (Fruit slot 24, `eat()`, auto-eat); confirms no multi-slot satiation table
- `docs/spec/inventory-items.md` §14.1 — slot layout (22=Talisman, 23=Rose, 24=Fruit, 25=Gold Statue, …)

## Files touched

| File | Change |
|------|--------|
| `src/game/ecs/resources.rs` | Add `BrotherRoster.active_name` |
| `src/game/ecs/scene.rs` | Set `active_name`; delete `FOOD_SATIATION`; stub `BuyItem` |
| `src/game/ecs/systems/item.rs` | Composed pickup/body-search messages; weapon-slot fix; bow arrows; tests |
