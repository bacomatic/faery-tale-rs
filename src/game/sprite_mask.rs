//! Sprite-depth masking: per-tile, per-sprite-column ground-line masking
//! ported from fmain.c lines 3134-3184 and fsubs.asm maskit().

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
