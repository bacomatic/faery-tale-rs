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

/// The 86-entry door table ported verbatim from original/fmain.c doorlist[].
pub static DOOR_TABLE: [DoorEntry; DOOR_COUNT] = [
    DoorEntry { src_region: 4, src_x: 0x1170, src_y: 0x5060, dst_region: 8, dst_x: 0x2870, dst_y: 0x8B60 },
    DoorEntry { src_region: 4, src_x: 0x1170, src_y: 0x5060, dst_region: 8, dst_x: 0x2870, dst_y: 0x8B60 },
    DoorEntry { src_region: 4, src_x: 0x1170, src_y: 0x5060, dst_region: 8, dst_x: 0x2870, dst_y: 0x8B60 },
    DoorEntry { src_region: 4, src_x: 0x1170, src_y: 0x5060, dst_region: 8, dst_x: 0x2870, dst_y: 0x8B60 },
    DoorEntry { src_region: 0, src_x: 0x1390, src_y: 0x1B60, dst_region: 9, dst_x: 0x1980, dst_y: 0x8C60 },
    DoorEntry { src_region: 6, src_x: 0x1770, src_y: 0x6AA0, dst_region: 8, dst_x: 0x2270, dst_y: 0x96A0 },
    DoorEntry { src_region: 6, src_x: 0x1970, src_y: 0x62A0, dst_region: 8, dst_x: 0x1F70, dst_y: 0x96A0 },
    DoorEntry { src_region: 4, src_x: 0x1AA0, src_y: 0x4BA0, dst_region: 8, dst_x: 0x13A0, dst_y: 0x95A0 },
    DoorEntry { src_region: 4, src_x: 0x1AA0, src_y: 0x4C60, dst_region: 8, dst_x: 0x13A0, dst_y: 0x9760 },
    DoorEntry { src_region: 4, src_x: 0x1B20, src_y: 0x4B60, dst_region: 8, dst_x: 0x1720, dst_y: 0x9560 },
    DoorEntry { src_region: 4, src_x: 0x1B80, src_y: 0x4B80, dst_region: 8, dst_x: 0x1580, dst_y: 0x9580 },
    DoorEntry { src_region: 4, src_x: 0x1B80, src_y: 0x4C40, dst_region: 8, dst_x: 0x1580, dst_y: 0x9740 },
    DoorEntry { src_region: 2, src_x: 0x1E70, src_y: 0x3B60, dst_region: 8, dst_x: 0x2880, dst_y: 0x9C60 },
    DoorEntry { src_region: 2, src_x: 0x2480, src_y: 0x33A0, dst_region: 8, dst_x: 0x2E80, dst_y: 0x8DA0 },
    DoorEntry { src_region: 8, src_x: 0x2960, src_y: 0x8760, dst_region: 8, dst_x: 0x2B00, dst_y: 0x92C0 },
    DoorEntry { src_region: 8, src_x: 0x2B00, src_y: 0x92C0, dst_region: 9, dst_x: 0x2960, dst_y: 0x8780 },
    DoorEntry { src_region: 6, src_x: 0x2C00, src_y: 0x7160, dst_region: 8, dst_x: 0x2AF0, dst_y: 0x9360 },
    DoorEntry { src_region: 2, src_x: 0x2F70, src_y: 0x2E60, dst_region: 8, dst_x: 0x3180, dst_y: 0x9A60 },
    DoorEntry { src_region: 6, src_x: 0x2F70, src_y: 0x63A0, dst_region: 8, dst_x: 0x1C70, dst_y: 0x96A0 },
    DoorEntry { src_region: 2, src_x: 0x3180, src_y: 0x38C0, dst_region: 8, dst_x: 0x2780, dst_y: 0x98C0 },
    DoorEntry { src_region: 4, src_x: 0x3470, src_y: 0x4B60, dst_region: 9, dst_x: 0x0470, dst_y: 0x8EE0 },
    DoorEntry { src_region: 0, src_x: 0x3DE0, src_y: 0x1BC0, dst_region: 8, dst_x: 0x2EE0, dst_y: 0x93C0 },
    DoorEntry { src_region: 0, src_x: 0x3E00, src_y: 0x1BC0, dst_region: 8, dst_x: 0x2F00, dst_y: 0x93C0 },
    DoorEntry { src_region: 3, src_x: 0x4270, src_y: 0x2560, dst_region: 8, dst_x: 0x2E80, dst_y: 0x9A60 },
    DoorEntry { src_region: 3, src_x: 0x4280, src_y: 0x3BC0, dst_region: 8, dst_x: 0x2980, dst_y: 0x98C0 },
    DoorEntry { src_region: 5, src_x: 0x45E0, src_y: 0x5380, dst_region: 8, dst_x: 0x25D0, dst_y: 0x9680 },
    DoorEntry { src_region: 3, src_x: 0x4780, src_y: 0x2FC0, dst_region: 8, dst_x: 0x2580, dst_y: 0x98C0 },
    DoorEntry { src_region: 7, src_x: 0x4860, src_y: 0x6640, dst_region: 8, dst_x: 0x1C60, dst_y: 0x9A40 },
    DoorEntry { src_region: 7, src_x: 0x4890, src_y: 0x66A0, dst_region: 8, dst_x: 0x1C90, dst_y: 0x9AA0 },
    DoorEntry { src_region: 5, src_x: 0x4960, src_y: 0x5B40, dst_region: 8, dst_x: 0x2260, dst_y: 0x9A40 },
    DoorEntry { src_region: 5, src_x: 0x4990, src_y: 0x5BA0, dst_region: 8, dst_x: 0x2290, dst_y: 0x98A0 },
    DoorEntry { src_region: 3, src_x: 0x49A0, src_y: 0x3CC0, dst_region: 8, dst_x: 0x0BA0, dst_y: 0x82C0 },
    DoorEntry { src_region: 3, src_x: 0x49D0, src_y: 0x3DC0, dst_region: 8, dst_x: 0x0BD0, dst_y: 0x84C0 },
    DoorEntry { src_region: 3, src_x: 0x49D0, src_y: 0x3E00, dst_region: 8, dst_x: 0x0BD0, dst_y: 0x8500 },
    DoorEntry { src_region: 3, src_x: 0x4A10, src_y: 0x3C80, dst_region: 8, dst_x: 0x0D10, dst_y: 0x8280 },
    DoorEntry { src_region: 3, src_x: 0x4A10, src_y: 0x3D40, dst_region: 8, dst_x: 0x0F10, dst_y: 0x8340 },
    DoorEntry { src_region: 3, src_x: 0x4A30, src_y: 0x3DC0, dst_region: 8, dst_x: 0x0E30, dst_y: 0x85C0 },
    DoorEntry { src_region: 3, src_x: 0x4A60, src_y: 0x3E80, dst_region: 8, dst_x: 0x1060, dst_y: 0x8580 },
    DoorEntry { src_region: 3, src_x: 0x4A70, src_y: 0x3C80, dst_region: 8, dst_x: 0x1370, dst_y: 0x8280 },
    DoorEntry { src_region: 3, src_x: 0x4A80, src_y: 0x3D40, dst_region: 8, dst_x: 0x1190, dst_y: 0x8340 },
    DoorEntry { src_region: 3, src_x: 0x4C70, src_y: 0x3260, dst_region: 8, dst_x: 0x2580, dst_y: 0x9C60 },
    DoorEntry { src_region: 5, src_x: 0x4D60, src_y: 0x5440, dst_region: 8, dst_x: 0x1F60, dst_y: 0x9C40 },
    DoorEntry { src_region: 5, src_x: 0x4D90, src_y: 0x4380, dst_region: 8, dst_x: 0x3080, dst_y: 0x8D80 },
    DoorEntry { src_region: 5, src_x: 0x4D90, src_y: 0x54A0, dst_region: 8, dst_x: 0x1F90, dst_y: 0x9CA0 },
    DoorEntry { src_region: 7, src_x: 0x4DE0, src_y: 0x6B80, dst_region: 8, dst_x: 0x29D0, dst_y: 0x9680 },
    DoorEntry { src_region: 5, src_x: 0x5360, src_y: 0x5840, dst_region: 8, dst_x: 0x2260, dst_y: 0x9840 },
    DoorEntry { src_region: 5, src_x: 0x5390, src_y: 0x58A0, dst_region: 8, dst_x: 0x2290, dst_y: 0x98A0 },
    DoorEntry { src_region: 5, src_x: 0x5460, src_y: 0x4540, dst_region: 8, dst_x: 0x1C60, dst_y: 0x9840 },
    DoorEntry { src_region: 7, src_x: 0x5470, src_y: 0x6480, dst_region: 8, dst_x: 0x2C80, dst_y: 0x8D80 },
    DoorEntry { src_region: 5, src_x: 0x5490, src_y: 0x45A0, dst_region: 8, dst_x: 0x1C90, dst_y: 0x98A0 },
    DoorEntry { src_region: 5, src_x: 0x55F0, src_y: 0x52E0, dst_region: 8, dst_x: 0x16E0, dst_y: 0x83E0 },
    DoorEntry { src_region: 5, src_x: 0x56C0, src_y: 0x53C0, dst_region: 8, dst_x: 0x1BC0, dst_y: 0x84C0 },
    DoorEntry { src_region: 5, src_x: 0x56C0, src_y: 0x5440, dst_region: 8, dst_x: 0x19C0, dst_y: 0x8540 },
    DoorEntry { src_region: 5, src_x: 0x56F0, src_y: 0x51A0, dst_region: 8, dst_x: 0x19F0, dst_y: 0x82A0 },
    DoorEntry { src_region: 5, src_x: 0x5700, src_y: 0x5240, dst_region: 8, dst_x: 0x1DF0, dst_y: 0x8340 },
    DoorEntry { src_region: 5, src_x: 0x5710, src_y: 0x5440, dst_region: 8, dst_x: 0x1C10, dst_y: 0x8640 },
    DoorEntry { src_region: 5, src_x: 0x5730, src_y: 0x5300, dst_region: 8, dst_x: 0x1A50, dst_y: 0x8400 },
    DoorEntry { src_region: 5, src_x: 0x5730, src_y: 0x5380, dst_region: 8, dst_x: 0x1C30, dst_y: 0x8480 },
    DoorEntry { src_region: 5, src_x: 0x5750, src_y: 0x51A0, dst_region: 8, dst_x: 0x1C60, dst_y: 0x82A0 },
    DoorEntry { src_region: 5, src_x: 0x5750, src_y: 0x5260, dst_region: 8, dst_x: 0x2050, dst_y: 0x8360 },
    DoorEntry { src_region: 5, src_x: 0x5760, src_y: 0x53C0, dst_region: 8, dst_x: 0x2060, dst_y: 0x84C0 },
    DoorEntry { src_region: 5, src_x: 0x5760, src_y: 0x5440, dst_region: 8, dst_x: 0x1E60, dst_y: 0x8540 },
    DoorEntry { src_region: 5, src_x: 0x5860, src_y: 0x5D40, dst_region: 8, dst_x: 0x1C60, dst_y: 0x9A40 },
    DoorEntry { src_region: 5, src_x: 0x5890, src_y: 0x5DA0, dst_region: 8, dst_x: 0x1C90, dst_y: 0x9CA0 },
    DoorEntry { src_region: 3, src_x: 0x58C0, src_y: 0x2E60, dst_region: 9, dst_x: 0x0AC0, dst_y: 0x8860 },
    DoorEntry { src_region: 7, src_x: 0x5960, src_y: 0x6F40, dst_region: 8, dst_x: 0x2260, dst_y: 0x9A40 },
    DoorEntry { src_region: 7, src_x: 0x5990, src_y: 0x6FA0, dst_region: 8, dst_x: 0x2290, dst_y: 0x9CA0 },
    DoorEntry { src_region: 7, src_x: 0x59A0, src_y: 0x6760, dst_region: 8, dst_x: 0x2AA0, dst_y: 0x8B60 },
    DoorEntry { src_region: 5, src_x: 0x59E0, src_y: 0x5880, dst_region: 8, dst_x: 0x27D0, dst_y: 0x9680 },
    DoorEntry { src_region: 1, src_x: 0x5E70, src_y: 0x1A60, dst_region: 8, dst_x: 0x2580, dst_y: 0x9A60 },
    DoorEntry { src_region: 3, src_x: 0x5EC0, src_y: 0x2960, dst_region: 9, dst_x: 0x11C0, dst_y: 0x8B60 },
    DoorEntry { src_region: 7, src_x: 0x6060, src_y: 0x7240, dst_region: 8, dst_x: 0x1960, dst_y: 0x9C40 },
    DoorEntry { src_region: 7, src_x: 0x6090, src_y: 0x72A0, dst_region: 8, dst_x: 0x1990, dst_y: 0x9CA0 },
    DoorEntry { src_region: 3, src_x: 0x60F0, src_y: 0x32C0, dst_region: 8, dst_x: 0x25F0, dst_y: 0x8BC0 },
    DoorEntry { src_region: 1, src_x: 0x64C0, src_y: 0x1860, dst_region: 9, dst_x: 0x03C0, dst_y: 0x8660 },
    DoorEntry { src_region: 5, src_x: 0x6560, src_y: 0x5D40, dst_region: 8, dst_x: 0x1F60, dst_y: 0x9A40 },
    DoorEntry { src_region: 5, src_x: 0x6590, src_y: 0x5DA0, dst_region: 8, dst_x: 0x1F90, dst_y: 0x98A0 },
    DoorEntry { src_region: 1, src_x: 0x65C0, src_y: 0x1A20, dst_region: 9, dst_x: 0x04B0, dst_y: 0x8840 },
    DoorEntry { src_region: 3, src_x: 0x6670, src_y: 0x2A60, dst_region: 8, dst_x: 0x2B80, dst_y: 0x9A60 },
    DoorEntry { src_region: 1, src_x: 0x6800, src_y: 0x1B60, dst_region: 8, dst_x: 0x2AF0, dst_y: 0x9060 },
    DoorEntry { src_region: 5, src_x: 0x6B50, src_y: 0x4380, dst_region: 8, dst_x: 0x2850, dst_y: 0x8D80 },
    DoorEntry { src_region: 7, src_x: 0x6BE0, src_y: 0x7C80, dst_region: 8, dst_x: 0x2BD0, dst_y: 0x9680 },
    DoorEntry { src_region: 3, src_x: 0x6C70, src_y: 0x2E60, dst_region: 8, dst_x: 0x2880, dst_y: 0x9A60 },
    DoorEntry { src_region: 7, src_x: 0x6D60, src_y: 0x6840, dst_region: 8, dst_x: 0x1F60, dst_y: 0x9A40 },
    DoorEntry { src_region: 7, src_x: 0x6D90, src_y: 0x68A0, dst_region: 8, dst_x: 0x1F90, dst_y: 0x9AA0 },
    DoorEntry { src_region: 5, src_x: 0x6EE0, src_y: 0x5280, dst_region: 8, dst_x: 0x31D0, dst_y: 0x9680 },
];

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
        let result = doorfind(&DOOR_TABLE, 1, 100, 100);
        assert!(result.is_none());
    }

    #[test]
    fn test_door_table_count() {
        assert_eq!(DOOR_TABLE.len(), DOOR_COUNT);
    }

    #[test]
    fn test_doorfind_exact_match() {
        // Dragon cave: region 0, (0x1390, 0x1B60)
        let result = doorfind(&DOOR_TABLE, 0, 0x1390, 0x1B60);
        assert!(result.is_some());
        let d = result.unwrap();
        assert_eq!(d.dst_region, 9);
        assert_eq!(d.dst_x, 0x1980);
    }
}
