//! One-off: dump rows-with-content for objects-sheet frames (cfile 3).
use std::path::Path;

mod game {
    #[allow(dead_code, unused_imports)]
    #[path = "/home/ddehaven/projects/faery-tale-rs/src/game/byteops.rs"]
    pub mod byteops;
    #[allow(dead_code, unused_imports)]
    #[path = "/home/ddehaven/projects/faery-tale-rs/src/game/adf.rs"]
    pub mod adf;
    #[allow(dead_code, unused_imports)]
    #[path = "/home/ddehaven/projects/faery-tale-rs/src/game/sprites.rs"]
    pub mod sprites;
}

use game::adf::AdfDisk;
use game::sprites::{SpriteSheet, OBJ_SPRITE_H, SPRITE_W};

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "game/E.faery".to_string());
    let adf = AdfDisk::open(Path::new(&path)).expect("open adf");
    let sheet = SpriteSheet::load_objects(&adf).expect("load objects");
    for f in 0..116 {
        let pix = match sheet.frame_pixels(f) { Some(p) => p, None => break };
        let mut min: i32 = -1;
        let mut max: i32 = -1;
        for row in 0..OBJ_SPRITE_H {
            let any = (0..SPRITE_W).any(|c| pix[row * SPRITE_W + c] != 31);
            if any {
                if min < 0 { min = row as i32; }
                max = row as i32;
            }
        }
        if min >= 0 {
            println!("frame {:3}: rows {}..={} (h={})", f, min, max, max - min + 1);
        } else {
            println!("frame {:3}: empty", f);
        }
    }
}
