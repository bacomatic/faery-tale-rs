use std::any::Any;

use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;

use crate::game::actor::ActorState;
use crate::game::collision;
use crate::game::debug_command::{DebugCommand, GodModeFlags, MagicEffect, StatId};
use crate::game::game_library::GameLibrary;
use crate::game::game_state::GameState;
use crate::game::key_bindings::GameAction;
use crate::game::scene::{Scene, SceneResources, SceneResult};

/// 8-way movement direction decoded from input state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    N, NE, E, SE, S, SW, W, NW, None,
}

/// Tracks which movement/action keys are currently held down.
struct InputState {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    fight: bool,
}

impl Default for InputState {
    fn default() -> Self {
        InputState { up: false, down: false, left: false, right: false, fight: false }
    }
}

pub struct GameplayScene {
    pub state: Box<GameState>,
    tick_accum: u32,
    autosave_enabled: bool,
    input: InputState,
    map_x: u16,
    map_y: u16,
}

impl GameplayScene {
    pub fn new() -> Self {
        GameplayScene {
            state: Box::new(GameState::new()),
            tick_accum: 0,
            autosave_enabled: true,
            input: InputState::default(),
            map_x: 0,
            map_y: 0,
        }
    }

    /// Decode 8-way direction from current input flags.
    fn current_direction(&self) -> Direction {
        match (self.input.up, self.input.down, self.input.left, self.input.right) {
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

    /// Apply player input: move hero and update actor facing/state.
    fn apply_player_input(&mut self) {
        let dir = self.current_direction();

        let (dx, dy): (i32, i32) = match dir {
            Direction::N    => ( 0, -1),
            Direction::NE   => ( 1, -1),
            Direction::E    => ( 1,  0),
            Direction::SE   => ( 1,  1),
            Direction::S    => ( 0,  1),
            Direction::SW   => (-1,  1),
            Direction::W    => (-1,  0),
            Direction::NW   => (-1, -1),
            Direction::None => ( 0,  0),
        };

        if dir != Direction::None {
            let new_x = (self.state.hero_x as i32 + dx * 2).clamp(0, u16::MAX as i32) as u16;
            let new_y = (self.state.hero_y as i32 + dy * 2).clamp(0, u16::MAX as i32) as u16;

            if !collision::proxcheck(&self.state, new_x, new_y) {
                self.state.hero_x = new_x;
                self.state.hero_y = new_y;
            }

            let facing: u8 = match dir {
                Direction::N  => 0,
                Direction::NE => 1,
                Direction::E  => 2,
                Direction::SE => 3,
                Direction::S  => 4,
                Direction::SW => 5,
                Direction::W  => 6,
                Direction::NW => 7,
                Direction::None => 0,
            };

            if let Some(player) = self.state.actors.first_mut() {
                player.facing = facing;
                if self.input.fight {
                    player.state = ActorState::Fighting(0);
                } else {
                    player.state = ActorState::Walking;
                }
            }
        } else if self.input.fight {
            if let Some(player) = self.state.actors.first_mut() {
                player.state = ActorState::Fighting(0);
            }
        } else if let Some(player) = self.state.actors.first_mut() {
            player.state = ActorState::Still;
        }
    }

    /// Advance all active actors by one frame (AI stub).
    fn update_actors(&mut self, _delta: u32) {
        for _actor in self.state.actors[0..self.state.anix].iter_mut() {
            // TODO: npc-002 will add AI here
        }
    }

    /// Clear and color the canvas according to the current viewstatus mode.
    fn render_by_viewstatus(&self, canvas: &mut Canvas<Window>) {
        match self.state.viewstatus {
            // Normal play or forced redraw
            0 | 98 | 99 => {
                canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 64));
                canvas.clear();
            }
            // Map view
            1 => {
                canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 48, 0));
                canvas.clear();
                // "MAP VIEW" — text rendering pending font wiring
            }
            // Message overlay
            2 => {
                canvas.set_draw_color(sdl2::pixels::Color::RGB(48, 48, 48));
                canvas.clear();
                // "MESSAGE" — text rendering pending font wiring
            }
            // Inventory screen
            4 => {
                canvas.set_draw_color(sdl2::pixels::Color::RGB(64, 32, 0));
                canvas.clear();
                // "INVENTORY" — text rendering pending font wiring
            }
            _ => {
                canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 64));
                canvas.clear();
            }
        }
    }

    /// Dispatch a game menu/command action (stub — will be fleshed out per-action later).
    fn do_option(&mut self, action: GameAction) {
        eprintln!("do_option: {:?}", action);
    }

    pub fn apply_command(&mut self, cmd: DebugCommand) {
        use DebugCommand::*;
        match cmd {
            SetStat { stat, value } => {
                let field = Self::stat_field_mut(&mut self.state, stat);
                *field = value;
            }
            AdjustStat { stat, delta } => {
                let field = Self::stat_field_mut(&mut self.state, stat);
                *field = field.saturating_add(delta);
            }
            SetInventory { index, value } => {
                let stuff = self.state.stuff_mut();
                if (index as usize) < stuff.len() {
                    stuff[index as usize] = value;
                }
            }
            AdjustInventory { index, delta } => {
                let stuff = self.state.stuff_mut();
                if (index as usize) < stuff.len() {
                    stuff[index as usize] = stuff[index as usize].saturating_add_signed(delta);
                }
            }
            TeleportSafe => {
                self.state.hero_x = self.state.safe_x;
                self.state.hero_y = self.state.safe_y;
            }
            TeleportCoords { x, y } => {
                self.state.hero_x = x;
                self.state.hero_y = y;
            }
            TeleportStoneRing { index } => {
                eprintln!("debug command not yet wired: TeleportStoneRing {{ index: {} }}", index);
            }
            ToggleMagicEffect { effect } => match effect {
                MagicEffect::Light => self.state.light_sticky = !self.state.light_sticky,
                MagicEffect::Secret => self.state.secret_sticky = !self.state.secret_sticky,
                MagicEffect::Freeze => self.state.freeze_sticky = !self.state.freeze_sticky,
            },
            SetGodMode { flags } => {
                self.state.god_mode = flags;
            }
            SetDayPhase { phase } => {
                self.state.daynight = phase;
            }
            SetGameTime { hour, minute } => {
                // Each hour = 1000 daynight ticks; each minute ≈ 1000/60
                let ticks = (hour as u16).saturating_mul(1000)
                    + (minute as u16).saturating_mul(1000) / 60;
                self.state.daynight = ticks % 24000;
            }
            HoldTimeOfDay { hold } => {
                self.state.freeze_sticky = hold;
            }
            ToggleAutosave { enable } => {
                self.autosave_enabled = enable;
            }
            cmd => {
                eprintln!("debug command not yet wired: {:?}", cmd);
            }
        }
    }

    fn stat_field_mut(state: &mut GameState, stat: StatId) -> &mut i16 {
        match stat {
            StatId::Vitality => &mut state.vitality,
            StatId::Brave => &mut state.brave,
            StatId::Luck => &mut state.luck,
            StatId::Kind => &mut state.kind,
            StatId::Wealth => &mut state.wealth,
            StatId::Hunger => &mut state.hunger,
            StatId::Fatigue => &mut state.fatigue,
        }
    }
}

impl Scene for GameplayScene {
    fn handle_event(&mut self, event: &Event) -> bool {
        match event {
            Event::KeyDown { keycode: Some(kc), repeat: false, .. } => match *kc {
                // Movement keys
                Keycode::Up | Keycode::W | Keycode::Kp8 => { self.input.up = true; true }
                Keycode::Down | Keycode::S | Keycode::Kp2 => { self.input.down = true; true }
                Keycode::Left | Keycode::A | Keycode::Kp4 => { self.input.left = true; true }
                Keycode::Right | Keycode::D | Keycode::Kp6 => { self.input.right = true; true }
                Keycode::Space | Keycode::F => { self.input.fight = true; true }
                // Game command keys
                Keycode::I => { self.do_option(GameAction::Inventory); true }
                Keycode::L => { self.do_option(GameAction::Look); true }
                Keycode::T => { self.do_option(GameAction::Take); true }
                Keycode::G => { self.do_option(GameAction::Give); true }
                Keycode::Y => { self.do_option(GameAction::Yell); true }
                Keycode::K => { self.do_option(GameAction::Speak); true }
                Keycode::M => { self.do_option(GameAction::Map); true }
                _ => false,
            },
            Event::KeyUp { keycode: Some(kc), .. } => match *kc {
                Keycode::Up | Keycode::W | Keycode::Kp8 => { self.input.up = false; true }
                Keycode::Down | Keycode::S | Keycode::Kp2 => { self.input.down = false; true }
                Keycode::Left | Keycode::A | Keycode::Kp4 => { self.input.left = false; true }
                Keycode::Right | Keycode::D | Keycode::Kp6 => { self.input.right = false; true }
                Keycode::Space | Keycode::F => { self.input.fight = false; true }
                _ => false,
            },
            _ => false,
        }
    }

    fn update(
        &mut self,
        canvas: &mut Canvas<Window>,
        _play_tex: &mut Texture,
        delta_ticks: u32,
        _game_lib: &GameLibrary,
        _resources: &mut SceneResources<'_, '_>,
    ) -> SceneResult {
        self.tick_accum += delta_ticks;
        self.state.tick(delta_ticks);

        // Autosave every 3600 ticks (~60s at 60Hz)
        if self.autosave_enabled && self.state.tick_counter % 3600 == 0 && self.state.tick_counter > 0 {
            if let Err(e) = crate::game::persist::save_game(&self.state, 0) {
                eprintln!("autosave failed: {e}");
            }
        }

        // Death / revive cycle (gameloop-106)
        if self.state.vitality <= 0 && !self.state.god_mode.contains(GodModeFlags::INVINCIBLE) {
            if let Some(next) = self.state.next_brother() {
                self.state.activate_brother(next);
                // TODO: trigger brother-transition placard (gameloop-104 handles scene transition)
                eprintln!("Brother died, switching to brother {}", next);
            } else {
                // All brothers dead — game over
                // TODO: return SceneResult::Done to trigger game over scene
                eprintln!("All brothers dead — GAME OVER");
            }
        }

        self.apply_player_input();
        self.update_actors(delta_ticks);

        // Camera: center hero in 288×160 viewport (gameloop-110)
        self.map_x = self.state.hero_x.saturating_sub(144);
        self.map_y = self.state.hero_y.saturating_sub(80);
        self.state.map_x = self.map_x;
        self.state.map_y = self.map_y;

        self.render_by_viewstatus(canvas);
        canvas.present();
        SceneResult::Continue
    }

    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}
