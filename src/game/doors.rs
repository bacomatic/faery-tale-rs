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
pub const HWOOD:  u8 = 1;
pub const VWOOD:  u8 = 2;
pub const HSTONE: u8 = 3;
pub const VSTONE: u8 = 4;
pub const HCITY:  u8 = 5;
pub const VCITY:  u8 = 6;
pub const CRYST:  u8 = 7;
pub const SECRET: u8 = 8;
pub const BLACK:  u8 = 9;
pub const MARBLE: u8 = 10;
pub const LOG:    u8 = 11;
pub const HSTON2: u8 = 13;
pub const VSTON2: u8 = 14;
pub const STAIR:  u8 = 15;
pub const DESERT: u8 = 17;
pub const CAVE:   u8 = 18;
// VLOG = 18 (same as CAVE); distinguish by context only

/// Find the door at an outdoor position (region < 8) using grid-aligned src coords.
/// Mirrors fmain.c outdoor doorfind: xtest = hero_x & 0xFFF0, ytest = hero_y & 0xFFE0.
pub const DOOR_PROXIMITY: u16 = 8;

pub fn doorfind(table: &[DoorEntry], region_num: u8, hero_x: u16, hero_y: u16) -> Option<DoorEntry> {
    for door in table {
        if door.src_region == region_num
            && hero_x.abs_diff(door.src_x) < DOOR_PROXIMITY
            && hero_y.abs_diff(door.src_y) < DOOR_PROXIMITY
        {
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
        let table = [DoorEntry { src_region: 0, src_x: 0x1390, src_y: 0x1B60, dst_region: 9, dst_x: 0x1980, dst_y: 0x8C60, door_type: CAVE }];
        let result = doorfind(&table, 1, 100, 100);
        assert!(result.is_none());
    }

    #[test]
    fn test_doorfind_exact_match() {
        // Dragon cave: region 0, (0x1390, 0x1B60)
        let table = [DoorEntry { src_region: 0, src_x: 0x1390, src_y: 0x1B60, dst_region: 9, dst_x: 0x1980, dst_y: 0x8C60, door_type: CAVE }];
        let result = doorfind(&table, 0, 0x1390, 0x1B60);
        assert!(result.is_some());
        let d = result.unwrap();
        assert_eq!(d.dst_region, 9);
        assert_eq!(d.dst_x, 0x1980);
    }

    #[test]
    fn test_doorfind_proximity_edge() {
        let table = [DoorEntry { src_region: 2, src_x: 100, src_y: 100, dst_region: 5, dst_x: 200, dst_y: 200, door_type: VWOOD }];
        assert!(doorfind(&table, 2, 107, 100).is_some());
        assert!(doorfind(&table, 2, 108, 100).is_none());
    }

    #[test]
    fn test_doorfind_exit_vertical() {
        // VWOOD door: dst_x = 0x0bd0, dst_y = 0x84c0 (village #1.a)
        // grid-aligned: xtest = 0x0bd0, ytest = 0x84c0 (already aligned)
        let table = [DoorEntry {
            src_region: 2, src_x: 0x49d0, src_y: 0x3dc0,
            dst_region: 8, dst_x: 0x0bd0, dst_y: 0x84c0, door_type: VWOOD,
        }];
        assert!(doorfind_exit(&table, 0x0bd0, 0x84c0).is_some());
        assert!(doorfind_exit(&table, 0x0bd0, 0x84e0).is_none());
    }

    #[test]
    fn test_doorfind_exit_horizontal() {
        // HWOOD door (type & 1 == 1): hero can be 16px right of dst_x
        let table = [DoorEntry {
            src_region: 2, src_x: 0x4a10, src_y: 0x3c80,
            dst_region: 8, dst_x: 0x0d10, dst_y: 0x8280, door_type: HWOOD,
        }];
        // Direct match
        assert!(doorfind_exit(&table, 0x0d10, 0x8280).is_some());
        // 16px right of dst_x also matches for horizontal
        assert!(doorfind_exit(&table, 0x0d20, 0x8280).is_some());
        // 32px right does not
        assert!(doorfind_exit(&table, 0x0d30, 0x8280).is_none());
    }

    #[test]
    fn test_exit_spawn_vertical() {
        let door = DoorEntry { src_x: 0x49d0, src_y: 0x3dc0, door_type: VWOOD, ..Default::default() };
        let (x, y) = exit_spawn(&door);
        assert_eq!(x, 0x49d0 + 20);
        assert_eq!(y, 0x3dc0 + 16);
    }

    #[test]
    fn test_exit_spawn_horizontal() {
        let door = DoorEntry { src_x: 0x4a10, src_y: 0x3c80, door_type: HWOOD, ..Default::default() };
        let (x, y) = exit_spawn(&door);
        assert_eq!(x, 0x4a10 + 16);
        assert_eq!(y, 0x3c80 + 34);
    }

    #[test]
    fn test_entry_spawn_vertical() {
        let door = DoorEntry { dst_x: 0x0bd0, dst_y: 0x84c0, door_type: VWOOD, ..Default::default() };
        let (x, y) = entry_spawn(&door);
        assert_eq!(x, 0x0bd0u16.wrapping_sub(1));
        assert_eq!(y, 0x84c0 + 16);
    }
}
