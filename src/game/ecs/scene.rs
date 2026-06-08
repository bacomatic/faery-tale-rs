//! [`EcsScene`] — the ECS-based gameplay scene, implementing the [`Scene`] trait.
//!
//! This is the sole gameplay scene since `GameplayScene` was removed.
//! Subsystems are being ported into ECS systems over Plans D–F.

use std::any::Any;
use std::collections::HashSet;

use hecs::World;
use sdl3::event::Event;
use sdl3::render::{Canvas, Texture};
use sdl3::video::Window;

use crate::game::debug_tui::DebugConsole;
use crate::game::direction::Direction;
use crate::game::ecs::components::{HeroStats, Inventory};
use crate::game::ecs::resources::Resources;
use crate::game::ecs::spawn::spawn_hero;
use crate::game::ecs::systems;
use crate::game::game_library::GameLibrary;
use crate::game::scene::{Scene, SceneResources, SceneResult};

use super::debug_commands;

// ── InputState ────────────────────────────────────────────────────────────────

/// Tracks which movement keys are currently held.  Direction flags are derived
/// by summing axis contributions from all held keys so that opposites cancel
/// (e.g. Left+Right → no horizontal movement).
///
/// Port of `InputState` from `gameplay_scene/mod.rs`.
struct InputState {
    up:    bool,
    down:  bool,
    left:  bool,
    right: bool,
    /// Set of movement keycodes currently physically held.
    pressed_movement_keys: HashSet<sdl3::keyboard::Keycode>,
    /// Gamepad left-stick contribution, each axis clamped to {-1, 0, +1}.
    gamepad_x: i32,
    gamepad_y: i32,
}

impl InputState {
    fn new() -> Self {
        Self {
            up:    false,
            down:  false,
            left:  false,
            right: false,
            pressed_movement_keys: HashSet::new(),
            gamepad_x: 0,
            gamepad_y: 0,
        }
    }

    /// Recompute up/down/left/right by summing contributions from all held
    /// movement keys and the gamepad stick.  Opposite directions cancel.
    fn recompute(&mut self) {
        use sdl3::keyboard::Keycode;
        let mut x: i32 = self.gamepad_x;
        let mut y: i32 = self.gamepad_y;
        for kc in &self.pressed_movement_keys {
            let (kx, ky): (i32, i32) = match kc {
                Keycode::Up    | Keycode::Kp8 => ( 0, -1),
                Keycode::Down  | Keycode::Kp2 => ( 0,  1),
                Keycode::Left  | Keycode::Kp4 => (-1,  0),
                Keycode::Right | Keycode::Kp6 => ( 1,  0),
                Keycode::Kp7               => (-1, -1),
                Keycode::Kp9               => ( 1, -1),
                Keycode::Kp1               => (-1,  1),
                Keycode::Kp3               => ( 1,  1),
                _ => (0, 0),
            };
            x += kx;
            y += ky;
        }
        self.up    = y < 0;
        self.down  = y > 0;
        self.left  = x < 0;
        self.right = x > 0;
    }

    /// Decode 8-way direction from current input flags.
    fn to_direction(&self) -> Direction {
        match (self.up, self.down, self.left, self.right) {
            (true,  false, false, false) => Direction::N,
            (true,  false, false, true)  => Direction::NE,
            (false, false, false, true)  => Direction::E,
            (false, true,  false, true)  => Direction::SE,
            (false, true,  false, false) => Direction::S,
            (false, true,  true,  false) => Direction::SW,
            (false, false, true,  false) => Direction::W,
            (true,  false, true,  false) => Direction::NW,
            _                            => Direction::None,
        }
    }
}

/// ECS-based gameplay scene (Plan D skeleton).
///
/// Owns the `hecs::World` and the singleton `Resources`. Each call to
/// `update()` runs one or more gameplay ticks followed by a render pass.
pub struct EcsScene {
    pub world:   World,
    pub res:     Resources,
    console:    Option<DebugConsole>,
    input:      InputState,
    last_mood:  u8,
    mood_tick:  u32,
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

        Self { world, res, console, input: InputState::new(), last_mood: u8::MAX, mood_tick: 0 }
    }

    /// Run one gameplay tick: advance all systems then drain debug commands.
    fn run_tick(&mut self) {
        // ── System schedule (mirrors order in systems/mod.rs) ────────────────
        systems::clock::run(&mut self.world, &mut self.res);
        systems::input::run(&mut self.world, &mut self.res);
        // sleep system not yet ported — skipped
        self.res.input_direction = self.input.to_direction();
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

/// Map current game state to a music group index (0–6).
///
/// Priority order mirrors `setmood()` from `gameplay_scene/game_event.rs`
/// and R-AUDIO-011.  Group indices correspond to 4-track offsets in the songs
/// file: Day=0, Battle=1, Night=2, Zone=4, Dungeon=5, Death=6.
fn compute_mood(
    vitality: i16,
    in_encounter_zone: bool,
    battleflag: bool,
    region_num: u8,
    lightlevel: u16,
) -> u8 {
    if vitality <= 0        { return 6; } // death
    if in_encounter_zone    { return 4; } // zone (astral plane)
    if battleflag           { return 1; } // battle
    if region_num > 7       { return 5; } // dungeon
    if lightlevel > 120     { 0 } else    { 2 } // day / night
}

impl EcsScene {
    /// Drain pending SFX events and update the music mood every 4 ticks
    /// (gameloop-113).  Mirrors the audio block in the old `GameplayScene::update`.
    fn run_audio(&mut self, resources: &mut SceneResources<'_, '_>) {
        // Drain queued SFX events.
        for ev in self.res.events.sfx.drain(..) {
            if let Some(audio) = resources.audio {
                audio.play_sfx(ev.sfx_id);
            }
        }

        // Evaluate mood every 4 ticks (gameloop-113).
        self.mood_tick += 1;
        if self.mood_tick >= 4 {
            self.mood_tick = 0;

            let vitality = self.world
                .get::<&crate::game::ecs::components::HeroStats>(self.res.hero_entity)
                .map(|s| s.vitality)
                .unwrap_or(100);

            let mood = compute_mood(
                vitality,
                self.res.encounter.in_encounter_zone,
                self.res.region.battleflag,
                self.res.region.region_num,
                self.res.clock.lightlevel,
            );

            if mood != self.last_mood {
                self.last_mood = mood;
                if let Some(audio) = resources.audio {
                    audio.set_score(mood);
                }
            }
        }
    }
}

impl Scene for EcsScene {
    fn handle_event(&mut self, event: &Event) -> bool {
        use sdl3::keyboard::Keycode;
        match event {
            Event::KeyDown { keycode: Some(kc), repeat: false, .. } => {
                match kc {
                    Keycode::Up    | Keycode::Kp8
                    | Keycode::Down  | Keycode::Kp2
                    | Keycode::Left  | Keycode::Kp4
                    | Keycode::Right | Keycode::Kp6
                    | Keycode::Kp7   | Keycode::Kp9
                    | Keycode::Kp1   | Keycode::Kp3 => {
                        self.input.pressed_movement_keys.insert(*kc);
                        self.input.recompute();
                        true
                    }
                    _ => false,
                }
            }
            Event::KeyUp { keycode: Some(kc), .. } => {
                match kc {
                    Keycode::Up    | Keycode::Kp8
                    | Keycode::Down  | Keycode::Kp2
                    | Keycode::Left  | Keycode::Kp4
                    | Keycode::Right | Keycode::Kp6
                    | Keycode::Kp7   | Keycode::Kp9
                    | Keycode::Kp1   | Keycode::Kp3 => {
                        self.input.pressed_movement_keys.remove(kc);
                        self.input.recompute();
                        true
                    }
                    _ => false,
                }
            }
            // Gamepad left stick → aggregate into direction.
            Event::ControllerAxisMotion { axis, value, .. } => {
                use sdl3::gamepad::Axis;
                const THRESHOLD: i16 = 8000;
                match axis {
                    Axis::LeftX => {
                        self.input.gamepad_x = if *value < -THRESHOLD { -1 }
                            else if *value > THRESHOLD { 1 } else { 0 };
                        self.input.recompute();
                        true
                    }
                    Axis::LeftY => {
                        self.input.gamepad_y = if *value < -THRESHOLD { -1 }
                            else if *value > THRESHOLD { 1 } else { 0 };
                        self.input.recompute();
                        true
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn update(
        &mut self,
        _canvas: &mut Canvas<Window>,
        _play_tex: &mut Texture,
        delta_ticks: u32,
        _game_lib: &GameLibrary,
        resources: &mut SceneResources<'_, '_>,
    ) -> SceneResult {
        // Run one tick per delta unit (capped to avoid spiral-of-death).
        let ticks = delta_ticks.min(4);
        for _ in 0..ticks.max(1) {
            self.run_tick();
        }

        self.run_audio(resources);

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
