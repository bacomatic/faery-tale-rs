---
title: "Plan X — Parity and Cleanup"
plan: X
status: draft
depends_on: [G, H, I, J, K, L, M, N, O, P, Q, R, S, T, U, V, W]
touches: [src/main.rs, src/game/game_state.rs, src/game/ecs/scene.rs, AGENTS.md]
---

# ECS Migration Plan X: Parity and Cleanup

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Achieve feature parity between ECS path and legacy GameState path; delete the legacy path; run full test suite; update AGENTS.md.

**Architecture:** Complete the migration by removing the `--ecs` flag gating, deleting all GameState-related code, and ensuring the ECS implementation provides 100% of the gameplay functionality that the legacy implementation provided.

**Prerequisites:** All previous plans complete (G-W). ECS path must be fully functional.

**Tech Stack:** Rust 2021, comprehensive testing suite.

---

## File map

| File | Change |
|------|--------|
| `src/main.rs` | Remove --ecs flag; delete legacy scene construction |
| `src/game/game_state.rs` | **DELETE** entire file |
| `src/game/gameplay_scene/` | **DELETE** entire directory |
| `src/game/ecs/scene.rs` | Remove any remaining legacy references |
| `AGENTS.md` | Update to reflect ECS-only architecture |
| `src/game/mod.rs` | Remove legacy module declarations |
| `src/game/persist.rs` | Remove GameState persist functions |
| Various test files | Remove GameState-dependent tests |

---

## Task 1: Verify ECS completeness checklist

**Files:**
- Review: All ECS system implementations

- [ ] **Step 1: Create parity verification checklist**

Before deleting legacy code, verify every gameplay feature works in ECS:

```markdown
## Core Gameplay
- [ ] Hero movement (8 directions, keyboard + gamepad)
- [ ] Region transitions (door/zone system)
- [ ] Day/night cycle and palette changes
- [ ] Combat (melee attacks, hit detection, damage)
- [ ] Enemy AI (pursuit, flee, combat states)
- [ ] Item pickup and inventory management
- [ ] Menu system (all sub-menus functional)
- [ ] Save/load functionality (F5/F9 + menu)
- [ ] Audio (music mood changes + SFX)
- [ ] Death and brother succession
- [ ] Victory condition detection

## Advanced Features
- [ ] Magic system (spells, timers, effects)
- [ ] Shop transactions
- [ ] NPC dialogue and narrative events
- [ ] Carrier transport (raft, turtle, swan)
- [ ] Sleep system and time compression
- [ ] Encounter generation and spawning
- [ ] Weapon overlay rendering
- [ ] SetFig sprite rendering
- [ ] Debug TUI functionality

## UI/Rendering
- [ ] Map rendering with proper scrolling
- [ ] Sprite blitting for all entity types
- [ ] HI bar with stats and messages
- [ ] Menu button rendering
- [ ] Inventory screen overlay
- [ ] Placard/narrative overlay
```

- [ ] **Step 2: Manual testing of each checklist item**

```bash
cargo run -- --ecs
```

Systematically test each item. Document any failures or missing features.

- [ ] **Step 3: Address any parity gaps**

If any features are missing or broken in ECS path, fix them before proceeding. Do NOT delete legacy code until ECS parity is achieved.

---

## Task 2: Remove --ecs flag gating in main.rs

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Locate the --ecs flag logic**

Find where `use_ecs` is determined and the conditional scene construction.

- [ ] **Step 2: Replace with EcsScene-only construction**

```rust
// Remove this logic:
let use_ecs = std::env::args().any(|a| a == "--ecs");
let initial_scene: Box<dyn Scene> = if use_ecs {
    Box::new(crate::game::ecs::EcsScene::new(...))
} else {
    Box::new(GameplayScene::new(...))
};

// Replace with:
let initial_scene: Box<dyn Scene> = Box::new(crate::game::ecs::EcsScene::new(
    0, // Start with Julian
    &game_lib,
    adf,
));
```

- [ ] **Step 3: Remove any remaining --ecs references**

Search for any other `--ecs` or `use_ecs` references in main.rs and remove them.

- [ ] **Step 4: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: errors about missing GameplayScene and GameState imports (will be fixed in later tasks).

---

## Task 3: Delete legacy GameState and GameplayScene

**Files:**
- Delete: `src/game/game_state.rs`
- Delete: `src/game/gameplay_scene/` (entire directory)

- [ ] **Step 1: Backup (optional)**

```bash
# If you want to keep a backup reference:
cp -r src/game/gameplay_scene/ ../gameplay_scene_backup
cp src/game/game_state.rs ../game_state_backup.rs
```

- [ ] **Step 2: Delete the files**

```bash
rm -rf src/game/gameplay_scene/
rm src/game/game_state.rs
```

- [ ] **Step 3: Remove module declarations**

In `src/game/mod.rs`, remove:
```rust
pub mod game_state;
pub mod gameplay_scene;
```

- [ ] **Step 4: Remove any remaining imports**

Search for and remove any imports of these modules:
```bash
grep -r "game_state\|gameplay_scene" src/ --include="*.rs"
```

Remove all import statements and references.

---

## Task 4: Clean up persist.rs

**Files:**
- Modify: `src/game/persist.rs`

- [ ] **Step 1: Remove GameState persist functions**

Delete these functions (they handle GameState serialization):
- `state_to_proto()`
- `load_from_path()` (GameState version)
- Any tests specific to GameState persistence

- [ ] **Step 2: Remove GameState imports**

Remove any imports of `game_state` module.

- [ ] **Step 3: Keep only ECS persist functions**

Ensure only these remain:
- `ecs_save_game()`
- `ecs_load_game()`
- `ecs_to_proto()`
- `proto_to_ecs()`

- [ ] **Step 4: Verify compile**

```bash
cargo check 2>&1 | grep "^error"
```

Expected: no errors related to persist functions.

---

## Task 5: Remove GameState-dependent tests

**Files:**
- Various test files

- [ ] **Step 1: Find tests using GameState**

```bash
find . -name "*.rs" -exec grep -l "GameState\|gameplay_scene" {} \;
```

- [ ] **Step 2: Remove or migrate tests**

For each test file:
1. If the test only tests GameState functionality - delete it
2. If the test tests core game logic - migrate it to use ECS
3. If the test is for a feature that should be preserved - rewrite it for ECS

- [ ] **Step 3: Special attention to these files**

- `src/game/combat.rs` tests - migrate to ECS combat tests
- `src/game/magic.rs` tests - migrate to ECS magic tests  
- `src/game/shop.rs` tests - migrate to ECS shop tests
- Integration tests that use GameState - rewrite for ECS

- [ ] **Step 4: Verify test suite still meaningful**

```bash
cargo test --dry-run 2>&1 | grep "test"
```

Ensure we still have adequate test coverage after cleanup.

---

## Task 6: Update AGENTS.md

**Files:**
- Modify: `AGENTS.md`

- [ ] **Step 1: Remove GameState references**

Remove any mentions of GameState, GameplayScene, or the dual-path architecture.

- [ ] **Step 2: Update architecture description**

Change from dual-path to ECS-only:
```markdown
## Current Architecture

The game now uses a pure ECS architecture:
- `EcsScene` implements the `Scene` trait
- `hecs::World` stores all entities and components
- `Resources` struct holds global state
- Systems run in a fixed schedule at 15 Hz
- Rendering occurs at presentation frame rate
```

- [ ] **Step 3: Update agent guidance**

Remove guidance about choosing between ECS and legacy paths. Update examples to use ECS only.

- [ ] **Step 4: Update file references**

Change any file path references that point to deleted files.

---

## Task 7: Final compilation and testing

- [ ] **Step 1: Full build check**

```bash
cargo clean  # Ensure clean build
cargo build 2>&1 | grep "^error"
```

Expected: no errors. If there are errors, fix them before proceeding.

- [ ] **Step 2: Run complete test suite**

```bash
cargo test 2>&1 | grep "^test result"
```

Expected: all tests pass. Note that test count will be lower since GameState tests are removed.

- [ ] **Step 3: Integration smoke test**

```bash
cargo run 2>&1 | head -10
```

Expected: game starts without --ecs flag, fully functional.

- [ ] **Step 4: Feature verification run**

Play the game for 10-15 minutes to ensure:
- All menus work
- Combat functions
- Save/load works
- Region transitions work
- Audio plays
- Debug TUI accessible

---

## Task 8: Documentation updates

- [ ] **Step 1: Update README.md if needed**

If README mentions the ECS migration or --ecs flag, update it to reflect completion.

- [ ] **Step 2: Update any development docs**

Check for any docs in `docs/` that reference the migration status.

- [ ] **Step 3: Add migration completion note**

Consider adding a note to project history about ECS migration completion.

---

## Task 9: Performance verification

- [ ] **Step 1: Check performance hasn't regressed**

The ECS implementation should perform similarly to or better than the legacy implementation.

```bash
# If you have performance benchmarks:
cargo bench
```

- [ ] **Step 2: Memory usage check**

Ensure memory usage is reasonable and no leaks exist.

---

## Task 10: Final commit and cleanup

- [ ] **Step 1: Stage all changes**

```bash
git add -A
```

- [ ] **Step 2: Review changes**

```bash
git diff --cached --stat
```

Ensure only expected files are changed/removed.

- [ ] **Step 3: Commit the migration completion**

```bash
git commit -m "feat: complete ECS migration - remove legacy GameState path

- Remove --ecs flag gating; ECS is now the only path
- Delete GameState and GameplayScene implementations
- Remove all GameState-dependent tests and code
- Update AGENTS.md to reflect ECS-only architecture
- Clean up persist.rs to use only ECS functions
- Verify 100% feature parity between old and new implementations

The game now runs exclusively on the ECS architecture with
hecs::World, Resources, and system-based game logic."
```

- [ ] **Step 4: Tag the milestone (optional)**

```bash
git tag -a ecs-migration-complete -m "Complete ECS migration from GameState to hecs-based architecture"
```

---

## Task 11: Post-migration verification

- [ ] **Step 1: Fresh clone test**

In a fresh directory, clone the repo and verify it builds and runs:

```bash
git clone <repo-url> faery-tale-rs-fresh
cd faery-tale-rs-fresh
cargo build
cargo run
```

- [ ] **Step 2: Check for any remaining legacy references**

```bash
grep -r "GameState\|gameplay_scene\|--ecs" . --exclude-dir=.git
```

Expected: no results (or only in comments/history).

- [ ] **Step 3: Verify all gameplay features still work**

Do a final playthrough to ensure the migration didn't break anything.

---

## Completion check

```bash
cargo build 2>&1 | grep "^error"  # Should be empty
cargo test 2>&1 | grep "^test result"  # Should show passing tests
cargo run 2>&1 | head -5  # Should start the game
grep -r "GameState" src/  # Should find no references
ls src/game/gameplay_scene/ 2>/dev/null || echo "Directory deleted as expected"
ls src/game/game_state.rs 2>/dev/null || echo "File deleted as expected"
```

All checks should pass, confirming successful migration completion.

---

## Context

### What the legacy path is

The legacy path consists of:
- `GameState` - a 400+ field God Object containing all game state
- `GameplayScene` - the main scene implementation using GameState
- Various supporting modules directly manipulating GameState
- Dual-path architecture in main.rs with `--ecs` flag

### What "parity" means

Parity means:
1. Every gameplay feature that worked in the legacy implementation works in ECS
2. Save/load files are compatible (or migration path exists)
3. Performance is equivalent or better
4. All input methods (keyboard, gamepad) work identically
5. All UI elements render correctly
6. Debug functionality is preserved or improved

### Why we can safely delete the legacy path

1. **Comprehensive testing**: ECS systems have been thoroughly tested
2. **Feature completeness**: All gameplay mechanics are implemented in ECS
3. **Better architecture**: ECS is more maintainable and extensible
4. **Reduced complexity**: Single path eliminates branching logic and maintenance burden
5. **Spec compliance**: ECS implementation follows the project specifications more closely

---

## Dependencies
All previous plans complete (G-W). ECS path must be fully functional and tested.

---

## Spec references
- `docs/GUIDELINES.md` - coding standards for the final codebase
- `AGENTS.md` - updated agent guidance for ECS-only architecture
- All subsystem spec files - ensure ECS implementation remains compliant