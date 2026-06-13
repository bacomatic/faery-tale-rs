extern crate sdl3;

mod game;

use clap::Parser;

use game::game_library;

use sdl3::event::{Event, WindowEvent};
use sdl3::keyboard::{Keycode, Scancode};
use sdl3::mouse::Cursor;
use sdl3::pixels::{Color, PixelFormat};
use sdl3::rect::{Point, Rect};
use sdl3::surface::Surface;

use std::path::Path;
use std::sync::Arc;

use crate::game::audio::{AudioSystem, Instruments};
use crate::game::colors::Palette;
use crate::game::copy_protect_scene::CopyProtectScene;
use crate::game::cursor::CursorAsset;
use crate::game::debug_command::{DebugCommand, DEFAULT_TICK_RATE_HZ};
use crate::game::debug_tui::{DebugConsole, DebugSnapshot};
use crate::game::game_clock::GameClock;
use crate::game::day_phase::DayPhase;
use crate::game::ecs::scene::EcsScene;
use crate::game::intro_scene::IntroScene;
use crate::game::placard_scene::PlacardScene;
use crate::game::render_resources::RenderResources;
use crate::game::scene::{Scene, SceneResult};
use crate::game::settings::{self, GameSettings};
use crate::game::songs::{SongLibrary, Track};
use crate::game::victory_scene::VictoryScene;

#[derive(Parser, Debug)]
#[command(name = "fmainrs", about = "The Faery Tale Adventure")]
struct Cli {
    /// Activate debug console in terminal
    #[arg(long, short)]
    debug: bool,
    /// Disable linear interpolation in the PCM mixer (use nearest-neighbor instead)
    #[arg(long)]
    no_interpolation: bool,
    /// Skip the intro sequence and jump straight to gameplay (requires --debug)
    #[arg(long, requires = "debug")]
    skip_intro: bool,
}

fn set_mouse(cursor: &CursorAsset, color: &Palette) -> Option<Cursor> {
    // build RGBA32 pixel data from cursor and palette
    let result = cursor.bitmap.generate_rgb32(color, Some(0));
    if result.is_err() {
        eprintln!(
            "Error generating RGB32 data for cursor: {}",
            result.err().unwrap()
        );
        return None;
    }

    let (mut pixels, stride) = result.unwrap();

    let orig_w = cursor.bitmap.width as u32;
    let orig_h = cursor.bitmap.height as u32;

    // create RGB surface from pixels, we need to use a Surface to create a color cursor
    let surface = Surface::from_data(
        &mut pixels,
        orig_w,
        orig_h,
        stride as u32,
        PixelFormat::RGBA32,
    )
    .unwrap();

    // Scale 2× for better visual appearance (matches the 2× line-doubled canvas)
    let mut scaled = Surface::new(orig_w * 2, orig_h * 2, PixelFormat::RGBA32).unwrap();
    // SDL3: blit_scaled needs an explicit scale mode; use LINEAR for smooth cursor scaling.
    surface
        .blit_scaled(None, &mut scaled, None, sdl3::sys::surface::SDL_ScaleMode::LINEAR)
        .unwrap();

    // create and set the cursor (hotspot also scaled 2×)
    let pointer = Cursor::from_surface(
        scaled,
        (cursor.hotspot.x * 2) as i32,
        (cursor.hotspot.y * 2) as i32,
    )
    .unwrap();
    pointer.set();

    Some(pointer)
}

fn diag(dc: &mut Option<crate::game::debug_tui::DebugConsole>, msg: impl Into<String>) {
    let msg = msg.into();
    match dc {
        Some(ref mut c) => c.log(msg),
        None => eprintln!("{msg}"),
    }
}

pub fn main() -> Result<(), String> {
    let cli = Cli::parse();

    let mut settings: GameSettings = settings::GameSettings::load();
    let mut pre_console_log: Vec<String> = Vec::new();

    let sdl_context = sdl3::init().unwrap();
    let video_subsystem = sdl_context
        .video()
        .expect("Could not initialize SDL3 video subsystem");

    // Initialize gamepad subsystem so SDL3 generates ControllerButton/Axis events.
    let gamepad_subsystem = sdl_context
        .gamepad()
        .map_err(|e| format!("Could not initialize gamepad subsystem: {}", e))?;
    let mut gamepads: Vec<sdl3::gamepad::Gamepad> = Vec::new();
    // Open any gamepads that are already connected at startup.
    if let Ok(ids) = gamepad_subsystem.gamepads() {
        for id in ids {
            if gamepad_subsystem.is_gamepad(id) {
                match gamepad_subsystem.open(id) {
                    Ok(c) => {
                        let name = c.name().unwrap_or_else(|| "Unknown".to_string());
                        pre_console_log.push(format!("Controller connected: {}", name));
                        gamepads.push(c);
                    }
                    Err(e) => pre_console_log.push(format!("Warning: could not open controller {}: {}", id.0, e)),
                }
            }
        }
    }

    let mut width = 640;
    let mut height = 480;
    if settings.window_size.is_some() {
        (width, height) = settings.window_size.unwrap();
    }

    let mut window_builder = video_subsystem.window("The Faery Tale Adventure", width, height);
    window_builder.resizable();

    if settings.fullscreen {
        window_builder.fullscreen();
    } else if settings.window_position.is_some() {
        let (x, y) = settings.window_position.unwrap();
        window_builder.position(x, y);
    } else {
        window_builder.position_centered();
    }

    let window = window_builder.build().unwrap();

    let mut canvas = window.into_canvas();
    // Set the logical size to 640x480 to preserve the original 4:3 aspect ratio,
    // using LETTERBOX mode to preserve aspect ratio with black bars.
    canvas
        .set_logical_size(
            640,
            480,
            sdl3::sys::render::SDL_RendererLogicalPresentation::LETTERBOX,
        )
        .unwrap();

    // load the game library
    let game_lib = game_library::load_game_library(Path::new("faery.toml"));
    if game_lib.is_err() {
        return Err(format!(
            "Failed to load game library: {}",
            game_lib.err().unwrap()
        ));
    }
    let game_lib = game_lib.unwrap();

    let tex_maker = canvas.texture_creator();

    let sys_palette = game_lib.find_palette("introcolors").unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();

    // Audio system — load songs and waveforms, init the software synthesizer.
    // Music playback is started by IntroScene (matching original: playscore() is
    // called mid-intro, not at startup) and stopped before gameplay begins.
    let audio_subsystem = sdl_context.audio().ok();
    let songs_path = game_lib
        .audio
        .as_ref()
        .map(|a| a.songs.as_str())
        .unwrap_or("game/songs");
    let instruments_path = game_lib
        .audio
        .as_ref()
        .map(|a| a.instruments.as_str())
        .unwrap_or("game/v6");
    let song_library: Option<SongLibrary> = SongLibrary::load(Path::new(songs_path));
    let intro_tracks: Option<[Arc<Track>; 4]> = song_library.as_ref().and_then(|songs| {
        songs
            .intro_tracks()
            .map(|t| t.map(|tr| Arc::new(tr.clone())))
    });
    let audio_system: Option<AudioSystem> = {
        match (
            audio_subsystem.as_ref(),
            Instruments::load(Path::new(instruments_path)),
        ) {
            (Some(audio_sub), Some(inst)) => {
                match AudioSystem::new(audio_sub, inst, cli.no_interpolation) {
                    Ok(sys) => Some(sys),
                    Err(e) => {
                        pre_console_log.push(format!("Warning: could not open audio device: {}", e));
                        None
                    }
                }
            }
            (None, _) => {
                pre_console_log.push("Warning: could not init SDL3 audio subsystem".to_string());
                None
            }
            (_, None) => {
                pre_console_log.push(format!(
                    "Warning: could not load {} (instruments file missing)",
                    instruments_path
                ));
                None
            }
        }
    };

    // Attach the song library to the audio system so set_score() can switch groups.
    let mut audio_system = audio_system;
    if let (Some(ref mut a), Some(lib)) = (audio_system.as_mut(), song_library.as_ref()) {
        a.attach_library(lib.clone());
    }

    let mut mouse_cursor: Option<Cursor> = None;
    if let Some(pointer) = game_lib.get_cursor("bow") {
        // Use the dedicated bow sprite palette (textcolors[16..19]) rather than
        // the general sys_palette; see ChangeSprite(&vp_text) in fmain.c.
        let bow_palette = game_lib.find_palette("bowcolors").unwrap_or(sys_palette);
        mouse_cursor = set_mouse(pointer, &bow_palette);
    }

    // Build all SDL3 rendering resources (font atlas, image atlas, render targets).
    let mut render_resources = RenderResources::build(&tex_maker, &game_lib, &sys_palette);

    let mut play_tex = tex_maker
        .create_texture_target(Some(PixelFormat::RGBA32), 320, 200)
        .unwrap();
    play_tex.set_scale_mode(sdl3::render::ScaleMode::Nearest);
    let mut scratch_tex = tex_maker
        .create_texture_target(Some(PixelFormat::RGBA32), 320, 200)
        .unwrap();
    scratch_tex.set_scale_mode(sdl3::render::ScaleMode::Nearest);

    let mut dirty: bool = true;
    let mut clear_flag = true;
    let mut kill_flag = false;
    let mut walker: Point = Point::new(0, 20);

    let mut clock: GameClock = GameClock::new();

    // Scene system — scenes chain: Intro → CopyProtect → EcsScene → BrotherPlacard → EcsScene
    // The scene_phase tracks what to start next when a scene completes.
    enum ScenePhase {
        Intro,
        CopyProtect,
        Gameplay,
        /// Sequencing brother placards: show dead placard (if any), then start placard (if any).
        BrotherPlacard { dead: Option<String>, start: Option<String> },
        /// Game over: kevin_dead placard displayed, then exit.
        GameOverPlacard,
        VictoryPlacard,
        VictoryImage,
    }
    // Holds the EcsScene while brother-succession placards are shown.
    let mut stashed_scene: Option<Box<dyn Scene>> = None;
    let (mut scene_phase, mut active_scene): (ScenePhase, Option<Box<dyn Scene>>) =
        if cli.skip_intro {
            let gs: Box<dyn Scene> = Box::new(EcsScene::new(&game_lib, None, false));
            (ScenePhase::Gameplay, Some(gs))
        } else {
            (
                ScenePhase::Intro,
                Some(Box::new(IntroScene::new(intro_tracks))),
            )
        };

    // Debug console (TUI in the launch terminal), active only when --debug is passed
    let mut debug_console: Option<DebugConsole> = if cli.debug {
        match DebugConsole::new() {
            Ok(dc) => Some(dc),
            Err(e) => {
                eprintln!("Warning: could not create debug console: {}", e);
                None
            }
        }
    } else {
        None
    };
    for msg in pre_console_log.drain(..) {
        diag(&mut debug_console, msg);
    }

    // Game-side FPS tracking
    let mut game_frame_count: u64 = 0;
    let mut game_fps_time = std::time::Instant::now();
    let mut game_fps: f64 = 0.0;
    let mut game_tick_count: u64 = 0;
    let mut game_tps: f64 = 0.0;
    // Debug step budget: when the console queues /step, this many frames get
    // the real delta while clock.paused remains true. See DEBUG_SPEC §Flow.
    let mut debug_step_budget: u32 = 0;
    let mut debug_tick_hz: u32 = DEFAULT_TICK_RATE_HZ;
    let mut debug_tick_accum: f64 = 0.0;

    'running: loop {
        let raw_delta = clock.update();
        // When the debug console has paused gameplay, freeze scene time by
        // zeroing the delta. Step frames temporarily consume from the budget.
        let delta_ticks = if clock.paused && debug_step_budget == 0 {
            0
        } else {
            if debug_step_budget > 0 {
                debug_step_budget -= 1;
            }
            raw_delta
        };

        // Apply debug tick rate override (15 by default; 30 = normal; set via /rate).
        // Accumulate fractional ticks so rates like 15 Hz work correctly
        // even though raw_delta is discrete (0 or 1 per frame at 60 fps).
        // Only gameplay is subject to the tick-rate throttle; intro/cutscene
        // scenes always run at the native 30 Hz tick rate.
        let delta_ticks = if matches!(scene_phase, ScenePhase::Gameplay) {
            debug_tick_accum += delta_ticks as f64 * (debug_tick_hz as f64 / 30.0);
            let d = debug_tick_accum as u32;
            debug_tick_accum -= d as f64;
            d
        } else {
            delta_ticks
        };

        // Update game FPS counter
        game_frame_count += 1;
        game_tick_count += delta_ticks as u64;
        let fps_elapsed = game_fps_time.elapsed().as_secs_f64();
        if fps_elapsed >= 1.0 {
            game_fps = game_frame_count as f64 / fps_elapsed;
            game_tps = game_tick_count as f64 / fps_elapsed;
            game_frame_count = 0;
            game_tick_count = 0;
            game_fps_time = std::time::Instant::now();
        }

        // Poll console input (non-blocking, crossterm)
        if let Some(ref mut dc) = debug_console {
            dc.poll_input();
            if dc.take_quit_request() {
                kill_flag = true;
            }
        }

        for mut event in event_pump.poll_iter() {
            // Translate mouse coordinates from physical window space to the 640×480
            // logical render space (accounting for letterbox scaling/offset).
            // Required in SDL3: logical presentation does NOT auto-transform events.
            event.convert_coords(&canvas);

            // Let the active scene consume events first
            if let Some(ref mut scene) = active_scene {
                if scene.handle_event(&event) {
                    continue; // scene consumed this event
                }
            }

            match event {
                // handle window events
                Event::Window {
                    win_event,
                    window_id,
                    ..
                } => {
                    if window_id != canvas.window().id() {
                        // ignore events for other windows
                        continue;
                    }

                    if let WindowEvent::Moved(x, y) = win_event {
                        settings.set_window_position((x, y));
                    } else if let WindowEvent::Resized(w, h) = win_event {
                        settings.set_window_size((w as u32, h as u32));
                    }
                    dirty = true;
                }
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    kill_flag = true;
                }
                Event::KeyDown {
                    scancode,
                    keymod: _,
                    repeat: false,
                    ..
                } => {
                    // println!("Key DOWN: scancode = {:?}, mod {}", scancode, keymod);
                    if scancode.is_none() {
                        continue;
                    }

                    let sc = scancode.unwrap();
                    match sc {
                        Scancode::M => {
                            // toggle mouse cursor
                            if mouse_cursor.is_some() {
                                mouse_cursor = None;
                            } else {
                                let pointer = game_lib.get_cursor("bow");
                                if pointer.is_some() {
                                    mouse_cursor = set_mouse(pointer.unwrap(), &sys_palette);
                                }
                            }
                        }

                        Scancode::Pause | Scancode::P => {
                            // toggle pause
                            if clock.paused {
                                clock.resume();
                            } else {
                                clock.pause();
                            }
                        }

                        Scancode::F11 => {
                            let want_fs = !settings.fullscreen;
                            settings.set_fullscreen(want_fs);
                            let _ = canvas.window_mut().set_fullscreen(want_fs);
                        }

                        _ => {}
                    }
                }
                /*
                Event::KeyUp {scancode, keymod, ..}
                => {
                    println!("Key UP: scancode = {:?}, mod {}", scancode, keymod);
                },
                 */
                Event::ControllerDeviceAdded { which, .. } => {
                    let jid = sdl3::sys::joystick::SDL_JoystickID(which);
                    if gamepad_subsystem.is_gamepad(jid) {
                        match gamepad_subsystem.open(jid) {
                            Ok(c) => {
                                let name = c.name().unwrap_or_else(|| "Unknown".to_string());
                                diag(&mut debug_console, format!("Controller connected: {}", name));
                                gamepads.push(c);
                            }
                            Err(e) => {
                                diag(&mut debug_console, format!("Warning: could not open controller {}: {}", which, e));
                            }
                        }
                    }
                }
                Event::ControllerDeviceRemoved { which, .. } => {
                    let jid = sdl3::sys::joystick::SDL_JoystickID(which);
                    gamepads.retain(|c| c.id().ok() != Some(jid));
                    diag(&mut debug_console, format!("Controller disconnected (id {})", which));
                }
                _ => {}
            }
        }

        if settings.dirty {
            let result = settings.save();
            if result.is_err() {
                diag(&mut debug_console, format!("Error saving settings: {}", result.err().unwrap()));
            }
        }

        // Scene rendering takes priority when active
        if let Some(ref mut scene) = active_scene {
            let mut resources = render_resources.prepare(&mut scratch_tex, audio_system.as_ref());
            let result = scene.update(
                &mut canvas,
                &mut play_tex,
                delta_ticks,
                &game_lib,
                &mut resources,
            );
            match result {
                SceneResult::Done => {
                    scene.on_exit();

                    // Chain to next scene based on current phase
                    match scene_phase {
                        ScenePhase::Intro => {
                            // After intro, start copy protection
                            // Pass `true` for skip to bypass during development
                            let skip_copy_protect = false;
                            let q_count = game_lib.get_copy_protect_count();
                            active_scene =
                                Some(Box::new(CopyProtectScene::new(skip_copy_protect, q_count)));
                            scene_phase = ScenePhase::CopyProtect;
                        }
                        ScenePhase::CopyProtect => {
                            // Copy protection finished — quit if failed.
                            let passed = scene
                                .as_any()
                                .downcast_ref::<CopyProtectScene>()
                                .map_or(false, |cp| cp.passed());
                            if !passed {
                                diag(&mut debug_console, "Copy protection failed — exiting.");
                                break 'running;
                            }
                            // Stop intro music before gameplay begins.
                            if let Some(ref a) = audio_system {
                                a.stop_score();
                            }
                            active_scene = Some(Box::new(EcsScene::new(
                                &game_lib,
                                None,
                                true,
                            )));
                            scene_phase = ScenePhase::Gameplay;
                            dirty = true;
                            clear_flag = true;
                        }
                        ScenePhase::Gameplay => {
                            // Gameplay exited via SceneResult::Done.
                            // If the Talisman win condition fired, transition
                            // into the victory sequence (placard → winpic);
                            // otherwise treat as restart.
                            let won = false; // TODO(Plan D): EcsScene victory detection
                            if won {
                                let hero = "Julian"; // TODO(Plan D): EcsScene hero name
                                if let Some(ref a) = audio_system {
                                    a.stop_score();
                                }
                                active_scene = Some(Box::new(
                                    PlacardScene::new("player_win", "pagecolors")
                                        .with_hold_ticks(80)
                                        .with_substitution(hero),
                                ));
                                scene_phase = ScenePhase::VictoryPlacard;
                            } else {
                                // Game over or restart — re-create gameplay scene
                                active_scene = Some(Box::new(EcsScene::new(&game_lib, None, true)));
                            }
                            dirty = true;
                        }
                        ScenePhase::BrotherPlacard { ref mut dead, ref mut start } => {
                            // Sequence: show dead placard first (if any), then start placard.
                            if let Some(dead_name) = dead.take() {
                                let remaining_start = start.take();
                                active_scene = Some(Box::new(
                                    PlacardScene::new(&dead_name, "pagecolors")
                                ));
                                scene_phase = ScenePhase::BrotherPlacard {
                                    dead: None,
                                    start: remaining_start,
                                };
                            } else if let Some(start_name) = start.take() {
                                active_scene = Some(Box::new(
                                    PlacardScene::new(&start_name, "pagecolors")
                                ));
                                scene_phase = ScenePhase::BrotherPlacard {
                                    dead: None,
                                    start: None,
                                };
                            } else {
                                // Both placards shown — restore the stashed EcsScene.
                                active_scene = stashed_scene.take();
                                scene_phase = ScenePhase::Gameplay;
                                dirty = true;
                            }
                        }
                        ScenePhase::GameOverPlacard => {
                            // kevin_dead placard done — exit.
                            break 'running;
                        }
                        ScenePhase::VictoryPlacard => {
                            // Victory placard done → show the winpic image.
                            active_scene = Some(Box::new(VictoryScene::new()));
                            scene_phase = ScenePhase::VictoryImage;
                            dirty = true;
                            clear_flag = true;
                        }
                        ScenePhase::VictoryImage => {
                            // Victory image fade-out complete → exit the app.
                            break 'running;
                        }
                    }
                }
                SceneResult::Quit => {
                    scene.on_exit();
                    break 'running;
                }
                SceneResult::Continue => {
                    canvas.present();
                }
                SceneResult::BrotherSuccession { dead_placard, start_placard } => {
                    // EcsScene has already swapped the hero entity internally.
                    // Stash the EcsScene and immediately launch the first placard.
                    stashed_scene = active_scene.take();
                    if let Some(ref dead_name) = dead_placard {
                        let remaining_start = start_placard;
                        active_scene = Some(Box::new(
                            PlacardScene::new(dead_name, "pagecolors")
                        ));
                        scene_phase = ScenePhase::BrotherPlacard {
                            dead:  None,
                            start: remaining_start,
                        };
                    } else if let Some(ref start_name) = start_placard {
                        active_scene = Some(Box::new(
                            PlacardScene::new(start_name, "pagecolors")
                        ));
                        scene_phase = ScenePhase::BrotherPlacard {
                            dead:  None,
                            start: None,
                        };
                    } else {
                        // No placards — restore EcsScene immediately.
                        active_scene = stashed_scene.take();
                        scene_phase = ScenePhase::Gameplay;
                    }
                    dirty = true;
                }
                SceneResult::GameOver => {
                    // All brothers dead — show kevin_dead placard then exit.
                    active_scene = Some(Box::new(
                        PlacardScene::new("kevin_dead", "pagecolors")
                    ));
                    scene_phase = ScenePhase::GameOverPlacard;
                    dirty = true;
                }
            }
        } else if dirty {
            let clear_canvas = clear_flag;

            let _ = canvas.with_texture_canvas(&mut play_tex, |play_canvas| {
                play_canvas.set_viewport(Rect::new(16, 0, 288, 400));

                if clear_flag == true {
                    play_canvas.set_draw_color(Color::from(&sys_palette.colors[0]));
                    play_canvas.clear();
                    clear_flag = false;
                }
            });

            if clear_canvas {
                canvas.set_draw_color(Color::BLACK);
                canvas.clear();
            }

            let screen_dest = Rect::new(0, 40, 640, 400);
            canvas.copy(&play_tex, None, screen_dest).unwrap();

            // The walker indicates active rendering, when it stops, there is nothing being drawn
            if debug_console.is_some() {
                canvas.set_draw_color(Color::BLACK);
                canvas.draw_line(walker, walker.offset(4, 0)).unwrap();

                walker.x += 4;
                if walker.x >= 640 {
                    walker.x = 0;
                }

                canvas.set_draw_color(Color::RED);
                canvas.draw_line(walker, walker.offset(4, 0)).unwrap();
            }

            canvas.present();

            dirty = false;
        }

        // Feed debug commands from console into EcsScene
        // and drain gameplay debug logs back to the console.
        if let (Some(ref mut dc), Some(ref mut scene)) =
            (debug_console.as_mut(), active_scene.as_mut())
        {
            let cmds = dc.drain_commands();
            if let Some(ecs) = scene.as_any_mut().downcast_mut::<EcsScene>() {
                for cmd in cmds {
                    if let DebugCommand::SetTickRate { hz } = cmd {
                        debug_tick_hz = hz;
                        dc.log(format!(
                            "Tick rate: {} Hz  ({:.2}x speed)",
                            hz,
                            hz as f64 / 30.0
                        ));
                    } else {
                        crate::game::ecs::debug_commands::handle(cmd, &mut ecs.world, &mut ecs.res);
                    }
                }
                for msg in ecs.res.diag_log.drain(..) {
                    dc.log(msg);
                }
                let song_group_count = song_library
                    .as_ref()
                    .map(|l| l.tracks.len() / SongLibrary::VOICES)
                    .unwrap_or(0);
                let current_song_group = audio_system.as_ref().and_then(|a| a.current_group());
                // Derive time fields from ECS clock.
                let ecs_daynight = ecs.res.clock.daynight;
                let ecs_dayperiod = (ecs_daynight / 2000) as u8;
                // daynight 0..24000 ≡ 24 h; hour = daynight * 24 / 24000
                let ecs_hour = (ecs_daynight as u32 * 24 / 24000) as u32;
                let ecs_minute = ((ecs_daynight as u32 * 24 * 60 / 24000) % 60) as u32;
                let ecs_day_phase = match ecs_dayperiod {
                    0..=3 => DayPhase::Midnight,
                    4..=5 => DayPhase::Morning,
                    6..=8 => DayPhase::Midday,
                    _     => DayPhase::Evening,
                };
                // Hero position + stats from ECS.
                let (hero_x, hero_y, hero_brother, hero_vit, hero_hunger, hero_fatigue,
                     hero_brave, hero_luck, hero_kind) = {
                    use crate::game::ecs::components::{Position, BrotherKind, HeroStats};
                    let px = ecs.world.get::<&Position>(ecs.res.hero_entity)
                        .map(|p| (p.x as u16, p.y as u16)).unwrap_or((0, 0));
                    let bk = ecs.world.get::<&BrotherKind>(ecs.res.hero_entity)
                        .map(|b| b.id).unwrap_or(0);
                    let hs = ecs.world.get::<&HeroStats>(ecs.res.hero_entity);
                    let (v, h, f, br, lk, ki) = hs.as_deref().map(|s| (
                        s.vitality, s.hunger, s.fatigue, s.brave, s.luck, s.kind,
                    )).unwrap_or_default();
                    (px.0, px.1, bk, v, h, f, br, lk, ki)
                };
                let status = DebugSnapshot {
                    fps: game_fps,
                    tps: game_tps,
                    game_ticks: clock.game_ticks,
                    paused: clock.paused,
                    is_paused: clock.paused,
                    scene_name: Some("Gameplay".to_owned()),
                    song_group_count,
                    current_song_group,
                    cave_mode: audio_system.as_ref().map_or(false, |a| a.is_cave_mode()),
                    game_day: ecs.res.clock.game_days,
                    game_hour: ecs_hour,
                    game_minute: ecs_minute,
                    daynight: ecs_daynight,
                    lightlevel: ecs.res.clock.lightlevel,
                    day_phase: ecs_day_phase,
                    hero_x,
                    hero_y,
                    brother: hero_brother,
                    region_num: ecs.res.region.region_num,
                    vitality: hero_vit,
                    hunger: hero_hunger,
                    fatigue: hero_fatigue,
                    brave: hero_brave as u16,
                    luck: hero_luck,
                    kind: hero_kind,
                    ..DebugSnapshot::default()
                };
                dc.update_status(status);
            } else {
                // Not yet in gameplay (intro / copy-protect scene)
                let song_group_count = song_library
                    .as_ref()
                    .map(|l| l.tracks.len() / SongLibrary::VOICES)
                    .unwrap_or(0);
                let current_song_group = audio_system.as_ref().and_then(|a| a.current_group());
                let status = DebugSnapshot {
                    fps: game_fps,
                    tps: game_tps,
                    game_day: 0,
                    game_hour: 0,
                    game_minute: 0,
                    day_phase: DayPhase::default(),
                    daynight: 0,
                    lightlevel: 0,
                    game_ticks: clock.game_ticks,
                    paused: clock.paused,
                    scene_name: Some("Intro".to_owned()),
                    song_group_count,
                    current_song_group,
                    ..DebugSnapshot::default()
                };
                dc.update_status(status);
                // Drain any leftover commands (no-op during intro)
                for _ in cmds {}
            }

            // Handle song play/stop requests
            if let Some(group) = dc.take_song_request() {
                if let (Some(ref a), Some(ref lib)) = (audio_system.as_ref(), song_library.as_ref())
                {
                    if !a.play_group(group, lib) {
                        dc.log(format!("Song group {} not available", group));
                    }
                }
            }
            if dc.take_stop_request() {
                if let Some(ref a) = audio_system {
                    a.stop_score();
                }
            }
            if let Some(cave) = dc.take_cave_mode_request() {
                if let Some(ref a) = audio_system {
                    a.set_cave_mode(cave);
                }
            }

            // Pause/resume requests from the debug console.
            if let Some(want_pause) = dc.take_pause_request() {
                if want_pause {
                    clock.pause();
                } else {
                    clock.resume();
                }
            }
            // Step requests: advance N ticks while staying paused.
            let step_budget = dc.take_step_request();
            if step_budget > 0 {
                debug_step_budget = debug_step_budget.saturating_add(step_budget);
                // Stepping implies paused; ensure the clock is in that state.
                if !clock.paused {
                    clock.pause();
                }
            }

            dc.render();
        } else if let Some(ref mut dc) = debug_console {
            // Console active but no scene yet
            let song_group_count = song_library
                .as_ref()
                .map(|l| l.tracks.len() / SongLibrary::VOICES)
                .unwrap_or(0);
            let current_song_group = audio_system.as_ref().and_then(|a| a.current_group());
            let status = DebugSnapshot {
                fps: game_fps,
                tps: game_tps,
                game_day: 0,
                game_hour: 0,
                game_minute: 0,
                day_phase: DayPhase::default(),
                daynight: 0,
                lightlevel: 0,
                game_ticks: clock.game_ticks,
                paused: clock.paused,
                scene_name: None,
                song_group_count,
                current_song_group,
                ..DebugSnapshot::default()
            };
            dc.update_status(status);
            dc.render();
        }

        // Drain any diag_log entries not consumed by the debug console (no-console path).
        if debug_console.is_none() {
            if let Some(ref mut scene) = active_scene {
                if let Some(ecs) = scene.as_any_mut().downcast_mut::<EcsScene>() {
                    for msg in ecs.res.diag_log.drain(..) {
                        eprintln!("{msg}");
                    }
                }
            }
        }

        if kill_flag {
            break 'running;
        }
    }

    Ok(())
}
