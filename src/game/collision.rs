//! Collision detection — terrain-aware proxcheck (player-102).
//! Terrain lookup: hero pixel (x,y) → sector coords → map_mem → sector_mem → terra_mem.
//! Ported from px_to_im (fsubs.asm) and prox (fsubs.asm).

use crate::game::world_data::WorldData;

/// Convert pixel position to terrain type using world data.
///
/// Mirrors px_to_im from fsubs.asm:
/// 1. Compute tile bitmask selector d4 from sub-tile position (bits 3,3,4 of x,y,y).
/// 2. imx = x/16, imy = y/32.
/// 3. secx = imx/16, secy = imy/8 → look up map_mem[secy*128+secx] = sec_num.
/// 4. tile_idx = sector_mem[sec_num*128 + (imy&7)*16 + (imx&15)].
/// 5. If terra_mem[tile_idx*4+2] & d4 == 0 → passable (return 0).
/// 6. Else return terra_mem[tile_idx*4+1] >> 4 (upper nibble = terrain type).
pub fn px_to_terrain_type(world: &WorldData, x: i32, y: i32) -> u8 {
    if x < 0 || y < 0 {
        return 0; // out of world bounds → passable
    }

    // Tile bitmask selector: from bits 3,3,4 of x,y,y (tested before coordinate shifts).
    let mut d4: u8 = 0x80;
    if x & 0x08 != 0 { d4 >>= 4; }
    if y & 0x08 != 0 { d4 >>= 1; }
    if y & 0x10 != 0 { d4 >>= 2; }

    // Image tile coords: imx = x/16, imy = y/32.
    let imx = (x >> 4) as usize;
    let imy = (y >> 5) as usize;

    // Sector coords: secx = imx/16 (0..127), secy = imy/8 (0..31).
    let secx = (imx >> 4).min(127);
    let secy = (imy >> 3).min(31);

    // Local sub-tile coords within the sector's tile grid.
    let local_x = imx & 15;
    let local_y = imy & 7;

    let sec_num = world.sector_at(secx, secy);
    let tile_idx = world.tile_at(sec_num, local_x, local_y) as usize;

    let base = tile_idx * 4;
    if base + 2 >= world.terra_mem.len() {
        return 0; // default passable
    }

    // Check per-sub-tile bitmask (tiles byte). If this bit is clear → passable here.
    if world.terra_mem[base + 2] & d4 == 0 {
        return 0;
    }

    // Return terrain type: upper nibble of terrain byte (terra_mem[base+1]).
    (world.terra_mem[base + 1] >> 4) & 0xF
}

/// Hard-blocking terrain for right foot (x+4, y+2): type==1 or >=10.
pub fn is_hard_block_right(terrain: u8) -> bool {
    terrain == 1 || terrain >= 10
}

/// Hard-blocking terrain for left foot (x-4, y+2): type==1 or >=8 (asymmetric — original).
pub fn is_hard_block_left(terrain: u8) -> bool {
    terrain == 1 || terrain >= 8
}

/// Check if movement to (x, y) is allowed.
/// Returns true = can move, false = blocked.
/// When world is None, movement is always allowed (pre-load passable fallback).
pub fn proxcheck(world: Option<&WorldData>, x: i32, y: i32) -> bool {
    let world = match world {
        Some(w) => w,
        None => return true,
    };
    let right_terrain = px_to_terrain_type(world, x + 4, y + 2);
    let left_terrain  = px_to_terrain_type(world, x - 4, y + 2);
    !is_hard_block_right(right_terrain) && !is_hard_block_left(left_terrain)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::world_data::WorldData;

    #[test]
    fn test_proxcheck_no_world() {
        assert!(proxcheck(None, 100, 100));
    }

    #[test]
    fn test_terrain_type_in_bounds() {
        let world = WorldData::empty();
        // Empty world → all-zero terra_mem → tiles byte 0 → d4 & 0 == 0 → passable (type 0)
        let t = px_to_terrain_type(&world, 0, 0);
        assert_eq!(t, 0);
    }

    #[test]
    fn test_terrain_type_negative_coords() {
        let world = WorldData::empty();
        // Negative coords (e.g. left foot probe at x=0) should return 0 (passable).
        let t = px_to_terrain_type(&world, -4, 2);
        assert_eq!(t, 0);
    }

    #[test]
    fn test_hard_block_types() {
        assert!(is_hard_block_right(1));
        assert!(is_hard_block_right(10));
        assert!(!is_hard_block_right(0));
        assert!(is_hard_block_left(8));
        assert!(!is_hard_block_left(7));
    }

    #[test]
    fn test_proxcheck_empty_world() {
        let world = WorldData::empty();
        // All-zero world: tiles bytes are 0, so every position is passable.
        assert!(proxcheck(Some(&world), 256, 256));
    }
}
