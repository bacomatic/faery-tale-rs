//! Sprite-depth masking: per-tile, per-sprite-column ground-line masking
//! ported from fmain.c lines 3134-3184 and fsubs.asm maskit().

use crate::game::map_renderer::{MapRenderer, MAP_DST_W, MAP_DST_H};
use crate::game::map_view::{SCROLL_TILES_W, SCROLL_TILES_H};
use crate::game::tile_atlas::{TILE_W, TILE_H, TOTAL_TILES};

/// Check whether a tile with masking type `k` should mask a sprite at the given position.
///
/// Returns true if the mask should be applied (sprite goes BEHIND the tile).
/// Returns false if the mask should be skipped (sprite stays in front).
///
/// Ported from fmain.c lines 3149-3179.
///
/// Arguments:
/// - `k`: mask type 0-7 from terra_mem[tile*4+1] & 0x0f
/// - `xm`: tile column index within sprite (0 = leftmost 16px column)
/// - `ystop`: ground - ((ym + ym_base) << 5), signed distance from ground to tile row
/// - `_ym`: tile row index — used only for case 6 caller logic, not checked here
/// - `is_bridge_sector`: true if hero_sector == 48
/// - `is_actor_1`: true if this is actor index 1 (raft)
pub fn should_mask_tile(
    k: u8,
    xm: u8,
    ystop: i32,
    _ym: u8,
    is_bridge_sector: bool,
    is_actor_1: bool,
) -> bool {
    match k {
        0 => false,
        1 => xm != 0,
        2 => ystop <= 35,
        3 => {
            if is_bridge_sector && !is_actor_1 {
                false
            } else {
                true
            }
        }
        4 => xm != 0 && ystop <= 35,
        5 => !(xm == 0 && ystop > 35),
        6 => true, // case 6 always masks; caller substitutes maptag when ym != 0
        7 => ystop <= 20,
        _ => false,
    }
}

/// Read one bit from a 64-byte shadow_mem tile mask.
/// Layout: 32 rows × 2 bytes/row (big-endian u16). MSB = leftmost pixel (col 0).
/// Returns true if the bit is set (pixel should be masked/overwritten with tile).
pub fn shadow_bit_at(shadow_tile: &[u8], row: usize, col: usize) -> bool {
    if row >= 32 || col >= 16 || shadow_tile.len() < 64 {
        return false;
    }
    let word = u16::from_be_bytes([shadow_tile[row * 2], shadow_tile[row * 2 + 1]]);
    let bit = 15 - col;
    (word >> bit) & 1 != 0
}

/// Describes a sprite that was blitted to the framebuf and needs masking.
pub struct BlittedSprite {
    /// Screen X of sprite's top-left corner in framebuf coordinates.
    pub screen_x: i32,
    /// Screen Y of sprite's top-left corner in framebuf coordinates.
    pub screen_y: i32,
    /// Sprite width in pixels.
    pub width: usize,
    /// Sprite height in pixels.
    pub height: usize,
    /// Ground line Y in framebuf coordinates (sprite feet position).
    /// For characters: screen_y + 32. For objects: screen_y + obj_height.
    pub ground: i32,
    /// True if the actor is in FALL state (fmain.c state==22).
    pub is_falling: bool,
}

/// Apply sprite-depth masking for one sprite against the tile map.
///
/// For each 16×32 tile that overlaps the sprite's bounding box, checks
/// the tile's mask_type against the sprite's ground-line position. If
/// masking applies, reads the shadow_mem bitmask and re-stamps tile
/// pixels over the sprite area in the framebuf.
pub fn apply_sprite_mask(
    mr: &mut MapRenderer,
    sprite: &BlittedSprite,
    hero_sector: u16,
    _actor_idx: usize,
) {
    let fb_w = MAP_DST_W as i32;
    let fb_h = MAP_DST_H as i32;
    let ox = mr.last_ox;
    let oy = mr.last_oy;

    let is_bridge_sector = hero_sector == 48;
    let is_actor_1 = _actor_idx == 1;

    let sprite_left = sprite.screen_x;
    let sprite_right = sprite.screen_x + sprite.width as i32 - 1;
    let sprite_top = sprite.screen_y;
    let sprite_bottom = sprite.screen_y + sprite.height as i32 - 1;

    // Convert framebuf pixel coords to tile grid coords.
    // Guard against negative values before converting to usize.
    let left_in_world = sprite_left + ox;
    let right_in_world = sprite_right + ox;
    let top_in_world = sprite_top + oy;
    let bottom_in_world = sprite_bottom + oy;

    if right_in_world < 0 || bottom_in_world < 0 { return; }

    let tx_start = if left_in_world < 0 { 0 } else { left_in_world as usize / TILE_W };
    let tx_end = (right_in_world.max(0) as usize) / TILE_W;
    let ty_start = if top_in_world < 0 { 0 } else { top_in_world as usize / TILE_H };
    let ty_end = (bottom_in_world.max(0) as usize) / TILE_H;

    let ground = sprite.ground;
    let ym_base = if top_in_world < 0 { 0u8 } else { (top_in_world >> 5) as u8 };

    for tx in tx_start..=tx_end {
        if tx >= SCROLL_TILES_W { continue; }
        let xm = (tx as i32 - (left_in_world.max(0) as i32 / TILE_W as i32)).max(0) as u8;

        for ty in ty_start..=ty_end {
            if ty >= SCROLL_TILES_H { continue; }

            let tile_idx = mr.last_minimap[ty * SCROLL_TILES_W + tx] as usize;
            if tile_idx >= TOTAL_TILES { continue; }

            let k = mr.atlas.mask_type[tile_idx];
            if k == 0 { continue; }

            let ym = ty as u8 - ym_base.min(ty as u8);
            let ystop = ground - ((ym as i32 + ym_base as i32) << 5);

            // FALL state handling
            let k = if sprite.is_falling {
                if tile_idx <= 220 { continue; } else { 3u8 }
            } else {
                k
            };

            // Case 6: substitute tile 64's maptag for rows above ground
            let maptag = if k == 6 && ym != 0 {
                let tile64 = 64usize.min(TOTAL_TILES - 1);
                mr.atlas.maptag[tile64]
            } else {
                mr.atlas.maptag[tile_idx]
            };

            if !should_mask_tile(k, xm, ystop, ym, is_bridge_sector, is_actor_1) {
                continue;
            }

            // Apply shadow_mem bitmask: re-stamp tile pixels where mask bit is set.
            let shadow_offset = maptag as usize * 64;
            if shadow_offset + 63 >= mr.shadow_mem.len() { continue; }
            let shadow_tile = &mr.shadow_mem[shadow_offset..shadow_offset + 64];
            let tile_pixels = mr.atlas.tile_pixels(tile_idx);

            let tile_screen_x = tx as i32 * TILE_W as i32 - ox;
            let tile_screen_y = ty as i32 * TILE_H as i32 - oy;

            for row in 0..TILE_H {
                let py = tile_screen_y + row as i32;
                if py < 0 || py >= fb_h { continue; }
                if py < sprite_top || py > sprite_bottom { continue; }

                for col in 0..TILE_W {
                    let px = tile_screen_x + col as i32;
                    if px < 0 || px >= fb_w { continue; }
                    if px < sprite_left || px > sprite_right { continue; }

                    if shadow_bit_at(shadow_tile, row, col) {
                        let fb_idx = (py * fb_w + px) as usize;
                        let tile_px = tile_pixels[row * TILE_W + col];
                        mr.framebuf[fb_idx] = tile_px;
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_type_0_never_masks() {
        assert!(!should_mask_tile(0, 0, 0, 0, false, false));
        assert!(!should_mask_tile(0, 1, 30, 0, false, false));
    }

    #[test]
    fn test_mask_type_1_right_half_only() {
        // xm == 0 → skip (left column not masked)
        assert!(!should_mask_tile(1, 0, 0, 0, false, false));
        // xm > 0 → mask applies
        assert!(should_mask_tile(1, 1, 0, 0, false, false));
        assert!(should_mask_tile(1, 2, 0, 0, false, false));
    }

    #[test]
    fn test_mask_type_2_ground_level() {
        // ystop > 35 → skip
        assert!(!should_mask_tile(2, 0, 36, 0, false, false));
        assert!(!should_mask_tile(2, 0, 100, 0, false, false));
        // ystop <= 35 → mask
        assert!(should_mask_tile(2, 0, 35, 0, false, false));
        assert!(should_mask_tile(2, 0, 0, 0, false, false));
        assert!(should_mask_tile(2, 0, -10, 0, false, false));
    }

    #[test]
    fn test_mask_type_3_bridge() {
        // hero_sector == 48 && actor is not index 1 → skip
        assert!(!should_mask_tile(3, 0, 0, 0, true, false));
        // hero_sector != 48 → mask
        assert!(should_mask_tile(3, 0, 0, 0, false, false));
        // hero_sector == 48 but actor IS index 1 → mask
        assert!(should_mask_tile(3, 0, 0, 0, true, true));
    }

    #[test]
    fn test_mask_type_4_right_and_ground() {
        // xm == 0 OR ystop > 35 → skip
        assert!(!should_mask_tile(4, 0, 0, 0, false, false)); // xm==0
        assert!(!should_mask_tile(4, 1, 36, 0, false, false)); // ystop>35
        assert!(!should_mask_tile(4, 0, 50, 0, false, false)); // both
        // xm > 0 AND ystop <= 35 → mask
        assert!(should_mask_tile(4, 1, 35, 0, false, false));
        assert!(should_mask_tile(4, 2, 0, 0, false, false));
    }

    #[test]
    fn test_mask_type_5_right_or_ground() {
        // xm == 0 AND ystop > 35 → skip
        assert!(!should_mask_tile(5, 0, 36, 0, false, false));
        // xm > 0 → mask even if ystop > 35
        assert!(should_mask_tile(5, 1, 50, 0, false, false));
        // ystop <= 35 → mask even if xm == 0
        assert!(should_mask_tile(5, 0, 35, 0, false, false));
        // both conditions fail → mask
        assert!(should_mask_tile(5, 1, 20, 0, false, false));
    }

    #[test]
    fn test_mask_type_7_near_top() {
        // ystop > 20 → skip
        assert!(!should_mask_tile(7, 0, 21, 0, false, false));
        // ystop <= 20 → mask
        assert!(should_mask_tile(7, 0, 20, 0, false, false));
        assert!(should_mask_tile(7, 0, 0, 0, false, false));
    }

    #[test]
    fn test_shadow_bit_at() {
        let mut shadow = vec![0u8; 64];
        // Row 0: set bit 15 (pixel 0) and bit 0 (pixel 15)
        shadow[0] = 0x80; // bit 15 set → pixel 0
        shadow[1] = 0x01; // bit 0 set → pixel 15
        // Row 1: all bits set
        shadow[2] = 0xFF;
        shadow[3] = 0xFF;

        assert!(shadow_bit_at(&shadow, 0, 0)); // row 0, col 0 (bit 15)
        assert!(!shadow_bit_at(&shadow, 0, 1)); // row 0, col 1
        assert!(shadow_bit_at(&shadow, 0, 15)); // row 0, col 15 (bit 0)
        assert!(!shadow_bit_at(&shadow, 0, 7)); // row 0, col 7

        // Row 1: all set
        for col in 0..16 {
            assert!(shadow_bit_at(&shadow, 1, col));
        }

        // Row 2: all clear
        for col in 0..16 {
            assert!(!shadow_bit_at(&shadow, 2, col));
        }
    }
}
