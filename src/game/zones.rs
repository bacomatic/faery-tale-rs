//! Encounter zones and extents: 23 trigger rectangles from the original.
//! Each zone can trigger encounters, carrier spawns, or special events.
//! Zone data lives in faery.toml; this module provides runtime helpers.

use crate::game::game_library::ZoneConfig;

/// Total zone entries in the original (indices 0–21 iterated; index 22 is fallback).
pub const EXT_COUNT: usize = 22;

/// Zone categories derived from etype at runtime.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ZoneType {
    /// Random encounter zone (etype 0–49). etype value is the danger modifier.
    Encounter,
    /// Forced/special encounter (etype 50–69): spiders, necromancer, astral, etc.
    Special,
    /// Carrier spawn point (etype 70–79): bird, turtle, dragon.
    Carrier,
    /// Peace/NPC zone (etype 80–89): palaces, villages, buildings — no random encounters.
    Peace,
}

impl ZoneType {
    /// Derive zone category from the raw etype value.
    pub fn from_etype(etype: u8) -> ZoneType {
        match etype {
            0..=49  => ZoneType::Encounter,
            50..=69 => ZoneType::Special,
            70..=79 => ZoneType::Carrier,
            _       => ZoneType::Peace,  // 80+
        }
    }
}

/// Check if a point is inside a zone using strict inequality (matching original).
/// The original uses `hero_x > x1 && hero_x < x2 && hero_y > y1 && hero_y < y2`.
pub fn zone_contains(z: &ZoneConfig, x: u16, y: u16) -> bool {
    x > z.x1 && x < z.x2 && y > z.y1 && y < z.y2
}

/// Find the first matching zone (indices 0..EXT_COUNT), or fall back to the last
/// entry (the "whole world" sentinel) if no specific zone matches.
/// Returns the index into the zones slice.
///
/// Mirrors fmain.c lines 3281–3287: iterate extent_list[0..EXT_COUNT], and if
/// nothing matches the extn pointer naturally falls through to extent_list[22].
pub fn find_zone(zones: &[ZoneConfig], x: u16, y: u16) -> Option<usize> {
    let scan_count = zones.len().min(EXT_COUNT);
    for i in 0..scan_count {
        if zone_contains(&zones[i], x, y) {
            return Some(i);
        }
    }
    // Fall through to sentinel (last entry) if it exists beyond the scan range.
    if zones.len() > EXT_COUNT {
        let sentinel = zones.len() - 1;
        if zone_contains(&zones[sentinel], x, y) {
            return Some(sentinel);
        }
    }
    None
}

/// Check if the position is in any random-encounter zone (etype < 50).
pub fn in_encounter_zone(zones: &[ZoneConfig], x: u16, y: u16) -> bool {
    if let Some(idx) = find_zone(zones, x, y) {
        ZoneType::from_etype(zones[idx].etype) == ZoneType::Encounter
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_zone(label: &str, etype: u8, x1: u16, y1: u16, x2: u16, y2: u16) -> ZoneConfig {
        ZoneConfig { label: label.to_string(), etype, x1, y1, x2, y2, v1: 0, v2: 0, v3: 0 }
    }

    #[test]
    fn zone_type_from_etype() {
        assert_eq!(ZoneType::from_etype(0), ZoneType::Encounter);
        assert_eq!(ZoneType::from_etype(3), ZoneType::Encounter);
        assert_eq!(ZoneType::from_etype(49), ZoneType::Encounter);
        assert_eq!(ZoneType::from_etype(50), ZoneType::Special);
        assert_eq!(ZoneType::from_etype(52), ZoneType::Special);
        assert_eq!(ZoneType::from_etype(60), ZoneType::Special);
        assert_eq!(ZoneType::from_etype(69), ZoneType::Special);
        assert_eq!(ZoneType::from_etype(70), ZoneType::Carrier);
        assert_eq!(ZoneType::from_etype(79), ZoneType::Carrier);
        assert_eq!(ZoneType::from_etype(80), ZoneType::Peace);
        assert_eq!(ZoneType::from_etype(83), ZoneType::Peace);
    }

    #[test]
    fn zone_contains_strict_inequality() {
        let z = make_zone("test", 3, 10, 20, 100, 200);
        // Interior point
        assert!(zone_contains(&z, 50, 100));
        // On boundary — strict inequality means these are NOT inside
        assert!(!zone_contains(&z, 10, 100)); // x == x1
        assert!(!zone_contains(&z, 100, 100)); // x == x2
        assert!(!zone_contains(&z, 50, 20)); // y == y1
        assert!(!zone_contains(&z, 50, 200)); // y == y2
        // Outside
        assert!(!zone_contains(&z, 5, 100));
        assert!(!zone_contains(&z, 50, 300));
    }

    #[test]
    fn zone_contains_inverted_y_coords() {
        // Zone 20 (around village) has y1=18719 > y2=17484.
        // With strict inequality, no point can satisfy y > 18719 && y < 17484.
        // This zone effectively never matches via coordinate check alone,
        // which matches original behavior (it's a metadata-only zone).
        let z = make_zone("around village", 3, 16953, 18719, 20240, 17484);
        assert!(!zone_contains(&z, 19000, 18000));
        assert!(!zone_contains(&z, 19000, 17000));
    }

    #[test]
    fn find_zone_returns_first_match() {
        let zones: Vec<ZoneConfig> = (0..23).map(|i| {
            if i == 3 {
                make_zone("spider pit", 53, 4063, 34819, 4909, 35125)
            } else if i == 22 {
                make_zone("whole world", 3, 0, 0, 32767, 40959)
            } else {
                make_zone("empty", 80, 0, 0, 0, 0)
            }
        }).collect();

        // Point inside spider pit
        assert_eq!(find_zone(&zones, 4500, 35000), Some(3));
        // Point not in any specific zone — falls through to sentinel
        assert_eq!(find_zone(&zones, 15000, 20000), Some(22));
    }

    #[test]
    fn find_zone_empty_list() {
        let zones: Vec<ZoneConfig> = vec![];
        assert_eq!(find_zone(&zones, 100, 100), None);
    }

    #[test]
    fn in_encounter_zone_checks_etype() {
        let zones: Vec<ZoneConfig> = (0..23).map(|i| {
            if i == 16 {
                // swamp region: etype=7 (< 50 = encounter zone)
                make_zone("swamp region", 7, 6156, 12755, 12316, 15905)
            } else if i == 12 {
                // peace zone: etype=80 (>= 80 = peace)
                make_zone("peace 1", 80, 2752, 33300, 8632, 35400)
            } else if i == 22 {
                // whole world fallback: etype=3 (< 50 = encounter)
                make_zone("whole world", 3, 0, 0, 32767, 40959)
            } else {
                make_zone("empty", 80, 0, 0, 0, 0)
            }
        }).collect();

        // In swamp region (etype=7 → Encounter)
        assert!(in_encounter_zone(&zones, 8000, 14000));
        // In peace zone (etype=80 → Peace)
        assert!(!in_encounter_zone(&zones, 5000, 34000));
        // Fallback to whole world (etype=3 → Encounter)
        assert!(in_encounter_zone(&zones, 15000, 20000));
    }
}
