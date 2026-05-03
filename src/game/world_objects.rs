//! Static world object data tables from the original game.
//!
//! Ports itrans[] from fmain2.c:1325-1332.
//! Object placement data (ob_listg, ob_list0-9) now lives in faery.toml.

/// Translates ob_id (obytes enum) → stuff[] inventory index.
/// Port of `itrans[]` from `fmain2.c:1325–1332`.
pub fn ob_id_to_stuff_index(ob_id: u8) -> Option<usize> {
    const ITRANS: &[(u8, u8)] = &[
        (11, 35),  // QUIVER → arrows (×10)
        (18, 9),   // B_STONE
        (19, 10),  // G_JEWEL
        (22, 11),  // VIAL
        (21, 12),  // C_ORB
        (23, 13),  // B_TOTEM
        (17, 14),  // G_RING
        (24, 15),  // J_SKULL
        (145, 4),  // M_WAND
        (27, 5),   // Golden Lasso
        (8, 2),    // Sword
        (9, 1),    // Mace
        (12, 0),   // Dirk
        (10, 3),   // Bow
        (147, 23), // ROSE
        (148, 24), // FRUIT
        (149, 25), // STATUE
        (150, 26), // BOOK
        (151, 6),  // SHELL
        (155, 7),  // Sun Stone
        (136, 27), // Herb
        (137, 28), // Writ
        (138, 29), // Bone
        (139, 22), // Talisman
        (140, 30), // Shard
        (25, 16),  // GOLD_KEY
        (153, 17), // GREEN_KEY
        (114, 18), // BLUE_KEY
        (242, 19), // RED_KEY
        (26, 20),  // GREY_KEY
        (154, 21), // WHITE_KEY
    ];
    for &(oid, sidx) in ITRANS {
        if oid == ob_id {
            return Some(sidx as usize);
        }
    }
    None
}

/// Translate a stuff[] inventory index to an ob_id value.
/// Inverse of ob_id_to_stuff_index.
pub fn stuff_index_to_ob_id(stuff_idx: usize) -> Option<u8> {
    const INVERSE: &[(usize, u8)] = &[
        (0, 12),
        (1, 9),
        (2, 8),
        (3, 10),
        (4, 145),
        (5, 27),
        (6, 151),
        (7, 155),
        (8, 11),
        (9, 18),
        (10, 19),
        (11, 22),
        (12, 21),
        (13, 23),
        (14, 17),
        (15, 24),
        (16, 25),
        (17, 153),
        (18, 114),
        (19, 242),
        (20, 26),
        (21, 154),
        (22, 139),
        (23, 147),
        (24, 148),
        (25, 149),
        (26, 150),
        (27, 136),
        (28, 137),
        (29, 138),
        (30, 140),
    ];
    for &(si, oid) in INVERSE {
        if si == stuff_idx {
            return Some(oid);
        }
    }
    None
}

/// Display name for a stuff[] inventory slot (used by container loot messages).
/// Matches inv_list[].name ordering from fmain.c:428.
pub fn stuff_index_name(idx: usize) -> &'static str {
    const NAMES: [&str; 31] = [
        "Dirk",
        "Mace",
        "Sword",
        "Bow",
        "Magic Wand",
        "Golden Lasso",
        "Sea Shell",
        "Sun Stone",
        "Arrows",
        "Blue Stone",
        "Green Jewel",
        "Glass Vial",
        "Crystal Orb",
        "Bird Totem",
        "Gold Ring",
        "Jade Skull",
        "Gold Key",
        "Green Key",
        "Blue Key",
        "Red Key",
        "Grey Key",
        "White Key",
        "Talisman",
        "Rose",
        "Fruit",
        "Gold Statue",
        "Book",
        "Herb",
        "Writ",
        "Bone",
        "Shard",
    ];
    NAMES.get(idx).copied().unwrap_or("an unknown thing")
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
            (11, 35),  // QUIVER
            (18, 9),   // B_STONE
            (19, 10),  // G_JEWEL
            (22, 11),  // VIAL
            (21, 12),  // C_ORB
            (23, 13),  // B_TOTEM
            (17, 14),  // G_RING
            (24, 15),  // J_SKULL
            (145, 4),  // M_WAND
            (27, 5),   // Golden Lasso
            (8, 2),    // Sword
            (9, 1),    // Mace
            (12, 0),   // Dirk
            (10, 3),   // Bow
            (147, 23), // ROSE
            (148, 24), // FRUIT
            (149, 25), // STATUE
            (150, 26), // BOOK
            (151, 6),  // SHELL
            (155, 7),  // Sun Stone
            (136, 27), // Herb
            (137, 28), // Writ
            (138, 29), // Bone
            (139, 22), // Talisman
            (140, 30), // Shard
            (25, 16),  // GOLD_KEY
            (153, 17), // GREEN_KEY
            (114, 18), // BLUE_KEY
            (242, 19), // RED_KEY
            (26, 20),  // GREY_KEY
            (154, 21), // WHITE_KEY
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
    fn test_stuff_to_ob_id_roundtrip() {
        for si in 0..=30 {
            if let Some(ob_id) = stuff_index_to_ob_id(si) {
                let back = ob_id_to_stuff_index(ob_id);
                if si == 8 {
                    // QUIVER (ob_id 11) maps to stuff 35 (arrows ×10), not 8
                    assert_eq!(back, Some(35));
                } else {
                    assert_eq!(back, Some(si), "roundtrip failed for stuff_idx {}", si);
                }
            }
        }
    }

    #[test]
    fn test_stuff_index_name() {
        assert_eq!(super::stuff_index_name(0), "Dirk");
        assert_eq!(super::stuff_index_name(2), "Sword");
        assert_eq!(super::stuff_index_name(8), "Arrows");
        assert_eq!(super::stuff_index_name(16), "Gold Key");
        assert_eq!(super::stuff_index_name(30), "Shard");
        assert_eq!(super::stuff_index_name(99), "an unknown thing");
    }
}
