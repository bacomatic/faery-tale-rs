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
}

/// Find the door that the hero is standing at, if any.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_doorfind_no_match() {
        let table = [DoorEntry { src_region: 0, src_x: 0x1390, src_y: 0x1B60, dst_region: 9, dst_x: 0x1980, dst_y: 0x8C60 }];
        let result = doorfind(&table, 1, 100, 100);
        assert!(result.is_none());
    }

    #[test]
    fn test_doorfind_exact_match() {
        // Dragon cave: region 0, (0x1390, 0x1B60)
        let table = [DoorEntry { src_region: 0, src_x: 0x1390, src_y: 0x1B60, dst_region: 9, dst_x: 0x1980, dst_y: 0x8C60 }];
        let result = doorfind(&table, 0, 0x1390, 0x1B60);
        assert!(result.is_some());
        let d = result.unwrap();
        assert_eq!(d.dst_region, 9);
        assert_eq!(d.dst_x, 0x1980);
    }

    #[test]
    fn test_doorfind_proximity_edge() {
        let table = [DoorEntry { src_region: 2, src_x: 100, src_y: 100, dst_region: 5, dst_x: 200, dst_y: 200 }];
        assert!(doorfind(&table, 2, 107, 100).is_some());
        assert!(doorfind(&table, 2, 108, 100).is_none());
    }
}
