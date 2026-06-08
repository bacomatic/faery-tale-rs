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

    /// Compose the map framebuf and blit it to the SDL canvas.
    fn render_map(&mut self, canvas: &mut Canvas<Window>) {
        let (renderer, world) = match (self.res.map.renderer.as_mut(), self.res.map.world.as_ref()) {
            (Some(r), Some(w)) => (r, w),
            _ => return,
        };

        renderer.compose(
            self.res.camera.map_x as u16,
            self.res.camera.map_y as u16,
            world,
        );

        if renderer.framebuf.is_empty() { return; }

        let pal = &self.res.palette.current_palette;
        let mut rgb_buf: Vec<u8> = Vec::with_capacity(renderer.framebuf.len() * 4);
        for &idx in &renderer.framebuf {
            let rgba = pal[(idx & 31) as usize];
            // ARGB8888 little-endian: bytes are [B, G, R, A]
            rgb_buf.push((rgba & 0xFF) as u8);
            rgb_buf.push(((rgba >> 8) & 0xFF) as u8);
            rgb_buf.push(((rgba >> 16) & 0xFF) as u8);
            rgb_buf.push(0xFF);
        }

        // Keep rgb_buf alive for the entire surface + texture + copy sequence.
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
        }

        self.run_audio(resources);

        // ── Render ────────────────────────────────────────────────────────────
        canvas.set_draw_color(sdl3::pixels::Color::RGB(0, 0, 0));
        canvas.clear();
        self.render_map(canvas);

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
