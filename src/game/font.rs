
use crate::game::byteops::*;
use crate::game::hunk::*;

use core::str;
use std::char;
use std::collections::HashMap;
use std::error::Error;
use std::fs;
use std::path::Path;

use sdl2::rect::Rect;

use serde::Deserialize;

// Amiga Font loader

// Font styles

pub const FS_NORMAL: u8 = 0x00;

pub const FSF_UNDERLINED: u8 = 0x01;
pub const FSF_BOLD: u8 = 0x02;
pub const FSF_ITALIC: u8 = 0x04;
pub const FSF_EXTENDED: u8 = 0x08;

// Font flags
pub const FPF_ROMFONT: u8 = 0x01;
pub const FPF_DISKFONT: u8 = 0x02;
pub const FPF_REVPATH: u8 = 0x04; // font for RTL languages
pub const FPF_TALLDOT: u8 = 0x08; // designed for 640x200 (hires non-interlaced) mode
pub const FPF_WIDEDOT: u8 = 0x10; // designed for 320x400 (lores interlaced) mode
pub const FPF_PROPORTIONAL: u8 = 0x20;
pub const FPF_DESIGNED: u8 = 0x40;

// asset type used by GameLibrary
// This only considers font size, not styles, because I don't need to do otherwise
#[derive(Debug, Deserialize)]
pub struct FontAsset {
    pub file: String,

    #[serde(skip)]
    loaded: bool,

    #[serde(skip)]
    pub fonts: HashMap<usize, DiskFont>
}

impl Default for FontAsset {
    fn default() -> Self {
        FontAsset {
            file: String::new(),
            loaded: false,
            fonts: HashMap::new()
        }
    }
}

impl FontAsset {
    pub fn get_font(&self, size: usize) -> Option<&DiskFont> {
        if self.loaded == false {
            println!("FontAsset is not yet loaded: {}", self.file);
            return None;
        }
        self.fonts.get(&size)
    }

    pub fn get_sizes(&self) -> Vec<usize> {
        let mut sizes: Vec<usize> = Vec::new();
        for (size, _) in &self.fonts {
            sizes.push(*size);
        }
        sizes
    }

    pub fn load(&mut self) -> Result<(), Box<dyn Error>> {
        // parse the .font file and load all sizes
        let fontfile = load_font_file(Path::new(&self.file)).unwrap();
        let basepath = Path::new(&self.file).parent().unwrap();

        for fc in fontfile.contents {
            // load each font size
            // font path is relative to the .font file
            let fontpath = basepath.join(Path::new(&fc.path));
            // some entries might be missing, warn and skip those
            if fontpath.exists() == false {
                println!("Warning: font file {:?} does not exist!", fontpath);
                continue;
            }

            let diskfont = load_font(&fontpath, &fc.name).unwrap();
            self.fonts.insert(diskfont.y_size, diskfont);
        }
        self.loaded = true;

        Ok(())
    }
}

/*
 * A .font file is just a FontContentsHeader struct followed by either an array of FontContents
 * or TFontContents, depending on the value of header.fch_FileID. I'm only parsing FontContents here.
 */
#[derive(Debug, Clone)]
pub struct FontFile {
    file_id: u16,   // should be either 0x0F00 or 0x0F02
    contents: Vec<FontContents>
}

impl Default for FontFile {
    fn default() -> Self {
        FontFile {
            file_id: 0,
            contents: Vec::new()
        }
    }
}

#[derive(Debug, Clone)]
pub struct FontContents {
    pub name: String,
    pub path: String,
    pub y_size: usize,
    pub style: u8,
    pub flags: u8
}

pub fn load_font_file(path: &Path) -> Option<FontFile> {
    let file_data: Vec<u8> = fs::read(path).unwrap();
    let mut offset: usize = 0;

    let mut fontfile = FontFile::default();

    fontfile.file_id = read_u16(&file_data, &mut offset);
    if fontfile.file_id != 0x0F00 /*&& fontfile.file_id != 0x0F02*/ {
        // unsupported font file type
        return None;
    }

    let font_count = read_u16(&file_data, &mut offset);

    for _ in 0 .. font_count {
        // read_string won't skip the unused bytes after the string, so we need to track that ourselves
        let start_offset = offset;
        let font_path = read_string(&file_data, &mut offset);
        offset = start_offset + 256; // MAXFONTPATH is 256 bytes

        let y_size = read_u16(&file_data, &mut offset) as usize;
        let style = read_u8(&file_data, &mut offset);
        let flags = read_u8(&file_data, &mut offset);

        // Build the name from the path components
        let name: String = Path::new(&font_path).iter()
            .map(|p| p.to_str().unwrap_or(""))
            .collect::<Vec<&str>>().join("");

        fontfile.contents.push( FontContents {
            name: name,
            path: font_path,
            y_size,
            style,
            flags
        });
    }

    Some(fontfile)
}

#[derive(Debug, Clone)]
pub struct DiskFont {
    pub name: String,    // name of this font (might be empty)
    pub y_size: usize,   // # pixels high
    pub x_size: usize,   // # pixels wide (for monospace fonts)
    pub style: u8,       // font style
    pub flags: u8,       // font flags
    pub baseline: usize, // # of pixel from top to use as text baseline
    pub boldsmear: usize,// # of pixels to smear for bold effect
    pub lo_char: u8,     // first ASCII character in font
    pub hi_char: u8,     // last ASCII character in font


    pub char_data: Vec<u8>,  // Character raw bitmap data
    pub modulo: usize,       // bytes per row in font char data
    pub char_loc: Vec<(usize,usize)>,   // for each char (offset, len)
                                        // offset is bit count from start of row
                                        // len is number of bits wide

        // these two will be empty for monospace fonts
    pub char_space: Vec<isize>,         // pixel width for each character, this could be negative for RTL
    pub char_kern: Vec<isize>,          // kerning (pixel gap to next char) for each character, could be negative
}

impl DiskFont {
    pub fn new() -> DiskFont {
        DiskFont {
            name: "".to_string(),
            y_size: 0,
            x_size: 0,
            style: 0,
            flags: 0,
            baseline: 0,
            boldsmear: 0,
            lo_char: 0,
            hi_char: 0,
            modulo: 0,
            char_data: Vec::new(),
            char_loc: Vec::new(),
            char_space: Vec::new(),
            char_kern: Vec::new()
        }
    }

    // Style accessors
    pub fn is_underlined(&self) -> bool {
        (self.style & FSF_UNDERLINED) != 0
    }

    pub fn is_bold(&self) -> bool {
        (self.style & FSF_BOLD) != 0
    }

    pub fn is_italic(&self) -> bool {
        (self.style & FSF_ITALIC) != 0
    }

    pub fn is_extended(&self) -> bool {
        (self.style & FSF_EXTENDED) != 0
    }

    // flag accessors
    pub fn is_revpath(&self) -> bool {
        (self.flags & FPF_REVPATH) != 0
    }

    pub fn is_talldot(&self) -> bool {
        (self.flags & FPF_TALLDOT) != 0
    }

    pub fn is_widedot(&self) -> bool {
        (self.flags & FPF_WIDEDOT) != 0
    }

    pub fn is_proportional(&self) -> bool {
        (self.flags & FPF_PROPORTIONAL) != 0
    }

    pub fn print_style(&self) {
        let mut styles: Vec<&str> = Vec::new();
        if self.is_underlined() {
            styles.push("underlined");
        }
        if self.is_bold() {
            styles.push("bold");
        }
        if self.is_italic() {
            styles.push("italic");
        }
        if self.is_extended() {
            styles.push("extended");
        }

        if styles.len() == 0 {
            println!("Font style: normal");
        } else {
            println!("Font style: {}", styles.join(", "));
        }
    }

    pub fn print_flags(&self) {
        let mut flags: Vec<&str> = Vec::new();
        if self.is_revpath() {
            flags.push("revpath");
        }
        if self.is_talldot() {
            flags.push("talldot");
        }
        if self.is_widedot() {
            flags.push("widedot");
        }
        if self.is_proportional() {
            flags.push("proportional");
        }
        if self.flags & FPF_DESIGNED != 0 {
            flags.push("designed");
        }

        if flags.len() == 0 {
            println!("Font flags: none");
        } else {
            println!("Font flags: {}", flags.join(", "));
        }
    }
    // Calculate the minimum size needed to store this font as a texture
    // This is likely not what the actual texture size will be
    pub fn get_font_bounds(&self) -> Rect {
        Rect::new(0_i32, 0_i32, self.modulo as u32, self.y_size as u32)
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

pub fn load_font(fontfile: &Path, name: &str) -> Result<DiskFont, String> {
    let mut disk_font = DiskFont::new();

    let hunk = load_hunkfile(fontfile)
        .map_err(|e| format!("Failed to load font file {:?}: {}", fontfile, e))?;
    if hunk.header.table_size != 1 {
        return Err(format!("Font file {:?} has more than one hunk, unsupported", fontfile));
    }

    // There should be one hunk loaded
    let ref hunk_data= hunk.hunks[0].data;
    let mut offset: usize = 0;

    // skip garbage at the beginning of the font data
    _ = read_u32(hunk_data, &mut offset); // MOVEQ #-1, D0; RTS <- instructions to return immediately
    // Link node
    _ = read_u32(hunk_data, &mut offset); // ln_Succ
    _ = read_u32(hunk_data, &mut offset); // ln_Prev
    let mut ln_type = read_u8(hunk_data, &mut offset); // ln_Type
    if ln_type != 12 { // NT_FONT = 12
        return Err(format!("Font file {:?} has invalid Node type (DiskFont) {ln_type}", fontfile));
    }

    offset += 1; // ln_Pri
    offset += 4; // ln_Name -> offset to font name in memory (don't care)

    // Start of actual DiskFont data
    let file_id = read_u16(hunk_data, &mut offset);
    if file_id != 0x0F80 {
        return Err(format!("Font file {:?} has invalid DiskFont ID {file_id:X}", fontfile));
    }
    offset += 2; // dfh_Revision, don't care
    offset += 4; // dfh_Segment, we don't really care because hunks don't need to be relocated (for now)

    // this is usually empty, or some marker like "FED" for fonts made with FED
    // this will get set after loading when reading the .font file
    let mut name_offset = offset;
    disk_font.name = read_string(hunk_data, &mut name_offset);
    offset += 32;   // dfh_Name[MAXFONTNAME] -> MAXFONTNAME = 32 (always skip 32 bytes here)

    // if name is empty, use the provided name
    if disk_font.name.len() == 0 {
        disk_font.name = name.to_string();
    }

    // struct TextFont dfh_TF
        // another Node...
    offset += 4; // ln_Succ
    offset += 4; // ln_Prev
    ln_type = read_u8(hunk_data, &mut offset); // ln_Type
    if ln_type != 12 { // NT_FONT = 12, double check
        return Err(format!("Font file {:?} has invalid Node type (TextFont) {ln_type}", fontfile));
    }

    offset += 1; // ln_Pri
    offset += 4; // ln_Name
    offset += 4; // mn_ReplyPort
    offset += 2; // reserved for 1.4

    // Finally, actual font information
    disk_font.y_size = read_i16(hunk_data, &mut offset) as usize;
    disk_font.style = read_u8(hunk_data, &mut offset);
    disk_font.flags = read_u8(hunk_data, &mut offset);
    disk_font.x_size = read_i16(hunk_data, &mut offset) as usize;
    disk_font.baseline = read_i16(hunk_data, &mut offset) as usize;
    disk_font.boldsmear = read_i16(hunk_data, &mut offset) as usize;
    offset += 2; // tf_Accessors (N/A)
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

    // table to quickly expand packed bitmaps to 8 bit alpha values, a nibble at a time
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
    let char_data: &mut Vec<u8> = &mut disk_font.char_data;
    for yy in 0 .. disk_font.y_size {
        // modulo is already bytes per row, so just use it
        let row_offset = font_data_offset + (yy * disk_font.modulo);
        for xx in 0 .. disk_font.modulo as usize {
            let cc = hunk_data[row_offset + xx] as usize;

            // extend 4 bits at a time, balance between stupid large LUT and processing individual bits
            char_data.extend_from_slice(foo[cc >> 4]);
            char_data.extend_from_slice(foo[cc & 0xF]);
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

        // These are only for proportional fonts, for monospace they should be zero

        // Load font spacing
        if font_space_offset > 0 {
            offset = font_space_offset + (index * 2);
            let char_space = read_i16(hunk_data, &mut offset) as isize;
            disk_font.char_space.push(char_space);
        }

        // Load font kerning
        if font_kern_offset > 0 {
            offset = font_kern_offset + (index * 2);
            let char_kern = read_i16(hunk_data, &mut offset) as isize;
            disk_font.char_kern.push(char_kern);
        }

        // println!("char {} : loc ({}, {}), space {}, kern {}", disk_font.lo_char as usize + index, char_off, char_len, char_space, char_kern);
    }

    Ok(disk_font)
}
