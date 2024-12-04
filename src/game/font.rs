extern crate sdl2;

use core::str;

use crate::game::byteops::*;
use crate::game::hunk::*;


use sdl2::rect::Rect;
use sdl2::render::Canvas;
use sdl2::render::RenderTarget;
use sdl2::render::Texture;

use std::path::Path;

// Amiga Font loader and renderer
// A loaded font can be rendered directly to a SDL Texture

// I'm using u8 for character data because the Rust char type is
// Unicode and it makes everything so much more complicated.

#[derive(Debug, Clone)]
pub struct DiskFont {
    pub name: String,    // name of this font (might be empty)
    pub y_size: usize,   // # pixels high
    pub x_size: usize,   // # pixels wide (for monospace fonts)
    pub style: u8,       // font style
    pub flags: u8,       // font flags
    pub baseline: usize, // # of pixel from top to use as text baseline
    pub lo_char: u8,     // first ASCII character in font
    pub hi_char: u8,     // last ASCII character in font


    pub char_data: Vec<u8>,  // Character raw bitmap data
    pub modulo: usize,       // bytes per row in font char data
    pub char_loc: Vec<(usize,usize)>,   // for each char (offset, len)
                                        // offset is bit count from start of row
                                        // len is number of bits wide
    pub char_space: Vec<isize>,         // pixel width for each character, this could be negative for RTL
    pub char_kern: Vec<isize>,          // kerning (pixel gap to next char) for each character, could be negative

    // cached pixel arrays used to generate textures, so we don't have to repeat
    // expensive operations.
    pixels_32: Vec<u8>,
}


// TODO: Break the texture functions out into a separate class to keep DiskFont self-contained


impl DiskFont {
    // Calculate the minimum size needed to store this font as a texture
    // This is likely not what the actual texture size will be
    pub fn get_texture_size(&self) -> Rect {
        Rect::new(0_i32, 0_i32, self.modulo as u32, self.y_size as u32)
    }

    // Draw all the font glyphs into the provided texture within the rect provided
    // returns the actual area updated
    // NOTE: this is hard coded to assume 32 bit ARGB format of some kind. What kind doesn't matter as
    //       each pixel is either white with full alpha or black with zero alpha.
    pub fn update_texture(&mut self, texture: &mut Texture, bounds: &Rect) -> Rect {
        let font_rect = Rect::new(bounds.x, bounds.y, self.modulo as u32, self.y_size as u32);
        let render_rect = font_rect.intersection(*bounds).unwrap();

        let tex_info = texture.query();
        // println!("texture info: {:?}", tex_info);
        assert_eq!(tex_info.format.byte_size_per_pixel(), 4); // Enforce 32 bits per pixel

        // build the pixel cache if needed
        if self.pixels_32.len() == 0 {
            for yy in 0 .. self.y_size {
                let offset = yy * self.modulo;
                for xx in 0 .. self.modulo {
                    let px = self.char_data[offset + xx];

                    // move to all four bytes
                    self.pixels_32.push(px);
                    self.pixels_32.push(px);
                    self.pixels_32.push(px);
                    self.pixels_32.push(px);
                }
            }
        }

        texture.update(render_rect, self.pixels_32.as_slice(), self.modulo * 4).unwrap();

        return render_rect;
    }

    // render a string to the given canvas
    // this does not handle newlines, it assumes the string will reside on a single line
    pub fn render_string<T: RenderTarget>(&self, s: &str, canvas: &mut Canvas<T>, texture: &mut Texture, x: i32, y: i32) {
        let cstr = s.as_bytes();

        let mut glyph_rect = Rect::new(x, y, 0, self.y_size as u32);
        for cc in cstr {
            if *cc >= self.lo_char && *cc <= self.hi_char {
                let cc_index = (cc - self.lo_char) as usize;
                let cc_loc = self.char_loc[cc_index];

                // Don't do anything for spaces, just skip ahead to the next coordinates
                if cc_loc.1 > 0 {
                    // grab glyph width and adjust glyph_rect
                    glyph_rect.set_width(cc_loc.1 as u32);
                    let src_rect = Rect::new(cc_loc.0 as i32, 0, cc_loc.1 as u32, self.y_size as u32);

                    // copy the glyph
                    canvas.copy(&texture, Some(src_rect), Some(glyph_rect)).unwrap();
                }

                // advance to the next glyph location
                glyph_rect.set_x(glyph_rect.x() + self.char_space[cc_index] as i32);
            }
        }
    }

    pub fn print(&self, s: &str) {
        // we need to ensure the string is ascii, and get a byte slice from it
        let cstr = s.as_bytes();

        for yy in 0 .. self.y_size {
            for pc in 0 .. cstr.len() {
                if cstr[pc] >= self.lo_char && cstr[pc] <= self.hi_char {
                    self.print_char_line((cstr[pc] - self.lo_char) as usize, yy, false);
                }
            }
            println!("");
        }
    }

    // print a single line of the given character
    // if the char is invalid (not ascii, not in the font range) then do nothing
    fn print_char_line(&self, char_index: usize, line: usize, mark: bool) {
        // get char location
        let char_loc = self.char_loc[char_index];
        let offset = (self.modulo * line) + char_loc.0;
        let total_width = self.char_space[char_index].abs() as usize; // FIXME: handle negative offsets
        let is_baseline = line == self.baseline;

        if mark {
            print!("|");
        }

        for xx in 0 .. char_loc.1 {
            let cc = self.char_data[offset + xx];
            if cc > 0 {
                print!("#");
            } else {
                print!("{}", if is_baseline && mark { "-" } else { " " });
            }
        }

        // fill out to the total width
        if mark {
            if is_baseline {
                print!("{:->1$}", "|", total_width - char_loc.1 + 1);
            } else {
                print!("{: >1$}", "|", total_width - char_loc.1 + 1);
            }
        } else {
            // still need to fill out to the total width
            print!("{: >1$}", "", total_width - char_loc.1);
        }
    }

    // Print every character glyph in the font to the terminal
    pub fn dump_font(&self) {
        for cc in self.lo_char ..= self.hi_char {
            self.print_char(cc, true);
        }
    }

    // print a single character, with or without bounding markers
    fn print_char(&self, c: u8, mark: bool) {
        if c >= self.lo_char && c <= self.hi_char {
            let char_index = (c - self.lo_char) as usize;
            let total_width = self.char_space[char_index].abs() as usize;

            // make sure it's a printable char first
            let char_str = if c.is_ascii_graphic() {
                str::from_utf8(&[c]).unwrap().to_string()
            } else {
                char::REPLACEMENT_CHARACTER.to_string()
            };

            if mark {
                println!("{:-^1$} : {2}", char_str, total_width + 2, total_width);
            }

            for index in 0 .. self.y_size as usize {
                self.print_char_line(char_index, index, mark);
                println!("");
            }

            if mark {
                println!("{:-^1$}", char_str, total_width + 2);
            }
        }
    }
}

pub fn load_font(fontfile: &Path) -> Result<DiskFont, HunkError> {
    let mut disk_font: DiskFont = DiskFont {
        name: "".to_string(),
        y_size: 0,
        x_size: 0,
        style: 0,
        flags: 0,
        baseline: 0,
        lo_char: 0,
        hi_char: 0,
        modulo: 0,
        char_data: Vec::new(),
        char_loc: Vec::new(),
        char_space: Vec::new(),
        char_kern: Vec::new(),
        pixels_32: Vec::new(),
    };

    let hunk = load_hunkfile(fontfile).unwrap();
    assert!(hunk.header.table_size > 0);

    // There should be one hunk loaded
    let ref hunk_data= hunk.hunks[0].data;
    let mut offset: usize = 0;

    // skip garbage at the beginning of the font data
    _ = read_u32(hunk_data, &mut offset); // MOVEQ #-1, D0; RTS <- instructions to return immediately
    // Link node
    _ = read_u32(hunk_data, &mut offset); // ln_Succ
    _ = read_u32(hunk_data, &mut offset); // ln_Prev
    let ln_type = read_u8(hunk_data, &mut offset); // ln_Type
    assert_eq!(ln_type, 12); // NT_FONT = 12

    _ = read_u8(hunk_data, &mut offset); // ln_Pri
    _ = read_u32(hunk_data, &mut offset); // ln_Name -> offset to font name in memory (don't care)

    // Start of actual DiskFont data
    let file_id = read_u16(hunk_data, &mut offset);
    assert_eq!(file_id, 0x0F80);
    _ = read_u16(hunk_data, &mut offset); // dfh_Revision, don't care
    _ = read_u32(hunk_data, &mut offset); // dfh_Segment, we don't really care because hunks don't need to be relocated (for now)
    disk_font.name = read_string(hunk_data, &mut offset);
    offset += 32 - disk_font.name.len();   // dfh_Name[MAXFONTNAME] -> MAXFONTNAME = 32

    // struct TextFont dfh_TF
        // another Node...
    _ = read_u32(hunk_data, &mut offset); // ln_Succ
    _ = read_u32(hunk_data, &mut offset); // ln_Prev
    let ln_type = read_u8(hunk_data, &mut offset); // ln_Type
    assert_eq!(ln_type, 12); // NT_FONT = 12, double check

    _ = read_u8(hunk_data, &mut offset); // ln_Pri
    _ = read_u32(hunk_data, &mut offset); // ln_Name
    _ = read_u32(hunk_data, &mut offset); // mn_ReplyPort
    _ = read_u16(hunk_data, &mut offset); // reserved for 1.4

    // Finally, actual font information
    disk_font.y_size = read_i16(hunk_data, &mut offset) as usize;
    disk_font.style = read_u8(hunk_data, &mut offset);
    disk_font.flags = read_u8(hunk_data, &mut offset);
    disk_font.x_size = read_i16(hunk_data, &mut offset) as usize;
    disk_font.baseline = read_i16(hunk_data, &mut offset) as usize;
    _ = read_i16(hunk_data, &mut offset); // tf_BoldSmear (don't care ?)
    _ = read_i16(hunk_data, &mut offset); // tf_Accessors (don't care)
    disk_font.lo_char = read_u8(hunk_data, &mut offset);
    disk_font.hi_char = read_u8(hunk_data, &mut offset);

    let font_data_offset = read_i32(hunk_data, &mut offset) as usize;
    disk_font.modulo = read_i16(hunk_data, &mut offset) as usize;

    let font_loc_offset = read_i32(hunk_data, &mut offset) as usize;
    let font_space_offset = read_i32(hunk_data, &mut offset) as usize;
    let font_kern_offset = read_i32(hunk_data, &mut offset) as usize;

    // println!("font data: {font_data_offset}, loc: {font_loc_offset}, space: {font_space_offset}, kern: {font_kern_offset}");
    // println!("end of DiskFont data. Offset = {offset}");

    // char data length is modulo (bytes per row) * y size
    let char_count = disk_font.hi_char as usize - disk_font.lo_char as usize;

    // copy the character data to disk_font
    // disk_font.char_data.extend_from_slice(&hunk_data[font_data_offset .. font_data_offset + data_len]);

    // this could be done programmatically, but whatever. I'm sure a million codebros will tell me I'm doing it wrong anyways.
    let foo: [&[u8; 4]; 16] = [
        &[0x00_u8, 0x00_u8, 0x00_u8, 0x00_u8],// b"    ",
        &[0x00_u8, 0x00_u8, 0x00_u8, 0xFF_u8],// b"   *",
        &[0x00_u8, 0x00_u8, 0xFF_u8, 0x00_u8],// b"  * ",
        &[0x00_u8, 0x00_u8, 0xFF_u8, 0xFF_u8],// b"  **",
        &[0x00_u8, 0xFF_u8, 0x00_u8, 0x00_u8],// b" *  ",
        &[0x00_u8, 0xFF_u8, 0x00_u8, 0xFF_u8],// b" * *",
        &[0x00_u8, 0xFF_u8, 0xFF_u8, 0x00_u8],// b" ** ",
        &[0x00_u8, 0xFF_u8, 0xFF_u8, 0xFF_u8],// b" ***",
        &[0xFF_u8, 0x00_u8, 0x00_u8, 0x00_u8],// b"*   ",
        &[0xFF_u8, 0x00_u8, 0x00_u8, 0xFF_u8],// b"*  *",
        &[0xFF_u8, 0x00_u8, 0xFF_u8, 0x00_u8],// b"* * ",
        &[0xFF_u8, 0x00_u8, 0xFF_u8, 0xFF_u8],// b"* **",
        &[0xFF_u8, 0xFF_u8, 0x00_u8, 0x00_u8],// b"**  ",
        &[0xFF_u8, 0xFF_u8, 0x00_u8, 0xFF_u8],// b"** *",
        &[0xFF_u8, 0xFF_u8, 0xFF_u8, 0x00_u8],// b"*** ",
        &[0xFF_u8, 0xFF_u8, 0xFF_u8, 0xFF_u8] // b"****"
    ];

    // Convert the bitmap into an 8 bit alpha map
    // This will be used later as a mask when rendering to a texture
    for yy in 0 .. disk_font.y_size {
        // modulo is already bytes per row, so just use it
        let row_offset = font_data_offset + (yy * disk_font.modulo);
        for xx in 0 .. disk_font.modulo as usize {
            let cc = hunk_data[row_offset + xx] as usize;

            // extend 4 bits at a time, balance between stupid large LUT and processing individual bits
            disk_font.char_data.extend_from_slice(foo[cc >> 4]);
            disk_font.char_data.extend_from_slice(foo[cc & 0xF]);
        }
    }

    // adjust modulo so it reflects the row size in char_data, which is now a byte array
    disk_font.modulo *= 8;

    for index in 0 ..= char_count {
        // Load char locations and lengths
        offset = font_loc_offset + (index * 4);
        let char_off = read_u16(hunk_data, &mut offset) as usize;
        let char_len = read_u16(hunk_data, &mut offset) as usize;
        disk_font.char_loc.push((char_off, char_len));

        // Load font spacing
        offset = font_space_offset + (index * 2);
        let char_space = read_i16(hunk_data, &mut offset) as isize;
        disk_font.char_space.push(char_space);

        // Load font kerning
        offset = font_kern_offset + (index * 2);
        let char_kern = read_i16(hunk_data, &mut offset) as isize;
        disk_font.char_kern.push(char_kern);

        // println!("char {} : loc ({}, {}), space {}, kern {}", disk_font.lo_char as usize + index, char_off, char_len, char_space, char_kern);
    }

    Ok(disk_font)
}
