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

/// Compute 8-way compass direction (0=N..7=NW) from (sx,sy) toward (tx,ty).
/// Mirrors the direction LUT in set_course (fmain2.c): uses com2[] mapping
/// from (xsign, ysign) to compass direction.
/// Returns 9 if at same position (STILL).
fn direction_to_target(sx: i16, sy: i16, tx: i16, ty: i16) -> u8 {
    let dx = tx as i32 - sx as i32;
    let dy = ty as i32 - sy as i32;
    if dx == 0 && dy == 0 {
        return 9; // STILL
    }
    // Suppress minor axis if major > 2× minor (set_course mode-1 smart seek).
    let adx = dx.abs();
    let ady = dy.abs();
    let eff_dx = if ady > adx * 2 { 0 } else { dx };
    let eff_dy = if adx > ady * 2 { 0 } else { dy };
    let xi = (eff_dx.signum() + 1) as usize;
    let yi = (eff_dy.signum() + 1) as usize;
    const COM2: [[u8; 3]; 3] = [
        [7, 0, 1],   // dy: -1 (NW, N, NE)
        [6, 9, 2],   // dy:  0 (W, STILL, E)
        [5, 4, 3],   // dy: +1 (SW, S, SE)
    ];
    COM2[yi][xi]
}

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
    /// Uses 8-way direction toward hero + proxcheck collision + wall-sliding.
    /// Returns true if NPC is adjacent to hero (triggers encounter).
    ///
    /// `world`: terrain data for collision checks (None = always passable).
    /// `indoor`: true for indoor regions (region >= 8), affects Y wrapping.
    pub fn tick(
        &mut self,
        hero_x: i16,
        hero_y: i16,
        world: Option<&crate::game::world_data::WorldData>,
        indoor: bool,
    ) -> bool {
        use crate::game::collision::{proxcheck, newx, newy, calc_dist};

        if !self.active { return false; }

        let dx = hero_x as i32 - self.x as i32;
        let dy = hero_y as i32 - self.y as i32;

        // Only chase within 200px range (original proximity).
        if dx.abs() > 200 || dy.abs() > 200 {
            return false;
        }

        let dist = calc_dist(self.x as i32, self.y as i32, hero_x as i32, hero_y as i32);

        if dist > 0 {
            let facing = direction_to_target(self.x, self.y, hero_x, hero_y);
            if facing < 9 {
                // speed=0 NPCs are treated as speed=1 to avoid stalling.
                let speed = self.speed.max(1) as i32;
                let proposed_x = newx(self.x as u16, facing, speed);
                let proposed_y = newy(self.y as u16, facing, speed, indoor);

                // Race-specific terrain bypass: wraith (race 2) skips terrain checks.
                let terrain_passable = self.race == RACE_WRAITH
                    || proxcheck(world, proposed_x as i32, proposed_y as i32);

                if terrain_passable {
                    self.x = proposed_x as i16;
                    self.y = proposed_y as i16;
                } else {
                    // Wall-sliding: try clockwise then counter-clockwise deviation
                    // (fmain2.c set_course deviation).
                    let dev_cw = (facing + 1) & 7;
                    let cw_x = newx(self.x as u16, dev_cw, speed);
                    let cw_y = newy(self.y as u16, dev_cw, speed, indoor);
                    if proxcheck(world, cw_x as i32, cw_y as i32) {
                        self.x = cw_x as i16;
                        self.y = cw_y as i16;
                    } else {
                        let dev_ccw = (facing.wrapping_sub(1)) & 7;
                        let ccw_x = newx(self.x as u16, dev_ccw, speed);
                        let ccw_y = newy(self.y as u16, dev_ccw, speed, indoor);
                        if proxcheck(world, ccw_x as i32, ccw_y as i32) {
                            self.x = ccw_x as i16;
                            self.y = ccw_y as i16;
                        }
                        // Else: fully blocked, NPC stays put this frame.
                    }
                }
            }
        }

        // Adjacent check uses pre-movement distance (consistent with original
        // fmain.c where encounter detection is a separate pass from move_figure).
        dist < 16
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
        let _ = npc.tick(100, 0, None, false); // hero at x=100
        assert!(npc.x > 0); // should have moved toward hero
    }

    #[test]
    fn test_npc_direction_to_hero() {
        // Hero directly east of NPC → facing should be 2 (E)
        assert_eq!(direction_to_target(100, 100, 200, 100), 2);
        // Hero directly north → facing 0 (N)
        assert_eq!(direction_to_target(100, 100, 100, 50), 0);
        // Hero NE → facing 1 (NE)
        assert_eq!(direction_to_target(100, 100, 200, 50), 1);
        // Hero SW → facing 5 (SW)
        assert_eq!(direction_to_target(100, 100, 50, 200), 5);
    }

    #[test]
    fn test_npc_tick_moves_toward_hero_with_direction_lut() {
        let mut npc = Npc {
            npc_type: NPC_TYPE_ORC,
            race: RACE_ENEMY,
            x: 1000,
            y: 1000,
            vitality: 10,
            gold: 0,
            speed: 2,
            active: true,
        };
        let old_x = npc.x;
        // Hero is directly east at (1100, 1000).
        // With no WorldData, proxcheck always passes.
        let adjacent = npc.tick(1100, 1000, None, false);
        assert!(!adjacent);
        // NPC should have moved east (x increased).
        assert!(npc.x > old_x, "NPC should move east toward hero");
    }

    #[test]
    fn test_npc_defeat() {
        let mut npc = Npc { active: true, gold: 50, vitality: 10, ..Default::default() };
        let (gold, _) = npc.defeat();
        assert!(!npc.active);
        assert_eq!(gold, 50);
    }
}
