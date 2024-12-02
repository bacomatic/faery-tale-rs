extern crate sdl2;

mod game;

use game::gfx::Palette4;
use game::font::DiskFont;

use sdl2::event::Event;
use sdl2::gfx::framerate::FPSManager;
use sdl2::keyboard::{Keycode, Scancode};
use sdl2::pixels::Color;
use sdl2::pixels::PixelFormatEnum;

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
    canvas.set_scale(2.0, 2.0).unwrap();


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

    let mut amber: DiskFont = game::font::load_font("game/fonts/Amber/9".to_string()).unwrap();
    // println!("amber font: {:?}", amber);
    // amber.dump_font();
    // amber.print("\"No need to shout, son!\" he said.");

    let font_bounds = amber.get_texture_size();
    let mut font_tex = tex_maker.create_texture_static(Some(PixelFormatEnum::BGRA8888), font_bounds.width(), font_bounds.height()).unwrap();

    amber.update_texture(&mut font_tex, &font_bounds);

    'running: loop {
        // FIXME: only redraw if dirty
        canvas.set_draw_color(Color::from(&sys_palette[color_index]));
        canvas.clear();

        font_tex.set_color_mod(200, 30, 0);
        font_tex.set_blend_mode(sdl2::render::BlendMode::Blend);

        // canvas.copy(&font_tex,
        //     Some(Rect::new(0, 0, font_bounds.width(), font_bounds.height())),
        //     Some(Rect::new(0, 0, font_bounds.width(), font_bounds.height())))
        //     .unwrap();

        amber.render_string("\"No need to shout, son!\" he said.", &mut canvas, &mut font_tex, 50, 50);

        let mut kill_flag = false;

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } |
                Event::KeyDown { keycode: Some(Keycode::Escape), .. }
                => {
                    kill_flag = true;
                },
                Event::KeyDown {scancode, keymod, repeat: false, ..}
                => {
                    println!("Key DOWN: scancode = {:?}, mod {}", scancode, keymod);
                    if scancode == Some(Scancode::Up) {
                        color_index = (color_index + 1) % 4;
                    }
                },
                Event::KeyUp {scancode, keymod, ..}
                => {
                    println!("Key UP: scancode = {:?}, mod {}", scancode, keymod);
                    if scancode == Some(Scancode::Down) {
                        if color_index == 0 {
                            color_index = 3;
                        } else {
                            color_index -= 1;
                        }
                    }
                },
                _ => {}
            }
        }

        // Game loop goes here

        if kill_flag {
            break 'running
        } else {
            canvas.present();
            fps_man.delay();
        }
    }

    Ok(())
}
