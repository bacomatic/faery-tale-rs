//! Encounter zones and extents: 22 trigger rectangles from the original.
//! Each zone can trigger encounters, carrier spawns, or special events.

/// Maximum number of trigger zones.
pub const ZONE_COUNT: usize = 22;

/// Zone trigger types.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ZoneType {
    None,
    Encounter,  // random enemy encounter
    Carrier,    // carrier (swan, horse, etc.) spawn point
    Special,    // scripted event (palace, tower, etc.)
}

/// A rectangular trigger zone.
#[derive(Debug, Clone, Copy)]
pub struct Zone {
    pub zone_type: ZoneType,
    pub x1: u16,
    pub y1: u16,
    pub x2: u16,
    pub y2: u16,
    pub region: u8,        // 0xFF = any region
    pub encounter_rate: u8, // 0-255, higher = more frequent
}

impl Zone {
    pub const ANY_REGION: u8 = 0xFF;

    pub fn contains(&self, region: u8, x: u16, y: u16) -> bool {
        (self.region == Self::ANY_REGION || self.region == region)
            && x >= self.x1 && x <= self.x2
            && y >= self.y1 && y <= self.y2
    }
}

/// Placeholder zone table. TODO: fill with real zone coordinates from ADF analysis.
pub static ZONE_TABLE: &[Zone] = &[
    Zone {
        zone_type: ZoneType::None,
        x1: 0, y1: 0, x2: 0, y2: 0,
        region: Zone::ANY_REGION,
        encounter_rate: 0,
    },
];

/// Find all zones containing the given position.
pub fn zones_at(region: u8, x: u16, y: u16) -> impl Iterator<Item = &'static Zone> {
    ZONE_TABLE.iter().filter(move |z| z.contains(region, x, y))
}

/// Check if the position is in any encounter zone.
pub fn in_encounter_zone(region: u8, x: u16, y: u16) -> bool {
    zones_at(region, x, y).any(|z| z.zone_type == ZoneType::Encounter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zones_at_empty() {
        let count = zones_at(0, 100, 100).count();
        // placeholder zone at 0,0 with size 0 won't match 100,100
        assert_eq!(count, 0);
    }

    #[test]
    fn test_zone_contains() {
        let z = Zone {
            zone_type: ZoneType::Encounter,
            x1: 10, y1: 10, x2: 100, y2: 100,
            region: 0,
            encounter_rate: 128,
        };
        assert!(z.contains(0, 50, 50));
        assert!(!z.contains(0, 5, 5));
        assert!(!z.contains(1, 50, 50));
    }
}
