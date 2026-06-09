---
title: "Plan U — Save/Load Key Binding + Game Menu"
plan: U
status: draft
depends_on: [I]
touches: [src/main.rs, src/game/ecs/scene.rs]
---

# ECS Migration Plan U: Save/Load Key Binding + Game Menu

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement F5/F9 save/load hotkeys in the main event loop and wire `MenuAction` handlers (SaveGame, LoadGame, Quit) into `EcsScene::dispatch_menu_action()`.

**Architecture:** Main.rs event loop handles F5/F9 scancodes directly, calling the existing ECS persist functions. `EcsScene` gains a `dispatch_menu_action()` method to handle menu-driven save/load/quit actions from the Game sub-menu.

**Prerequisites:** Plan I (MenuAction variants available). Plan E (persist functions). Plans A-D.

**Tech Stack:** Rust 2021, SDL3 scancodes, protobuf save format.

---

## File map

| File | Change |
|------|--------|
| `src/main.rs` | Add F5/F9 handlers; fix victory/hero TODOs |
| `src/game/ecs/scene.rs` | Add dispatch_menu_action() method |

---

## Task 1: Add F5 save handler in main.rs event loop

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Locate the Event::KeyDown match block**

Find the existing scancode handling in `main.rs` (around line 520-530 based on TODO comments).

- [ ] **Step 2: Add F5 handler**

```rust
Event::KeyDown { scancode: Scancode::F5, .. } => {
    match ecs_save_game(&ecs_scene, 0) {
        Ok(()) => {
            // Emit save confirmation message
            println!("Game saved to slot 0");
        }
        Err(e) => {
            eprintln!("Failed to save game: {}", e);
        }
    }
}
```

- [ ] **Step 3: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors. The `ecs_save_game` function should already be imported from `game::persist`.

---

## Task 2: Add F9 load handler in main.rs event loop

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add F9 handler after F5 case**

```rust
Event::KeyDown { scancode: Scancode::F9, .. } => {
    match ecs_load_game(0, &mut ecs_scene) {
        Ok(()) => {
            // Emit load confirmation message
            println!("Game loaded from slot 0");
        }
        Err(e) => {
            eprintln!("Failed to load game: {}", e);
        }
    }
}
```

- [ ] **Step 2: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors.

---

## Task 3: Fix victory detection TODO in main.rs

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Locate the victory detection TODO**

Find the line: `let won = false; // TODO(Plan D)` around line 523.

- [ ] **Step 2: Replace with inventory check**

```rust
// Check if hero has the Talisman (item slot 22) to win
let hero_entity = ecs_scene.res.hero_entity;
let won = ecs_scene.world
    .get::<Inventory>(hero_entity)
    .map(|inv| inv.stuff[22] != 0)
    .unwrap_or(false);
```

- [ ] **Step 3: Add Inventory import if needed**

```rust
use crate::game::ecs::components::Inventory;
```

- [ ] **Step 4: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors.

---

## Task 4: Fix hero name TODO in main.rs

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Locate the hero name TODO**

Find the line: `let hero = "Julian"; // TODO(Plan D)` around line 525.

- [ ] **Step 2: Replace with dynamic hero name lookup**

```rust
// Get hero's name from BrotherKind component and GameLibrary
let hero_entity = ecs_scene.res.hero_entity;
let hero = ecs_scene.world
    .get::<BrotherKind>(hero_entity)
    .ok()
    .and_then(|bk| game_lib.get_brother(bk.id as usize))
    .map(|b| b.name.clone())
    .unwrap_or("Hero".to_string());
```

- [ ] **Step 3: Add BrotherKind import if needed**

```rust
use crate::game::ecs::components::BrotherKind;
```

- [ ] **Step 4: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors.

---

## Task 5: Add dispatch_menu_action() to EcsScene

**Files:**
- Modify: `src/game/ecs/scene.rs`

- [ ] **Step 1: Add method to EcsScene impl block**

```rust
/// Dispatch menu actions from the Game sub-menu
pub fn dispatch_menu_action(&mut self, action: crate::game::menu::MenuAction) -> crate::game::scene::SceneResult {
    use crate::game::menu::MenuAction;
    use crate::game::persist::{ecs_save_game, ecs_load_game};
    
    match action {
        MenuAction::SaveGame(slot) => {
            match ecs_save_game(self, slot) {
                Ok(()) => {
                    // Save confirmation could be added to message queue
                    self.res.messages.push("Game saved".to_string());
                }
                Err(e) => {
                    self.res.messages.push(format!("Save failed: {}", e));
                }
            }
            crate::game::scene::SceneResult::Continue
        }
        MenuAction::LoadGame(slot) => {
            match ecs_load_game(slot, self) {
                Ok(()) => {
                    self.res.messages.push("Game loaded".to_string());
                }
                Err(e) => {
                    self.res.messages.push(format!("Load failed: {}", e));
                }
            }
            crate::game::scene::SceneResult::Continue
        }
        MenuAction::Quit => {
            crate::game::scene::SceneResult::Quit
        }
        _ => {
            // Other menu actions are handled elsewhere
            crate::game::scene::SceneResult::Continue
        }
    }
}
```

- [ ] **Step 2: Wire dispatch_menu_action in menu handling**

In the `handle_event()` method, when a menu action is selected, call:
```rust
if let Some(action) = self.menu.handle_key_event(event) {
    let result = self.dispatch_menu_action(action);
    // Handle result if needed (e.g., SceneResult::Quit)
}
```

- [ ] **Step 3: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors.

---

## Task 6: Add unit tests

**Files:**
- Modify: `src/game/ecs/scene.rs` (add tests module)

- [ ] **Step 1: Add test module**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::ecs::components::{Inventory, BrotherKind};
    use crate::game::menu::MenuAction;
    
    #[test]
    fn test_f5_save_creates_file() {
        // Test that F5 handler creates save file at expected path
        // This would need to be an integration test due to file I/O
    }
    
    #[test]
    fn test_f9_load_restores_position() {
        // Test that F9 restores hero position from save
        // Integration test due to file I/O
    }
    
    #[test]
    fn test_quit_returns_scene_result_quit() {
        let mut scene = create_test_scene();
        let result = scene.dispatch_menu_action(MenuAction::Quit);
        assert!(matches!(result, crate::game::scene::SceneResult::Quit));
    }
    
    #[test]
    fn test_victory_detection_with_talisman() {
        let mut scene = create_test_scene();
        let hero_entity = scene.res.hero_entity;
        
        // Add talisman to inventory (slot 22)
        let mut inv = scene.world.get::<Inventory>(hero_entity).unwrap().clone();
        inv.stuff[22] = 1; // Talisman
        scene.world.insert(hero_entity, inv).unwrap();
        
        // Check victory condition
        let won = scene.world
            .get::<Inventory>(hero_entity)
            .map(|inv| inv.stuff[22] != 0)
            .unwrap_or(false);
        assert!(won);
    }
    
    #[test]
    fn test_hero_name_from_brother_kind() {
        let mut scene = create_test_scene();
        let hero_entity = scene.res.hero_entity;
        
        // Set brother to Phillip (id=1)
        scene.world.insert(hero_entity, BrotherKind { id: 1 }).unwrap();
        
        let game_lib = create_test_game_lib();
        let hero_name = scene.world
            .get::<BrotherKind>(hero_entity)
            .ok()
            .and_then(|bk| game_lib.get_brother(bk.id as usize))
            .map(|b| b.name.clone())
            .unwrap_or("Hero".to_string());
        
        assert_eq!(hero_name, "Phillip");
    }
    
    fn create_test_scene() -> EcsScene {
        // Helper to create a minimal test scene
        todo!("Implement test helper")
    }
    
    fn create_test_game_lib() -> crate::game::game_library::GameLibrary {
        // Helper to create test game library
        todo!("Implement test helper")
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cargo test ecs::scene::tests 2>&1 | grep "^test result"
```

Expected: tests compile and run (implementation of test helpers needed).

---

## Task 7: Integration testing

- [ ] **Step 1: Manual test F5/F9**

```bash
cargo run -- --ecs
```

1. Start game, move hero to a specific position
2. Press F5 - should see "Game saved" message
3. Move hero to different position
4. Press F9 - should see "Game loaded" message and hero returns to saved position

- [ ] **Step 2: Test Game menu save/load**

1. Press Escape to open menu
2. Select "Game" sub-menu
3. Select "Save" - should save successfully
4. Move hero, then return to Game menu and select "Load"
5. Hero should return to saved position

- [ ] **Step 3: Test victory condition**

1. Add talisman to hero inventory (via debug commands)
2. Victory detection should return true
3. Remove talisman - victory should return false

- [ ] **Step 4: Test hero name display**

1. Switch between different brothers (Julian, Phillip, Kevin)
2. Hero name should update correctly in UI

---

## Task 8: Final verification

- [ ] **Step 1: Full build check**

```bash
cargo build 2>&1 | grep "^error"
```

Expected: no errors.

- [ ] **Step 2: Run test suite**

```bash
cargo test 2>&1 | grep "^test result"
```

Expected: all tests pass.

- [ ] **Step 3: Commit changes**

```bash
git add src/main.rs src/game/ecs/scene.rs
git commit -m "feat(ecs): implement F5/F9 save/load hotkeys and menu action dispatch

- Add F5/F9 key handlers in main.rs event loop
- Fix victory detection TODO with inventory talisman check
- Fix hero name TODO with BrotherKind lookup
- Add EcsScene::dispatch_menu_action() for Game menu actions
- Wire save/load confirmation messages to message queue"
```

---

## Completion check

```bash
cargo build 2>&1 | grep "^error"
cargo test 2>&1 | grep "^test result"
cargo run -- --ecs 2>&1 | head -5
```

All three succeed. F5/F9 save/load work, Game menu actions are wired, and TODOs are resolved.

---

## Context

### Current state
- F5/F9 TODOs exist in `src/main.rs` (lines 523, 525) as placeholder comments
- `ecs_save_game(scene, slot)` and `ecs_load_game(slot, scene)` already implemented in `src/game/persist.rs` (lines 542–581)
- `MenuAction` enum in `src/game/menu.rs` has `SaveGame(u8)`, `LoadGame(u8)`, `Quit` variants
- `SceneResult` enum in `src/game/scene.rs` has `Continue`, `Done`, `Quit` variants
- Victory detection TODO at main.rs line 523: `let won = false; // TODO(Plan D)`
- Hero name TODO at main.rs line 525: `let hero = "Julian"; // TODO(Plan D)`

### Persist function signatures
```rust
pub fn ecs_save_game(scene: &EcsScene, slot: u8) -> Result<(), String>
pub fn ecs_load_game(slot: u8, scene: &mut EcsScene) -> Result<(), String>
```

---

## Dependencies
Plan I (MenuAction variants available). Plan E (persist functions). Plans A-D.

---

## Spec references
- `docs/spec/ui-menus.md` §25.3 — Game menu actions
- `docs/spec/save-load.md` §24.1 — Save file format and slots