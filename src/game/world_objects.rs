//! Static world object data tables from the original game.
//!
//! Ports ob_listg[], ob_list0-9[], itrans[], and dstobs[] from fmain2.c:1294-1516.

pub type RawObject = (u16, u16, u8, u8); // (x, y, ob_id, ob_stat)

/// Translates ob_id (obytes enum) → stuff[] inventory index.
/// Port of `itrans[]` from `fmain2.c:1325–1332`.
pub fn ob_id_to_stuff_index(ob_id: u8) -> Option<usize> {
    const ITRANS: &[(u8, u8)] = &[
        (11, 35),   // QUIVER → arrows (×10)
        (18, 9),    // B_STONE
        (19, 10),   // G_JEWEL
        (22, 11),   // VIAL
        (21, 12),   // C_ORB
        (23, 13),   // B_TOTEM
        (17, 14),   // G_RING
        (24, 15),   // J_SKULL
        (145, 4),   // M_WAND
        (27, 5),    // Golden Lasso
        (8, 2),     // Sword
        (9, 1),     // Mace
        (12, 0),    // Dirk
        (10, 3),    // Bow
        (147, 23),  // ROSE
        (148, 24),  // FRUIT
        (149, 25),  // STATUE
        (150, 26),  // BOOK
        (151, 6),   // SHELL
        (155, 7),   // Sun Stone
        (136, 27),  // Herb
        (137, 28),  // Writ
        (138, 29),  // Bone
        (139, 22),  // Talisman
        (140, 30),  // Shard
        (25, 16),   // GOLD_KEY
        (153, 17),  // GREEN_KEY
        (114, 18),  // BLUE_KEY
        (242, 19),  // RED_KEY
        (26, 20),   // GREY_KEY
        (154, 21),  // WHITE_KEY
    ];
    for &(oid, sidx) in ITRANS {
        if oid == ob_id {
            return Some(sidx as usize);
        }
    }
    None
}

pub const OB_LISTG: &[RawObject] = &[
    (0, 0, 0, 0),              // 0: special item (for 'give')
    (0, 0, 28, 0),             // 1: dead brother 1
    (0, 0, 28, 0),             // 2: dead brother 2
    (19316, 15747, 11, 0),     // 3: ghost brother 1
    (18196, 15735, 11, 0),     // 4: ghost brother 2
    (12439, 36202, 10, 3),     // 5: spectre (setfig)
    (11092, 38526, 149, 1),    // 6: gold statue (seahold)
    (25737, 10662, 149, 1),    // 7: gold statue (ogre den)
    (2910, 39023, 149, 1),     // 8: gold statue (octal room)
    (12025, 37639, 149, 0),    // 9: gold statue (sorceress) — hidden
    (6700, 33766, 149, 0),     // 10: gold statue (priest) — hidden
];

pub const OB_LIST0: &[RawObject] = &[
    (3340, 6735, 12, 3),       // ranger west (setfig)
    (9678, 7035, 12, 3),       // ranger east (setfig)
    (4981, 6306, 12, 3),       // ranger north (setfig)
];

pub const OB_LIST1: &[RawObject] = &[
    (23087, 5667, 102, 1),     // TURTLE eggs
];

pub const OB_LIST2: &[RawObject] = &[
    (13668, 15000, 0, 3),      // wizard (setfig)
    (10627, 13154, 0, 3),      // wizard (setfig)
    (4981, 10056, 12, 3),      // ranger (setfig)
    (13950, 11087, 16, 1),     // SACKS
    (10344, 36171, 151, 1),    // SHELL
];

pub const OB_LIST3: &[RawObject] = &[
    (19298, 16128, 15, 1),     // CHEST
    (18310, 15969, 13, 3),     // beggar (setfig)
    (20033, 14401, 0, 3),      // wizard (setfig)
    (24794, 13102, 13, 3),     // beggar (setfig)
    (21626, 15446, 18, 1),     // B_STONE at stone ring
    (21616, 15456, 13, 1),     // MONEY
    (21636, 15456, 17, 1),     // G_RING
    (20117, 14222, 19, 1),     // G_JEWEL
    (24185, 9840, 16, 1),      // SACKS
    (25769, 10617, 13, 1),     // MONEY
    (25678, 10703, 18, 1),     // B_STONE
    (17177, 10599, 20, 1),     // SCRAP
];

pub const OB_LIST4: &[RawObject] = &[
    (0, 0, 0, 0),              // dummy
    (0, 0, 0, 0),              // dummy
    (6817, 19693, 13, 3),      // beggar (setfig)
];

pub const OB_LIST5: &[RawObject] = &[
    (22184, 21156, 13, 3),     // beggar (setfig)
    (18734, 17595, 17, 1),     // G_RING
    (21294, 22648, 15, 1),     // CHEST
    (22956, 19955, 0, 3),      // wizard (setfig)
    (28342, 22613, 0, 3),      // wizard (setfig)
];

pub const OB_LIST6: &[RawObject] = &[
    (24794, 13102, 13, 3),     // dummy (beggar setfig)
];

pub const OB_LIST7: &[RawObject] = &[
    (23297, 5797, 102, 1),     // TURTLE (dummy object)
];

pub const OB_LIST8: &[RawObject] = &[
    (6700, 33756, 1, 3),       // priest
    (5491, 33780, 5, 3),       // king
    (5592, 33764, 6, 3),       // noble
    (5514, 33668, 2, 3),       // guard
    (5574, 33668, 2, 3),       // guard
    (8878, 38995, 0, 3),       // wizard
    (7776, 34084, 0, 3),       // wizard
    (5514, 33881, 3, 3),       // guard
    (5574, 33881, 3, 3),       // guard
    (10853, 35656, 4, 3),      // princess
    (12037, 37614, 7, 3),      // sorceress
    (11013, 36804, 9, 3),      // witch
    (9631, 38953, 8, 3),       // bartender
    (10191, 38953, 8, 3),      // bartender
    (10649, 38953, 8, 3),      // bartender
    (2966, 33964, 8, 3),       // bartender
    (9532, 40002, 31, 1),      // FOOTSTOOL
    (6747, 33751, 31, 1),      // FOOTSTOOL
    (11410, 36169, 155, 1),    // sunstone (27+128)
    (9550, 39964, 23, 1),      // B_TOTEM
    (9552, 39964, 23, 1),      // B_TOTEM
    (9682, 39964, 23, 1),      // B_TOTEM
    (9684, 39964, 23, 1),      // B_TOTEM
    (9532, 40119, 23, 1),      // B_TOTEM (table)
    (9575, 39459, 14, 1),      // URN
    (9590, 39459, 14, 1),      // URN
    (9605, 39459, 14, 1),      // URN
    (9680, 39453, 22, 1),      // VIAL
    (9682, 39453, 22, 1),      // VIAL
    (9784, 39453, 22, 1),      // VIAL
    (9668, 39554, 15, 1),      // CHEST
    (11090, 39462, 13, 1),     // MONEY
    (11108, 39458, 23, 1),     // B_TOTEM
    (11118, 39459, 23, 1),     // B_TOTEM
    (11128, 39459, 23, 1),     // B_TOTEM
    (11138, 39458, 23, 1),     // B_TOTEM
    (11148, 39459, 23, 1),     // B_TOTEM
    (11158, 39459, 23, 1),     // B_TOTEM
    (11855, 36206, 31, 1),     // FOOTSTOOL
    (11909, 36198, 15, 1),     // CHEST
    (11918, 36246, 23, 1),     // B_TOTEM
    (11928, 36246, 23, 1),     // B_TOTEM
    (11938, 36246, 23, 1),     // B_TOTEM
    (12212, 38481, 15, 1),     // CHEST
    (11652, 38481, 242, 1),    // RED_KEY
    (10427, 39977, 31, 1),     // FOOTSTOOL
    (10323, 40071, 14, 1),     // URN
    (10059, 38472, 16, 1),     // SACKS
    (10344, 36171, 151, 1),    // SHELL
    (11936, 36207, 20, 1),     // SCRAP
    (9674, 35687, 14, 1),      // URN
    (5473, 38699, 147, 1),     // ROSE
    (7185, 34342, 148, 1),     // FRUIT
    (7190, 34342, 148, 1),     // FRUIT
    (7195, 34342, 148, 1),     // FRUIT
    (7185, 34347, 148, 1),     // FRUIT
    (7190, 34347, 148, 1),     // FRUIT
    (7195, 34347, 148, 1),     // FRUIT
    (6593, 34085, 148, 1),     // FRUIT
    (6598, 34085, 148, 1),     // FRUIT
    (6593, 34090, 148, 1),     // FRUIT
    (6598, 34090, 148, 1),     // FRUIT
    (3872, 33546, 25, 5),      // GOLD_KEY (hidden)
    (3887, 33510, 23, 5),      // B_TOTEM (hidden)
    (4495, 33510, 22, 5),      // VIAL (hidden)
    (3327, 33383, 24, 5),      // J_SKULL (hidden)
    (4221, 34119, 11, 5),      // QUIVER (hidden)
    (7610, 33604, 22, 5),      // VIAL (hidden)
    (7616, 33522, 13, 5),      // MONEY (hidden)
    (9570, 35768, 18, 5),      // B_STONE (hidden)
    (9668, 35769, 11, 5),      // QUIVER (hidden)
    (9553, 38951, 17, 5),      // G_RING (hidden)
    (10062, 39005, 24, 5),     // J_SKULL (hidden)
    (10577, 38951, 22, 5),     // VIAL (hidden)
    (11062, 39514, 13, 5),     // MONEY (hidden)
    (8845, 39494, 154, 5),     // WHITE_KEY (hidden)
    (6542, 39494, 19, 5),      // G_JEWEL (hidden)
    (7313, 38992, 242, 5),     // RED_KEY (hidden)
];

pub const OB_LIST9: &[RawObject] = &[
    (7540, 38528, 145, 1),     // M_WAND
    (9624, 36559, 145, 1),     // M_WAND
    (9624, 37459, 145, 1),     // M_WAND
    (8337, 36719, 145, 1),     // M_WAND
    (8154, 34890, 15, 1),      // CHEST
    (7826, 35741, 15, 1),      // CHEST
    (3460, 37260, 0, 3),       // wizard (setfig)
    (8485, 35725, 13, 1),      // MONEY
    (3723, 39340, 138, 1),     // king's bone (128+10)
];

pub const OB_TABLES: [&[RawObject]; 10] = [
    OB_LIST0, OB_LIST1, OB_LIST2, OB_LIST3, OB_LIST4,
    OB_LIST5, OB_LIST6, OB_LIST7, OB_LIST8, OB_LIST9,
];

pub const DSTOBS_INIT: [bool; 10] = [
    false, false, false, false, false,
    false, false, false, true, true,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_itrans_known_mappings() {
        // Items that should NOT map (not in inventory)
        assert_eq!(ob_id_to_stuff_index(15), None); // CHEST
        assert_eq!(ob_id_to_stuff_index(16), None); // SACKS
        assert_eq!(ob_id_to_stuff_index(14), None); // URN
        assert_eq!(ob_id_to_stuff_index(13), None); // MONEY

        // Items that should map
        assert_eq!(ob_id_to_stuff_index(25), Some(16)); // GOLD_KEY
        assert_eq!(ob_id_to_stuff_index(242), Some(19)); // RED_KEY
        assert_eq!(ob_id_to_stuff_index(145), Some(4)); // M_WAND
        assert_eq!(ob_id_to_stuff_index(149), Some(25)); // STATUE
        assert_eq!(ob_id_to_stuff_index(151), Some(6)); // SHELL
    }

    #[test]
    fn test_itrans_covers_all_documented_items() {
        let mappings = [
            (11, 35),   // QUIVER
            (18, 9),    // B_STONE
            (19, 10),   // G_JEWEL
            (22, 11),   // VIAL
            (21, 12),   // C_ORB
            (23, 13),   // B_TOTEM
            (17, 14),   // G_RING
            (24, 15),   // J_SKULL
            (145, 4),   // M_WAND
            (27, 5),    // Golden Lasso
            (8, 2),     // Sword
            (9, 1),     // Mace
            (12, 0),    // Dirk
            (10, 3),    // Bow
            (147, 23),  // ROSE
            (148, 24),  // FRUIT
            (149, 25),  // STATUE
            (150, 26),  // BOOK
            (151, 6),   // SHELL
            (155, 7),   // Sun Stone
            (136, 27),  // Herb
            (137, 28),  // Writ
            (138, 29),  // Bone
            (139, 22),  // Talisman
            (140, 30),  // Shard
            (25, 16),   // GOLD_KEY
            (153, 17),  // GREEN_KEY
            (114, 18),  // BLUE_KEY
            (242, 19),  // RED_KEY
            (26, 20),   // GREY_KEY
            (154, 21),  // WHITE_KEY
        ];

        for (ob_id, expected_idx) in &mappings {
            assert_eq!(
                ob_id_to_stuff_index(*ob_id),
                Some(*expected_idx as usize),
                "ob_id {} should map to stuff_idx {}",
                ob_id,
                expected_idx
            );
        }
    }

    #[test]
    fn test_ob_list3_has_starting_chest() {
        // The starting chest at (19298, 16128) should be in OB_LIST3
        let chest_found = OB_LIST3.iter().any(|&(x, y, ob_id, ob_stat)| {
            x == 19298 && y == 16128 && ob_id == 15 && ob_stat == 1
        });
        assert!(chest_found, "CHEST at (19298, 16128) should be in OB_LIST3");
    }

    #[test]
    fn test_ob_tables_regions_count() {
        assert_eq!(OB_TABLES.len(), 10, "Should have 10 regions");
        assert!(
            OB_LIST8.len() > 50,
            "OB_LIST8 should have more than 50 objects"
        );
    }

    #[test]
    fn test_dstobs_init() {
        assert_eq!(DSTOBS_INIT.len(), 10, "Should have 10 region flags");

        // Regions 0-7 should be false
        for i in 0..8 {
            assert!(!DSTOBS_INIT[i], "DSTOBS_INIT[{}] should be false", i);
        }

        // Regions 8-9 should be true
        for i in 8..10 {
            assert!(DSTOBS_INIT[i], "DSTOBS_INIT[{}] should be true", i);
        }
    }
}
