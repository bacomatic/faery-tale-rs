use std::any::Any;

use sdl2::event::Event;
use sdl2::render::{Canvas, Texture};
use sdl2::video::Window;

use crate::game::debug_command::{DebugCommand, MagicEffect, StatId};
use crate::game::game_library::GameLibrary;
use crate::game::game_state::GameState;
use crate::game::scene::{Scene, SceneResources, SceneResult};

pub struct GameplayScene {
    pub state: Box<GameState>,
    tick_accum: u32,
    autosave_enabled: bool,
}

impl GameplayScene {
    pub fn new() -> Self {
        GameplayScene {
            state: Box::new(GameState::new()),
            tick_accum: 0,
            autosave_enabled: true,
        }
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
    fn handle_event(&mut self, _event: &Event) -> bool {
        false
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
        canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 64));
        canvas.clear();
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
