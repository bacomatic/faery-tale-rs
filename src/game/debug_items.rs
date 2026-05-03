//! Central Item Name Reference for debug commands.
//!
//! Per DEBUG_SPECIFICATION.md §"Item Name Reference", this module provides
//! the single source of truth for item name ↔ id ↔ stuff_index resolution
//! used by `/give`, `/take`, `/items`, and `/inventory` debug commands.

/// Item reference entry: canonical name, stuff[] index, id, and aliases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemRef {
    /// The unique item ID (same as stuff_index for these items)
    pub id: u8,
    /// The index in the stuff[] array
    pub stuff_index: u8,
    /// Canonical item name from the spec
    pub name: &'static str,
    /// Alternate names/aliases accepted by the lookup
    pub aliases: &'static [&'static str],
}

/// Complete item reference table from DEBUG_SPECIFICATION.md §"Item Name Reference".
/// Order and canonical names match the spec exactly.
pub const ITEM_REFERENCE: &[ItemRef] = &[
    ItemRef {
        id: 0,
        stuff_index: 0,
        name: "dirk",
        aliases: &[],
    },
    ItemRef {
        id: 1,
        stuff_index: 1,
        name: "mace",
        aliases: &[],
    },
    ItemRef {
        id: 2,
        stuff_index: 2,
        name: "sword",
        aliases: &[],
    },
    ItemRef {
        id: 3,
        stuff_index: 3,
        name: "bow",
        aliases: &[],
    },
    ItemRef {
        id: 4,
        stuff_index: 4,
        name: "wand",
        aliases: &[],
    },
    ItemRef {
        id: 5,
        stuff_index: 5,
        name: "lasso",
        aliases: &[],
    },
    ItemRef {
        id: 6,
        stuff_index: 6,
        name: "shell",
        aliases: &[],
    },
    ItemRef {
        id: 7,
        stuff_index: 7,
        name: "sunstone",
        aliases: &["sun stone"],
    },
    ItemRef {
        id: 8,
        stuff_index: 8,
        name: "arrows",
        aliases: &["arrow"],
    },
    ItemRef {
        id: 9,
        stuff_index: 9,
        name: "blue_stone",
        aliases: &["blue stone", "bluestone"],
    },
    ItemRef {
        id: 10,
        stuff_index: 10,
        name: "green_jewel",
        aliases: &["jewel"],
    },
    ItemRef {
        id: 11,
        stuff_index: 11,
        name: "glass_vial",
        aliases: &["vial"],
    },
    ItemRef {
        id: 12,
        stuff_index: 12,
        name: "crystal_orb",
        aliases: &["orb"],
    },
    ItemRef {
        id: 13,
        stuff_index: 13,
        name: "bird_totem",
        aliases: &["totem"],
    },
    ItemRef {
        id: 14,
        stuff_index: 14,
        name: "gold_ring",
        aliases: &["ring"],
    },
    ItemRef {
        id: 15,
        stuff_index: 15,
        name: "jade_skull",
        aliases: &[],
    },
    ItemRef {
        id: 16,
        stuff_index: 16,
        name: "gold_key",
        aliases: &["key"],
    },
    ItemRef {
        id: 17,
        stuff_index: 17,
        name: "green_key",
        aliases: &[],
    },
    ItemRef {
        id: 18,
        stuff_index: 18,
        name: "blue_key",
        aliases: &[],
    },
    ItemRef {
        id: 19,
        stuff_index: 19,
        name: "red_key",
        aliases: &[],
    },
    ItemRef {
        id: 20,
        stuff_index: 20,
        name: "grey_key",
        aliases: &["gray key"],
    },
    ItemRef {
        id: 21,
        stuff_index: 21,
        name: "white_key",
        aliases: &[],
    },
    ItemRef {
        id: 22,
        stuff_index: 22,
        name: "talisman",
        aliases: &[],
    },
    ItemRef {
        id: 23,
        stuff_index: 23,
        name: "rose",
        aliases: &[],
    },
    ItemRef {
        id: 24,
        stuff_index: 24,
        name: "apple",
        aliases: &["fruit"],
    },
    ItemRef {
        id: 25,
        stuff_index: 25,
        name: "statue",
        aliases: &[],
    },
    ItemRef {
        id: 28,
        stuff_index: 28,
        name: "writ",
        aliases: &[],
    },
    ItemRef {
        id: 29,
        stuff_index: 29,
        name: "bone",
        aliases: &[],
    },
];

/// Look up an item by name (case-insensitive) or numeric ID string.
///
/// Matches against both the canonical name and any aliases.
/// Also accepts decimal ID strings like "22".
///
/// # Examples
/// ```
/// # use faery_tale::game::debug_items::lookup_by_name;
/// assert!(lookup_by_name("talisman").is_some());
/// assert!(lookup_by_name("TALISMAN").is_some());
/// assert!(lookup_by_name("22").is_some());
/// assert!(lookup_by_name("sun stone").is_some());
/// ```
pub fn lookup_by_name(s: &str) -> Option<&'static ItemRef> {
    let lower = s.trim().to_ascii_lowercase();
    if lower.is_empty() {
        return None;
    }

    // Try numeric ID first
    if let Ok(id) = lower.parse::<u8>() {
        return lookup_by_id(id);
    }

    // Search canonical name and aliases
    ITEM_REFERENCE.iter().find(|item| {
        item.name.eq_ignore_ascii_case(&lower)
            || item
                .aliases
                .iter()
                .any(|alias| alias.eq_ignore_ascii_case(&lower))
    })
}

/// Look up an item by its ID.
pub fn lookup_by_id(id: u8) -> Option<&'static ItemRef> {
    ITEM_REFERENCE.iter().find(|item| item.id == id)
}

/// Get the complete item reference table.
pub fn all() -> &'static [ItemRef] {
    ITEM_REFERENCE
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_length() {
        // Per DEBUG_SPECIFICATION.md, the table has 28 items
        assert_eq!(
            ITEM_REFERENCE.len(),
            28,
            "Item reference table should have 28 entries"
        );
    }

    #[test]
    fn test_unique_ids() {
        let mut seen_ids = std::collections::HashSet::new();
        for item in ITEM_REFERENCE {
            assert!(seen_ids.insert(item.id), "Duplicate item id: {}", item.id);
        }
    }

    #[test]
    fn test_unique_stuff_indices() {
        let mut seen_indices = std::collections::HashSet::new();
        for item in ITEM_REFERENCE {
            assert!(
                seen_indices.insert(item.stuff_index),
                "Duplicate stuff_index: {}",
                item.stuff_index
            );
        }
    }

    #[test]
    fn test_lookup_by_name_canonical() {
        let talisman = lookup_by_name("talisman");
        assert!(talisman.is_some());
        assert_eq!(talisman.unwrap().id, 22);
        assert_eq!(talisman.unwrap().name, "talisman");
    }

    #[test]
    fn test_lookup_case_insensitive() {
        assert_eq!(lookup_by_name("talisman").map(|i| i.id), Some(22));
        assert_eq!(lookup_by_name("Talisman").map(|i| i.id), Some(22));
        assert_eq!(lookup_by_name("TALISMAN").map(|i| i.id), Some(22));
        assert_eq!(lookup_by_name("TaLiSmAn").map(|i| i.id), Some(22));
    }

    #[test]
    fn test_lookup_by_numeric_string() {
        assert_eq!(lookup_by_name("22").map(|i| i.name), Some("talisman"));
        assert_eq!(lookup_by_name("0").map(|i| i.name), Some("dirk"));
        assert_eq!(lookup_by_name("29").map(|i| i.name), Some("bone"));
        assert!(lookup_by_name("99").is_none());
    }

    #[test]
    fn test_lookup_by_id() {
        let dirk = lookup_by_id(0);
        assert!(dirk.is_some());
        assert_eq!(dirk.unwrap().name, "dirk");

        let talisman = lookup_by_id(22);
        assert!(talisman.is_some());
        assert_eq!(talisman.unwrap().name, "talisman");

        assert!(lookup_by_id(26).is_none()); // gap in table
        assert!(lookup_by_id(99).is_none());
    }

    #[test]
    fn test_all() {
        let items = all();
        assert_eq!(items.len(), 28);
        assert_eq!(items[0].name, "dirk");
        assert_eq!(items[27].name, "bone");
    }

    #[test]
    fn test_aliases() {
        // Test spec-documented aliases
        assert_eq!(lookup_by_name("sun stone").map(|i| i.id), Some(7));
        assert_eq!(lookup_by_name("arrow").map(|i| i.id), Some(8));
        assert_eq!(lookup_by_name("blue stone").map(|i| i.id), Some(9));
        assert_eq!(lookup_by_name("jewel").map(|i| i.id), Some(10));
        assert_eq!(lookup_by_name("vial").map(|i| i.id), Some(11));
        assert_eq!(lookup_by_name("orb").map(|i| i.id), Some(12));
        assert_eq!(lookup_by_name("totem").map(|i| i.id), Some(13));
        assert_eq!(lookup_by_name("ring").map(|i| i.id), Some(14));
        assert_eq!(lookup_by_name("key").map(|i| i.id), Some(16));
        assert_eq!(lookup_by_name("gray key").map(|i| i.id), Some(20));
        assert_eq!(lookup_by_name("fruit").map(|i| i.id), Some(24));
    }

    #[test]
    fn test_whitespace_handling() {
        assert_eq!(lookup_by_name("  talisman  ").map(|i| i.id), Some(22));
        assert_eq!(lookup_by_name("\ttalisman\n").map(|i| i.id), Some(22));
    }
}
