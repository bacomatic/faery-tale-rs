# ECS Migration Plan D: ECS Scene and main.rs Integration

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Create `EcsScene` implementing the `Scene` trait, wire it into `main.rs`, verify gameplay parity with `GameplayScene`, then delete `GameplayScene` and all code it exclusively owned.

**Architecture:** `EcsScene` owns a `hecs::World` and a `Resources` struct. Its `update()` method runs the system schedule from Plan C. `main.rs` is updated to construct `EcsScene` instead of `GameplayScene`. Once parity is confirmed, `GameplayScene` and `GameState` are deleted.

**Prerequisites:** Plans A, B, C complete. All systems implemented and tested.

**Tech Stack:** Rust 2021, `hecs = "0.11"`, SDL3.

---

## File map

| File | Change |
|---|---|
| `src/game/ecs/scene.rs` | **Create** — `EcsScene` implementing `Scene` |
| `src/game/ecs/mod.rs` | Add `pub mod scene;` |
| `src/main.rs` | Replace `GameplayScene` construction with `EcsScene` |
| `src/game/gameplay_scene/` | **Delete** entire directory (after parity confirmed) |
| `src/game/game_state.rs` | **Delete** (after parity confirmed) |
| `src/game/actor.rs` | Keep (component types still used directly) |
| `src/game/npc.rs` | Keep `Npc`, `NpcTable` for region loading; `NpcTable::load()` used by RegionSystem |

---

## Task 1: Create `EcsScene`

**Files:**
- Create: `src/game/ecs/scene.rs`
- Modify: `src/game/ecs/mod.rs`

- [ ] **Step 1: Create `src/game/ecs/scene.rs`**

```rust
//! EcsScene — the main gameplay scene backed by a hecs::World.
//! Replaces GameplayScene. Implements the Scene trait.

use hecs::World;
use sdl3::render::{Canvas, Texture};
use sdl3::video::Window;

use crate::game::ecs::resources::Resources;
use crate::game::ecs::spawn::spawn_hero;
use crate::game::ecs::components::{HeroStats, Inventory};
use crate::game::ecs::systems;
use crate::game::scene::{Scene, SceneResources, SceneResult};
use crate::game::game_library::GameLibrary;
use crate::game::debug_tui::DebugConsole;
use crate::game::adf::AdfDisk;

pub struct EcsScene {
    pub world:   World,
    pub res:     Resources,
    tick_accum:  u32,
    debug:       DebugConsole,
}

/// Gameplay ticks per second (NTSC 15 Hz).
const TICKS_PER_SEC: u32 = 15;
/// Milliseconds per gameplay tick.
const MS_PER_TICK: u32 = 1000 / TICKS_PER_SEC;

impl EcsScene {
    /// Construct a new ECS scene and spawn the hero at the given brother's start position.
    pub fn new(
        brother_id: u8,
        game_lib: &GameLibrary,
        adf: AdfDisk,
    ) -> Self {
        let mut world = World::new();

        // Derive starting stats and position from GameLibrary.
        let cfg = &game_lib.brothers[brother_id as usize];
        let stats = HeroStats {
            vitality: 100,
            brave:    cfg.brave as i16,
            luck:     cfg.luck  as i16,
            kind:     cfg.kind  as i16,
            wealth:   cfg.wealth as i16,
            hunger:   0,
            fatigue:  0,
            gold:     0,
        };
        let hero = spawn_hero(
            &mut world,
            cfg.spawn.x as f32,
            cfg.spawn.y as f32,
            brother_id,
            stats,
            Inventory::empty(),
        );

        let mut res = Resources::new(hero);
        res.region.region_num = cfg.spawn.region;
        res.region.new_region = cfg.spawn.region;

        Self {
            world,
            res,
            tick_accum: 0,
            debug: DebugConsole::new(),
        }
    }
}

impl Scene for EcsScene {
    fn handle_event(&mut self, event: &sdl3::event::Event) -> bool {
        // InputSystem handles SDL events directly from the event queue in its run().
        // For key/gamepad events we update InputState here.
        systems::input::handle_event(&mut self.res, event)
    }

    fn update(
        &mut self,
        canvas: &mut Canvas<Window>,
        play_tex: &mut Texture,
        delta_ticks: u32,
        game_lib: &GameLibrary,
        resources: &mut SceneResources<'_, '_>,
    ) -> SceneResult {
        self.tick_accum += delta_ticks;

        // Process pending debug commands.
        for cmd in self.debug.poll_commands() {
            crate::game::ecs::debug_commands::handle(cmd, &mut self.world, &mut self.res);
        }

        // Run gameplay ticks at 15 Hz.
        while self.tick_accum >= MS_PER_TICK {
            self.tick_accum -= MS_PER_TICK;
            self.run_tick(game_lib);
        }

        // Render at presentation frame rate.
        self.render(canvas, play_tex, game_lib, resources);

        // Debug TUI render (no-op when feature disabled).
        self.debug.render(&self.world, &self.res, canvas);

        if self.res.view.viewstatus == 99 {
            // Force redraw consumed.
            self.res.view.viewstatus = 0;
        }

        if self.res.region.new_region != self.res.region.region_num {
            systems::region::run(&mut self.world, &mut self.res, game_lib);
        }

        SceneResult::Continue
    }

    fn on_exit(&mut self) {}

    fn as_any(&self) -> &dyn std::any::Any { self }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}

impl EcsScene {
    fn run_tick(&mut self, game_lib: &GameLibrary) {
        // Clear event queues from previous tick.
        self.res.events.clear();

        // System schedule (15 Hz).
        systems::clock::run(&mut self.world, &mut self.res);
        systems::input::run(&mut self.world, &mut self.res);

        if self.res.encounter.sleeping {
            systems::sleep::run(&mut self.world, &mut self.res);
            return;
        }

        systems::movement::run(&mut self.world, &mut self.res);
        systems::carrier::run(&mut self.world, &mut self.res);
        systems::collision::run(&self.world, &mut self.res);
        systems::door::run(&mut self.world, &mut self.res, game_lib);
        systems::zone::run(&self.world, &mut self.res, game_lib);
        systems::npc_ai::run(&mut self.world, &mut self.res);
        systems::npc_movement::run(&mut self.world, &mut self.res);
        systems::combat::run(&mut self.world, &mut self.res);
        systems::missile::run(&mut self.world, &mut self.res);
        systems::encounter::run(&mut self.world, &mut self.res, game_lib);
        systems::proximity::run(&mut self.world, &mut self.res, game_lib);
        systems::item::run(&mut self.world, &mut self.res, game_lib);
        systems::narrative::run(&mut self.world, &mut self.res);
        systems::death::run(&mut self.world, &mut self.res);

        // Drain event queues into consumers.
        self.drain_events(game_lib);
    }

    fn drain_events(&mut self, game_lib: &GameLibrary) {
        // Messages → MessageQueue
        for ev in self.res.events.message.drain(..) {
            self.res.messages.push(ev.text);
        }
        // Speech → MessageQueue
        for ev in self.res.events.speech.drain(..) {
            let text = crate::game::events::speak(&self.res.narr, ev.speech_id, &ev.brother_name);
            self.res.messages.push(text);
        }
        // SFX → pending_sfx
        for ev in self.res.events.sfx.drain(..) {
            self.res.pending_sfx.push(ev.sfx_id);
        }
        // BrotherDied → spawn Bones + spawn new hero
        for ev in self.res.events.brother.drain(..) {
            crate::game::ecs::spawn::spawn_bones(
                &mut self.world,
                ev.x, ev.y,
                self.res.region.region_num,
                ev.brother_id,
                ev.stuff,
            );
            // Spawn next living brother.
            let next_id = self.next_living_brother(ev.brother_id, game_lib);
            if let Some(id) = next_id {
                let cfg = &game_lib.brothers[id as usize];
                let stats = self.starting_stats_for_brother(id, game_lib);
                let inv = Inventory { stuff: self.res.brother.inactive_inventories[id as usize] };
                world.despawn(self.res.hero_entity).ok();
                let new_hero = crate::game::ecs::spawn::spawn_hero(
                    &mut self.world,
                    cfg.spawn.x as f32,
                    cfg.spawn.y as f32,
                    id,
                    stats,
                    inv,
                );
                self.res.hero_entity = new_hero;
                self.res.brother.active_brother = id as usize;
            }
            // If no living brothers remain → game over (handled by ViewState).
        }
    }

    fn next_living_brother(&self, just_died: u8, game_lib: &GameLibrary) -> Option<u8> {
        // Brothers succeed in order: Julian(0) → Phillip(1) → Kevin(2).
        let dead_ids: std::collections::HashSet<u8> = self.world
            .query::<&crate::game::ecs::components::BrotherKind>()
            .with::<&crate::game::ecs::components::Bones>()
            .iter()
            .map(|(_, bk)| bk.id)
            .chain(std::iter::once(just_died))
            .collect();
        (0u8..3).find(|id| !dead_ids.contains(id))
    }

    fn starting_stats_for_brother(&self, id: u8, game_lib: &GameLibrary) -> HeroStats {
        let cfg = &game_lib.brothers[id as usize];
        HeroStats {
            vitality: 100,
            brave:    cfg.brave  as i16,
            luck:     cfg.luck   as i16,
            kind:     cfg.kind   as i16,
            wealth:   cfg.wealth as i16,
            hunger:   0,
            fatigue:  0,
            gold:     0,
        }
    }

    fn render(
        &mut self,
        canvas: &mut Canvas<Window>,
        play_tex: &mut Texture,
        game_lib: &GameLibrary,
        resources: &mut SceneResources<'_, '_>,
    ) {
        systems::render::palette::run(&self.world, &mut self.res, game_lib);
        systems::render::map::run(&self.world, &mut self.res, canvas, play_tex);
        systems::render::sprite::run(&self.world, &self.res, canvas);
        systems::render::hibar::run(&self.world, &self.res, canvas, resources);
    }
}
```

- [ ] **Step 2: Add to `src/game/ecs/mod.rs`**

```rust
pub mod scene;
pub use scene::EcsScene;
```

- [ ] **Step 3: Create `src/game/ecs/debug_commands.rs`**

```rust
//! Handles DebugCommand values from the debug console.
use hecs::World;
use crate::game::ecs::resources::Resources;
use crate::game::ecs::components::{HeroStats, Position};
use crate::game::debug_tui::DebugCommand;

pub fn handle(cmd: DebugCommand, world: &mut World, res: &mut Resources) {
    match cmd {
        DebugCommand::SetGodMode(flags) => {
            res.brother.god_mode = flags;
        }
        DebugCommand::Teleport { x, y } => {
            if let Ok(mut pos) = world.get_mut::<Position>(res.hero_entity) {
                pos.set(x as f32, y as f32);
            }
        }
        DebugCommand::SetVitality(v) => {
            if let Ok(mut stats) = world.get_mut::<HeroStats>(res.hero_entity) {
                stats.vitality = v;
            }
        }
        // Add remaining commands as they are ported from GameplayScene debug_commands.rs.
        _ => {}
    }
}
```

- [ ] **Step 4: Add `pub mod debug_commands;` to `src/game/ecs/mod.rs`**

- [ ] **Step 5: Fix compile errors**

```bash
cargo check 2>&1 | grep "^error" | head -30
```

Most expected errors:
- Missing `systems::input::handle_event` — stub it returning `false` until InputSystem is complete.
- Missing `systems::sleep` — add stub module.
- Missing `self.res.messages`, `self.res.narr`, `self.res.pending_sfx` — add these fields to `Resources` if not already present.
- `game_lib.brothers[id].spawn` — verify field name against `BrotherConfig` in `game_library.rs`.

Fix each error. Add stub implementations where needed.

- [ ] **Step 6: Verify compile**

```bash
cargo build 2>&1 | grep "^error"
```
Expected: no errors.

- [ ] **Step 7: Commit**

```bash
git add src/game/ecs/scene.rs src/game/ecs/debug_commands.rs src/game/ecs/mod.rs
git commit -m "feat(ecs): EcsScene implementing Scene trait with full system schedule"
```

---

## Task 2: Wire `EcsScene` into `main.rs`

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Add `EcsScene` construction in `main.rs`**

Find where `GameplayScene::new(...)` is constructed in `main.rs`. Add a parallel construction of `EcsScene` guarded by a runtime flag or compile-time feature. The simplest approach is a `--ecs` CLI argument:

```rust
// In main(), after argument parsing:
let use_ecs = std::env::args().any(|a| a == "--ecs");

let initial_scene: Box<dyn Scene> = if use_ecs {
    Box::new(crate::game::ecs::EcsScene::new(
        0, // Julian
        &game_lib,
        adf,
    ))
} else {
    Box::new(GameplayScene::new(...))
};
```

- [ ] **Step 2: Thread `AdfDisk` through to `EcsScene`**

`EcsScene::new` needs an `AdfDisk`. Check how `GameplayScene` receives it and mirror the pattern.

- [ ] **Step 3: Run the game with `--ecs` flag**

```bash
cargo run -- --ecs 2>&1 | head -20
```
The game should start without panicking. It will likely render a black screen until all render systems are ported, but it must not crash.

- [ ] **Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: wire EcsScene into main.rs behind --ecs flag"
```

---

## Task 3: Debug TUI with EcsScene

**Files:**
- Modify: `src/game/ecs/scene.rs`

The `DebugConsole::render(&world, &res, canvas)` call in `EcsScene::update()` is already present from Task 1. Verify it compiles and the TUI is usable when `--ecs` and `debug-tui` feature flags are both active:

- [ ] **Step 1: Build with debug-tui feature**

```bash
cargo build --features debug-tui 2>&1 | grep "^error"
```
Expected: no errors.

- [ ] **Step 2: Run with both flags**

```bash
cargo run --features debug-tui -- --ecs 2>&1 | head -5
```
Expected: game starts.

- [ ] **Step 3: Commit if needed**

```bash
git add -A
git commit -m "feat(ecs): debug TUI renders from World + Resources in EcsScene"
```

---

## Task 4: Gameplay parity verification

Before deleting `GameplayScene`, verify that `EcsScene` produces equivalent behavior for key scenarios. This is manual testing with the running game. Run both scenes and compare:

- [ ] **Hero spawns at correct position for each brother**
- [ ] **Movement: hero walks in all 8 directions**
- [ ] **Day/night cycle advances visually**
- [ ] **Enemies spawn and pursue hero**
- [ ] **Combat: melee and ranged work**
- [ ] **Item pickup works**
- [ ] **Region transition occurs when hero walks to boundary**
- [ ] **Death and brother succession: Julian dies, Phillip spawns**
- [ ] **Bones entity appears at Julian's death position**
- [ ] **Phillip finds Bones and loots Julian's inventory**
- [ ] **Save and load round-trip (Plan E prerequisite)**

Note any discrepancies as GitHub issues before proceeding to deletion.

---

## Task 5: Delete `GameplayScene` and `GameState`

**Only perform this task after Task 4 parity verification is complete.**

- [ ] **Step 1: Remove `--ecs` flag gating in `main.rs`**

`EcsScene` becomes the only path. Delete the `use_ecs` branch.

- [ ] **Step 2: Delete `GameplayScene` files**

```bash
rm -r src/game/gameplay_scene/
```

- [ ] **Step 3: Delete `GameState`**

```bash
rm src/game/game_state.rs
```

- [ ] **Step 4: Remove module declarations**

In `src/game/mod.rs`, remove:
```rust
pub mod game_state;
pub mod gameplay_scene;
```

- [ ] **Step 5: Fix all resulting compile errors**

```bash
cargo check 2>&1 | grep "^error" | head -30
```

These will be references to `GameState`, `GameplayScene`, `Actor` fields, etc. from:
- `src/main.rs` (snapshot building — gone in Plan F)
- `src/game/persist.rs` (serialization — replaced in Plan E)
- `src/game/debug_tui/bridge.rs` (snapshot types — removed in Plan F)
- Any remaining test that imports from `gameplay_scene`

Fix each error by removing the reference or replacing it with the ECS equivalent.

- [ ] **Step 6: Run full test suite**

```bash
cargo test 2>&1 | grep "^test result"
```
Expected: all test suites pass. The gameplay_scene test count (~685 tests) will disappear from the count — that is expected, as those tests tested `GameplayScene` which no longer exists. The ECS system tests from Plan C replace them.

- [ ] **Step 7: Final commit**

```bash
git add -A
git commit -m "feat(ecs): delete GameplayScene and GameState — EcsScene is now the only gameplay path"
```

---

## Completion check

```bash
cargo build 2>&1 | grep "^error"
cargo test 2>&1 | grep "^test result"
cargo run -- 2>&1 | head -5
```

All three succeed. `GameplayScene` and `GameState` no longer exist in the codebase.
