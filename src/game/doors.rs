//! Door and portal system: doorfind() transition logic.
//! The door table is loaded from [[doors]] in faery.toml via GameLibrary.

/// A single door entry mapping source → destination.
#[derive(Debug, Clone, Copy, Default)]
pub struct DoorEntry {
    pub src_region: u8,
    pub src_x: u16,
    pub src_y: u16,
    pub dst_region: u8,
    pub dst_x: u16,
    pub dst_y: u16,
    pub door_type: u8,
}

// Door type constants matching original fmain.c defines.
// Horizontals all have lsb set (type & 1 == 1).
pub const HWOOD: u8 = 1;
pub const VWOOD: u8 = 2;
pub const HSTONE: u8 = 3;
pub const VSTONE: u8 = 4;
pub const HCITY: u8 = 5;
pub const VCITY: u8 = 6;
pub const CRYST: u8 = 7;
pub const SECRET: u8 = 8;
pub const BLACK: u8 = 9;
pub const MARBLE: u8 = 10;
pub const LOG: u8 = 11;
pub const HSTON2: u8 = 13;
pub const VSTON2: u8 = 14;
pub const STAIR: u8 = 15;
pub const DESERT: u8 = 17;
pub const CAVE: u8 = 18;
// VLOG = 18 (same as CAVE); distinguish by context only

/// Key type enum matching original fmain.c `enum ky`.
/// Values are the inventory slot index relative to KEYBASE (stuff[16+slot]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyReq {
    /// No key required — door opens freely.
    NoKey,
    /// Requires stuff[16 + slot] > 0 to open; slot 0=GOLD,1=GREEN,2=KBLUE,3=RED,4=GREY,5=WHITE.
    Key(u8),
    /// Talisman (stuff[30]) required — BLACK doors (doom tower, pass fort, witch's castle).
    Talisman,
    /// DESERT oasis doors — require 5 gold statues (stuff[25] >= 5) to pass.
    GoldStatues,
}

/// Return the key requirement for a given door type.
/// Derived from `open_list[17]` in fmain.c.
/// Slot values: 0=GOLD, 1=GREEN, 2=KBLUE, 3=RED, 4=GREY, 5=WHITE.
pub fn key_req(door_type: u8) -> KeyReq {
    match door_type {
        HWOOD | VWOOD | HCITY | VCITY | LOG | STAIR | CAVE => KeyReq::NoKey,
        HSTONE | VSTONE => KeyReq::Key(1), // GREEN
        CRYST => KeyReq::Key(2),           // KBLUE
        SECRET => KeyReq::Key(3),          // RED
        HSTON2 | VSTON2 => KeyReq::Key(4), // GREY
        MARBLE => KeyReq::Key(5),          // WHITE
        BLACK => KeyReq::Talisman,
        DESERT => KeyReq::GoldStatues,
        _ => KeyReq::NoKey,
    }
}

/// Tile replacement for an opened door, mirroring `open_list[17]` in fmain.c.
/// `new1` replaces the main cell; `new2` (if non-zero) replaces a secondary cell
/// in the direction encoded by `above`:
///   1 = above (imx, imy-1)
///   2 = right (imx+1, imy)          ← most common
///   3 = left  (imx-1, imy)
///   4 = multi: (imx,imy-1)=87, (imx+1,imy)=86, (imx+1,imy-1)=88
///   other (used in GOLDEN 3-wide): right=new2, right+1=above_val
#[derive(Debug, Clone, Copy)]
pub struct DoorTileReplacement {
    pub new1: u8,
    pub new2: u8,
    pub above: u8,
}

/// Return the tile replacement spec for a door type.
/// Derived from open_list[17] in fmain.c by matching door type constants to entries.
/// Returns None for types whose tile IDs are unknown or region-specific.
pub fn door_tile_replacement(door_type: u8) -> Option<DoorTileReplacement> {
    // open_list entries (door_id, map_id, new1, new2, above, keytype):
    //  {120, 360, 125, 126, 2, NOKEY}  HWOOD
    //  {122, 360, 127,   0, 0, NOKEY}  VWOOD
    //  {64,  360, 123, 124, 2, GREEN}  HSTONE
    //  {64,  280, 124, 125, 2, GREY}   HSTON2
    //  {77,  280, 126,   0, 0, GREY}   VSTON2
    //  {82,  480,  84,  85, 2, KBLUE}  CRYST
    //  {64,  480, 105, 106, 2, GREEN}  DESERT/OASIS
    //  {128, 240, 154, 155, 1, WHITE}  MARBLE
    //  {39,  680,  41,  42, 2, GOLD}   BLACK horizontal (HGATE)
    //  {25,  680,  27,  26, 3, GOLD}   BLACK vertical (VGATE, above=3=left)
    //  {114, 760, 116, 117, 1, RED}    SECRET
    //  {187, 800,  76,  77, 2, NOKEY}  LOG horizontal
    //  {73,  720,  75,   0, 0, NOKEY}  STAIR
    //  {165, 800,  85,  86, 4, GREEN}  HCITY/VCITY (multi-tile gate)
    //  {210, 840, 208, 209, 2, NOKEY}  CAVE
    match door_type {
        HWOOD => Some(DoorTileReplacement {
            new1: 125,
            new2: 126,
            above: 2,
        }),
        VWOOD => Some(DoorTileReplacement {
            new1: 127,
            new2: 0,
            above: 0,
        }),
        HSTONE => Some(DoorTileReplacement {
            new1: 123,
            new2: 124,
            above: 2,
        }),
        VSTONE => None, // tile ID unknown; no open_list entry observed
        HCITY | VCITY => Some(DoorTileReplacement {
            new1: 85,
            new2: 86,
            above: 4,
        }),
        CRYST => Some(DoorTileReplacement {
            new1: 84,
            new2: 85,
            above: 2,
        }),
        SECRET => Some(DoorTileReplacement {
            new1: 116,
            new2: 117,
            above: 1,
        }),
        BLACK => Some(DoorTileReplacement {
            new1: 41,
            new2: 42,
            above: 2,
        }), // horizontal variant
        MARBLE => Some(DoorTileReplacement {
            new1: 154,
            new2: 155,
            above: 1,
        }),
        LOG => Some(DoorTileReplacement {
            new1: 76,
            new2: 77,
            above: 2,
        }),
        HSTON2 => Some(DoorTileReplacement {
            new1: 124,
            new2: 125,
            above: 2,
        }),
        VSTON2 => Some(DoorTileReplacement {
            new1: 126,
            new2: 0,
            above: 0,
        }),
        STAIR => Some(DoorTileReplacement {
            new1: 75,
            new2: 0,
            above: 0,
        }),
        DESERT => Some(DoorTileReplacement {
            new1: 105,
            new2: 106,
            above: 2,
        }),
        CAVE => Some(DoorTileReplacement {
            new1: 208,
            new2: 209,
            above: 2,
        }),
        _ => None,
    }
}

/// Apply the open-door tile replacement to `world`'s sector_mem.
/// `probe_x, probe_y` are the pixel coordinates that detected terrain-15 (from the
/// bump path's right_t/left_t probes). Mirrors fmain.c doorfind() tile-write logic:
///   1. Walk left up to 2×16px while still terrain-15 (find leftmost tile cell).
///   2. Walk down 1×32px if terrain-15 below (for doors whose origin is above).
///   3. Write new1 to (imx, imy); write new2 to secondary cell per above direction.
pub fn apply_door_tile_replacement(
    world: &mut crate::game::world_data::WorldData,
    door_type: u8,
    probe_x: i32,
    probe_y: i32,
) {
    use crate::game::collision::px_to_terrain_type;
    let rep = match door_tile_replacement(door_type) {
        Some(r) => r,
        None => return,
    };

    // Align to tile origin (mirrors doorfind grid-align steps).
    let mut px = probe_x;
    let py = probe_y;
    if px >= 16 && px_to_terrain_type(world, px - 16, py) == 15 {
        px -= 16;
    }
    if px >= 16 && px_to_terrain_type(world, px - 16, py) == 15 {
        px -= 16;
    }
    // Note: original also checks y+32; uncommon in practice for outdoor doors.
    // (The y+32 step handles doors whose reference coord is at the upper tile.)

    let imx = (px >> 4) as usize;
    let imy = (py >> 5) as usize;

    world.set_tile_at_image(imx, imy, rep.new1);

    if rep.new2 != 0 {
        match rep.above {
            1 => {
                world.set_tile_at_image(imx, imy.saturating_sub(1), rep.new2);
            }
            2 => {
                world.set_tile_at_image(imx + 1, imy, rep.new2);
            }
            3 => {
                world.set_tile_at_image(imx.saturating_sub(1), imy, rep.new2);
            }
            4 => {
                // Multi-tile gate (HCITY/VCITY): hardcoded 4-cell pattern from fmain.c.
                world.set_tile_at_image(imx, imy.saturating_sub(1), 87);
                world.set_tile_at_image(imx + 1, imy, 86);
                world.set_tile_at_image(imx + 1, imy.saturating_sub(1), 88);
            }
            _ => {
                // 3-wide door (e.g. GOLDEN gate): right=new2, right+1=above val.
                world.set_tile_at_image(imx + 1, imy, rep.new2);
                world.set_tile_at_image(imx + 2, imy, rep.above);
            }
        }
    }
}

/// Bump-detection search window (pixels). Mirrors the original's grid-aligned exact match
/// while tolerating sub-pixel approach offsets.
pub const BUMP_PROX_X: i32 = 32; // 2 × 16px X grid cell
pub const BUMP_PROX_Y: i32 = 64; // 2 × 32px Y grid cell

/// Find the nearest outdoor door within bump proximity, returning its (index, entry).
/// Used by both the terrain-15 bump path and the USE→KEYS TryKey path.
pub fn doorfind_nearest_by_bump_radius(
    table: &[DoorEntry],
    region_num: u8,
    hero_x: u16,
    hero_y: u16,
) -> Option<(usize, DoorEntry)> {
    table
        .iter()
        .enumerate()
        .filter(|(_, d)| {
            d.src_region == region_num
                && (d.src_x as i32 - hero_x as i32).abs() < BUMP_PROX_X
                && (d.src_y as i32 - hero_y as i32).abs() < BUMP_PROX_Y
        })
        .min_by_key(|(_, d)| {
            let dx = d.src_x as i32 - hero_x as i32;
            let dy = d.src_y as i32 - hero_y as i32;
            dx * dx + dy * dy
        })
        .map(|(i, d)| (i, *d))
}

/// Find a door at the outdoor position using the original's exact grid-aligned matching.
/// Mirrors fmain.c binary-search Phase-2 match conditions:
///   xtest = hero_x & 0xFFF0, ytest = hero_y & 0xFFE0
///   Horizontal (type & 1): d->xc1 <= xtest <= d->xc1+16 AND d->yc1 == ytest
///     (+16 because the original skips only when d->xc1+16 < xtest, covering 2 tile cells)
///   Vertical            : d->xc1 == xtest               AND d->yc1 == ytest
/// Sub-tile position guard (caller responsibility):
///   Horizontal: skip if hero_y & 0x10 != 0  (lower half → not yet through)
///   Vertical  : skip if hero_x & 15 > 6     (right portion → not yet through)
pub fn doorfind(
    table: &[DoorEntry],
    region_num: u8,
    hero_x: u16,
    hero_y: u16,
) -> Option<DoorEntry> {
    let xtest = hero_x & 0xFFF0;
    let ytest = hero_y & 0xFFE0;
    for door in table {
        if door.src_region != region_num || door.src_y != ytest {
            continue;
        }
        let x_match = if door.door_type & 1 != 0 {
            // horizontal door: xtest in [src_x, src_x+16] — covers both tile cells of a 2-wide door.
            // Original: `d->xc1 + 16 < xtest` → skip right, so match iff xtest <= xc1+16.
            xtest >= door.src_x && xtest <= door.src_x.saturating_add(16)
        } else {
            // vertical door: exact x grid-alignment
            xtest == door.src_x
        };
        if x_match {
            return Some(*door);
        }
    }
    None
}

/// Find the door at an indoor position (region >= 8) by matching dst coords.
/// Mirrors fmain.c indoor branch:
///   xtest = hero_x & 0xFFF0, ytest = hero_y & 0xFFE0
///   d->yc2 == ytest && (d->xc2 == xtest || (d->xc2 == xtest - 16 && d->type & 1))
pub fn doorfind_exit(table: &[DoorEntry], hero_x: u16, hero_y: u16) -> Option<DoorEntry> {
    let xtest = hero_x & 0xFFF0;
    let ytest = hero_y & 0xFFE0;
    for door in table {
        if door.dst_y == ytest
            && (door.dst_x == xtest
                || (door.door_type & 1 != 0 && door.dst_x == xtest.wrapping_sub(16)))
        {
            // Fine-grained sub-tile position check (mirrors original fmain.c nodoor2 conditions):
            // Horizontal door (type & 1): hero must be in lower half of the tile row (y & 0x10 != 0).
            // Vertical door: hero must not be at the very left edge of the tile (x & 15 >= 2).
            if door.door_type & 1 != 0 {
                if hero_y & 0x10 == 0 {
                    continue; // not yet in the doorway row
                }
            } else if hero_x & 15 < 2 {
                continue; // too far left within tile — not in the doorway
            }
            return Some(*door);
        }
    }
    None
}

/// Compute the indoor spawn position when entering a door from outside.
/// Mirrors fmain.c outdoor entry case: position placed just inside the door opening.
pub fn entry_spawn(door: &DoorEntry) -> (u16, u16) {
    if door.door_type == CAVE {
        (door.dst_x.wrapping_add(24), door.dst_y.wrapping_add(16))
    } else if door.door_type & 1 != 0 {
        // Horizontal
        (door.dst_x.wrapping_add(16), door.dst_y)
    } else {
        // Vertical
        (door.dst_x.wrapping_sub(1), door.dst_y.wrapping_add(16))
    }
}

/// Compute the outdoor spawn position when exiting a door from inside.
/// Mirrors fmain.c indoor exit case: position placed just outside the door opening.
pub fn exit_spawn(door: &DoorEntry) -> (u16, u16) {
    if door.door_type == CAVE {
        (door.src_x.wrapping_sub(4), door.src_y.wrapping_add(16))
    } else if door.door_type & 1 != 0 {
        // Horizontal
        (door.src_x.wrapping_add(16), door.src_y.wrapping_add(34))
    } else {
        // Vertical
        (door.src_x.wrapping_add(20), door.src_y.wrapping_add(16))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doorfind_no_match() {
        let table = [DoorEntry {
            src_region: 0,
            src_x: 0x1390,
            src_y: 0x1B60,
            dst_region: 9,
            dst_x: 0x1980,
            dst_y: 0x8C60,
            door_type: CAVE,
        }];
        let result = doorfind(&table, 1, 100, 100);
        assert!(result.is_none());
    }

    #[test]
    fn test_doorfind_exact_match() {
        // Dragon cave: region 0, (0x1390, 0x1B60)
        let table = [DoorEntry {
            src_region: 0,
            src_x: 0x1390,
            src_y: 0x1B60,
            dst_region: 9,
            dst_x: 0x1980,
            dst_y: 0x8C60,
            door_type: CAVE,
        }];
        let result = doorfind(&table, 0, 0x1390, 0x1B60);
        assert!(result.is_some());
        let d = result.unwrap();
        assert_eq!(d.dst_region, 9);
        assert_eq!(d.dst_x, 0x1980);
    }

    #[test]
    fn test_doorfind_grid_match() {
        // VWOOD (vertical, type & 1 == 0): exact grid x match required.
        // Door at grid-aligned (0x60, 0x60); hero anywhere in same 16×32px cell triggers it.
        let table = [DoorEntry {
            src_region: 2,
            src_x: 0x60,
            src_y: 0x60,
            dst_region: 5,
            dst_x: 200,
            dst_y: 200,
            door_type: VWOOD,
        }];
        // hero_x & 0xFFF0 = 0x60, hero_y & 0xFFE0 = 0x60 → match
        assert!(doorfind(&table, 2, 0x60, 0x60).is_some());
        assert!(doorfind(&table, 2, 0x6F, 0x7F).is_some()); // right edge of cell
                                                            // hero_x & 0xFFF0 = 0x70 ≠ 0x60 → no match (vertical requires exact x)
        assert!(doorfind(&table, 2, 0x70, 0x60).is_none());
        // HWOOD (horizontal, type & 1 == 1): xtest in [src_x, src_x+16] covers both tile cells.
        // Original: `d->xc1 + 16 < xtest` → skip, so match iff xtest <= src_x+16.
        let htable = [DoorEntry {
            src_region: 2,
            src_x: 0x60,
            src_y: 0x60,
            dst_region: 5,
            dst_x: 200,
            dst_y: 200,
            door_type: HWOOD,
        }];
        assert!(doorfind(&htable, 2, 0x60, 0x60).is_some()); // xtest=0x60 = src_x → left cell
        assert!(doorfind(&htable, 2, 0x6F, 0x60).is_some()); // xtest=0x60, still left cell
        assert!(doorfind(&htable, 2, 0x70, 0x60).is_some()); // xtest=0x70 = src_x+16 → right cell ✓
        assert!(doorfind(&htable, 2, 0x80, 0x60).is_none()); // xtest=0x80 > src_x+16 → no match
    }

    #[test]
    fn test_doorfind_exit_vertical() {
        // VWOOD door: dst_x = 0x0bd0, dst_y = 0x84c0 (village #1.a)
        // VWOOD exit is triggered via bump detection — probe_x = hero_x+4 lands inside the door tile.
        // entry_spawn puts hero at (0x0bcf, 0x84d0); walking north, right probe = 0x0bd3.
        let table = [DoorEntry {
            src_region: 2,
            src_x: 0x49d0,
            src_y: 0x3dc0,
            dst_region: 8,
            dst_x: 0x0bd0,
            dst_y: 0x84c0,
            door_type: VWOOD,
        }];
        // Bump probe inside door tile: xtest=0x0bd0 ✓, ytest=0x84c0 ✓, x & 15 = 3 >= 2 ✓ → match
        assert!(doorfind_exit(&table, 0x0bd3, 0x84c2).is_some());
        // Fine check fails: x & 15 = 0 < 2 → no match
        assert!(doorfind_exit(&table, 0x0bd0, 0x84c0).is_none());
        // Wrong ytest
        assert!(doorfind_exit(&table, 0x0bd3, 0x84e0).is_none());
    }

    #[test]
    fn test_doorfind_exit_horizontal() {
        // HWOOD door (type & 1 == 1): hero can be 16px right of dst_x
        let table = [DoorEntry {
            src_region: 2,
            src_x: 0x4a10,
            src_y: 0x3c80,
            dst_region: 8,
            dst_x: 0x0d10,
            dst_y: 0x8280,
            door_type: HWOOD,
        }];
        // y=0x8290: ytest=0x8280 ✓, y & 0x10 = 0x10 ✓ → direct match
        assert!(doorfind_exit(&table, 0x0d10, 0x8290).is_some());
        // 16px right of dst_x also matches for horizontal
        assert!(doorfind_exit(&table, 0x0d20, 0x8290).is_some());
        // y=0x8280: fine check fails (y & 0x10 == 0) → no match
        assert!(doorfind_exit(&table, 0x0d10, 0x8280).is_none());
        // 32px right → xtest off by too much → no match
        assert!(doorfind_exit(&table, 0x0d30, 0x8290).is_none());
    }

    #[test]
    fn test_exit_spawn_vertical() {
        let door = DoorEntry {
            src_x: 0x49d0,
            src_y: 0x3dc0,
            door_type: VWOOD,
            ..Default::default()
        };
        let (x, y) = exit_spawn(&door);
        assert_eq!(x, 0x49d0 + 20);
        assert_eq!(y, 0x3dc0 + 16);
    }

    #[test]
    fn test_exit_spawn_horizontal() {
        let door = DoorEntry {
            src_x: 0x4a10,
            src_y: 0x3c80,
            door_type: HWOOD,
            ..Default::default()
        };
        let (x, y) = exit_spawn(&door);
        assert_eq!(x, 0x4a10 + 16);
        assert_eq!(y, 0x3c80 + 34);
    }

    #[test]
    fn test_entry_spawn_vertical() {
        let door = DoorEntry {
            dst_x: 0x0bd0,
            dst_y: 0x84c0,
            door_type: VWOOD,
            ..Default::default()
        };
        let (x, y) = entry_spawn(&door);
        assert_eq!(x, 0x0bd0u16.wrapping_sub(1));
        assert_eq!(y, 0x84c0 + 16);
    }

    #[test]
    fn test_doorfind_nearest_by_bump_radius_no_match() {
        let table = [DoorEntry {
            src_region: 0,
            src_x: 500,
            src_y: 500,
            dst_region: 8,
            dst_x: 0,
            dst_y: 0,
            door_type: HWOOD,
        }];
        // Wrong region
        assert!(doorfind_nearest_by_bump_radius(&table, 1, 500, 500).is_none());
        // Too far X
        assert!(
            doorfind_nearest_by_bump_radius(&table, 0, 500 + BUMP_PROX_X as u16, 500).is_none()
        );
        // Too far Y
        assert!(
            doorfind_nearest_by_bump_radius(&table, 0, 500, 500 + BUMP_PROX_Y as u16).is_none()
        );
    }

    #[test]
    fn test_doorfind_nearest_by_bump_radius_match() {
        let table = [
            DoorEntry {
                src_region: 2,
                src_x: 100,
                src_y: 100,
                dst_region: 8,
                dst_x: 0,
                dst_y: 0,
                door_type: HWOOD,
            },
            DoorEntry {
                src_region: 2,
                src_x: 200,
                src_y: 100,
                dst_region: 8,
                dst_x: 0,
                dst_y: 0,
                door_type: VWOOD,
            },
        ];
        // Within radius of first door only
        let result = doorfind_nearest_by_bump_radius(&table, 2, 110, 100);
        assert!(result.is_some());
        let (idx, door) = result.unwrap();
        assert_eq!(idx, 0);
        assert_eq!(door.door_type, HWOOD);
        // Nearest of two doors both in range — hero at x=120, door1 Δ20, door2 Δ80 (out of range)
        // Use doors closer together so both fit within BUMP_PROX_X=32: door1@100, door2@130
        let table2 = [
            DoorEntry {
                src_region: 2,
                src_x: 100,
                src_y: 100,
                dst_region: 8,
                dst_x: 0,
                dst_y: 0,
                door_type: HWOOD,
            },
            DoorEntry {
                src_region: 2,
                src_x: 130,
                src_y: 100,
                dst_region: 8,
                dst_x: 0,
                dst_y: 0,
                door_type: VWOOD,
            },
        ];
        // Hero at x=120: Δ20 from door1, Δ10 from door2 → door2 is nearest
        let result2 = doorfind_nearest_by_bump_radius(&table2, 2, 120, 100);
        assert!(result2.is_some());
        let (idx2, door2) = result2.unwrap();
        assert_eq!(idx2, 1);
        assert_eq!(door2.door_type, VWOOD);
    }
}
