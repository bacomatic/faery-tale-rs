extern crate sdl2;

mod game;

use game::font_texture::FontTexture;
use game::game_library;
use game::gfx::Palette4;
use game::font::DiskFont;

use sdl2::event::Event;
use sdl2::gfx::framerate::FPSManager;
use sdl2::keyboard::{Keycode, Scancode};
use sdl2::pixels::Color;

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

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

pub fn main() -> Result<(), String> {
    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();

    let mut fps_man = FPSManager::new();
    fps_man.set_framerate(60)?;

    let window = video_subsystem.window("The Faery Tale Adventure", 1280, 960)
        .position_centered()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas().build().unwrap();

    // load the game library
    let game_lib = game_library::load_game_library(Path::new("faery.json")).unwrap();
    let tex_maker = canvas.texture_creator();

    let ref orange = Color::RGB(230, 100, 0);

    let sys_palette: Palette4 = [
        (0, 0, 0).into(),
        0x0FFF.into(),
        [0, 0, 255].into(),
        orange.into()
    ];

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut color_index = 0;

    // FIXME: implement font/asset loader and move all this code there

    let amber: DiskFont = game::font::load_font(Path::new("game/fonts/Amber/9")).unwrap();
    let topaz: DiskFont = game::font::load_font(Path::new("game/fonts/topaz13/8")).unwrap();

    let amber_bounds = amber.get_font_bounds();
    // leave a little space between the two font atlases
    let mut topaz_bounds = topaz.get_font_bounds();
    topaz_bounds.offset(0, amber_bounds.height() as i32 + 4);

    let atlas_bounds = amber_bounds.union(topaz_bounds);

    // Build font textures, create a single shared texture for all font atlases
    let font_texture = Rc::new(RefCell::new(tex_maker.create_texture_static(Some(sdl2::pixels::PixelFormatEnum::BGRA8888), atlas_bounds.width(), atlas_bounds.height()).unwrap()));

    let amber_text = Rc::new(RefCell::new(FontTexture::new(&amber, &amber_bounds, Rc::downgrade(&font_texture))));
    let topaz_text = Rc::new(RefCell::new(FontTexture::new(&topaz, &topaz_bounds, Rc::downgrade(&font_texture))));

    // SDL treats all pixels as 1:1 aspect ratio, we're working with graphics intended for a 4:3 (standard NTSC) aspect ratio
    // Set up the render texture at the original 640x400 then stretch to the window size. This will correct the aspect ratio so the game looks correct
    // 640x400 -> 1280x960 produces a 4:3 aspect ratio.

    // let mut play_tex = tex_maker.create_texture_target(tex_maker.default_pixel_format(), 640, 400).unwrap();
    let mut play_tex = tex_maker.create_texture_target(tex_maker.default_pixel_format(), 320, 200).unwrap();

    let mut dirty: bool = true;
    let mut mode: i32 = 1;

    let mut placard: CycleInt = CycleInt::new(game_lib.get_placard_count().saturating_sub(1));
    let mut message_index: CycleInt = CycleInt::new(100);

    let mut text_color: CycleInt = CycleInt::new(3);
    text_color.set(3);

    // use a weak reference here because we need to be able to swap fonts
    let mut text_font = Rc::downgrade(&amber_text);

    'running: loop {
        if dirty {
            let _ = canvas.with_texture_canvas(&mut play_tex, |mut play_canvas| {
                play_canvas.set_draw_color(Color::from(&sys_palette[color_index]));
                play_canvas.clear();

                // set text color
                {
                    let color = &sys_palette[text_color.value];

                    let mut result = font_texture.try_borrow_mut();
                    match result {
                        Err(e) => {
                            println!("Error borrowing font texture: {}", e);
                        },
                        Ok(ref mut texture) => {
                            texture.set_color_mod(color.r(), color.g(), color.b());
                            texture.set_blend_mode(sdl2::render::BlendMode::Blend);
                        }
                    }
                }

                match mode {
                    1 => {
                        game_lib.draw_placard_n(placard.value, &text_font.upgrade().unwrap().borrow(), &mut play_canvas);
                        // game_lib.print_placard_n(placard.value);
                    }
                    2 => {
                        amber_text.borrow().render_string("\"No need to shout, son!\" he said.",
                            &mut play_canvas,
                            50,
                            50);
                    }
                    _ => {}
                }
            });

            canvas.copy(&play_tex, None, None).unwrap();
            canvas.present();

            dirty = false;
        }

        let mut kill_flag = false;

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. }
                => {
                    kill_flag = true;
                },
                Event::KeyDown {keycode, scancode, keymod, repeat: false, ..}
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
                            match mode {
                                0 => { message_index.modify(increase) }
                                1 => { placard.modify(increase) }
                                _ => {}
                            }
                            dirty = true;
                        }

                        Scancode::F => {
                            // FIXME: name isn't reliable here, need a better way to identify fonts
                            if text_font.upgrade().unwrap().borrow().name() == amber_text.borrow().name() {
                                text_font = Rc::downgrade(&topaz_text);
                            } else {
                                text_font = Rc::downgrade(&amber_text);
                            }
                            dirty = true;
                        }

                        Scancode::C => {
                            // Cycle text color
                            text_color.modify(keymod.intersects(sdl2::keyboard::Mod::LSHIFTMOD | sdl2::keyboard::Mod::RSHIFTMOD));
                            dirty = true;
                        }

                        Scancode::Num1 | Scancode::Num2 | Scancode::Num3 | Scancode::Num4
                        | Scancode::Num5 | Scancode::Num6 | Scancode::Num7 | Scancode::Num8
                        | Scancode::Num9 | Scancode::Num0 => {
                            // keycode is i32 containing ASCII '0'+code, so NUM_0 == '0'
                            mode = keycode.unwrap_or_else(|| Keycode::NUM_0).into_i32() - '0' as i32;
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

        // Game loop goes here

        if kill_flag {
            break 'running
        } else {
            fps_man.delay();
        }
    }

    Ok(())
}
