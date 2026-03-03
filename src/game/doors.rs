//! Door and portal system: 86-door table with doorfind() transition logic.
//! Ported from fmain.c doorfind() and the hardcoded door table.

/// Maximum number of door entries.
pub const DOOR_COUNT: usize = 86;

/// A single door entry mapping source → destination.
#[derive(Debug, Clone, Copy, Default)]
pub struct DoorEntry {
    pub src_region: u8,
    pub src_x: u16,
    pub src_y: u16,
    pub dst_region: u8,
    pub dst_x: u16,
    pub dst_y: u16,
}

/// The door table (placeholders — real values from ADF or hardcoded constants in original).
/// Format matches the 86-entry table from fmain.c / fdata.c.
/// TODO: Replace with real door positions once ADF is analyzed.
pub static DOOR_TABLE: &[DoorEntry] = &[
    // Placeholder doors — entry 0 is always unused (type 0 = no door)
    DoorEntry { src_region: 0, src_x: 0, src_y: 0, dst_region: 0, dst_x: 0, dst_y: 0 },
];

/// Find the door that the hero is standing at, if any.
/// Checks within DOOR_PROXIMITY pixels of door position.
pub const DOOR_PROXIMITY: u16 = 8;

pub fn doorfind(region_num: u8, hero_x: u16, hero_y: u16) -> Option<DoorEntry> {
    for door in DOOR_TABLE {
        if door.src_region == region_num
            && hero_x.abs_diff(door.src_x) < DOOR_PROXIMITY
            && hero_y.abs_diff(door.src_y) < DOOR_PROXIMITY
        {
            return Some(*door);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doorfind_no_match() {
        // With only placeholder door at 0,0 region 0, non-zero position should not match
        let result = doorfind(1, 100, 100);
        assert!(result.is_none());
    }

    #[test]
    fn test_door_count_reasonable() {
        // Door table should have at most DOOR_COUNT entries
        assert!(DOOR_TABLE.len() <= DOOR_COUNT);
    }
}
