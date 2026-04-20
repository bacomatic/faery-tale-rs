extern crate sdl2;

mod game;

use clap::Parser;

use game::game_library;

use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::{Keycode, Scancode};
use sdl2::mouse::Cursor;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::{Point, Rect};
use sdl2::surface::Surface;

use std::path::Path;
use std::sync::Arc;

use crate::game::debug_console::{DebugConsole, DebugSnapshot};
use crate::game::game_clock::GameClock;
use crate::game::game_state::DayPhase;
use crate::game::settings::{self, GameSettings};
use crate::game::cursor::CursorAsset;
use crate::game::colors::Palette;
use crate::game::render_resources::RenderResources;
use crate::game::scene::{Scene, SceneResult};
use crate::game::intro_scene::IntroScene;
use crate::game::copy_protect_scene::CopyProtectScene;
use crate::game::placard_scene::PlacardScene;
use crate::game::gameplay_scene::GameplayScene;
use crate::game::victory_scene::VictoryScene;
use crate::game::audio::{AudioSystem, Instruments};
use crate::game::songs::{SongLibrary, Track};

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
    /// Echo every story-transcript message to the console as it is generated
    #[arg(long)]
    echo_transcript: bool,
}

fn set_mouse(cursor: &CursorAsset, color: &Palette) -> Option<Cursor> {
    // build RGBA32 pixel data from cursor and palette
    let result = cursor.bitmap.generate_rgb32(color, Some(0));
    if result.is_err() {
        println!("Error generating RGB32 data for cursor: {}", result.err().unwrap());
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
        PixelFormatEnum::RGBA32).unwrap();

    // Scale 2× for better visual appearance (matches the 2× line-doubled canvas)
    let mut scaled = Surface::new(orig_w * 2, orig_h * 2, PixelFormatEnum::RGBA32).unwrap();
    surface.blit_scaled(None, &mut scaled, None).unwrap();

    // create and set the cursor (hotspot also scaled 2×)
    let pointer = Cursor::from_surface(
        scaled,
        (cursor.hotspot.x * 2) as i32,
        (cursor.hotspot.y * 2) as i32).unwrap();
    pointer.set();

    Some(pointer)
}

pub fn main() -> Result<(), String> {
    let cli = Cli::parse();

    let mut settings: GameSettings = settings::GameSettings::load();

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().expect("Could not initialize SDL2 video subsystem");

    // Initialize game controller subsystem so SDL2 generates ControllerButton/Axis events.
    let game_controller_subsystem = sdl_context.game_controller()
        .map_err(|e| format!("Could not initialize game controller subsystem: {}", e))?;
    let mut controllers: Vec<sdl2::controller::GameController> = Vec::new();
    // Open any controllers that are already connected at startup.
    for i in 0..game_controller_subsystem.num_joysticks().unwrap_or(0) {
        if game_controller_subsystem.is_game_controller(i) {
            match game_controller_subsystem.open(i) {
                Ok(c) => {
                    println!("Controller connected: {}", c.name());
                    controllers.push(c);
                }
                Err(e) => println!("Warning: could not open controller {}: {}", i, e),
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

    // TODO: full screen mode, use window size for screen resolution
    if settings.window_position.is_some() {
        let (x, y) = settings.window_position.unwrap();
        window_builder.position(x, y);
    } else {
        window_builder.position_centered();
    }

    let window = window_builder
        .build()
        .unwrap();

    let mut canvas = window.into_canvas()
        .accelerated()
        .target_texture()
        .present_vsync()
        .build().unwrap();
    // Set the logical size to 640x480 to preserve the original 4:3 aspect ratio
    canvas.set_logical_size(640, 480).unwrap();

    // load the game library
    let game_lib = game_library::load_game_library(Path::new("faery.toml"));
    if game_lib.is_err() {
        return Err(format!("Failed to load game library: {}", game_lib.err().unwrap()));
    }
    let game_lib = game_lib.unwrap();

    let tex_maker = canvas.texture_creator();

    let sys_palette = game_lib.find_palette("introcolors").unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();

    // Audio system — load songs and waveforms, init the software synthesizer.
    // Music playback is started by IntroScene (matching original: playscore() is
    // called mid-intro, not at startup) and stopped before gameplay begins.
    let songs_path = game_lib.audio.as_ref().map(|a| a.songs.as_str()).unwrap_or("game/songs");
    let instruments_path = game_lib.audio.as_ref().map(|a| a.instruments.as_str()).unwrap_or("game/v6");
    let song_library: Option<SongLibrary> = SongLibrary::load(Path::new(songs_path));
    let intro_tracks: Option<[Arc<Track>; 4]> = song_library
        .as_ref()
        .and_then(|songs| songs.intro_tracks().map(|t| t.map(|tr| Arc::new(tr.clone()))));
    let audio_system: Option<AudioSystem> = {
        match Instruments::load(Path::new(instruments_path)) {
            Some(inst) => match AudioSystem::new(&sdl_context, inst, cli.no_interpolation) {
                Ok(sys) => Some(sys),
                Err(e) => { println!("Warning: could not open audio device: {}", e); None }
            },
            None => { println!("Warning: could not load {} (instruments file missing)", instruments_path); None }
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

    // Build all SDL2 rendering resources (font atlas, image atlas, render targets).
    let mut render_resources = RenderResources::build(&tex_maker, &game_lib, &sys_palette);

    let mut play_tex = tex_maker.create_texture_target(PixelFormatEnum::RGBA32, 320, 200).unwrap();
    let mut scratch_tex = tex_maker.create_texture_target(PixelFormatEnum::RGBA32, 320, 200).unwrap();

    let mut dirty: bool = true;
    let mut clear_flag = true;
    let mut kill_flag = false;
    let mut walker: Point = Point::new(0, 20);

    let mut clock: GameClock = GameClock::new();

    // Scene system — scenes chain: Intro → CopyProtect → PlacardStart → (gameplay)
    // The scene_phase tracks what to start next when a scene completes.
    enum ScenePhase { Intro, CopyProtect, PlacardStart, Gameplay, VictoryPlacard, VictoryImage }
    let (mut scene_phase, mut active_scene): (ScenePhase, Option<Box<dyn Scene>>) =
        if cli.skip_intro {
            let mut gs = GameplayScene::new();
            gs.init_from_library(&game_lib);
            gs.set_echo_transcript(cli.echo_transcript);
            (ScenePhase::Gameplay, Some(Box::new(gs)))
        } else {
            (ScenePhase::Intro, Some(Box::new(IntroScene::new(intro_tracks))))
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

    // Game-side FPS tracking
    let mut game_frame_count: u64 = 0;
    let mut game_fps_time = std::time::Instant::now();
    let mut game_fps: f64 = 0.0;

    'running: loop {
        let delta_ticks = clock.update();

        // Update game FPS counter
        game_frame_count += 1;
        let fps_elapsed = game_fps_time.elapsed().as_secs_f64();
        if fps_elapsed >= 1.0 {
            game_fps = game_frame_count as f64 / fps_elapsed;
            game_frame_count = 0;
            game_fps_time = std::time::Instant::now();
        }

        // Poll console input (non-blocking, crossterm)
        if let Some(ref mut dc) = debug_console {
            dc.poll_input();
            if dc.take_quit_request() {
                kill_flag = true;
            }
        }

        for event in event_pump.poll_iter() {

            // Let the active scene consume events first
            if let Some(ref mut scene) = active_scene {
                if scene.handle_event(&event) {
                    continue; // scene consumed this event
                }
            }

            match event {
                // handle window events
                Event::Window { win_event, window_id, .. } => {
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
                },
                Event::Quit { .. } |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. }
                => {
                    kill_flag = true;
                },
                Event::KeyDown {scancode, keymod: _, repeat: false, .. }
                => {
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

                        Scancode::Pause |
                        Scancode::P => {
                            // toggle pause
                            if clock.paused {
                                clock.resume();
                            } else {
                                clock.pause();
                            }
                        }

                        _ => {}
                    }
                },
                /*
                Event::KeyUp {scancode, keymod, ..}
                => {
                    println!("Key UP: scancode = {:?}, mod {}", scancode, keymod);
                },
                 */
                Event::ControllerDeviceAdded { which, .. } => {
                    if game_controller_subsystem.is_game_controller(which) {
                        match game_controller_subsystem.open(which) {
                            Ok(c) => {
                                println!("Controller connected: {}", c.name());
                                controllers.push(c);
                            }
                            Err(e) => println!("Warning: could not open controller {}: {}", which, e),
                        }
                    }
                }
                Event::ControllerDeviceRemoved { which, .. } => {
                    controllers.retain(|c| c.instance_id() != which);
                    println!("Controller disconnected (id {})", which);
                }
                _ => {}
            }
        }

        if settings.dirty {
            let result = settings.save();
            if result.is_err() {
                println!("Error saving settings: {}", result.err().unwrap());
            }
        }

        // Scene rendering takes priority when active
        if let Some(ref mut scene) = active_scene {
            let mut resources = render_resources.prepare(&mut scratch_tex, audio_system.as_ref());
            let result = scene.update(&mut canvas, &mut play_tex, delta_ticks, &game_lib, &mut resources);
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
                            active_scene = Some(Box::new(CopyProtectScene::new(skip_copy_protect, q_count)));
                            scene_phase = ScenePhase::CopyProtect;
                        }
                        ScenePhase::CopyProtect => {
                            // Copy protection finished — quit if failed
                            let passed = scene.as_any()
                                .downcast_ref::<CopyProtectScene>()
                                .map_or(false, |cp| cp.passed());
                            if !passed {
                                println!("Copy protection failed — exiting.");
                                break 'running;
                            }
                            active_scene = Some(Box::new(PlacardScene::new(
                                "julian_start",
                                "pagecolors",
                            ).with_hold_ticks(300))); // 10s at 30Hz
                            scene_phase = ScenePhase::PlacardStart;
                        }
                        ScenePhase::PlacardStart => {
                            // Placard shown — stop music before gameplay begins.
                            // Original: stopscore() called after copy protection, before main loop.
                            if let Some(ref a) = audio_system {
                                a.stop_score();
                            }
                            let mut gs = GameplayScene::new();
                            gs.init_from_library(&game_lib);
                            gs.set_echo_transcript(cli.echo_transcript);
                            active_scene = Some(Box::new(gs));
                            scene_phase = ScenePhase::Gameplay;
                            dirty = true;
                            clear_flag = true;
                        }
                        ScenePhase::Gameplay => {
                            // Gameplay exited via SceneResult::Done.
                            // If the Talisman win condition fired, transition
                            // into the victory sequence (placard → winpic);
                            // otherwise treat as restart.
                            let won = scene.as_any().downcast_ref::<GameplayScene>()
                                .map(|gs| gs.is_victory()).unwrap_or(false);
                            if won {
                                let hero = scene.as_any().downcast_ref::<GameplayScene>()
                                    .map(|gs| gs.hero_name()).unwrap_or("Julian");
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
                                // Game over or restart — re-create GameplayScene
                                let mut gs = GameplayScene::new();
                                gs.init_from_library(&game_lib);
                                gs.set_echo_transcript(cli.echo_transcript);
                                active_scene = Some(Box::new(gs));
                            }
                            dirty = true;
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
            canvas.copy(&play_tex, None, Some(screen_dest)).unwrap();

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

        // Feed debug commands from console into GameplayScene
        // and drain gameplay debug logs back to the console.
        if let (Some(ref mut dc), Some(ref mut scene)) = (debug_console.as_mut(), active_scene.as_mut()) {
            let cmds = dc.drain_commands();
            if let Some(gs) = scene.as_any_mut().downcast_mut::<GameplayScene>() {
                for cmd in cmds {
                    gs.apply_command(cmd);
                }
                for msg in gs.drain_logs() {
                    dc.log(msg);
                }
                // Build status snapshot
                let song_group_count = song_library
                    .as_ref()
                    .map(|l| l.tracks.len() / SongLibrary::VOICES)
                    .unwrap_or(0);
                let current_song_group = audio_system.as_ref().and_then(|a| a.current_group());
                let (gday, ghour, gminute) = gs.state.daynight_to_wall_clock();
                let status = DebugSnapshot {
                    fps: game_fps,
                    game_day: gday,
                    game_hour: ghour,
                    game_minute: gminute,
                    day_phase: gs.state.get_day_phase(),
                    daynight: gs.state.daynight,
                    lightlevel: gs.state.lightlevel,
                    game_ticks: clock.game_ticks,
                    paused: clock.paused,
                    scene_name: Some("Gameplay".to_owned()),
                    hero_x: gs.state.hero_x,
                    hero_y: gs.state.hero_y,
                    brother: gs.state.brother,
                    region_num: gs.state.region_num,
                    vitality: gs.state.vitality,
                    hunger: gs.state.hunger,
                    fatigue: gs.state.fatigue,
                    god_mode_flags: gs.state.god_mode.bits(),
                    time_held: gs.state.freeze_sticky,
                    autosave_enabled: false, // field is private; toggled by command
                    song_group_count,
                    current_song_group,
                    cave_mode: audio_system.as_ref().map_or(false, |a| a.is_cave_mode()),
                    current_zone_idx: {
                        let (idx, _) = gs.current_zone_info();
                        idx
                    },
                    current_zone_label: {
                        let (_, label) = gs.current_zone_info();
                        label
                    },
                    vfx_jewel_active: gs.state.light_timer > 0,
                    vfx_light_sticky: gs.state.light_sticky,
                    vfx_secret_active: gs.state.region_num == 9 && gs.state.secret_timer > 0,
                    vfx_witch_active: gs.is_witch_active(),
                    vfx_teleport_active: gs.is_teleport_active(),
                    vfx_palette_xfade: gs.is_palette_xfade_active(),
                    time_period: crate::game::debug_console::day_phase_label(gs.state.get_day_phase()),
                    is_paused: clock.paused,
                    princess_captive: gs.state.world_objects.get(9).map_or(false, |o| o.ob_stat != 0),
                    princess_rescues: gs.state.princess as u16,
                    statues_collected: gs.state.stuff().get(25).copied().unwrap_or(0) as u8,
                    has_writ: gs.state.stuff().get(28).copied().unwrap_or(0) != 0,
                    has_talisman: gs.state.stuff().get(22).copied().unwrap_or(0) != 0,
                    encounter_number: gs.state.encounter_number,
                    encounter_type: gs.state.encounter_type as u8,
                    active_enemy_count: gs.state.anix as u8,
                    actors: gs.state.actors.iter().enumerate()
                        .filter(|(_, a)| a.is_active())
                        .take(20)
                        .map(|(slot, a)| crate::game::debug_console::ActorSnapshot::from_actor(slot as u8, a))
                        .collect(),
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
                for _ in cmds { }
            }

            // Handle song play/stop requests
            if let Some(group) = dc.take_song_request() {
                if let (Some(ref a), Some(ref lib)) = (audio_system.as_ref(), song_library.as_ref()) {
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

        if kill_flag {
            break 'running
        }
    }

    Ok(())
}
