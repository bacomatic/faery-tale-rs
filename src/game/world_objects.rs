//! Static world object data tables from the original game.
//!
//! Ports itrans[] from fmain2.c:1325-1332.
//! Object placement data (ob_listg, ob_list0-9) now lives in faery.toml.

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
}
