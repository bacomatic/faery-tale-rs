extern crate sdl2;

mod game;

use game::font_texture::FontTexture;
use game::game_library;

use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::{Keycode, Scancode};
use sdl2::mouse::Cursor;
use sdl2::pixels::{Color, PixelFormatEnum};
use sdl2::rect::{Point, Rect};
use sdl2::surface::Surface;

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use crate::game::game_clock::GameClock;
use crate::game::settings::{self, GameSettings};
use crate::game::placard::*;
use crate::game::cursor::CursorAsset;
use crate::game::gfx::Palette;
use crate::game::render_task::RenderTask;

#[derive(Debug, Clone, Copy)]
struct CycleInt {
    pub value: usize,
    pub max: usize
}

impl CycleInt {
    pub fn new(max: usize) -> CycleInt {
        CycleInt {value: 0, max: max}
    }

    pub fn modify(&mut self, increase: bool) {
        if increase {
            self.inc();
        } else {
            self.dec();
        }
    }

    pub fn set(&mut self, value: usize) -> usize {
        if value > self.max {
            self.value = self.max;
        } else {
            self.value = value;
        }
        self.value
    }

    pub fn inc(&mut self) -> usize {
        self.value += 1;
        if self.value > self.max {
            self.value = 0;
        }
        self.value
    }

    pub fn dec(&mut self) -> usize {
        if self.value == 0 {
            self.value = self.max;
        } else {
            self.value -= 1;
        }
        self.value
    }
}

pub struct NameCycler {
    names: Vec<String>,
    index: CycleInt
}

impl NameCycler {
    pub fn new(names: Vec<String>) -> NameCycler {
        let max = if names.len() == 0 { 0 } else { names.len() - 1 };
        NameCycler {
            names,
            index: CycleInt::new(max)
        }
    }

    pub fn get_current(&self) -> Option<&String> {
        if self.names.len() == 0 {
            None
        } else {
            Some(&self.names[self.index.value])
        }
    }

    pub fn modify(&mut self, increase: bool) {
        self.index.modify(increase);
    }

    pub fn set(&mut self, name: &str) -> Option<usize> {
        for (i, n) in self.names.iter().enumerate() {
            if n == name {
                self.index.set(i);
                return Some(i);
            }
        }
        None
    }
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

    let sys_palette = game_lib.find_palette("pagecolors").unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut color_index = 0;

    let mut mouse_cursor: Option<Cursor> = None;

    let pointer = game_lib.get_cursor("bow");
    if pointer.is_some() {
        mouse_cursor = set_mouse(pointer.unwrap(), &sys_palette);
    }

    let placard_names = game_lib.get_placard_names();
    let mut placard_cycler = NameCycler::new(placard_names);
    placard_cycler.set("julian_start");

    let font_names = game_lib.get_font_names();
    let mut font_cycler = NameCycler::new(font_names);
    font_cycler.set("amber");

    // TODO: Move somewhere else....
    let amber = game_lib.find_font("amber", 9).unwrap();
    let topaz = game_lib.find_font("topaz", 8).unwrap();

    let amber_bounds = amber.get_font_bounds();
    // leave a little space between the two font atlases
    let mut topaz_bounds = topaz.get_font_bounds();
    topaz_bounds.offset(0, amber_bounds.height() as i32 + 4);

    let atlas_bounds = amber_bounds.union(topaz_bounds);

    // Build font textures, create a single shared texture for all font atlases
    let font_texture = Rc::new(RefCell::new(tex_maker.create_texture_static(Some(sdl2::pixels::PixelFormatEnum::BGRA8888), atlas_bounds.width(), atlas_bounds.height()).unwrap()));

    let amber_text = Rc::new(RefCell::new(FontTexture::new(&amber, &amber_bounds, Rc::downgrade(&font_texture))));
    let topaz_text = Rc::new(RefCell::new(FontTexture::new(&topaz, &topaz_bounds, Rc::downgrade(&font_texture))));

    let mut play_tex = tex_maker.create_texture_target(PixelFormatEnum::BGRA8888, 320, 200).unwrap();

    let mut dirty: bool = true;

    let mut text_color: CycleInt = CycleInt::new(sys_palette.colors.len() - 1);
    text_color.set(24);

    // use a weak reference here because we need to be able to swap fonts
    let mut text_font = Rc::downgrade(&amber_text);

    let mut placard_task: Option<Box<dyn RenderTask>> = Some(Box::new(start_placard_renderer(&Point::new(0,0), sys_palette)));
    let mut update_text = true;
    let mut clear_flag = true;

    let mut kill_flag = false;

    let mut walker: Point = Point::new(0,20);

    let mut clock: GameClock = GameClock::new();
    let mut last_minute: u32 = 0;

    'running: loop {
        for event in event_pump.poll_iter() {
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
                        Scancode::Up => {
                            color_index = (color_index + 1) % 4;
                            dirty = true;
                        }
                        Scancode::Down => {
                            if color_index == 0 {
                                color_index = 3;
                            } else {
                                color_index -= 1;
                            }
                            dirty = true;
                        }
                        Scancode::Left | Scancode::Right => {
                            let increase = sc == Scancode::Right;
                            placard_cycler.modify(increase);
                            clear_flag = true;
                            update_text = true;
                            placard_task = Some(Box::new(start_placard_renderer(&Point::new(0,0), sys_palette)));
                            // FIXME: push placard render task, which will kick off the border renderer
                            dirty = true;
                        }

                        Scancode::A => {
                            if keymod.intersects(sdl2::keyboard::Mod::LSHIFTMOD | sdl2::keyboard::Mod::RSHIFTMOD) {
                                // advance to 4:00 AM
                                clock.advance_game_wall_clock_to(4, 0);
                            } else {
                                // jump ahead 2 hours
                                clock.advance_game_wall_clock_by(2, 0);
                            }
                        }

                        Scancode::B => {
                            clear_flag = true;
                            dirty = true;
                        }

                        Scancode::F => {
                            font_cycler.modify(true);
                            match font_cycler.get_current() {
                                Some(fname) => {
                                    match fname.as_str() {
                                        "amber" => {
                                            text_font = Rc::downgrade(&amber_text);
                                        },
                                        "topaz" => {
                                            text_font = Rc::downgrade(&topaz_text);
                                        },
                                        _ => {}
                                    }
                                },
                                None => {}
                            }
                            if text_font.upgrade().unwrap().borrow().name() == amber_text.borrow().name() {
                                text_font = Rc::downgrade(&topaz_text);
                            } else {
                                text_font = Rc::downgrade(&amber_text);
                            }
                            update_text = true;
                            dirty = true;
                        }

                        Scancode::C => {
                            // Cycle text color
                            text_color.modify(keymod.intersects(sdl2::keyboard::Mod::LSHIFTMOD | sdl2::keyboard::Mod::RSHIFTMOD));
                            update_text = true;
                            dirty = true;
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

                        Scancode::Num1 | Scancode::Num2 | Scancode::Num3 | Scancode::Num4
                        | Scancode::Num5 | Scancode::Num6 | Scancode::Num7 | Scancode::Num8
                        | Scancode::Num9 | Scancode::Num0 => {
                            // keycode is i32 containing ASCII '0'+code, so NUM_0 == '0'
                            // mode = keycode.unwrap_or_else(|| Keycode::NUM_0).into_i32() - '0' as i32;
                            dirty = true;
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

        clock.update();
        let (_day, _hour, minute) = clock.get_game_wall_clock();
        if minute != last_minute {
            last_minute = minute;
            dirty = true;
        }

        if dirty {
            // FIXME: move render tasks to an object to track them and poll that instead
            let mut still_dirty = false;
            let clear_canvas = clear_flag;

            let _ = canvas.with_texture_canvas(&mut play_tex, |mut play_canvas| {

                play_canvas.set_viewport(Rect::new(16, 0, 288, 400));

                if clear_flag == true {
                    play_canvas.set_draw_color(Color::from(&sys_palette.colors[0]));
                    play_canvas.clear();
                    clear_flag = false;
                }

                if placard_task.is_some() {
                    let result = placard_task.as_mut().unwrap().update(&mut play_canvas, 1, None);
                    if result == false {
                        placard_task = None;
                    } else {
                        still_dirty = true;
                    }
                }

                if update_text {
                    {
                        let mut tex_borrow = font_texture.borrow_mut();
                        let color = sys_palette.get_color(text_color.value).unwrap();
                        tex_borrow.set_color_mod(color.r(), color.g(), color.b());
                    }
                    let result = game_lib.find_placard(placard_cycler.get_current().unwrap());
                    if result.is_some() {
                        let placard = result.unwrap();
                        placard.draw(&text_font.upgrade().unwrap().borrow(), &mut play_canvas);
                    }
                    update_text = false;
                }
            });

            if clear_canvas {
                canvas.set_draw_color(Color::BLACK);
                canvas.clear();
            }

            let screen_dest = Rect::new(0, 40, 640, 400);
            canvas.copy(&play_tex, None, Some(screen_dest)).unwrap();

            // The walker indicates active rendering, when it stops, there is nothing being drawn
            {
                canvas.set_draw_color(Color::BLACK);
                canvas.draw_line(walker, walker.offset(4, 0)).unwrap();

                walker.x += 4;
                if walker.x >= 640 {
                    walker.x = 0;
                }

                canvas.set_draw_color(Color::RED);
                canvas.draw_line(walker, walker.offset(4, 0)).unwrap();
            }

            {
                // draw game clock
                let (day, hour, minute) = clock.get_game_wall_clock();
                let time_str = format!("Day {:02} - Time {:02}:{:02}", day, hour, minute);
                topaz_text.borrow().render_string(
                    &time_str,
                    &mut canvas,
                    20,
                    10
                );
            }

            canvas.present();

            dirty = still_dirty;
        }

        if kill_flag {
            break 'running
        }
    }

    Ok(())
}
