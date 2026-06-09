---
title: "Plan J — Inventory Screen + Item Use"
plan: J
status: draft
depends_on: [A, B, C, D, I]
touches:
  - src/game/ecs/scene.rs
  - src/game/ecs/systems/item.rs
  - src/game/ecs/components.rs
  - src/game/ecs/events.rs
---

# ECS Migration Plan J: Inventory Screen + Item Use

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement full item pickup logic including inventory slot assignment and body search; add the inventory screen rendering path (`viewstatus == 1`); and wire menu-driven item use handlers (`MenuAction::SetWeapon`, `UseItem`, `TakeAction`).

**Architecture:** `ItemSystem` (`src/game/ecs/systems/item.rs`) is extended from its current stub to perform full `Inventory` mutation, loot transfer from `Loot`/`Bones` components, and gold/wealth bookkeeping. `EcsScene::render_hibar()` gains an inventory overlay path. `dispatch_menu_action()` (added in Plan I) is extended with use-item effects.

**Prerequisites:** Plans A–D complete. Plan I complete (menu system, `dispatch_menu_action()` skeleton exists). `Inventory`, `WorldObj`, `Loot`, `HeroStats`, `Bones` components defined in `components.rs`. `ItemEvent` and `MessageEvent` defined in `events.rs`.

**Tech Stack:** Rust 2021, `hecs = "0.11"`, SDL3. No new dependencies.

---

## Context

### `Inventory` component (`src/game/ecs/components.rs`, line 116)

```rust
pub struct Inventory {
    pub stuff: [u8; 36],
}
```

Slot layout (from `docs/spec/inventory-items.md` §14.1 and `reference/logic/inventory.md`):

| Slots | Contents |
|-------|----------|
| 0 | Dirk |
| 1 | Mace |
| 2 | Sword |
| 3 | Bow |
| 4 | Wand |
| 5 | Lasso |
| 6 | Shell |
| 7 | Sun Stone |
| 8 | Arrows (count) |
| 9–15 | Magic items (Stone, Jewel, Vial, Orb, Totem, Ring, Skull) |
| 16–21 | Keys (Gold, Green, Blue, Red, Grey, White) |
| 22–27 | Food types |
| 28 | Writ |
| 29 | Bone |
| 30–34 | Quest items / misc |
| 35 | Transient quiver (arrows in flight; not saved) |

`stuff[slot]` is a count (0 = not owned). For weapons and unique items, the count is either 0 or 1. For arrows and food it is the actual quantity.

### `WorldObj` component (`src/game/ecs/components.rs`, line 243)

```rust
pub struct WorldObj {
    pub ob_id:   u8,   // inventory slot index this object corresponds to
    pub ob_stat: u8,   // 0=taken, 1=ground item, 3=setfig-held, 5=hidden
    pub region:  u8,
    pub visible: bool,
    pub goal:    u8,
}
```

`ob_id` is the `stuff[]` array slot to increment when the item is picked up.

### `Loot` component (`src/game/ecs/components.rs`, line 223)

```rust
pub struct Loot {
    pub weapon: u8,   // weapon slot index (0-4); 0 = no weapon
    pub gold:   i16,  // gold carried
    pub looted: bool, // true once body has been searched
}
```

Present on `Enemy` entities. When an enemy dies (vitality ≤ 0) the `Loot` component stays on the entity. `handle_search()` reads it and transfers `weapon` and `gold` to the hero.

### `Bones` entity

A `Bones` entity is spawned when a brother dies (see `EcsScene::drain_messages()` / `spawn_bones()`). It carries `BrotherKind + Inventory + WorldObj` (no `Loot` component). Searching a `Bones` entity transfers the entire `Inventory.stuff` array to the hero's inventory (slot-by-slot max merge).

### Current `item.rs` state

`src/game/ecs/systems/item.rs` (111 lines):
- `run()`: drains `res.events.item`, calls `handle_take()` or `handle_search()`.
- `handle_take()` (line 29): sets `ob_stat = 0`, `visible = false`, emits `"Taken."` — stub, does NOT modify `Inventory`.
- `handle_search()` (line 42): sets `ob_stat = 0` — stub, does NOT transfer loot.

Both functions have `TODO(Plan D)` comments explicitly deferring full implementation to a later plan. This plan implements them.

### `ItemEvent` variants (`src/game/ecs/events.rs`, line 118)

```rust
pub enum ItemEvent {
    TakeItem  { entity: hecs::Entity },
    SearchBody { entity: hecs::Entity },
}
```

`TakeItem` is emitted when the player presses Take (Plan I `dispatch_menu_action(MenuAction::Take)`). `SearchBody` is emitted by `ItemSystem` itself after an enemy is found dead and adjacent.

### Current `handle_take_item` flow in old `GameplayScene` (reference)

From `gameplay_scene/items.rs` (ported reference, not the ECS path):
1. Scan `world_objects` for `ob_stat == 1` within 16px.
2. Set `ob_stat = 0`, decrement `setstuff` count for that region slot.
3. Increment `stuff[ob_id]` on hero.
4. If it was a bones slot, transfer entire bones inventory.
5. Emit narr `event_msg[id]` for item (from `faery.toml`).
6. Play SFX (sfx_id = 5 for item pickup).

### Inventory screen rendering

When `res.view.viewstatus == 1`, the game overlays the playfield with a grid showing the current inventory. The original game shows 7 slots per row (the active menu sub-mode determines which 7 items are visible). For Plan J we render a simple full-inventory grid:

- Grid origin: map playfield area (x=32, y=40 canvas, same as `PLAYFIELD_X`/`PLAYFIELD_Y`).
- Cell size: 40×40 canvas pixels (20×20 native, roughly 16×16 sprite + label).
- Slots per row: 7. Rows: 5 (35 slots shown; slot 35 excluded).
- Object sprite sheet: cfile index 3 (`CFILE_BLOCKS[3] = 1312`), which is the `objects` sprite sheet. `CFILE_FRAME_COUNTS[3] = 116`.
- Close key: any key press while `viewstatus == 1` resets it to 0 and returns.
- Item name: use the 5-char label from `LABEL7` (USE menu labels) for weapon slots 0–8. For other slots, show the slot number or a placeholder label until the full name table is available.

### Eating / food item use

From `docs/spec/survival.md` §18.3:

- Food items occupy `stuff[22..=27]` (slot 22 = most basic food, slot 27 = richest).
- Eating a food item: `stuff[slot]--`; `hunger` decremented by the food's satiation value.
- Satiation values are defined per item in `docs/spec/survival.md` §18.3.
- The `HeroStats.hunger` field is an `i16`; clamped to 0.

For Plan J, implement the eat action for `MenuAction::UseItem` when the selected slot is a food item (slots 22–27). Weapon equip (`MenuAction::SetWeapon`) was already wired in Plan I.

---

## File map

| File | Change |
|------|--------|
| `src/game/ecs/systems/item.rs` | Major: implement `handle_take()` and `handle_search()` with full inventory mutation |
| `src/game/ecs/scene.rs` | Add inventory screen render path; extend `dispatch_menu_action()` for use-item effects |
| `src/game/ecs/components.rs` | Read-only — verify `Inventory`, `Loot`, `WorldObj` shapes |
| `src/game/ecs/events.rs` | Read-only — verify `ItemEvent`, `MessageEvent` |

---

## Task 1: Implement `handle_take()` in `ItemSystem`

**File:** `src/game/ecs/systems/item.rs`

Replace the current stub `handle_take()` with a full implementation that transfers the item to the hero's inventory.

- [ ] **Step 1: Add hero `Inventory` query to `handle_take()`**

  ```rust
  fn handle_take(world: &mut World, res: &mut Resources, entity: hecs::Entity) {
      // Read ob_id before mutating.
      let ob_id = match world.get::<&WorldObj>(entity) {
          Ok(obj) if obj.ob_stat == 1 => obj.ob_id,
          _ => return, // already taken or invalid
      };

      // Mark item as taken in the world.
      if let Ok(mut obj) = world.get::<&mut WorldObj>(entity) {
          obj.ob_stat = 0;
          obj.visible = false;
      }

      // Transfer to hero inventory.
      if let Ok(mut inv) = world.get::<&mut Inventory>(res.hero_entity) {
          let slot = ob_id as usize;
          if slot < inv.stuff.len() {
              inv.stuff[slot] = inv.stuff[slot].saturating_add(1);
          }
      }

      // Emit item-pickup SFX.
      res.events.sfx.push(crate::game::ecs::events::SfxEvent { sfx_id: 5 });

      // Emit scroll message (item name from narr event_msg table).
      // The event_msg index for item ob_id is ob_id itself (fmain.c:2743 reference).
      // Use a placeholder until the narr table is confirmed.
      res.events.message.push(MessageEvent {
          text: format!("Taken."), // TODO(Plan J): replace with narr.event_msg[ob_id]
      });
  }
  ```

- [ ] **Step 2: Verify no panic when `ob_stat != 1`**

  The guard `if obj.ob_stat == 1` ensures double-take is a no-op. This is correct: `ItemEvent::TakeItem` can be emitted speculatively; `handle_take` validates.

- [ ] **Step 3: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

---

## Task 2: Implement `handle_search()` — body search for enemies and Bones

**File:** `src/game/ecs/systems/item.rs`

Replace the stub `handle_search()` with a full implementation covering both dead enemy `Loot` and `Bones` entities.

- [ ] **Step 1: Implement enemy body search (transfers `Loot` component)**

  ```rust
  fn handle_search(world: &mut World, res: &mut Resources, entity: hecs::Entity) {
      use crate::game::ecs::components::{Bones, Inventory, Loot, WorldObj};
      use crate::game::ecs::events::{MessageEvent, SfxEvent};

      // Case 1: Bones entity — merge entire inventory into hero's.
      let is_bones = world.get::<&Bones>(entity).is_ok();
      if is_bones {
          // Read bones inventory (clone to avoid simultaneous borrow).
          let bones_stuff: Option<[u8; 36]> = world
              .get::<&Inventory>(entity)
              .ok()
              .map(|inv| inv.stuff);

          if let Some(bones_stuff) = bones_stuff {
              if let Ok(mut hero_inv) = world.get::<&mut Inventory>(res.hero_entity) {
                  for (slot, &count) in bones_stuff.iter().enumerate() {
                      if count > 0 && slot < hero_inv.stuff.len() {
                          hero_inv.stuff[slot] = hero_inv.stuff[slot].saturating_add(count);
                      }
                  }
              }
              res.events.message.push(MessageEvent {
                  text: "You search the remains.".to_string(),
              });
          }

          // Mark bones as looted.
          if let Ok(mut obj) = world.get::<&mut WorldObj>(entity) {
              obj.ob_stat = 0;
              obj.visible = false;
          }
          return;
      }

      // Case 2: Dead enemy with Loot component.
      let loot_data: Option<(u8, i16)> = world.get::<&Loot>(entity).ok().and_then(|loot| {
          if loot.looted { None } else { Some((loot.weapon, loot.gold)) }
      });

      if let Some((weapon, gold)) = loot_data {
          // Mark as looted immediately to prevent double-search.
          if let Ok(mut loot) = world.get::<&mut Loot>(entity) {
              loot.looted = true;
          }
          // Transfer weapon to hero inventory.
          if weapon > 0 {
              let slot = weapon as usize;
              if let Ok(mut inv) = world.get::<&mut Inventory>(res.hero_entity) {
                  if slot < inv.stuff.len() {
                      inv.stuff[slot] = inv.stuff[slot].saturating_add(1);
                  }
              }
          }
          // Transfer gold to hero HeroStats.wealth.
          if gold > 0 {
              if let Ok(mut stats) = world.get::<&mut HeroStats>(res.hero_entity) {
                  stats.wealth = stats.wealth.saturating_add(gold);
              }
          }
          // Emit SFX + message.
          res.events.sfx.push(SfxEvent { sfx_id: 5 });
          res.events.message.push(MessageEvent {
              text: "Searched.".to_string(),
          });
      }

      // Mark world object as looted.
      if let Ok(mut obj) = world.get::<&mut WorldObj>(entity) {
          obj.ob_stat = 0;
      }
  }
  ```

  Note: `HeroStats` needs to be imported at the top of `item.rs`:
  ```rust
  use crate::game::ecs::components::{Bones, HeroStats, Inventory, Loot, WorldObj};
  ```

- [ ] **Step 2: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

---

## Task 3: Add `render_inventory()` to `EcsScene`

**File:** `src/game/ecs/scene.rs`

When `res.view.viewstatus == 1`, render an inventory overlay over the playfield. This overlay replaces the map view for one frame; pressing any key clears `viewstatus` back to 0.

- [ ] **Step 1: Add `use` imports for rendering inventory**

  The object sprite sheet is `res.sprites.object_sprites` (loaded via `SpriteSheet::load_objects()`). It is cfile index 3 (blocks 1312–1347 from ADF).

- [ ] **Step 2: Add `render_inventory()` method**

  ```rust
  /// Render the inventory overlay (viewstatus == 1).
  /// Shows all 35 inventory slots in a 7-column grid over the map area.
  /// Pressing any key (handled in handle_event → clears viewstatus) dismisses it.
  fn render_inventory(&mut self, canvas: &mut Canvas<Window>) {
      use crate::game::ecs::components::Inventory;
      use crate::game::menu::{LABEL7};

      let stuff = match self.world.get::<&Inventory>(self.res.hero_entity) {
          Ok(inv) => inv.stuff,
          Err(_)  => return,
      };

      // Black background over the playfield area.
      canvas.set_draw_color(sdl3::pixels::Color::RGB(0, 0, 0));
      canvas.fill_rect(sdl3::rect::Rect::new(
          PLAYFIELD_X, PLAYFIELD_Y, PLAYFIELD_CANVAS_W, PLAYFIELD_CANVAS_H,
      )).ok();

      // Grid: 7 cols × 5 rows = 35 slots, each 40×40 canvas px.
      const COLS: usize = 7;
      const CELL_W: u32 = 40;
      const CELL_H: u32 = 40;

      // Build label table from LABEL7 (USE menu: slots 0-8 = weapons/items).
      // LABEL7 chunks: Dirk,Mace,Sword,Bow,Wand,Lasso,Shell,Key,Sun,(blank)
      let label7_chunks: Vec<&str> = (0..LABEL7.len())
          .step_by(5)
          .map(|i| LABEL7[i..i + 5].trim_end())
          .collect();

      for slot in 0..35usize {
          let count = stuff[slot];
          let col = slot % COLS;
          let row = slot / COLS;
          let cell_x = PLAYFIELD_X + (col as i32) * CELL_W as i32;
          let cell_y = PLAYFIELD_Y + (row as i32) * CELL_H as i32;

          // Draw cell background (dimmed if count == 0).
          if count > 0 {
              canvas.set_draw_color(sdl3::pixels::Color::RGB(40, 30, 10));
          } else {
              canvas.set_draw_color(sdl3::pixels::Color::RGB(15, 10, 5));
          }
          canvas.fill_rect(sdl3::rect::Rect::new(
              cell_x + 1, cell_y + 1, CELL_W - 2, CELL_H - 2,
          )).ok();

          if count == 0 { continue; }

          // Draw object sprite for this slot (from object sprite sheet, frame = slot).
          // Object sprites are 16×16 (different from actor sprites which are 16×32).
          // Place sprite at top-left of cell, centered horizontally.
          if let Some(ref obj_sheet) = self.res.sprites.object_sprites {
              if let Some(frame_pixels) = obj_sheet.frame_pixels(slot) {
                  // Blit 16×16 object sprite to canvas via a small surface.
                  let pal = &self.res.palette.current_palette;
                  let mut rgba_buf: Vec<u8> = Vec::with_capacity(16 * 16 * 4);
                  for &idx in frame_pixels.iter().take(16 * 16) {
                      let color = if idx == 31 { 0u32 } else { pal[(idx & 31) as usize] };
                      rgba_buf.push((color & 0xFF) as u8);
                      rgba_buf.push(((color >> 8) & 0xFF) as u8);
                      rgba_buf.push(((color >> 16) & 0xFF) as u8);
                      rgba_buf.push(if idx == 31 { 0 } else { 0xFF });
                  }
                  let tc = canvas.texture_creator();
                  if let Ok(surface) = sdl3::surface::Surface::from_data(
                      &mut rgba_buf, 16, 16, 16 * 4,
                      sdl3::pixels::PixelFormat::ARGB8888,
                  ) {
                      if let Ok(mut tex) = tc.create_texture_from_surface(&surface) {
                          tex.set_scale_mode(sdl3::render::ScaleMode::Nearest);
                          let dst = sdl3::rect::Rect::new(
                              cell_x + 4, cell_y + 2, 32, 32, // ×2 scale
                          );
                          canvas.copy(&tex, None, dst).ok();
                      }
                  }
              }
          }

          // Draw slot label (weapon name or slot index fallback).
          // NOTE: always call set_color_mod before render_string (AGENTS.md rule).
          // (amber_font not available in this non-hibar context; use a debug label for now)
          // TODO(Plan J render): wire topaz font for item labels, per spec §14.x.
          // For now the slot number is sufficient for development visibility.
      }
  }
  ```

  > **Note on font:** The spec references a "topaz font" for inventory labels. The `amber_font` held in `SceneResources` is used in `render_hibar()`. `render_inventory()` is called from `update()` where `SceneResources` is available — pass it through as a parameter when wiring.

- [ ] **Step 3: Call `render_inventory()` from `update()`**

  In `Scene::update()`, after `self.render_map(canvas)` and before `self.render_hibar(canvas, resources)`:

  ```rust
  if self.res.view.viewstatus == 1 {
      self.render_inventory(canvas);
      // Don't render the map beneath it — skip further map rendering this frame.
  }
  ```

- [ ] **Step 4: Clear `viewstatus` on any key press when inventory is open**

  In `handle_event()`, at the top of the `Event::KeyDown` arm, add:

  ```rust
  if self.res.view.viewstatus == 1 {
      self.res.view.viewstatus = 0;
      return true; // consume the key
  }
  ```

  This mirrors the original game behavior: any keypress closes the inventory screen.

- [ ] **Step 5: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

---

## Task 4: Wire `MenuAction::UseItem` in `dispatch_menu_action()`

**File:** `src/game/ecs/scene.rs`

Plan I's `dispatch_menu_action()` has stubs for most variants. Plan J fills in item-use effects. The primary actions to wire here are:

- `MenuAction::SetWeapon(n)` — already wired in Plan I; verify it also marks the weapon as "equipped" visually (weapon slot highlight in USE menu requires `set_options()` to re-run, which happens next tick automatically).
- Food item use: when the player is in the USE menu and selects a food slot (slots 22–27), decrement `stuff[slot]` and reduce `hunger`.

The USE menu (`MenuMode::Use`) action path: when a USE-mode item (slots 0–8 in `LABEL7`) is clicked/keybound, `MenuState::dispatch_do_option()` returns `MenuAction::SetWeapon(hit)` for slots 0–4 (weapons), `MenuAction::SummonTurtle` for slot 5 (Lasso — carrier), `MenuAction::TryKey` for slot 7, `MenuAction::UseSunstone` for slot 8.

Food items in slots 22–27 are not in the USE menu directly. The original game handles food via a separate Buy menu row (`MenuMode::Buy`, `BuyItem(n)` where n=0..6 maps to food/arrows/vial etc.). For the ECS port, when `BuyItem(n)` is dispatched and the game context is a shop, it buys; when there is no active shop, it acts as a "use food" shortcut.

For Plan J, implement the food-eat path for `MenuAction::BuyItem(n)` when `n` corresponds to a food slot:

- [ ] **Step 1: Add food satiation table**

  Near the top of `scene.rs` (or in a `const` block), add:

  ```rust
  /// Satiation amount per food slot (stuff[22..=27]).
  /// Source: docs/spec/survival.md §18.3.
  const FOOD_SATIATION: [i16; 6] = [25, 35, 45, 55, 65, 80];
  ```

- [ ] **Step 2: Extend `dispatch_menu_action()` for food use**

  In the `MenuAction::BuyItem(n)` arm (currently a TODO stub), add:

  ```rust
  MenuAction::BuyItem(n) => {
      // When no active shop, treat BuyItem(n) as "use food item in slot (22 + n)".
      // A shop context will be wired in Plan K; for now route all BuyItem to eat.
      use crate::game::ecs::components::{HeroStats, Inventory};
      let food_slot = 22usize + (n as usize).min(5);
      let satiation = FOOD_SATIATION[(n as usize).min(5)];
      // Decrement count and apply hunger reduction.
      let mut ate = false;
      if let Ok(mut inv) = self.world.get::<&mut Inventory>(self.res.hero_entity) {
          if inv.stuff[food_slot] > 0 {
              inv.stuff[food_slot] -= 1;
              ate = true;
          }
      }
      if ate {
          if let Ok(mut stats) = self.world.get::<&mut HeroStats>(self.res.hero_entity) {
              stats.hunger = (stats.hunger - satiation).max(0);
          }
          self.res.events.message.push(crate::game::ecs::events::MessageEvent {
              text: "Eaten.".to_string(),
          });
      }
  }
  ```

- [ ] **Step 3: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

---

## Task 5: Wire `MenuAction::TakeAction` path (body search trigger)

**File:** `src/game/ecs/scene.rs`

In Plan I, `MenuAction::Take` emits an `ItemEvent::TakeItem` for the nearest ground item. This plan extends it to also check for dead enemy bodies (`ob_stat == 0` + `Loot.looted == false`) and emit `ItemEvent::SearchBody` for those.

- [ ] **Step 1: Update the `MenuAction::Take` branch in `dispatch_menu_action()`**

  Replace the Plan I stub with an extended version:

  ```rust
  MenuAction::Take => {
      use crate::game::ecs::components::{Bones, Loot, Position, WorldObj};
      use crate::game::ecs::events::ItemEvent;

      let hero_pos = self.world
          .get::<&Position>(self.res.hero_entity)
          .map(|p| (p.x, p.y))
          .unwrap_or((0.0, 0.0));

      let mut best_item:   Option<(hecs::Entity, f32)> = None;
      let mut best_corpse: Option<(hecs::Entity, f32)> = None;

      for (entity, (obj, pos)) in self.world.query::<(&WorldObj, &Position)>().iter() {
          let dx = pos.x - hero_pos.0;
          let dy = pos.y - hero_pos.1;
          let dist = (dx * dx + dy * dy).sqrt();
          if dist > 16.0 { continue; }

          if obj.ob_stat == 1 && obj.visible {
              // Ground item.
              if best_item.map_or(true, |(_, d)| dist < d) {
                  best_item = Some((entity, dist));
              }
          }
      }

      // Also scan for unlooted corpses (dead enemies with Loot).
      for (entity, (loot, pos)) in self.world.query::<(&Loot, &Position)>().iter() {
          if loot.looted { continue; }
          let dx = pos.x - hero_pos.0;
          let dy = pos.y - hero_pos.1;
          let dist = (dx * dx + dy * dy).sqrt();
          if dist <= 16.0 {
              if best_corpse.map_or(true, |(_, d)| dist < d) {
                  best_corpse = Some((entity, dist));
              }
          }
      }

      // Prioritize ground items over corpses; if no ground item, try corpse.
      if let Some((entity, _)) = best_item {
          self.res.events.item.push(ItemEvent::TakeItem { entity });
      } else if let Some((entity, _)) = best_corpse {
          self.res.events.item.push(ItemEvent::SearchBody { entity });
      }
  }
  ```

- [ ] **Step 2: Verify compile**

  ```bash
  cargo check 2>&1 | grep "^error"
  ```

- [ ] **Step 3: Commit**

  ```bash
  git add src/game/ecs/systems/item.rs src/game/ecs/scene.rs
  git commit -m "feat(ecs): Plan J — item pickup, body search, inventory screen, food use"
  ```

---

## Spec references

- `docs/spec/inventory-items.md` §14.1–14.9: inventory slot layout, pickup logic, use effects, item names
- `docs/spec/survival.md` §18.3: hunger system, food satiation values, eating mechanics
- `reference/logic/inventory.md` (research branch): original `handle_take_item()` + `search_body()` flow, `ob_id`↔`stuff[]` mapping, loot table

---

## Test plan

All tests are unit tests in `src/game/ecs/systems/item.rs` under `#[cfg(test)] mod tests`. Use the existing `hero_stats()`, `spawn_hero()`, `spawn_ground_item()`, `spawn_bones()` helpers already present in that module.

| # | Test name | Setup | What it verifies |
|---|-----------|-------|------------------|
| 1 | `take_item_increments_inventory` | Hero at `(100,100)`; ground item with `ob_id=2, ob_stat=1` at `(102,100)`; emit `TakeItem` | `hero.Inventory.stuff[2] == 1` after `run()` |
| 2 | `take_item_already_taken_no_op` | Ground item with `ob_stat=0` (already taken); emit `TakeItem` | Inventory unchanged; no message emitted |
| 3 | `search_body_transfers_loot_gold` | Enemy entity with `Loot { weapon: 0, gold: 10, looted: false }`; emit `SearchBody` | `hero.HeroStats.wealth += 10`; `loot.looted == true` |
| 4 | `search_body_transfers_loot_weapon` | Enemy entity with `Loot { weapon: 1, gold: 0, looted: false }`; emit `SearchBody` | `hero.Inventory.stuff[1] == 1`; `loot.looted == true` |
| 5 | `search_bones_merges_inventory` | Bones entity with `Inventory { stuff }` where `stuff[3] = 1`; emit `SearchBody` | `hero.Inventory.stuff[3] == 1` after merge |

```rust
#[cfg(test)]
mod tests {
    use hecs::World;
    use crate::game::ecs::components::*;
    use crate::game::ecs::resources::Resources;
    use crate::game::ecs::spawn::*;
    use crate::game::ecs::events::ItemEvent;
    use super::run;

    fn hero_stats() -> HeroStats {
        HeroStats { vitality: 100, brave: 0, luck: 0, kind: 0,
                    wealth: 0, hunger: 0, fatigue: 0, gold: 0 }
    }

    #[test]
    fn take_item_increments_inventory() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        let item = spawn_ground_item(&mut world, 102.0, 100.0,
            WorldObj { ob_id: 2, ob_stat: 1, region: 0, visible: true, goal: 0 });
        res.events.item.push(ItemEvent::TakeItem { entity: item });
        run(&mut world, &mut res);
        let inv = world.get::<&Inventory>(hero).unwrap();
        assert_eq!(inv.stuff[2], 1, "stuff[2] (Sword slot) should be 1 after pickup");
    }

    #[test]
    fn take_item_already_taken_is_noop() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        let item = spawn_ground_item(&mut world, 102.0, 100.0,
            WorldObj { ob_id: 2, ob_stat: 0, region: 0, visible: false, goal: 0 }); // already taken
        res.events.item.push(ItemEvent::TakeItem { entity: item });
        run(&mut world, &mut res);
        let inv = world.get::<&Inventory>(hero).unwrap();
        assert_eq!(inv.stuff[2], 0, "Inventory should be unchanged for already-taken item");
        assert_eq!(res.events.message.len(), 0, "No message for already-taken item");
    }

    #[test]
    fn search_body_transfers_gold() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        // Spawn a dead enemy entity with a Loot component.
        let enemy = world.spawn((
            crate::game::ecs::components::Enemy,
            crate::game::ecs::components::Position::new(102.0, 100.0),
            WorldObj { ob_id: 0, ob_stat: 0, region: 0, visible: false, goal: 0 },
            Loot { weapon: 0, gold: 10, looted: false },
        ));
        res.events.item.push(ItemEvent::SearchBody { entity: enemy });
        run(&mut world, &mut res);
        let stats = world.get::<&HeroStats>(hero).unwrap();
        assert_eq!(stats.wealth, 10, "Hero wealth should increase by looted gold");
        let loot = world.get::<&Loot>(enemy).unwrap();
        assert!(loot.looted, "Loot should be marked as looted after search");
    }

    #[test]
    fn search_body_transfers_weapon() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        let enemy = world.spawn((
            crate::game::ecs::components::Enemy,
            crate::game::ecs::components::Position::new(102.0, 100.0),
            WorldObj { ob_id: 0, ob_stat: 0, region: 0, visible: false, goal: 0 },
            Loot { weapon: 1, gold: 0, looted: false }, // weapon slot 1 = Mace
        ));
        res.events.item.push(ItemEvent::SearchBody { entity: enemy });
        run(&mut world, &mut res);
        let inv = world.get::<&Inventory>(hero).unwrap();
        assert_eq!(inv.stuff[1], 1, "Mace (slot 1) should be in hero inventory after looting");
    }

    #[test]
    fn search_bones_merges_inventory() {
        let mut world = World::new();
        let hero = spawn_hero(&mut world, 100.0, 100.0, 0, hero_stats(), Inventory::empty());
        let mut res = Resources::new(hero);
        // Bones entity has a bow (slot 3) in its inventory.
        let mut bones_stuff = [0u8; 36];
        bones_stuff[3] = 1; // Bow
        let bones = spawn_bones(&mut world, 105.0, 100.0, 0, 0, bones_stuff);
        res.events.item.push(ItemEvent::SearchBody { entity: bones });
        run(&mut world, &mut res);
        let inv = world.get::<&Inventory>(hero).unwrap();
        assert_eq!(inv.stuff[3], 1, "Bow (slot 3) should be merged into hero inventory from Bones");
    }
}
```

---

## Files touched

| File | Change |
|------|--------|
| `src/game/ecs/systems/item.rs` | Major: implement `handle_take()` with full `Inventory` mutation; implement `handle_search()` for `Loot` enemy bodies and `Bones` entities; add `HeroStats` import |
| `src/game/ecs/scene.rs` | Add `render_inventory()` method; call from `update()`; extend `dispatch_menu_action()` with food-eat path; extend `MenuAction::Take` to scan corpses; add `FOOD_SATIATION` constant; dismiss inventory on key press in `handle_event()` |
| `src/game/ecs/components.rs` | Read-only — verify `Inventory`, `Loot`, `WorldObj`, `Bones`, `HeroStats` shapes match plan assumptions |
| `src/game/ecs/events.rs` | Read-only — verify `ItemEvent`, `MessageEvent`, `SfxEvent` definitions |

---

## Completion check

```bash
cargo build 2>&1 | grep "^error"
cargo test ecs::systems::item 2>&1 | grep "^test result"
cargo run 2>&1 | head -5
```

All three succeed. Ground item pickup increments the hero's inventory slot. Body search transfers gold and weapons. Pressing `I` (Inventory) opens the inventory screen; any key closes it. Food items can be consumed via the Buy menu shortcut. All 5 tests in `item.rs` pass.
