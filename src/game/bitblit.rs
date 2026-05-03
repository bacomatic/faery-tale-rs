// Amiga-style bitplane blitting utilities for BitMap.
//
// These mirror the Amiga blitter's BltBitMap functionality:
// - extract_region: copy a sub-rectangle out of a BitMap
// - set_plane: replace a single bitplane's data
// - blt_copy_region: copy a sub-rectangle between two BitMaps (plane-masked)
//
// All operations work on bitplane data (Vec<Vec<u8>>) and are SDL2-independent.

use crate::game::bitmap::BitMap;

/// Extract a rectangular sub-region from `src` as a new `BitMap`.
///
/// The source region at (sx, sy) with dimensions (w, h) is copied into a
/// new bitmap whose stride is word-aligned per Amiga conventions.
/// Handles non-byte-aligned x coordinates (bit-level shifting).
pub fn extract_region(src: &BitMap, sx: usize, sy: usize, w: usize, h: usize) -> BitMap {
    assert!(
        sx + w <= src.width,
        "extract_region: x+w exceeds source width"
    );
    assert!(
        sy + h <= src.height,
        "extract_region: y+h exceeds source height"
    );
    assert!(
        w > 0 && h > 0,
        "extract_region: width and height must be > 0"
    );

    // Destination stride: word-aligned bytes per row
    let dst_stride = ((w + 15) >> 3) & !1_usize;
    let mut planes: Vec<Vec<u8>> = Vec::with_capacity(src.depth);

    let bit_offset = sx & 7; // how many bits into the first source byte

    for plane in &src.planes {
        let mut dst_plane = vec![0u8; dst_stride * h];

        for row in 0..h {
            let src_row_start = (sy + row) * src.stride;
            let dst_row_start = row * dst_stride;
            let src_byte_start = sx >> 3;

            if bit_offset == 0 {
                // Byte-aligned: straight copy
                let bytes_needed = (w + 7) >> 3;
                for col in 0..bytes_needed {
                    let si = src_row_start + src_byte_start + col;
                    if si < plane.len() {
                        dst_plane[dst_row_start + col] = plane[si];
                    }
                }
            } else {
                // Non-byte-aligned: shift pairs of source bytes
                let bytes_needed = (w + 7) >> 3;
                let shift_left = bit_offset;
                let shift_right = 8 - bit_offset;

                for col in 0..bytes_needed {
                    let si = src_row_start + src_byte_start + col;
                    let hi = if si < plane.len() { plane[si] } else { 0 };
                    let lo = if si + 1 < plane.len() {
                        plane[si + 1]
                    } else {
                        0
                    };
                    dst_plane[dst_row_start + col] = (hi << shift_left) | (lo >> shift_right);
                }
            }

            // Mask off trailing bits beyond the requested width
            let tail_bits = w & 7;
            if tail_bits > 0 {
                let last_byte = dst_row_start + ((w - 1) >> 3);
                let mask = 0xFF_u8 << (8 - tail_bits);
                dst_plane[last_byte] &= mask;
            }
        }

        planes.push(dst_plane);
    }

    BitMap::from_planes(planes, w, h, src.depth, dst_stride)
}

/// Replace a single plane's data in `dst`.
///
/// `plane_data` must be exactly `dst.stride * dst.height` bytes.
/// `plane_index` is zero-based and must be < `dst.depth`.
pub fn set_plane(dst: &mut BitMap, plane_index: usize, plane_data: &[u8]) {
    assert!(
        plane_index < dst.depth,
        "set_plane: plane_index out of range"
    );
    let expected = dst.stride * dst.height;
    assert_eq!(
        plane_data.len(),
        expected,
        "set_plane: plane_data length ({}) != stride*height ({})",
        plane_data.len(),
        expected
    );
    dst.planes[plane_index] = plane_data.to_vec();
    dst.invalidate_cache();
}

/// Copy a rectangular region from `src` to `dst`, affecting only planes
/// whose corresponding bit is set in `plane_mask`.
///
/// Mirrors Amiga `BltBitMap(src, sx, sy, dst, dx, dy, w, h, 0xC0, plane_mask, NULL)`.
/// minterm 0xC0 = D := A (straight copy of source to destination).
pub fn blt_copy_region(
    src: &BitMap,
    sx: usize,
    sy: usize,
    dst: &mut BitMap,
    dx: usize,
    dy: usize,
    w: usize,
    h: usize,
    plane_mask: u8,
) {
    assert!(
        sx + w <= src.width,
        "blt_copy_region: source region exceeds width"
    );
    assert!(
        sy + h <= src.height,
        "blt_copy_region: source region exceeds height"
    );
    assert!(
        dx + w <= dst.width,
        "blt_copy_region: dest region exceeds width"
    );
    assert!(
        dy + h <= dst.height,
        "blt_copy_region: dest region exceeds height"
    );

    let num_planes = src.depth.min(dst.depth);

    for pp in 0..num_planes {
        if (plane_mask >> pp) & 1 == 0 {
            continue;
        }

        let src_plane = &src.planes[pp];
        let dst_plane = &mut dst.planes[pp];

        for row in 0..h {
            for col in 0..w {
                let src_x = sx + col;
                let src_y = sy + row;
                let dst_x = dx + col;
                let dst_y = dy + row;

                let src_byte = src_y * src.stride + (src_x >> 3);
                let src_bit = 7 - (src_x & 7);
                let bit_val = (src_plane[src_byte] >> src_bit) & 1;

                let dst_byte = dst_y * dst.stride + (dst_x >> 3);
                let dst_bit = 7 - (dst_x & 7);
                if bit_val == 1 {
                    dst_plane[dst_byte] |= 1 << dst_bit;
                } else {
                    dst_plane[dst_byte] &= !(1 << dst_bit);
                }
            }
        }
    }

    dst.invalidate_cache();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a simple 16×4 bitmap with 2 planes for testing.
    /// Plane 0: all 0xFF (all bits set)
    /// Plane 1: alternating 0xAA / 0x55 per row
    fn make_test_bitmap() -> BitMap {
        let w = 16;
        let h = 4;
        let depth = 2;
        let stride = 2; // 16 pixels = 2 bytes
        let mut planes = Vec::new();

        // Plane 0: all 1s
        planes.push(vec![0xFF; stride * h]);

        // Plane 1: alternating
        let mut p1 = Vec::new();
        for row in 0..h {
            for _col in 0..stride {
                p1.push(if row % 2 == 0 { 0xAA } else { 0x55 });
            }
        }
        planes.push(p1);

        BitMap::from_planes(planes, w, h, depth, stride)
    }

    #[test]
    fn test_extract_region_byte_aligned() {
        let src = make_test_bitmap();
        let sub = extract_region(&src, 0, 0, 8, 2);
        assert_eq!(sub.width, 8);
        assert_eq!(sub.height, 2);
        assert_eq!(sub.depth, 2);
        assert_eq!(sub.stride, 2); // word-aligned: ((8+15)>>3)&!1 = 2
                                   // Plane 0: should be 0xFF for the first byte, 0 for padding
        assert_eq!(sub.planes[0][0], 0xFF);
        assert_eq!(sub.planes[0][1], 0x00); // padding byte
                                            // Plane 1 row 0: 0xAA -> first 8 bits = 0xAA
        assert_eq!(sub.planes[1][0], 0xAA);
    }

    #[test]
    fn test_extract_region_non_byte_aligned() {
        // Create a 64-pixel wide bitmap (stride=8) to test x=567%64=55 style offsets
        let w = 64;
        let h = 4;
        let stride = 8;
        let depth = 1;
        // Fill plane 0 with a known pattern: each byte = its column index
        let mut plane0 = Vec::new();
        for _row in 0..h {
            for col in 0..stride {
                plane0.push((col * 17 + 1) as u8); // arbitrary non-trivial pattern
            }
        }
        let planes = vec![plane0];
        let src = BitMap::from_planes(planes, w, h, depth, stride);

        // Extract from x=7 (bit offset 7), width=8, should shift left by 7
        let sub = extract_region(&src, 7, 0, 8, 1);
        assert_eq!(sub.width, 8);
        assert_eq!(sub.height, 1);
        // Source byte 0 = 0x01 (col 0 * 17 + 1), byte 1 = 0x12 (col 1 * 17 + 1)
        // Shifted: (0x01 << 7) | (0x12 >> 1) = 0x80 | 0x09 = 0x89
        assert_eq!(sub.planes[0][0], 0x89);
    }

    #[test]
    fn test_extract_full_region() {
        let src = make_test_bitmap();
        let sub = extract_region(&src, 0, 0, 16, 4);
        assert_eq!(sub.width, src.width);
        assert_eq!(sub.height, src.height);
        assert_eq!(sub.planes[0], src.planes[0]);
        assert_eq!(sub.planes[1], src.planes[1]);
    }

    #[test]
    fn test_set_plane() {
        let mut bm = make_test_bitmap();
        let new_data = vec![0x42; bm.stride * bm.height];
        set_plane(&mut bm, 1, &new_data);
        assert_eq!(bm.planes[1], new_data);
    }

    #[test]
    #[should_panic(expected = "plane_index out of range")]
    fn test_set_plane_out_of_range() {
        let mut bm = make_test_bitmap();
        set_plane(&mut bm, 5, &[0; 8]);
    }

    #[test]
    fn test_blt_copy_region() {
        let src = make_test_bitmap(); // 16×4, 2 planes
        let mut dst = BitMap::build(16, 4, 2).unwrap(); // all zeros

        // Copy src (0,0)→dst (0,0), 8×2, plane_mask=0x01 (plane 0 only)
        blt_copy_region(&src, 0, 0, &mut dst, 0, 0, 8, 2, 0x01);

        // Plane 0 of dst should have the first 8 bits set in rows 0-1
        assert_eq!(dst.planes[0][0], 0xFF); // row 0, byte 0
        assert_eq!(dst.planes[0][1], 0x00); // row 0, byte 1 (not copied)
                                            // Plane 1 should remain all zeros (plane_mask didn't include it)
        assert_eq!(dst.planes[1][0], 0x00);
    }

    #[test]
    fn test_blt_copy_region_both_planes() {
        let src = make_test_bitmap();
        let mut dst = BitMap::build(16, 4, 2).unwrap();

        // Copy all planes
        blt_copy_region(&src, 0, 0, &mut dst, 0, 0, 16, 4, 0x03);

        assert_eq!(dst.planes[0], src.planes[0]);
        assert_eq!(dst.planes[1], src.planes[1]);
    }
}
