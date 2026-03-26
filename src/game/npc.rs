//! NPC data loading and actor initialization.
//! Ports the carrier/enemy table loading from fmain.c hdrive.c.

use crate::game::adf::AdfDisk;

/// Maximum number of NPCs in any region (original limit).
pub const MAX_NPCS: usize = 16;

/// NPC type codes from cfile (carrier table).
pub const NPC_TYPE_NONE: u8 = 0;
pub const NPC_TYPE_HUMAN: u8 = 1;
pub const NPC_TYPE_SWAN: u8 = 2;
pub const NPC_TYPE_HORSE: u8 = 3;
pub const NPC_TYPE_DRAGON: u8 = 4;
pub const NPC_TYPE_GHOST: u8 = 5;
pub const NPC_TYPE_ORC: u8 = 6;
pub const NPC_TYPE_WRAITH: u8 = 7;
pub const NPC_TYPE_SKELETON: u8 = 8;
/// Container NPC type (chest/barrel — drops items on defeat).
pub const NPC_TYPE_CONTAINER: u8 = 0x80;
/// Raft carrier NPC type (player-107).
pub const NPC_TYPE_RAFT: u8 = 9;

/// NPC race/behavior codes.
pub const RACE_NORMAL: u8 = 0;
pub const RACE_UNDEAD: u8 = 1;
pub const RACE_WRAITH: u8 = 2;
pub const RACE_ENEMY: u8 = 3;
pub const RACE_SNAKE: u8 = 4;
/// Shopkeeper race code.
pub const RACE_SHOPKEEPER: u8 = 0x88;
/// Beggar race code (from fmain.c do_option GIVE case: race 0x8d).
pub const RACE_BEGGAR: u8 = 0x8d;

/// An NPC/actor record.
#[derive(Debug, Clone, Default)]
pub struct Npc {
    pub npc_type: u8,
    pub race: u8,
    pub x: i16,
    pub y: i16,
    pub vitality: i16,
    pub gold: i16,
    pub speed: u8,
    pub active: bool,
}

impl Npc {
    /// Parse one NPC record from a 16-byte block.
    /// Original cfile format: type(1), race(1), x(2), y(2), vit(2), gold(2), speed(1), pad(5)
    pub fn from_bytes(data: &[u8]) -> Self {
        if data.len() < 16 {
            return Npc::default();
        }
        Npc {
            npc_type: data[0],
            race: data[1],
            x: i16::from_be_bytes([data[2], data[3]]),
            y: i16::from_be_bytes([data[4], data[5]]),
            vitality: i16::from_be_bytes([data[6], data[7]]),
            gold: i16::from_be_bytes([data[8], data[9]]),
            speed: data[10],
            active: data[0] != NPC_TYPE_NONE,
        }
    }

    /// Update NPC position for one frame tick.
    /// Returns true if NPC is now adjacent to hero (triggers encounter).
    pub fn tick(&mut self, hero_x: i16, hero_y: i16) -> bool {
        if !self.active { return false; }

        let dx = hero_x as i32 - self.x as i32;
        let dy = hero_y as i32 - self.y as i32;

        let speed = self.speed as i16;
        if dx.abs() > 200 || dy.abs() > 200 {
            return false;
        }
        let dist_sq = dx * dx + dy * dy;
        if dist_sq < 200 * 200 {
            if dx.abs() > dy.abs() {
                self.x += (dx.signum() as i16) * speed;
            } else {
                self.y += (dy.signum() as i16) * speed;
            }
        }

        dist_sq < 16 * 16
    }

    /// Defeat this NPC (on encounter win).
    pub fn defeat(&mut self) -> (i16, i16) {
        self.active = false;
        let loot = (self.gold, self.vitality);
        self.vitality = 0;
        loot
    }
}

/// Load NPC records for a region from ADF block data.
///
/// The carrier file (cfile) begins at ADF block 888 (placeholder — verify from hdrive.c).
/// Each region has up to MAX_NPCS records of 16 bytes each = 256 bytes per region.
pub const CFILE_START_BLOCK: usize = 888;

pub struct NpcTable {
    pub npcs: [Npc; MAX_NPCS],
}

impl NpcTable {
    pub fn load(adf: &AdfDisk, region_num: u8) -> Self {
        let region_offset = region_num as usize * MAX_NPCS * 16;
        let block = CFILE_START_BLOCK + region_offset / 512;
        let data = adf.load_blocks(block as u32, 1);
        let start = region_offset % 512;
        let mut npcs: [Npc; MAX_NPCS] = Default::default();
        for i in 0..MAX_NPCS {
            let off = start + i * 16;
            if off + 16 <= data.len() {
                npcs[i] = Npc::from_bytes(&data[off..off + 16]);
            }
        }
        NpcTable { npcs }
    }

    /// Count of active NPCs.
    pub fn active_count(&self) -> usize {
        self.npcs.iter().filter(|n| n.active).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::adf::AdfDisk;

    #[test]
    fn test_npc_from_zero_bytes() {
        let data = [0u8; 16];
        let npc = Npc::from_bytes(&data);
        assert!(!npc.active);
        assert_eq!(npc.npc_type, 0);
    }

    #[test]
    fn test_npc_table_load() {
        let adf = AdfDisk::from_bytes(vec![0u8; 2048 * 512]);
        let table = NpcTable::load(&adf, 0);
        assert_eq!(table.active_count(), 0);
    }

    #[test]
    fn test_npc_from_bytes_human() {
        let mut data = [0u8; 16];
        data[0] = NPC_TYPE_HUMAN; // type
        data[1] = RACE_NORMAL;    // race
        data[2] = 0x01; data[3] = 0x00; // x = 256
        let npc = Npc::from_bytes(&data);
        assert!(npc.active);
        assert_eq!(npc.npc_type, NPC_TYPE_HUMAN);
        assert_eq!(npc.x, 256);
    }

    #[test]
    fn test_npc_tick_chase() {
        let mut npc = Npc {
            npc_type: NPC_TYPE_ORC,
            race: RACE_ENEMY,
            x: 0, y: 0,
            vitality: 10,
            gold: 5,
            speed: 2,
            active: true,
        };
        let _ = npc.tick(100, 0); // hero at x=100
        assert!(npc.x > 0); // should have moved toward hero
    }

    #[test]
    fn test_npc_defeat() {
        let mut npc = Npc { active: true, gold: 50, vitality: 10, ..Default::default() };
        let (gold, _) = npc.defeat();
        assert!(!npc.active);
        assert_eq!(gold, 50);
    }
}
