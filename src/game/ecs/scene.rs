//! [`EcsScene`] — the ECS-based gameplay scene, implementing the [`Scene`] trait.
//!
//! This is the entry point for Plan D: a parallel gameplay implementation that
//! runs the ECS system schedule each tick while the old `GameplayScene` is still
//! the default. `EcsScene` will gradually absorb subsystems from `GameplayScene`
//! over Plans D–F.

use std::any::Any;

use hecs::World;
use sdl3::event::Event;
use sdl3::render::{Canvas, Texture};
use sdl3::video::Window;

use crate::game::debug_tui::DebugConsole;
use crate::game::ecs::components::{HeroStats, Inventory};
use crate::game::ecs::resources::Resources;
use crate::game::ecs::spawn::spawn_hero;
use crate::game::ecs::systems;
use crate::game::game_library::GameLibrary;
use crate::game::scene::{Scene, SceneResources, SceneResult};

use super::debug_commands;

/// ECS-based gameplay scene (Plan D skeleton).
///
/// Owns the `hecs::World` and the singleton `Resources`. Each call to
/// `update()` runs one or more gameplay ticks followed by a render pass.
pub struct EcsScene {
    world:   World,
    res:     Resources,
    console: Option<DebugConsole>,
}

impl EcsScene {
    /// Construct a new `EcsScene`, spawning the hero at the location specified
    /// in `faery.toml` for brother 0 (Julian).  Falls back to `(100, 100)` if
    /// the library has no brother or location data.
    pub fn new(game_lib: &GameLibrary, console: Option<DebugConsole>) -> Self {
        let mut world = World::new();

        // Resolve hero starting position from the library.
        let (start_x, start_y, start_region) = game_lib
            .get_brother(0)
            .and_then(|bro| game_lib.find_location(&bro.spawn))
            .map(|loc| (loc.x as f32, loc.y as f32, loc.region))
            .unwrap_or((100.0, 100.0, 0));

        // Build default hero stats (overridden by library values below).
        let stats = game_lib
            .get_brother(0)
            .map(|bro| HeroStats {
                vitality: 100,
                brave:    bro.brave,
                luck:     bro.luck,
                kind:     bro.kind,
                wealth:   bro.wealth,
                hunger:   0,
                fatigue:  0,
                gold:     0,
            })
            .unwrap_or(HeroStats {
                vitality: 100,
                brave:    50,
                luck:     50,
                kind:     50,
                wealth:   50,
                hunger:   0,
                fatigue:  0,
                gold:     0,
            });

        let hero = spawn_hero(&mut world, start_x, start_y, 0, stats, Inventory::empty());

        let mut res = Resources::new(hero);
        res.region.region_num = start_region;

        Self { world, res, console }
    }

    /// Run one gameplay tick: advance all systems then drain debug commands.
    fn run_tick(&mut self) {
        // ── System schedule (mirrors order in systems/mod.rs) ────────────────
        systems::clock::run(&mut self.world, &mut self.res);
        systems::input::run(&mut self.world, &mut self.res);
        // sleep system not yet ported — skipped
        systems::movement::run(&mut self.world, &mut self.res);
        systems::carrier::run(&mut self.world, &mut self.res);
        systems::collision::run(&self.world, &mut self.res);
        systems::door::run(&self.world, &mut self.res);
        systems::zone::run(&self.world, &mut self.res);
        systems::npc_ai::run(&mut self.world, &mut self.res);
        systems::npc_movement::run(&mut self.world, &mut self.res);
        systems::combat::run(&mut self.world, &mut self.res);
        systems::missile::run(&mut self.world, &mut self.res);
        systems::encounter::run(&mut self.world, &mut self.res);
        systems::proximity::run(&self.world, &mut self.res);
        systems::item::run(&mut self.world, &mut self.res);
        systems::narrative::run(&mut self.world, &mut self.res);
        systems::death::run(&mut self.world, &mut self.res);
        systems::region::run(&mut self.world, &mut self.res);

        // ── Debug command dispatch ────────────────────────────────────────────
        if let Some(console) = &mut self.console {
            for cmd in console.drain_commands() {
                debug_commands::handle(cmd, &mut self.world, &mut self.res);
            }
        }
    }
}

impl Scene for EcsScene {
    fn handle_event(&mut self, _event: &Event) -> bool {
        // TODO(Plan D): forward events to InputSystem once InputState is migrated.
        false
    }

    fn update(
        &mut self,
        _canvas: &mut Canvas<Window>,
        _play_tex: &mut Texture,
        delta_ticks: u32,
        _game_lib: &GameLibrary,
        _resources: &mut SceneResources<'_, '_>,
    ) -> SceneResult {
        // Run one tick per delta unit (capped to avoid spiral-of-death).
        let ticks = delta_ticks.min(4);
        for _ in 0..ticks.max(1) {
            self.run_tick();
        }

        // Render the debug console overlay if present.
        if let Some(console) = &mut self.console {
            console.render();
        }

        SceneResult::Continue
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    // EcsScene::new() requires a GameLibrary loaded from disk, which is not
    // available in unit tests.  The system-level tests live in each system's
    // own module.  Smoke tests for debug_commands are in debug_commands.rs.
}
