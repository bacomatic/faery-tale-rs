use crate::game::map_renderer::MapRenderer;
use crate::game::message_queue::MessageQueue;
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
use crate::game::key_bindings::{GameAction, KeyBindings};
use crate::game::scene::{Scene, SceneResources, SceneResult};

/// State for the key rebinding mode (F2 to enter, Escape to exit).
pub struct RebindingState {
    pub active: bool,
    pub waiting_for_action: Option<GameAction>,
}

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
    pub messages: MessageQueue,
    tick_accum: u32,
    autosave_enabled: bool,
    input: InputState,
    map_x: u16,
    map_y: u16,
    last_mood: u8,
    mood_tick: u32,
    map_renderer: Option<MapRenderer>,
    map_world: Option<crate::game::world_data::WorldData>,
    rebinding: RebindingState,
    local_bindings: KeyBindings,
    last_region_num: u8,
    palette_transition: Option<crate::game::palette::PaletteTransition>,
    last_indoor: bool,
    pub in_encounter_zone: bool,
}

impl GameplayScene {
    pub fn new() -> Self {
        GameplayScene {
            state: Box::new(GameState::new()),
            messages: MessageQueue::new(),
            tick_accum: 0,
            autosave_enabled: true,
            input: InputState::default(),
            map_x: 0,
            map_y: 0,
            last_mood: u8::MAX,
            mood_tick: 0,
            map_renderer: None,
            map_world: None,
            rebinding: RebindingState { active: false, waiting_for_action: None },
            local_bindings: KeyBindings::default_bindings(),
            last_region_num: u8::MAX,
            palette_transition: None,
            last_indoor: false,
            in_encounter_zone: false,
        }
    }

    /// Returns true when it is daytime (lightlevel > 60).
    pub fn is_daytime(state: &GameState) -> bool {
        state.lightlevel > 60
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

        let prev_x = self.state.hero_x;
        let prev_y = self.state.hero_y;

        if dir != Direction::None {
            // Speed: flying=4px, water terrain (type 2-5)=1px, default=2px.
            let speed: i32 = if self.state.flying != 0 {
                4
            } else if let Some(ref world) = self.map_world {
                let terrain = collision::px_to_terrain_type(
                    world,
                    self.state.hero_x as i32,
                    self.state.hero_y as i32,
                );
                if (2..=5).contains(&terrain) { 1 } else { 2 }
            } else {
                2
            };

            let new_x = (self.state.hero_x as i32 + dx * speed).clamp(0, 0x7FF0) as u16;
            let new_y = (self.state.hero_y as i32 + dy * speed).clamp(0, 0x3FF0) as u16;

            if self.state.flying != 0 || collision::proxcheck(self.map_world.as_ref(), new_x as i32, new_y as i32) {
                self.state.hero_x = new_x;
                self.state.hero_y = new_y;
                if let Some(door) = crate::game::doors::doorfind(self.state.region_num, new_x, new_y) {
                    self.state.region_num = door.dst_region;
                    self.state.hero_x = door.dst_x;
                    self.state.hero_y = door.dst_y;
                    eprintln!("{:?}", crate::game::game_event::GameEvent::RegionTransition { region: door.dst_region });
                }
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

            let moved = self.state.hero_x != prev_x || self.state.hero_y != prev_y;
            if let Some(player) = self.state.actors.first_mut() {
                player.facing = facing;
                player.moving = moved;
                if self.input.fight {
                    player.state = ActorState::Fighting(0);
                } else {
                    player.state = ActorState::Walking;
                }
            }
            self.state.facing = facing;
        } else {
            if let Some(player) = self.state.actors.first_mut() {
                player.moving = false;
                if self.input.fight {
                    player.state = ActorState::Fighting(0);
                } else {
                    player.state = ActorState::Still;
                }
            }
        }
    }

    /// Advance all active actors by one frame (AI stub).
    fn update_actors(&mut self, _delta: u32) {
        for _actor in self.state.actors[0..self.state.anix].iter_mut() {
            // TODO: npc-002 will add AI here
        }
    }

    /// Clear and color the canvas according to the current viewstatus mode.
    fn render_by_viewstatus(&mut self, canvas: &mut Canvas<Window>) {
        match self.state.viewstatus {
            // Normal play or forced redraw
            0 | 98 | 99 => {
                canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 64));
                canvas.clear();
                eprintln!("{}", crate::game::hiscreen::format_hiscreen(&self.state));
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
        match action {
            GameAction::BuyFood => {
                if self.state.eat_food() {
                    eprintln!("eat_food: consumed food, hunger={}", self.state.hunger);
                } else {
                    eprintln!("eat_food: no food in pack");
                }
            }
            GameAction::Inventory => {
                eprintln!("Inventory: {}", self.state.inventory_summary());
                self.state.viewstatus = 4;
                self.messages.push("Inventory opened");
            }
            GameAction::Rebind => {
                self.rebinding.active = !self.rebinding.active;
                eprintln!("Rebinding mode: {}", self.rebinding.active);
            }
            _ => {}
        }
    }

    /// Handle a game event produced by gameplay logic.
    pub fn handle_game_event(&mut self, event: crate::game::game_event::GameEvent) {
        use crate::game::game_event::GameEvent;
        match event {
            GameEvent::Message { text } => {
                self.messages.push(text);
            }
            _ => {}
        }
    }

    /// Select music group 0-6 based on current game state (mirrors original setmood()).
    fn setmood(&self) -> u8 {
        let s = &self.state;
        if s.vitality <= 0 { return 6; }
        if s.hero_x >= 0x2400 && s.hero_x <= 0x3100 && s.hero_y >= 0x8200 && s.hero_y <= 0x8a00 { return 4; }
        if s.battleflag { return 1; }
        if s.region_num > 7 { return 5; }
        if s.lightlevel > 120 { return 0; }
        2
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
        // If rebinding mode is active and waiting for a key, capture the next keypress.
        if self.rebinding.active {
            if let Event::KeyDown { keycode: Some(kc), repeat: false, .. } = event {
                if *kc == Keycode::Escape {
                    self.rebinding.active = false;
                    self.rebinding.waiting_for_action = None;
                    eprintln!("Rebinding mode: false");
                    return true;
                }
                if let Some(action) = self.rebinding.waiting_for_action.take() {
                    self.local_bindings.set_binding(action, vec![*kc]);
                    eprintln!("Rebound {:?} to {:?}", action, kc);
                    return true;
                }
            }
        }
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
                _ => {
                    // KeyBindings fallback for any unhandled keycode (keys-104)
                    let kb = crate::game::key_bindings::KeyBindings::default_bindings();
                    if let Some(action) = kb.action_for_key(*kc) {
                        self.do_option(action);
                        true
                    } else {
                        false
                    }
                }
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

        // setmood: check music group every 8 ticks (gameloop-113)
        self.mood_tick += delta_ticks;
        if self.mood_tick >= 8 {
            self.mood_tick = 0;
            let mood = self.setmood();
            if mood != self.last_mood {
                self.last_mood = mood;
                eprintln!("setmood: switching to group {}", mood);
                if let Some(audio) = _resources.audio {
                    audio.set_score(mood);
                }
            }
        }

        // Region palette transition (world-109)
        let region = self.state.region_num;
        if region != self.last_region_num {
            eprintln!("region_num changed: {} -> {} ({:?})", self.last_region_num, region,
                crate::game::game_event::GameEvent::RegionTransition { region });
            let from = self.palette_transition
                .as_ref()
                .map(|pt| pt.to)
                .unwrap_or([crate::game::palette::BLACK; crate::game::palette::PALETTE_SIZE]);
            let to = [crate::game::palette::BLACK; crate::game::palette::PALETTE_SIZE];
            self.palette_transition = Some(crate::game::palette::PaletteTransition::new(from, to));
            self.last_region_num = region;
        }
        if let Some(ref mut pt) = self.palette_transition {
            if !pt.is_done() {
                let palette = pt.tick();
                if let (Some(ref mut mr), Some(ref world)) = (&mut self.map_renderer, &self.map_world) {
                    mr.atlas.rebuild(world, &palette);
                }
            }
        }

        // Indoor/outdoor mode detection (world-108)
        let indoor = self.state.region_num > 7;
        if indoor != self.last_indoor {
            if indoor {
                eprintln!("{:?}", crate::game::game_event::GameEvent::EnterIndoor { door_index: self.state.region_num });
            } else {
                eprintln!("{:?}", crate::game::game_event::GameEvent::ExitIndoor);
            }
            self.last_indoor = indoor;
        }

        // Encounter zone check (world-111)
        self.in_encounter_zone = crate::game::zones::in_encounter_zone(
            self.state.region_num, self.state.hero_x, self.state.hero_y);

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

        // Compose map viewport when in normal play view (world-105)
        if self.state.viewstatus == 0 {
            if let (Some(ref mut mr), Some(ref world)) = (&mut self.map_renderer, &self.map_world) {
                mr.compose(self.state.hero_x, self.state.hero_y, world);
                eprintln!("map composed");
            }
        }

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
