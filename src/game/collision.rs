//! Collision detection — terrain-aware proxcheck (player-102).
//! Terrain lookup: hero pixel (x,y) → sector coords → map_mem → sector_mem → terra_mem.
//! Ported from px_to_im (fsubs.asm) and prox (fsubs.asm).

use crate::game::world_data::WorldData;

/// Convert pixel position to terrain type using world data.
///
/// Mirrors px_to_im from fsubs.asm:
/// 1. Compute tile bitmask selector d4 from sub-tile position (bits 3,3,4 of x,y,y).
/// 2. imx = x/16, imy = y/32.
/// 3. xs = imx/16 (sector col 0..127), ys = imy/8 (sector row 0..127).
/// 4. sec_num = map_mem[ys*128 + xs]  (flat 128×128 overworld map).
/// 5. tile_idx = sector_mem[sec_num*128 + (imy&7)*16 + (imx&15)].
/// 6. If terra_mem[tile_idx*4+2] & d4 == 0 → passable (return 0).
/// 7. Else return terra_mem[tile_idx*4+1] >> 4 (upper nibble = terrain type).
///
/// For indoor regions (region_num >= 8): the original uses xreg=0, yreg=128.
/// px_to_im subtracts (yreg*256 = 0x8000) from y before computing imy, so that
/// the indoor pixel coordinate range (0x8000–0x9FFF) maps into the indoor
/// map_mem rows (0..31).
pub fn px_to_terrain_type(world: &WorldData, x: i32, y: i32) -> u8 {
    if x < 0 || y < 0 {
        return 0; // out of world bounds → passable
    }

    // Indoor maps: subtract yreg offset (0x8000) so that y maps into the indoor
    // map_mem row range (0..31 sectors = 0..255 tile rows = 0..8191 pixels).
    let y = if world.region_num >= 8 { y - 0x8000 } else { y };
    if y < 0 {
        return 0; // below indoor map base → passable
    }

    // Tile bitmask selector: from bits 3,3,4 of x,y,y (tested before coordinate shifts).
    let mut d4: u8 = 0x80;
    if x & 0x08 != 0 { d4 >>= 4; }
    if y & 0x08 != 0 { d4 >>= 1; }
    if y & 0x10 != 0 { d4 >>= 2; }

    // Image tile coords: imx = x/16, imy = y/32.
    let imx = (x >> 4) as usize;
    let imy = (y >> 5) as usize;

    // Absolute sector coords in the flat 128×128 map.
    let xs = imx >> 4; // sector col 0..127
    let ys = imy >> 3; // sector row 0..127

    // Local sub-tile coords within the sector's tile grid.
    let local_x = imx & 15;
    let local_y = imy & 7;

    let sec_num = world.sector_at(xs, ys);
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

/// Check if position (x, y) collides with any actor in the `others` list.
/// Uses the original game's bounding box: |dx| < 11, |dy| < 9.
/// Mirrors fmain2.c proxcheck() actor-vs-actor loop (lines 395–427).
pub fn actor_collides(x: i32, y: i32, others: &[(i32, i32)]) -> bool {
    for &(ox, oy) in others {
        let dx = x - ox;
        let dy = y - oy;
        if dx > -11 && dx < 11 && dy > -9 && dy < 9 {
            return true;
        }
    }
    false
}

/// Octagonal distance approximation from fmain2.c:446-463.
/// Used by nearest_fig() for NPC/object proximity checks.
/// Returns: if x > 2*y → x; if y > 2*x → y; else (x+y)*5/7.
pub fn calc_dist(ax: i32, ay: i32, bx: i32, by: i32) -> i32 {
    let x = (ax - bx).abs();
    let y = (ay - by).abs();
    if x > y + y {
        x
    } else if y > x + x {
        y
    } else {
        (x + y) * 5 / 7
    }
}

/// X displacement per direction. Mirrors xdir[] from fsubs.asm.
const XDIR: [i32; 8] = [0, 2, 3, 2, 0, -2, -3, -2];
/// Y displacement per direction. Mirrors ydir[] from fsubs.asm.
const YDIR: [i32; 8] = [-3, -2, 0, 2, 3, 2, 0, -2];

/// Compute new X from current + direction * distance (port of newx from fsubs.asm).
pub fn newx(x: u16, dir: u8, dist: i32) -> u16 {
    // Assembly uses lsr.w (logical right shift) on the 16-bit product, not arithmetic
    // division. Per movement.md fsubs.asm:1293 — wrap_u16(prod) >> 1.
    let prod = XDIR[(dir & 7) as usize] * dist;
    let step = (prod as u16) >> 1;
    x.wrapping_add(step) & 0x7fff
}

/// Compute new Y from current + direction * distance (port of newy from fsubs.asm).
/// Bit 15 of `y` (indoor flag) is preserved in the result per fsubs.asm:1316.
pub fn newy(y: u16, dir: u8, dist: i32) -> u16 {
    // Assembly uses lsr.w (logical right shift) on the 16-bit product, not arithmetic
    // division. Per movement.md fsubs.asm:1316 — wrap_u16(prod) >> 1.
    // Bit 15 carries the indoor/outdoor flag and must be preserved.
    let flag = y & 0x8000;
    let prod = YDIR[(dir & 7) as usize] * dist;
    let step = (prod as u16) >> 1;
    (y.wrapping_add(step) & 0x7fff) | flag
}

/// Hero-specific terrain passability check.
/// Hero can walk into lava (8) and pit (9) tiles — environ effects applied later by
/// `update_environ`. Per fmain2.c:282: `if i == 0 and (t == 8 or t == 9): t = 0`.
/// Pass `has_crystal = stuff[30] != 0` to also bypass terrain-12 (crystal wall) for
/// the primary-direction probe. Per fmain.c:1611 — crystal bypass is primary-only.
pub fn hero_proxcheck(world: Option<&WorldData>, x: i32, y: i32, has_crystal: bool) -> bool {
    let world = match world {
        Some(w) => w,
        None => return true,
    };
    let mut rt = px_to_terrain_type(world, x + 4, y + 2);
    let mut lt = px_to_terrain_type(world, x - 4, y + 2);
    if rt == 8 || rt == 9 || (has_crystal && rt == 12) { rt = 0; }
    if lt == 8 || lt == 9 || (has_crystal && lt == 12) { lt = 0; }
    !is_hard_block_right(rt) && !is_hard_block_left(lt)
}

/// Full terra lookup chain for one probe point — used by the `/terrain` debug command.
pub struct TerrainProbe {
    pub x: i32,
    pub y: i32,
    pub d4: u8,
    pub imx: usize,
    pub imy: usize,
    pub xs: usize,
    pub ys: usize,
    pub map_offset: usize,
    pub sec_num: u8,
    pub local_x: usize,
    pub local_y: usize,
    pub sector_offset: usize,
    pub tile_idx: u8,
    pub terra_bytes: [u8; 4],
    pub tiles_and_d4: u8,
    pub terrain_type: u8,
}

/// Run the full px_to_terrain_type lookup, recording every intermediate value.
pub fn terrain_probe(world: &WorldData, x: i32, y: i32) -> TerrainProbe {
    if x < 0 || y < 0 {
        return TerrainProbe {
            x, y, d4: 0, imx: 0, imy: 0, xs: 0, ys: 0,
            map_offset: 0, sec_num: 0, local_x: 0, local_y: 0,
            sector_offset: 0, tile_idx: 0, terra_bytes: [0; 4],
            tiles_and_d4: 0, terrain_type: 0,
        };
    }

    // Indoor maps: subtract yreg offset (0x8000) — same adjustment as px_to_terrain_type.
    let y = if world.region_num >= 8 { y - 0x8000 } else { y };
    if y < 0 {
        return TerrainProbe {
            x, y, d4: 0, imx: 0, imy: 0, xs: 0, ys: 0,
            map_offset: 0, sec_num: 0, local_x: 0, local_y: 0,
            sector_offset: 0, tile_idx: 0, terra_bytes: [0; 4],
            tiles_and_d4: 0, terrain_type: 0,
        };
    }

    let mut d4: u8 = 0x80;
    if x & 0x08 != 0 { d4 >>= 4; }
    if y & 0x08 != 0 { d4 >>= 1; }
    if y & 0x10 != 0 { d4 >>= 2; }

    let imx = (x >> 4) as usize;
    let imy = (y >> 5) as usize;
    let xs = imx >> 4;
    let ys = imy >> 3;
    let local_x = imx & 15;
    let local_y = imy & 7;

    let map_offset = ys * 128 + xs;
    let sec_num = world.sector_at(xs, ys);
    let sector_offset = (sec_num as usize) * 128 + local_y * 16 + local_x;
    let tile_idx = world.tile_at(sec_num, local_x, local_y);

    let base = (tile_idx as usize) * 4;
    let terra_bytes = if base + 3 < world.terra_mem.len() {
        [world.terra_mem[base], world.terra_mem[base+1],
         world.terra_mem[base+2], world.terra_mem[base+3]]
    } else {
        [0; 4]
    };

    let tiles_and_d4 = terra_bytes[2] & d4;
    let terrain_type = if tiles_and_d4 == 0 { 0 } else { (terra_bytes[1] >> 4) & 0xF };

    TerrainProbe {
        x, y, d4, imx, imy, xs, ys, map_offset, sec_num,
        local_x, local_y, sector_offset, tile_idx, terra_bytes,
        tiles_and_d4, terrain_type,
    }
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

#[cfg(test)]
mod calc_dist_tests {
    use super::calc_dist;

    #[test]
    fn test_calc_dist_cardinal() {
        // Pure X distance: x > 2*y → return x
        assert_eq!(calc_dist(100, 0, 0, 0), 100);
        // Pure Y distance: y > 2*x → return y
        assert_eq!(calc_dist(0, 0, 0, 200), 200);
    }

    #[test]
    fn test_calc_dist_diagonal() {
        // Equal distances: (x+y)*5/7
        // x=70, y=70 → neither > 2*other → (70+70)*5/7 = 100
        assert_eq!(calc_dist(0, 0, 70, 70), 100);
    }

    #[test]
    fn test_calc_dist_asymmetric() {
        // x=10, y=30: y > 2*x → return y = 30
        assert_eq!(calc_dist(0, 0, 10, 30), 30);
        // x=30, y=10: x > 2*y → return x = 30
        assert_eq!(calc_dist(0, 0, 30, 10), 30);
        // x=20, y=15: neither > 2*other → (20+15)*5/7 = 25
        assert_eq!(calc_dist(0, 0, 20, 15), 25);
    }

    #[test]
    fn test_calc_dist_negative_coords() {
        // Uses absolute differences, so sign shouldn't matter
        assert_eq!(calc_dist(100, 200, 100, 200), 0);
        assert_eq!(calc_dist(50, 50, 100, 50), 50);
    }
}

#[cfg(test)]
mod newxy_tests {
    use super::{newx, newy};

    #[test]
    fn test_newx_cardinal() {
        // dir=2 (East), dist=2: prod=6, step=3
        assert_eq!(newx(100, 2, 2), 103);
        // dir=6 (West), dist=2: prod=-6, wrap_u16=0xFFFA, lsr=0x7FFD(32765), 100+32765=32865&0x7FFF=97
        assert_eq!(newx(100, 6, 2), 97);
    }

    #[test]
    fn test_newy_cardinal() {
        // dir=0 (North), dist=2: prod=-6, step=0x7FFD=32765, 100+32765=32865&0x7FFF=97
        assert_eq!(newy(100, 0, 2), 97);
        // dir=4 (South), dist=2: prod=6, step=3
        assert_eq!(newy(100, 4, 2), 103);
    }

    #[test]
    fn test_newx_diagonal() {
        // dir=1 (NE), dist=2: prod=4, step=2
        assert_eq!(newx(100, 1, 2), 102);
    }

    #[test]
    fn test_newx_odd_negative_logical_shift() {
        // F15.1: odd dist with negative direction uses lsr.w, not arithmetic /2.
        // dir=6 (West, XDIR=-3), dist=1: prod=-3, wrap_u16=0xFFFD, lsr=0x7FFE=32766
        // 100 + 32766 = 32866 & 0x7FFF = 98. (arithmetic /2 would give 99.)
        assert_eq!(newx(100, 6, 1), 98);
        // dir=0 (North, YDIR=-3 but via newx still XDIR=0): North XDIR[0]=0, no effect.
        // dir=6 (West), dist=3: prod=-9, wrap_u16=0xFFF7, lsr=0x7FFB=32763, 100+32763=32863&0x7FFF=95
        assert_eq!(newx(100, 6, 3), 95);
    }

    #[test]
    fn test_newy_odd_negative_logical_shift() {
        // F15.1: odd dist with negative direction — same lsr.w fix.
        // dir=0 (North, YDIR=-3), dist=1: prod=-3, step=0x7FFE=32766
        // 100 + 32766 = 32866 & 0x7FFF = 98. (arithmetic /2 would give 99.)
        assert_eq!(newy(100, 0, 1), 98);
    }

    #[test]
    fn test_newy_indoor_flag_preserved() {
        // Bit 15 (indoor flag 0x8000) must survive the calculation.
        // dir=4 (South), dist=2: prod=6, step=3 — indoor base 0x8100
        assert_eq!(newy(0x8100, 4, 2), 0x8103);
        // dir=0 (North), dist=2: step=32765, (0x8100+32765)&0x7FFF|0x8000 = 0x80FE
        assert_eq!(newy(0x8100, 0, 2), 0x80FD);
    }
}

#[cfg(test)]
mod actor_collision_tests {
    use super::*;

    #[test]
    fn test_actor_collides_overlapping() {
        let others = vec![(100i32, 100i32)];
        assert!(actor_collides(100, 100, &others));
    }

    #[test]
    fn test_actor_collides_within_bbox() {
        let others = vec![(100i32, 100i32)];
        assert!(actor_collides(110, 108, &others)); // dx=10, dy=8
    }

    #[test]
    fn test_actor_collides_at_boundary() {
        let others = vec![(100i32, 100i32)];
        assert!(!actor_collides(111, 100, &others)); // dx=11 — outside
        assert!(!actor_collides(100, 109, &others)); // dy=9 — outside
    }

    #[test]
    fn test_actor_collides_negative_direction() {
        let others = vec![(100i32, 100i32)];
        assert!(actor_collides(90, 92, &others));   // dx=-10, dy=-8
        assert!(!actor_collides(89, 100, &others));  // dx=-11 — outside
        assert!(!actor_collides(100, 91, &others));  // dy=-9 — outside
    }

    #[test]
    fn test_actor_collides_empty_list() {
        assert!(!actor_collides(100, 100, &[]));
    }

    #[test]
    fn test_actor_collides_multiple_actors() {
        let others = vec![(50i32, 50i32), (200, 200)];
        assert!(actor_collides(55, 55, &others));
        assert!(!actor_collides(100, 100, &others));
    }
}
