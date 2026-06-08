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

use crate::game::colors::RGB4;
use crate::game::debug_tui::DebugConsole;
use crate::game::direction::Direction;
use crate::game::ecs::components::{HeroStats, Inventory};
use crate::game::ecs::resources::Resources;
use crate::game::ecs::spawn::spawn_hero;
use crate::game::ecs::systems;
use crate::game::game_library::GameLibrary;
use crate::game::map_renderer::{MAP_DST_H, MAP_DST_W};
use crate::game::palette::{amiga_color_to_rgba, Palette, PALETTE_SIZE};
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
// ── Render layout constants (from display-rendering.md) ───────────────────────
const CANVAS_MARGIN_Y:    i32 = 40;
const PLAYFIELD_X:        i32 = 32;
const PLAYFIELD_Y:        i32 = CANVAS_MARGIN_Y;
const PLAYFIELD_LORES_W:  u32 = 288;
const PLAYFIELD_LORES_H:  u32 = 140;
const PLAYFIELD_CANVAS_W: u32 = PLAYFIELD_LORES_W * 2;
const PLAYFIELD_CANVAS_H: u32 = PLAYFIELD_LORES_H * 2;
const HIBAR_NATIVE_H:     u32 = 57;
const HIBAR_H:            u32 = HIBAR_NATIVE_H * 2;
const HIBAR_Y:            i32 = CANVAS_MARGIN_Y + PLAYFIELD_CANVAS_H as i32 + 6;

pub struct EcsScene {
    pub world:          World,
    pub res:            Resources,
    console:            Option<DebugConsole>,
    input:              InputState,
    last_mood:          u8,
    mood_tick:          u32,
    adf_load_done:      bool,
    /// RGB4 base palette used as input to fade_page() for day/night computation.
    base_colors:        Option<crate::game::colors::Palette>,
    /// Scroll-area message queue (up to 4 visible at once, most recent last).
    messages:           Vec<String>,
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

        Self {
            world,
            res,
            console,
            input: InputState::new(),
            last_mood: u8::MAX,
            mood_tick: 0,
            adf_load_done: false,
            base_colors: None,
            messages: Vec::new(),
        }
    }

    /// Drain message and speech events from the current tick into the message queue.
    /// Called from `update()` after each tick so `game_lib` is available for narr lookup.
    fn drain_messages(&mut self, game_lib: &GameLibrary) {
        // Plain text messages.
        for ev in self.res.events.message.drain(..) {
            self.messages.push(ev.text);
        }
        // Speech events resolved through narr table.
        for ev in self.res.events.speech.drain(..) {
            let text = crate::game::events::speak(&game_lib.narr, ev.speech_id, &ev.brother_name);
            self.messages.push(text);
        }
        // Trim to 64 messages — the hibar only shows the last 4.
        if self.messages.len() > 64 {
            let overflow = self.messages.len() - 64;
            self.messages.drain(0..overflow);
        }
    }

    /// Lazy-load the ADF disk image, WorldData, MapRenderer, and sprite sheets.
    /// Mirrors the `render-world-load` block from the old `GameplayScene::update`.
    fn load_world(&mut self, game_lib: &GameLibrary) {
        self.adf_load_done = true;

        let adf_path = game_lib
            .disk
            .as_ref()
            .map(|d| d.adf.as_str())
            .unwrap_or("game/image");

        let adf = match crate::game::adf::AdfDisk::open(std::path::Path::new(adf_path)) {
            Ok(a) => a,
            Err(e) => { eprintln!("EcsScene: AdfDisk::open failed: {e}"); return; }
        };

        let region = self.res.region.region_num;
        let world_result = game_lib.find_region_config(region).map(|cfg| {
            let map_blocks: Vec<u32> = if region < 8 {
                [0u8, 2, 4, 6]
                    .iter()
                    .filter_map(|&r| game_lib.find_region_config(r))
                    .map(|c| c.map_block)
                    .collect()
            } else {
                vec![cfg.map_block]
            };
            crate::game::world_data::WorldData::load(
                &adf,
                region,
                cfg.sector_block,
                &map_blocks,
                cfg.terra_block,
                cfg.terra2_block,
                &cfg.image_blocks,
            )
        });

        let world = match world_result {
            Some(Ok(w)) => w,
            Some(Err(e)) => { eprintln!("EcsScene: WorldData::load failed: {e}"); return; }
            None => { eprintln!("EcsScene: no region config for region {region}"); return; }
        };

        // Shadow memory bitmask for sprite depth masking.
        let shadow_mem = game_lib.disk.as_ref()
            .filter(|d| d.shadow_count > 0)
            .map(|d| crate::game::world_data::load_shadow_mem(&adf, d.shadow_block, d.shadow_count))
            .unwrap_or_default();

        let renderer = crate::game::map_renderer::MapRenderer::new(&world, shadow_mem.clone());

        // Sprite sheets: player (0-2), enemies (4-12), setfigs (13-17).
        while self.res.sprites.sheets.len() < 18 {
            self.res.sprites.sheets.push(None);
        }
        for cfile_idx in [0u8, 1, 2, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17] {
            self.res.sprites.sheets[cfile_idx as usize] =
                crate::game::sprites::SpriteSheet::load(&adf, cfile_idx);
        }
        self.res.sprites.object_sprites = crate::game::sprites::SpriteSheet::load_objects(&adf);

        // Palette.
        self.base_colors = build_base_colors_palette(game_lib, region);
        self.res.palette.current_palette = region_palette(game_lib, region);
        self.res.palette.dirty = true;

        // Store map data (adf kept alive so region transitions can reload).
        self.res.map.renderer = Some(renderer);
        self.res.map.world    = Some(world);

        // Snap camera to hero's spawn position.
        self.snap_camera();

        eprintln!("EcsScene: world loaded for region {region}");
    }

    /// Center the camera on the hero's current world position.
    fn snap_camera(&mut self) {
        if let Ok(pos) = self.world.get::<&crate::game::ecs::components::Position>(self.res.hero_entity) {
            const CX: f32 = 144.0;
            const CY: f32 = 70.0;
            const WRAP: f32 = 0x8000 as f32;
            self.res.camera.map_x = (pos.x - CX).rem_euclid(WRAP);
            self.res.camera.map_y = (pos.y - CY).rem_euclid(WRAP);
        }
    }

    /// Compose the map framebuf, blit sprites into it, then copy to the SDL canvas.
    fn render_map(&mut self, canvas: &mut Canvas<Window>) {
        let map_x = self.res.camera.map_x as u16;
        let map_y = self.res.camera.map_y as u16;

        // Step 1: compose tiles into the indexed framebuf.
        if let (Some(renderer), Some(world_data)) = (
            self.res.map.renderer.as_mut(),
            self.res.map.world.as_ref(),
        ) {
            renderer.compose(map_x, map_y, world_data);
        }

        if self.res.map.renderer.as_ref().map_or(true, |r| r.framebuf.is_empty()) {
            return;
        }

        // Step 2: blit sprites.  Temporarily take the framebuf out of the renderer
        // so we can pass both framebuf (mut) and res.sprites (immut) simultaneously.
        let mut framebuf = if let Some(r) = self.res.map.renderer.as_mut() {
            std::mem::take(&mut r.framebuf)
        } else {
            return;
        };
        let cycle        = self.res.clock.cycle as usize;
        let hero_entity  = self.res.hero_entity;
        blit_actors_inner(
            &self.world,
            hero_entity,
            &self.res.sprites.sheets,
            cycle,
            map_x,
            map_y,
            &mut framebuf,
        );
        // Put the framebuf back.
        if let Some(r) = self.res.map.renderer.as_mut() {
            r.framebuf = framebuf;
        }

        // Step 3: convert indexed framebuf to RGBA and blit to canvas.
        let pal = &self.res.palette.current_palette;
        let framebuf = &self.res.map.renderer.as_ref().unwrap().framebuf;
        let mut rgb_buf: Vec<u8> = Vec::with_capacity(framebuf.len() * 4);
        for &idx in framebuf {
            let rgba = pal[(idx & 31) as usize];
            // ARGB8888 little-endian: bytes are [B, G, R, A]
            rgb_buf.push((rgba & 0xFF) as u8);
            rgb_buf.push(((rgba >> 8) & 0xFF) as u8);
            rgb_buf.push(((rgba >> 16) & 0xFF) as u8);
            rgb_buf.push(0xFF);
        }

        {
            let tc = canvas.texture_creator();
            if let Ok(surface) = sdl3::surface::Surface::from_data(
                &mut rgb_buf,
                MAP_DST_W,
                MAP_DST_H,
                MAP_DST_W * 4,
                sdl3::pixels::PixelFormat::ARGB8888,
            ) {
                if let Ok(mut tex) = tc.create_texture_from_surface(&surface) {
                    tex.set_scale_mode(sdl3::render::ScaleMode::Nearest);
                    let src = sdl3::rect::Rect::new(0, 0, PLAYFIELD_LORES_W, PLAYFIELD_LORES_H);
                    let dst = sdl3::rect::Rect::new(
                        PLAYFIELD_X, PLAYFIELD_Y, PLAYFIELD_CANVAS_W, PLAYFIELD_CANVAS_H,
                    );
                    let _ = canvas.copy(&tex, src, dst);
                }
            }
        }
        drop(rgb_buf);
    }

    /// Render the HI bar (stats, messages, compass) at the bottom of the canvas.
    /// Port of `GameplayScene::render_hibar`.
    fn render_hibar(
        &mut self,
        canvas: &mut Canvas<Window>,
        resources: &mut SceneResources<'_, '_>,
    ) {
        // Gather hero stats from the ECS world.
        let (brave, luck, kind, vitality, wealth) =
            match self.world.get::<&crate::game::ecs::components::HeroStats>(self.res.hero_entity) {
                Ok(s) => (s.brave, s.luck, s.kind, s.vitality, s.wealth),
                Err(_) => return,
            };

        // Last 4 messages visible in the scroll area.
        let msg_count = self.messages.len().min(4);
        let msg_start = self.messages.len().saturating_sub(4);
        let msgs_visible: Vec<&str> = self.messages[msg_start..].iter().map(|s| s.as_str()).collect();

        let hiscreen_opt = resources.find_image("hiscreen");
        let amber_font   = resources.amber_font;
        let compass_normal    = resources.compass_normal;
        let compass_highlight = resources.compass_highlight;

        // Current input direction → compass arrow index.
        let input_dir = self.input.to_direction();
        let compass_arrow = compass_dir_index(input_dir);
        let compass_regions = compass_hit_regions();

        let tc = canvas.texture_creator();
        if let Ok(mut hibar_tex) =
            tc.create_texture_target(sdl3::pixels::PixelFormat::RGBA32, 640, HIBAR_NATIVE_H)
        {
            hibar_tex.set_scale_mode(sdl3::render::ScaleMode::Nearest);
            let _ = canvas.with_texture_canvas(&mut hibar_tex, |hc| {
                hc.set_draw_color(sdl3::pixels::Color::RGB(0, 0, 0));
                hc.clear();

                if let Some(hiscreen) = hiscreen_opt {
                    hiscreen.draw_scaled(hc, sdl3::rect::Rect::new(0, 0, 640, HIBAR_NATIVE_H));
                } else {
                    hc.set_draw_color(sdl3::pixels::Color::RGB(80, 60, 20));
                    hc.fill_rect(sdl3::rect::Rect::new(0, 0, 640, HIBAR_NATIVE_H)).ok();
                }

                amber_font.set_color_mod(0xAA, 0x55, 0x00);
                amber_font.render_string(&format!("Brv:{:3}", brave), hc, 14, 52);
                amber_font.render_string(&format!("Lck:{:3}", luck), hc, 90, 52);
                amber_font.render_string(&format!("Knd:{:3}", kind), hc, 168, 52);
                amber_font.render_string(&format!("Vit:{:3}", vitality), hc, 245, 52);
                amber_font.render_string(&format!("Wlth:{:3}", wealth), hc, 321, 52);

                // Scroll messages (up to 4, bottom-anchored at y=42).
                for (i, msg) in msgs_visible.iter().enumerate() {
                    let line_from_bottom = (msg_count - 1 - i) as i32;
                    let y = 42 - line_from_bottom * 10;
                    amber_font.render_string(msg, hc, 16, y);
                }
                amber_font.set_color_mod(255, 255, 255);

                // Compass.
                const COMPASS_X: i32 = 567;
                const COMPASS_SRC_Y: i32 = 15;
                const COMPASS_SRC_W: u32 = 48;
                const COMPASS_SRC_H: u32 = 24;
                let compass_dest = sdl3::rect::Rect::new(COMPASS_X, COMPASS_SRC_Y, COMPASS_SRC_W, COMPASS_SRC_H);
                if let Some(normal_tex) = compass_normal {
                    hc.copy(normal_tex, None, compass_dest).ok();
                }
                if compass_arrow < compass_regions.len() {
                    let (rx, ry, rw, rh) = compass_regions[compass_arrow];
                    if rw > 1 || rh > 1 {
                        if let Some(hl_tex) = compass_highlight {
                            let src = sdl3::rect::Rect::new(rx, ry, rw as u32, rh as u32);
                            let dst = sdl3::rect::Rect::new(COMPASS_X + rx, COMPASS_SRC_Y + ry, rw as u32, rh as u32);
                            hc.copy(hl_tex, src, dst).ok();
                        }
                    }
                }
            });
            canvas.copy(
                &hibar_tex,
                sdl3::rect::Rect::new(0, 0, 640, HIBAR_NATIVE_H),
                sdl3::rect::Rect::new(0, HIBAR_Y, 640, HIBAR_H),
            ).ok();
        }; // semicolon: drops Result<Texture> temporary before tc is dropped
    }

    /// Run one gameplay tick: advance all systems then drain debug commands.
    fn run_tick(&mut self) {
        // ── System schedule (mirrors order in systems/mod.rs) ────────────────
        systems::clock::run(&mut self.world, &mut self.res);

        // Palette update every 4 ticks or when dirty (SPEC §17.5).
        let daynight = self.res.clock.daynight;
        if (daynight & 3) == 0 || self.res.palette.dirty {
            self.res.palette.dirty = false;
            if let Some(base) = self.base_colors.clone() {
                let lightlevel    = self.res.clock.lightlevel;
                let light_on      = self.res.clock.light_timer > 0;
                let secret_active = self.res.region.region_num == 9
                    && self.res.clock.secret_timer > 0;
                self.res.palette.current_palette = compute_current_palette(
                    &base, self.res.region.region_num, lightlevel, light_on, secret_active,
                );
            }
        }
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
        canvas: &mut Canvas<Window>,
        _play_tex: &mut Texture,
        delta_ticks: u32,
        game_lib: &GameLibrary,
        resources: &mut SceneResources<'_, '_>,
    ) -> SceneResult {
        // Lazy-load world data on first frame.
        if !self.adf_load_done {
            self.load_world(game_lib);
        }

        // Run gameplay ticks (capped to avoid spiral-of-death).
        let ticks = delta_ticks.min(4).max(1);
        for _ in 0..ticks {
            self.run_tick();
            self.drain_messages(game_lib);
        }

        self.run_audio(resources);

        // ── Render ────────────────────────────────────────────────────────────
        canvas.set_draw_color(sdl3::pixels::Color::RGB(0, 0, 0));
        canvas.clear();
        self.render_map(canvas);
        self.render_hibar(canvas, resources);

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

// ── Sprite render helpers (ported from gameplay_scene/rendering.rs) ──────────

/// Compute an actor's framebuf-relative position from world coords.
/// Mirrors `GameplayScene::actor_rel_pos` (fmain.c sprite blit offset).
fn actor_rel_pos(abs_x: f32, abs_y: f32, map_x: u16, map_y: u16) -> (i32, i32) {
    const WRAP: i32 = 0x8000;
    const OX: i32 = -8;
    const OY: i32 = -26;
    let dx = (abs_x as i32 - map_x as i32 + OX).rem_euclid(WRAP);
    let rel_x = if dx > WRAP / 2 { dx - WRAP } else { dx };
    let dy = (abs_y as i32 - map_y as i32 + OY).rem_euclid(WRAP);
    let rel_y = if dy > WRAP / 2 { dy - WRAP } else { dy };
    (rel_x, rel_y)
}

/// Blit a 16-wide sprite frame into the indexed framebuf, skipping transparent pixels (index 31).
fn blit_sprite_to_framebuf(
    frame_pixels: &[u8],
    rel_x: i32,
    rel_y: i32,
    max_rows: usize,
    framebuf: &mut [u8],
    fb_w: i32,
    fb_h: i32,
) {
    use crate::game::sprites::{SPRITE_H, SPRITE_W};
    let row_limit = max_rows.min(SPRITE_H) as i32;
    for row in 0..row_limit {
        let dst_y = rel_y + row;
        if dst_y < 0 || dst_y >= fb_h { continue; }
        for col in 0..SPRITE_W as i32 {
            let dst_x = rel_x + col;
            if dst_x < 0 || dst_x >= fb_w { continue; }
            let src_idx = frame_pixels[(row as usize) * SPRITE_W + col as usize];
            if src_idx == 31 { continue; }
            framebuf[(dst_y * fb_w + dst_x) as usize] = src_idx;
        }
    }
}

/// Map facing direction to walking sprite frame base (diroffs[0..7]).
fn facing_to_frame_base(facing: Direction) -> usize {
    const DIROFFS_WALK: [usize; 9] = [16, 16, 24, 24, 0, 0, 8, 8, 0];
    DIROFFS_WALK[facing as usize]
}

/// Map facing direction to fighting sprite frame base (diroffs[8..15]).
fn facing_to_fight_frame_base(facing: Direction) -> usize {
    const DIROFFS_FIGHT: [usize; 9] = [56, 56, 68, 68, 32, 32, 44, 44, 32];
    DIROFFS_FIGHT[facing as usize]
}

/// Map (npc_type, race) → cfile index.  Returns None for setfigs and skipped types.
fn npc_type_to_cfile(npc_type: u8, race: u8) -> Option<usize> {
    use crate::game::npc::*;
    match npc_type {
        NPC_TYPE_NONE | NPC_TYPE_CONTAINER => None,
        NPC_TYPE_HUMAN if race == RACE_ENEMY => Some(6),
        NPC_TYPE_HUMAN => None, // SetFig — skip here
        NPC_TYPE_SWAN   => Some(11),
        NPC_TYPE_HORSE  => Some(5),
        NPC_TYPE_DRAGON => Some(10),
        NPC_TYPE_GHOST | NPC_TYPE_WRAITH | NPC_TYPE_SKELETON => Some(7),
        NPC_TYPE_ORC    => Some(6),
        NPC_TYPE_SNAKE | NPC_TYPE_SPIDER | NPC_TYPE_DKNIGHT => Some(8),
        NPC_TYPE_LORAII | NPC_TYPE_NECROMANCER => Some(9),
        NPC_TYPE_RAFT   => Some(4),
        _ => Some(6),
    }
}

/// Blit all visible actors (hero + enemies) into the indexed framebuf.
/// Takes individual fields to avoid borrow conflicts with the framebuf.
/// Must be called after `MapRenderer::compose()` and before palette conversion.
fn blit_actors_inner(
    world: &World,
    hero_entity: hecs::Entity,
    sheets: &[Option<crate::game::sprites::SpriteSheet>],
    cycle: usize,
    map_x: u16,
    map_y: u16,
    framebuf: &mut Vec<u8>,
) {
    use crate::game::actor::ActorState;
    use crate::game::ecs::components::{
        ActorMotion, AiState, BrotherKind, CombatState, Enemy, Facing, FrustFlag, Hero, Position,
    };
    use crate::game::npc::NpcState;
    use crate::game::sprites::{SPRITE_H, SPRITE_W, STATELIST};

    let fb_w = MAP_DST_W as i32;
    let fb_h = MAP_DST_H as i32;

    // ── Hero ──────────────────────────────────────────────────────────────────
    type HeroQuery<'a> = (
        &'a Hero,
        &'a Position,
        &'a Facing,
        Option<&'a ActorMotion>,
        Option<&'a CombatState>,
        Option<&'a FrustFlag>,
        Option<&'a BrotherKind>,
    );
    let mut hero_q = world.query_one::<HeroQuery<'_>>(hero_entity);
    if let Ok((_, pos, facing_c, motion_opt, combat_opt, frust_opt, brother_opt)) = hero_q.get() {
        let cfile_idx: usize = brother_opt.map(|b: &BrotherKind| b.id as usize).unwrap_or(0).min(2);
        if let Some(Some(ref sheet)) = sheets.get(cfile_idx) {
            let (rel_x, mut rel_y) = actor_rel_pos(pos.x, pos.y, map_x, map_y);
            let environ: i8 = motion_opt.map(|m: &ActorMotion| m.environ).unwrap_or(0);
            let body_rows: usize = if environ == 2 {
                SPRITE_H.saturating_sub(10)
            } else if environ > 2 {
                rel_y += environ as i32;
                SPRITE_H.saturating_sub(environ as usize)
            } else {
                SPRITE_H
            };

            if rel_x > -(SPRITE_W as i32) && rel_x < fb_w
                && rel_y > -(SPRITE_H as i32) && rel_y < fb_h
            {
                let hero_facing  = facing_c.dir;
                let is_moving: bool  = motion_opt.map(|m: &ActorMotion| m.moving).unwrap_or(false);
                let combat_state: Option<&ActorState> = combat_opt.map(|c: &CombatState| &c.state);
                let frustflag: u8    = frust_opt.map(|f: &FrustFlag| f.count).unwrap_or(0);

                let frame = if frustflag >= 41 {
                    40
                } else if frustflag >= 21 {
                    84 + ((cycle >> 1) & 1)
                } else if let Some(ActorState::Fighting(f)) = combat_state {
                    let fight_base = facing_to_fight_frame_base(hero_facing);
                    fight_base + (*f as usize).min(8)
                } else {
                    let frame_base = facing_to_frame_base(hero_facing);
                    if is_moving { frame_base + cycle % 8 } else { frame_base + 1 }
                };

                let body_frame = if let Some(ActorState::Fighting(_)) = combat_state {
                    STATELIST.get(frame).map(|e| e.figure as usize).unwrap_or(frame)
                } else {
                    frame
                };

                if let Some(fp) = sheet.frame_pixels(body_frame) {
                    blit_sprite_to_framebuf(fp, rel_x, rel_y, body_rows, framebuf, fb_w, fb_h);
                }
            }
        }
    }

    // ── Enemies ───────────────────────────────────────────────────────────────
    let mut enemy_q = world.query::<(
        &Enemy,
        &Position,
        &Facing,
        &crate::game::ecs::components::EnemyKind,
        Option<&AiState>,
    )>();
    for (idx, (_, pos, facing_c, kind, ai_opt)) in enemy_q.iter().enumerate() {
        let Some(cfile_idx) = npc_type_to_cfile(kind.npc_type, kind.race) else { continue; };
        let Some(Some(ref sheet)) = sheets.get(cfile_idx) else { continue; };

        let (rel_x, rel_y) = actor_rel_pos(pos.x, pos.y, map_x, map_y);
        if rel_x <= -(SPRITE_W as i32) || rel_x >= fb_w
            || rel_y <= -(SPRITE_H as i32) || rel_y >= fb_h
        {
            continue;
        }

        let npc_state = ai_opt.map(|a| &a.state).unwrap_or(&NpcState::Still);
        let frame_base = facing_to_frame_base(facing_c.dir);
        let frame = match npc_state {
            NpcState::Walking => frame_base + ((cycle + idx) & 7),
            NpcState::Still   => frame_base + 1,
            _                 => frame_base,
        } % sheet.num_frames.max(1);

        if let Some(fp) = sheet.frame_pixels(frame) {
            blit_sprite_to_framebuf(fp, rel_x, rel_y, SPRITE_H, framebuf, fb_w, fb_h);
        }
    }
}

// ── Compass helpers ───────────────────────────────────────────────────────────

/// Map a Direction to its compass arrow index (0..9, where 9=None).
fn compass_dir_index(dir: Direction) -> usize {
    // Amiga order: NW=0, N=1, NE=2, E=3, SE=4, S=5, SW=6, W=7, None=8
    dir as usize
}

/// Compass hit-regions: (rx, ry, rw, rh) pixel rects within the compass glyph.
/// Ported from `GameplayScene::compass_regions` (gameplay_scene/mod.rs).
fn compass_hit_regions() -> [(i32, i32, i32, i32); 9] {
    [
        (0, 0, 16, 12),   // NW
        (16, 0, 16, 8),   // N
        (32, 0, 16, 12),  // NE
        (37, 8, 11, 8),   // E
        (32, 12, 16, 12), // SE
        (16, 16, 16, 8),  // S
        (0, 12, 16, 12),  // SW
        (0, 8, 11, 8),    // W
        (0, 0, 0, 0),     // None — no highlight
    ]
}

// ── Palette helpers (ported from gameplay_scene/region.rs) ───────────────────

/// Build the initial RGBA palette for a region (no day/night fade applied).
fn region_palette(game_lib: &GameLibrary, region: u8) -> Palette {
    let mut palette = [0xFF808080_u32; PALETTE_SIZE];
    if let Some(base) = game_lib.find_palette("pagecolors") {
        for (i, entry) in base.colors.iter().enumerate().take(PALETTE_SIZE) {
            palette[i] = amiga_color_to_rgba(entry.color);
        }
    }
    palette[31] = amiga_color_to_rgba(match region { 4 => 0x0980, 9 => 0x0445, _ => 0x0bdf });
    palette
}

/// Build a base `colors::Palette` with the per-region color-31 override applied.
fn build_base_colors_palette(
    game_lib: &GameLibrary,
    region: u8,
) -> Option<crate::game::colors::Palette> {
    let base = game_lib.find_palette("pagecolors")?;
    let mut cloned = base.clone();
    let color31: u16 = match region { 4 => 0x0980, 9 => 0x0445, _ => 0x0bdf };
    if let Some(c) = cloned.colors.get_mut(31) {
        *c = RGB4::from(color31);
    }
    Some(cloned)
}

/// Recompute the display palette from base colors + current lighting state.
/// Mirrors `GameplayScene::compute_current_palette` (SPEC §17.5–17.6).
fn compute_current_palette(
    base: &crate::game::colors::Palette,
    region_num: u8,
    lightlevel: u16,
    light_on: bool,
    secret_active: bool,
) -> Palette {
    if region_num >= 8 {
        // Indoors: full brightness; jewel tint applied by fade_page.
        let faded = crate::game::palette_fader::fade_page(100, 100, 100, true, light_on, base);
        let mut pal = [0xFF808080_u32; PALETTE_SIZE];
        for (i, entry) in faded.colors.iter().enumerate().take(PALETTE_SIZE) {
            pal[i] = amiga_color_to_rgba(entry.color);
        }
        pal[31] = amiga_color_to_rgba(match (region_num, secret_active) {
            (9, true)  => 0x00f0,
            (9, false) => 0x0445,
            _          => 0x0bdf,
        });
        return pal;
    }
    let ll = lightlevel as i32;
    let boost = if light_on { 200i32 } else { 0 };
    let r_pct = (ll - 80 + boost) as i16;
    let g_pct = (ll - 61) as i16;
    let b_pct = (ll - 62) as i16;
    let faded = crate::game::palette_fader::fade_page(r_pct, g_pct, b_pct, true, light_on, base);
    let mut out = [0xFF808080_u32; PALETTE_SIZE];
    for (i, entry) in faded.colors.iter().enumerate().take(PALETTE_SIZE) {
        out[i] = amiga_color_to_rgba(entry.color);
    }
    out[31] = amiga_color_to_rgba(match region_num { 4 => 0x0980, _ => 0x0bdf });
    out
}

#[cfg(test)]
mod tests {
    // EcsScene::new() requires a GameLibrary loaded from disk, which is not
    // available in unit tests.  The system-level tests live in each system's
    // own module.  Smoke tests for debug_commands are in debug_commands.rs.
}
