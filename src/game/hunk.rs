// Hunk file loader
// Given a filename, load a hunk file into memory
// and process it

use std::fs;

use crate::game::byteops::*;

// All on-disk data are big endian format
// use:
// i32.from_be(value)
// i16.from_be(value)

#[derive(Debug, Clone)]
pub struct HunkError;

const ALLOC_FLAG_MASK: u32 = 0x3FFFFFFF_u32; // use to mask off the mem flags

// Amiga HUNK file magic cookie
const MAGIC_COOKIE: u32 = 0x03F3;

// hunk IDs
const HUNK_UNIT: u32 = 0x03E7;      // ?? should not encounter
const HUNK_CODE: u32 = 0x03E9;      // hunk of executable code
const HUNK_DATA: u32 = 0x03EA;      // hunk of data, may have extra trailing data (?)
const HUNK_BSS: u32 = 0x03EB;       // one longword of the size of zeroed memory to allocate
const HUNK_RELOC32: u32 = 0x03EC;   // 32 bit relocation block using LONG offsets
const HUNK_RELOC32SHORT: u32 = 0x03FC; // 32 bit relocation block using WORD offsets
const HUNK_ABSRELOC16: u32 = 0x03FD; // absolute relocation, similar to RELOC32SHORT
const HUNK_SYMBOL: u32 = 0x03F0;    // debug symbols
const HUNK_DEBUG: u32 = 0x03F1;     // application defined debug data
const HUNK_END: u32 = 0x03F2;       // end of hunks
const HUNK_OVERLAY: u32 = 0x03F5;   // overlay section for dynamically loading code into memory (rarely used)
const HUNK_BREAK: u32 = 0x03F6;     // ?

// Only used for linking, not used in load files
const HUNK_RELOC16: u32 = 0x03ED;
const HUNK_RELOC8: u32 = 0x03EE;
// Not used in load files
const HUNK_DREL32: u32 = 0x03F7;
const HUNK_DREL16: u32 = 0x03F8;
const HUNK_DREL8: u32 = 0x03F9;
const HUNK_LIB: u32 = 0x03FA;
const HUNK_INDEX: u32 = 0x03FB;


// HUNK_HEADER data
#[derive(Debug, Clone)]
pub struct HunkHeader {
    pub table_size: u32,        // Number of hunks to load
    pub first_hunk: u32,        // Index of first hunk
    pub last_hunk: u32,         // Index of last hunk
    pub hunk_sizes: Vec<usize>    // Array of sizes for each hunk
}

#[derive(Debug, Clone)]
pub struct Hunk {
    pub hunk_id: u32,
    pub hunk_size: usize,
    pub data: Vec<u8>
}

#[derive(Debug, Clone)]
pub struct HunkData {
    pub header: HunkHeader,
    pub hunks: Vec<Hunk>
}

// FIXME: there is no bounds checking or error handling, using this with bad or unsupported hunks will cause panics

pub fn load_hunkfile(filepath: String) -> Result<HunkData, HunkError> {
    // Just read the whole thing into memory first
    let file_data: Vec<u8> = fs::read(filepath).unwrap(); // FIXME: proper error handling
    let mut offset: usize = 0;

    // check for magic cookie
    let cookie = read_u32(&file_data, &mut offset);
    assert_eq!(cookie, MAGIC_COOKIE);

    let mut hunk = HunkData {
        header: HunkHeader {
            table_size: 0,
            first_hunk: 0,
            last_hunk: 0,
            hunk_sizes: Vec::new()
        },
        hunks: Vec::new()
    };

    // HUNK_HEADER structure:
    // resident_libs: u32[] -> Must be one u32 with value 0 for load files, otherwise it's a bad hunk
    // table size: u32      -> Hunk table size (highest hunk number + 1) - This is NOT the actual hunk count, but almost always is
    // first_hunk: u32      -> first hunk slot to be used
    // last_hunk: u32       -> last hunk slot to be used
    // hunk_sizes: u32[last_hunk - first_hunk + 1] -> sizes of each hunk on disk plus AllocMem flags in two highest bits

    // If both bit 31 and 30 are set in mem flags, then another longword will follow the size, but I've yet to encounter
    // this so I'm not going to implement it

    let strings = read_u32(&file_data, &mut offset);
    assert_eq!(strings, 0);
    assert_eq!(offset, 8); // make sure we're borrowing correctly

    hunk.header.table_size = read_u32(&file_data, &mut offset);
    hunk.header.first_hunk = read_u32(&file_data, &mut offset);
    hunk.header.last_hunk = read_u32(&file_data, &mut offset);

    let hunk_count = (hunk.header.last_hunk - hunk.header.first_hunk + 1) as usize;
    for _index in [hunk_count] {
        let mut size = read_u32(&file_data, &mut offset) & ALLOC_FLAG_MASK; // don't care about the flags
        size *= 4; // Hunk size is number of LONGs
        hunk.header.hunk_sizes.push(size as usize);
    }

    let mut hunk_index: usize = hunk.header.first_hunk as usize;

    'hunkloop: loop {
        let hunk_id = read_u32(&file_data, &mut offset);
        println!("Hunk ID: {:X}", hunk_id);

        if hunk_id == HUNK_CODE || hunk_id == HUNK_DATA {
            let saved_size = hunk.header.hunk_sizes[hunk_index];
            let size = read_u32(&file_data, &mut offset) as usize * 4;
            assert_eq!(saved_size, size);

            // hunk size includes hunk ID and size on disk, so subtract that
            let mut data: Vec<u8> = Vec::new();
            data.extend_from_slice(&file_data[offset .. offset + size - 4]);
            assert_eq!(data.len(), size - 4);
            offset += size;

            hunk.hunks.push(Hunk {
                hunk_id: hunk_id,
                hunk_size: size,
                data: data
            });

            hunk_index += 1;
        } else if hunk_id == HUNK_RELOC32 {
            /*
             * RELOC block structure:
             * repeated:
             * LONG - N offsets, if zero then end of relo lists
             * LONG - hunk number for relocations
             * LONG[N] - offsets to process
             */
            'reloloop: loop {
                let count = read_u32(&file_data, &mut offset);
                println!("RELO count: {}", count);
                if count == 0 {
                    println!("No more relo blocks");
                    break 'reloloop;
                }
                let hunk_num = read_u32(&file_data, &mut offset) as usize;
                let ref hunk_data = hunk.hunks[hunk_num].data;

                println!("Hunk {}", hunk_num);

                for _index in 0 .. count as usize {
                    let mut rel_offset = read_u32(&file_data, &mut offset) as usize;
                    let _value = read_u32(hunk_data, &mut rel_offset);
                    // println!("   {index}: {} => 0x{value:X}", rel_offset-4);
                }
            }
        } else if hunk_id == HUNK_END {
            break 'hunkloop;
        }
    }

    Ok(hunk)
}
