extern crate sdl2;

mod game;

use clap::Parser;

use game::font_texture::FontTexture;
use game::game_library;

use std::collections::HashMap;

use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::{Keycode, Scancode};
use sdl2::mouse::Cursor;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::{Point, Rect};
use sdl2::surface::Surface;

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use crate::game::debug_window::{DebugWindow, DebugState};
use crate::game::game_clock::GameClock;
use crate::game::settings::{self, GameSettings};
use crate::game::image_texture;
use crate::game::cursor::CursorAsset;
use crate::game::colors::Palette;
use crate::game::scene::{Scene, SceneResources, SceneResult};
use crate::game::intro_scene::IntroScene;
use crate::game::copy_protect_scene::CopyProtectScene;
use crate::game::placard_scene::PlacardScene;
use crate::game::audio::{AudioSystem, Instruments};
use crate::game::songs::{SongLibrary, Track};

#[derive(Parser, Debug)]
#[command(name = "fmainrs", about = "The Faery Tale Adventure")]
struct Cli {
    /// Activate debug mode (opens a separate debug window)
    #[arg(long, short)]
    debug: bool,
    /// Disable linear interpolation in the PCM mixer (use nearest-neighbor instead)
    #[arg(long)]
    no_interpolation: bool,
}

fn set_mouse(cursor: &CursorAsset, color: &Palette) -> Option<Cursor> {
    // build RGBA32 pixel data from cursor and palette
    let result = cursor.bitmap.generate_rgb32(color, Some(0));
    if result.is_err() {
        println!("Error generating RGB32 data for cursor: {}", result.err().unwrap());
        return None;
    }

    let (mut pixels, stride) = result.unwrap();

    // create RGB surface from pixels, we need to use a Surface to create a color cursor
    let surface = Surface::from_data(
        &mut pixels,
        cursor.bitmap.width as u32,
        cursor.bitmap.height as u32,
        stride as u32,
        PixelFormatEnum::RGBA32).unwrap();

    // create and set the cursor
    let pointer = Cursor::from_surface(
        surface,
        cursor.hotspot.x as i32,
        cursor.hotspot.y as i32).unwrap();
    pointer.set();

    Some(pointer)
}

pub fn main() -> Result<(), String> {
    let cli = Cli::parse();

    let mut settings: GameSettings = settings::GameSettings::load();

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().expect("Could not initialize SDL2 video subsystem");


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
    let song_library: Option<SongLibrary> = SongLibrary::load(Path::new("game/songs"));
    let intro_tracks: Option<[Arc<Track>; 4]> = song_library
        .as_ref()
        .and_then(|songs| songs.intro_tracks().map(|t| t.map(|tr| Arc::new(tr.clone()))));
    let audio_system: Option<AudioSystem> = {
        match Instruments::load(Path::new("game/v6")) {
            Some(inst) => match AudioSystem::new(&sdl_context, inst, cli.no_interpolation) {
                Ok(sys) => Some(sys),
                Err(e) => { println!("Warning: could not open audio device: {}", e); None }
            },
            None => { println!("Warning: could not load game/v6 (instruments file missing)"); None }
        }
    };

    let mut mouse_cursor: Option<Cursor> = None;

    let pointer = game_lib.get_cursor("bow");
    if pointer.is_some() {
        mouse_cursor = set_mouse(pointer.unwrap(), &sys_palette);
    }

    // TODO: Move somewhere else....
    let amber = game_lib.find_font("amber", 9).unwrap();
    let topaz = game_lib.find_font("topaz", 8).unwrap();

    let amber_bounds = amber.get_font_bounds();
    // leave a little space between the two font atlases
    let mut topaz_bounds = topaz.get_font_bounds();
    topaz_bounds.offset(0, amber_bounds.height() as i32 + 4);

    let atlas_bounds = amber_bounds.union(topaz_bounds);

    // Build font textures, create a single shared texture for all font atlases
    let mut ft = tex_maker.create_texture_static(Some(sdl2::pixels::PixelFormatEnum::RGBA32), atlas_bounds.width(), atlas_bounds.height()).unwrap();
    ft.set_blend_mode(sdl2::render::BlendMode::Blend);
    let font_texture = Rc::new(RefCell::new(ft));

    let amber_font = FontTexture::new(&amber, &amber_bounds, Rc::downgrade(&font_texture));
    let topaz_font = FontTexture::new(&topaz, &topaz_bounds, Rc::downgrade(&font_texture));


    // Build the image textures, using a shared texture.
    // Also build a name-to-index map so scenes can look up images by name.
    let image_atlas_bounds = Rect::new(0, 0, 4096, 4096);
    let image_texture = Rc::new(RefCell::new(tex_maker.create_texture_static(Some(sdl2::pixels::PixelFormatEnum::RGBA32), image_atlas_bounds.width(), image_atlas_bounds.height()).unwrap()));

    let mut next_x: u32 = 0;
    let mut next_y: u32 = 0;
    let mut max_y: u32 = 0; // track tallest image in the row, we won't bother packing tightly
    let mut image_textures: Vec<image_texture::ImageTexture> = Vec::new();
    let mut image_name_map: HashMap<String, usize> = HashMap::new();

    let image_names = game_lib.get_image_names();
    for name in &image_names {
        let img = game_lib.find_image(name).unwrap();
        let iff_image = &img.image;
        if iff_image.is_none() {
            println!("Warning: ImageAsset {} has no IffImage data", img.path);
            continue;
        }
        let iff_image = iff_image.as_ref().unwrap();

        // calculate position in texture atlas
        if next_x + iff_image.width as u32 > image_atlas_bounds.width() {
            // move to next row
            next_x = 0;
            next_y += max_y;
            max_y = 0;
        }
        let image_texture_bounds = Rect::new(
            next_x as i32,
            next_y as i32,
            iff_image.width as u32,
            iff_image.height as u32
        );
        let mut img_tex = image_texture::ImageTexture::new(
            iff_image,
            &image_texture_bounds,
            Rc::downgrade(&image_texture)
        );
        if iff_image.colormap.is_some() {
            let colormap = iff_image.colormap.as_ref().unwrap();
            img_tex.update(colormap, iff_image.transparent_color);
        } else {
            img_tex.update(&sys_palette, iff_image.transparent_color);
        }

        next_x += iff_image.width as u32;
        if iff_image.height as u32 > max_y {
            max_y = iff_image.height as u32;
        }
        image_name_map.insert(name.clone(), image_textures.len());
        image_textures.push(img_tex);
    }

    let mut play_tex = tex_maker.create_texture_target(PixelFormatEnum::RGBA32, 320, 200).unwrap();
    let mut scratch_tex = tex_maker.create_texture_target(PixelFormatEnum::RGBA32, 320, 200).unwrap();

    let mut dirty: bool = true;

    let mut clear_flag = true;

    let mut kill_flag = false;

    let mut walker: Point = Point::new(0,20);

    let mut clock: GameClock = GameClock::new();
    let mut last_minute: u32 = 0;

    // Scene system — scenes chain: Intro → CopyProtect → PlacardStart → (gameplay)
    // The scene_phase tracks what to start next when a scene completes.
    enum ScenePhase { Intro, CopyProtect, PlacardStart, Gameplay }
    let mut scene_phase = ScenePhase::Intro;
    let mut active_scene: Option<Box<dyn Scene>> = Some(Box::new(IntroScene::new(intro_tracks)));

    // Debug window (separate SDL2 window), created only when --debug is passed
    let mut debug_window: Option<DebugWindow> = if cli.debug {
        let game_pos = settings.window_position;
        let game_size = settings.window_size.unwrap_or((width, height));
        match DebugWindow::new(&video_subsystem, &topaz, &settings, game_pos, game_size) {
            Ok(dw) => {
                println!("Debug window opened");
                Some(dw)
            }
            Err(e) => {
                println!("Warning: could not create debug window: {}", e);
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

    // Pre-compute sorted name lists for the debug window tabs
    let debug_placard_names = game_lib.get_placard_names();
    let debug_image_names = game_lib.get_image_names();

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

        for event in event_pump.poll_iter() {
            // Let the debug window consume its own events first
            if let Some(ref mut dw) = debug_window {
                if dw.handle_event(&event, &mut settings) {
                    continue;
                }
            }

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
                Event::KeyDown {scancode, keymod, repeat: false, .. }
                => {
                    // println!("Key DOWN: scancode = {:?}, mod {}", scancode, keymod);
                    if scancode.is_none() {
                        continue;
                    }

                    let sc = scancode.unwrap();
                    match sc {
                        Scancode::A => {
                            if keymod.intersects(sdl2::keyboard::Mod::LSHIFTMOD | sdl2::keyboard::Mod::RSHIFTMOD) {
                                // advance to 4:00 AM
                                clock.advance_game_wall_clock_to(4, 0);
                            } else {
                                // jump ahead 2 hours
                                clock.advance_game_wall_clock_by(2, 0);
                            }
                        }

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
                _ => {}
            }
        }

        if settings.dirty {
            let result = settings.save();
            if result.is_err() {
                println!("Error saving settings: {}", result.err().unwrap());
            }
        }

        let (_day, _hour, minute) = clock.get_game_wall_clock();
        if minute != last_minute {
            last_minute = minute;
            dirty = true;
        }

        // Scene rendering takes priority when active
        if let Some(ref mut scene) = active_scene {
            // Build rendering resources for the scene
            let mut resources = SceneResources {
                image_textures: &mut image_textures,
                image_name_map: &image_name_map,
                amber_font: &amber_font,
                topaz_font: &topaz_font,
                scratch: &mut scratch_tex,
                audio: audio_system.as_ref(),
            };
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
                            )));
                            scene_phase = ScenePhase::PlacardStart;
                        }
                        ScenePhase::PlacardStart => {
                            // Placard shown — stop music before gameplay begins.
                            // Original: stopscore() called after copy protection, before main loop.
                            if let Some(ref a) = audio_system {
                                a.stop_score();
                            }
                            active_scene = None;
                            scene_phase = ScenePhase::Gameplay;
                            dirty = true;
                            clear_flag = true;
                        }
                        ScenePhase::Gameplay => {
                            active_scene = None;
                            dirty = true;
                            clear_flag = true;
                        }
                    }
                }
                SceneResult::Continue => {
                    // Scene handles its own rendering and canvas.present()
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
            if debug_window.is_some() {
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

        // Render the debug window (separate from game canvas)
        if let Some(ref mut dw) = debug_window {
            let scene_name: Option<&str> = if active_scene.is_some() {
                Some("IntroScene")
            } else {
                None
            };

            let placard_idx = dw.placard_index();
            let current_placard = if placard_idx < debug_placard_names.len() {
                game_lib.find_placard(&debug_placard_names[placard_idx])
            } else {
                None
            };

            let img_idx = dw.image_index();
            let image_dims = if img_idx < image_textures.len() {
                let b = image_textures[img_idx].get_bounds();
                Some((b.width(), b.height()))
            } else {
                None
            };

            let song_group_count = song_library
                .as_ref()
                .map(|l| l.tracks.len() / SongLibrary::VOICES)
                .unwrap_or(0);
            let current_song_group = audio_system.as_ref().and_then(|a| a.current_group());

            let (gday, ghour, gminute) = clock.get_game_wall_clock();
            let state = DebugState {
                game_day: gday,
                game_hour: ghour,
                game_minute: gminute,
                day_phase: clock.get_day_phase(),
                game_ticks: clock.game_ticks,
                mono_ticks: clock.mono_ticks,
                paused: clock.paused,
                scene_name,
                fps: game_fps,
                placard_names: &debug_placard_names,
                current_placard,
                sys_palette: &sys_palette,
                image_names: &debug_image_names,
                image_dimensions: image_dims,
                song_group_count,
                current_song_group,
            };
            dw.render(&state);

            // Handle song play/stop requests from the Songs tab
            if let Some(group) = dw.take_song_request() {
                if let (Some(ref a), Some(ref lib)) = (audio_system.as_ref(), song_library.as_ref()) {
                    if !a.play_group(group, lib) {
                        println!("Debug: song group {} not available", group);
                    }
                }
            }
            if dw.take_stop_request() {
                if let Some(ref a) = audio_system {
                    a.stop_score();
                }
            }
        }

        if kill_flag {
            break 'running
        }
    }

    Ok(())
}
