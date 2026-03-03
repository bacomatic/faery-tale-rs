//! Collision detection — terrain-aware proxcheck (player-102).
//! Terrain lookup: hero pixel (x,y) → sector coords → map_mem → sector_mem → terra_mem.

use crate::game::world_data::WorldData;

/// Convert pixel position to terrain type using world data.
/// terra_mem: 4 bytes per tile; byte +1 upper nibble = terrain type.
pub fn px_to_terrain_type(world: &WorldData, x: i32, y: i32) -> u8 {
    let sx = ((x >> 4) as usize).min(127);
    let sy = ((y >> 5) as usize).min(31);
    let sec_num = world.sector_at(sx, sy) as usize;
    let sub_x = (x & 0xF) as usize;
    let sub_y = (y & 0x7) as usize;
    let tile_idx = world.tile_at(sec_num as u8, sub_x, sub_y) as usize;
    let terra_offset = tile_idx * 4 + 1;
    if terra_offset < world.terra_mem.len() {
        (world.terra_mem[terra_offset] >> 4) & 0xF
    } else {
        0 // default passable
    }
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
    use crate::game::adf::AdfDisk;

    #[test]
    fn test_proxcheck_no_world() {
        assert!(proxcheck(None, 100, 100));
    }

    #[test]
    fn test_terrain_type_in_bounds() {
        let adf = AdfDisk::from_bytes(vec![0u8; 2048 * 512]);
        let world = WorldData::load(&adf, 0).unwrap();
        // Empty world has all-zero terra_mem → terrain type 0 (passable)
        let t = px_to_terrain_type(&world, 0, 0);
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
}
