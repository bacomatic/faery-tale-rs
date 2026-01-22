
// IFF ILBM image loading

use serde::Deserialize;

use crate::game::byteops::*;
use crate::game::colors::Palette;
use crate::game::colors::RGB4;

use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct ImageAsset {
    #[serde(rename = "file")]
    pub path: String,

    #[serde(skip)]
    pub image: Option<IffImage>
}

/*
 * IFF images are chunked files
 * Each file starts with a 'FORM' chunk, which contains a type and other chunks
 * An ILBM image is a FORM of type 'ILBM', containing BMHD, CMAP, and BODY chunks
 */

const FOURCC_FORM: u32 = 0x464F524D; // 'FORM'
const FOURCC_ILBM: u32 = 0x494C424D; // 'ILBM'
const FOURCC_BMHD: u32 = 0x424D4844; // 'BMHD'
const FOURCC_CMAP: u32 = 0x434D4150; // 'CMAP'
const FOURCC_BODY: u32 = 0x424F4459; // 'BODY'

const MASK_NONE: u8 = 0;
const MASK_HAS_MASK: u8 = 1;
const MASK_HAS_TRANSPARENCY: u8 = 2;
const MASK_LASSO: u8 = 3;

const COMPRESSION_NONE: u8 = 0;
const COMPRESSION_BYTE_RUN1: u8 = 1;


#[derive(Debug)]
pub struct IffImage {
    pub width: usize,
    pub height: usize,
    pub bitplanes: usize,
    pub colormap: Option<Palette>,
    pub transparent_color: Option<usize>,
    pub pixels: Vec<u8>
}

impl IffImage {
    pub fn load_from_file(path: &Path) -> Result<IffImage, String> {
        // load the file data
        let file_data = std::fs::read(path).map_err(|e| format!("Failed to read IFF image file {:?}: {}", path, e))?;

        let result = IffImage::load_from_data(&file_data);
        if result.is_err() {
            return Err(format!("Failed to load IFF image from file {:?}: {}", path, result.err().unwrap()));
        }
        Ok(result.unwrap())
    }

    pub fn load_from_data(input_data: &Vec<u8>) -> Result<IffImage, String> {
        let mut offset: usize = 0;

        // read the FORM header
        let form_id = read_u32(input_data, &mut offset);
        if form_id != FOURCC_FORM {
            return Err("Missing FORM header".to_string());
        }
        let _form_size = read_u32(input_data, &mut offset); // don't really care about this
        let form_type = read_u32(input_data, &mut offset);
        if form_type != FOURCC_ILBM {
            return Err("FORM type is not ILBM".to_string());
        }

        let mut image = IffImage {
            width: 0,
            height: 0,
            bitplanes: 0,
            colormap: None,
            transparent_color: None,
            pixels: Vec::new()
        };

        let mut compressed = false;

        // now read chunks until we find BMHD, CMAP, and BODY, skipping any unknown chunks
        while offset < input_data.len() {
            let chunk_id = read_u32(&input_data, &mut offset);
            let chunk_size = read_u32(&input_data, &mut offset) as usize;

            match chunk_id {
                FOURCC_BMHD => {
                    // read bitmap header
                    let mut header_offset = offset;
                    image.width = read_u16(&input_data, &mut header_offset) as usize;
                    image.height = read_u16(&input_data, &mut header_offset) as usize;
                    header_offset += 4; // skip x,y position
                    image.bitplanes = input_data[header_offset] as usize;
                    header_offset += 1;

                    let masking = input_data[header_offset];
                    header_offset += 1;

                    let compression = input_data[header_offset];
                    match compression {
                        COMPRESSION_NONE => {},
                        COMPRESSION_BYTE_RUN1 => {
                            compressed = true;
                        },
                        _ => {
                            return Err(format!("Unsupported compression type {:?} in BMHD", compression));
                        }
                    }
                    header_offset += 1; // skip pad byte

                    // get transparent color if present
                    if masking == MASK_HAS_TRANSPARENCY {
                        let transparent_color = read_u16(&input_data, &mut header_offset) as usize;
                        image.transparent_color = Some(transparent_color);
                    } else {
                        image.transparent_color = None;
                    }
                    // skip the rest of the BMHD fields we don't care about
                    offset += chunk_size;
                }
                FOURCC_CMAP => {
                    // read colormap
                    let mut colormap = Palette { colors: Vec::new() };
                    for _ in 0 .. (chunk_size / 3) {
                        colormap.colors.push(RGB4::from((
                            input_data[offset],
                            input_data[offset + 1],
                            input_data[offset + 2]
                        )));
                        offset += 3;
                    }
                    image.colormap = Some(colormap);
                }
                FOURCC_BODY => {
                    // read body data
                    if !compressed {
                        // uncompressed, just read the data
                        let pixels = input_data.get(offset..offset+chunk_size);
                        if pixels.is_none() {
                            return Err("BODY chunk in ILBM is truncated".to_string());
                        }
                        image.pixels.clear();
                        image.pixels.extend(pixels.unwrap());
                        offset += chunk_size;
                        continue;
                    } else {
                        // compressed with ByteRun1
                        let mut body_offset: usize = 0;
                        let mut pixel_data: Vec<u8> = Vec::with_capacity(image.height * ((image.width + 15) / 16) * 2 * image.bitplanes);
                        while body_offset < chunk_size {
                            let n = input_data[offset + body_offset] as i8;
                            body_offset += 1;
                            if n >= 0 {
                                // copy next n+1 bytes literally
                                let copy_size = (n as usize) + 1;
                                let bytes = input_data.get(offset + body_offset .. offset + body_offset + copy_size);
                                if bytes.is_none() {
                                    return Err("BODY chunk in ILBM is truncated during ByteRun1 literal copy".to_string());
                                }
                                pixel_data.extend_from_slice(bytes.unwrap());
                                body_offset += copy_size;
                            } else if n >= -127 {
                                // next byte is repeated (-n)+1 times
                                let repeat_count = ((-n) as usize) + 1;
                                let byte_opt = input_data.get(offset + body_offset);
                                if byte_opt.is_none() {
                                    return Err("BODY chunk in ILBM is truncated during ByteRun1 repeat".to_string());
                                }
                                let byte = *byte_opt.unwrap();
                                for _ in 0 .. repeat_count {
                                    pixel_data.push(byte);
                                }
                                body_offset += 1;
                            } // n == -128 is a no-op
                        }
                        image.pixels = pixel_data;
                        offset += chunk_size;
                    }
                }
                _ => {
                    // skip unknown chunks
                    offset += chunk_size;
                }
            }
            // offset should be on an even byte boundary
            if offset % 2 != 0 {
                offset += 1;
            }
        }

        Ok(image)
    }
}
