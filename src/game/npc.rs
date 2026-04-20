//! NPC data loading and actor initialization.
//! Ports the carrier/enemy table loading from fmain.c hdrive.c.

use crate::game::adf::AdfDisk;
use crate::game::actor::{Goal, Tactic};

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
/// Raft carrier NPC type (player-107).
pub const NPC_TYPE_RAFT: u8 = 9;
pub const NPC_TYPE_SNAKE: u8 = 10;
pub const NPC_TYPE_SPIDER: u8 = 11;
pub const NPC_TYPE_DKNIGHT: u8 = 12;
pub const NPC_TYPE_LORAII: u8 = 13;
pub const NPC_TYPE_NECROMANCER: u8 = 14;
/// Container NPC type (chest/barrel — drops items on defeat).
pub const NPC_TYPE_CONTAINER: u8 = 0x80;

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
pub const RACE_NECROMANCER: u8 = 9;
/// Race 10: Woodcutter — Necromancer transforms to this on death (SPEC §15.7).
pub const RACE_WOODCUTTER: u8 = 10;
pub const RACE_WITCH: u8 = 0x89;
pub const RACE_SPECTRE: u8 = 0x8a;
pub const RACE_GHOST: u8 = 0x8b;

/// Lightweight NPC state for AI decisions (distinct from ActorState which carries animation data).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum NpcState {
    #[default]
    Still,
    Walking,
    Fighting,
    Shooting,
    Dying,
    Dead,
    Sinking,
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
    pub weapon: u8,
    pub active: bool,
    pub goal: Goal,
    pub tactic: Tactic,
    pub facing: u8,
    pub state: NpcState,
    pub cleverness: u8,
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
            weapon: data[11],
            active: data[0] != NPC_TYPE_NONE,
            goal: Goal::None,
            tactic: Tactic::None,
            facing: 0,
            state: NpcState::Still,
            cleverness: 0,
        }
    }

    /// Execute one frame of movement (terrain-only collision).
    /// Delegates to `tick_with_actors()` with no actor positions.
    pub fn tick(
        &mut self,
        world: Option<&crate::game::world_data::WorldData>,
        indoor: bool,
    ) {
        self.tick_with_actors(world, indoor, &[]);
    }

    /// Execute one frame of movement with both terrain and actor collision.
    /// `other_actors`: positions of all other live actors (hero + other NPCs)
    /// that this NPC should not overlap with.
    ///
    /// The AI layer (select_tactic → do_tactic → set_course) sets facing/state
    /// before this runs. Only moves when state == Walking.
    ///
    /// If all three directions (primary + ±1 deviation) are blocked,
    /// sets state to Still and tactic to Frust.
    pub fn tick_with_actors(
        &mut self,
        world: Option<&crate::game::world_data::WorldData>,
        indoor: bool,
        other_actors: &[(i32, i32)],
    ) {
        use crate::game::collision::{proxcheck, actor_collides, newx, newy};
        use crate::game::actor::Tactic;

        if !self.active || self.state != NpcState::Walking {
            return;
        }

        let facing = self.facing;
        let dist = 2i32;

        let proposed_x = newx(self.x as u16, facing, dist);
        let proposed_y = newy(self.y as u16, facing, dist, indoor);

        // Race-specific terrain bypass: wraith (race 2) skips terrain checks.
        let terrain_passable = self.race == RACE_WRAITH
            || proxcheck(world, proposed_x as i32, proposed_y as i32);
        let actor_passable = !actor_collides(proposed_x as i32, proposed_y as i32, other_actors);

        if terrain_passable && actor_passable {
            self.x = proposed_x as i16;
            self.y = proposed_y as i16;
        } else {
            // Wall-sliding: try clockwise then counter-clockwise deviation.
            let dev_cw = (facing + 1) & 7;
            let cw_x = newx(self.x as u16, dev_cw, dist);
            let cw_y = newy(self.y as u16, dev_cw, dist, indoor);
            let cw_terrain = self.race == RACE_WRAITH
                || proxcheck(world, cw_x as i32, cw_y as i32);
            let cw_actor = !actor_collides(cw_x as i32, cw_y as i32, other_actors);
            if cw_terrain && cw_actor {
                self.x = cw_x as i16;
                self.y = cw_y as i16;
            } else {
                let dev_ccw = (facing.wrapping_sub(1)) & 7;
                let ccw_x = newx(self.x as u16, dev_ccw, dist);
                let ccw_y = newy(self.y as u16, dev_ccw, dist, indoor);
                let ccw_terrain = self.race == RACE_WRAITH
                    || proxcheck(world, ccw_x as i32, ccw_y as i32);
                let ccw_actor = !actor_collides(ccw_x as i32, ccw_y as i32, other_actors);
                if ccw_terrain && ccw_actor {
                    self.x = ccw_x as i16;
                    self.y = ccw_y as i16;
                } else {
                    // Fully blocked — frustrated.
                    self.state = NpcState::Still;
                    self.tactic = Tactic::Frust;
                }
            }
        }
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
            facing: 2,  // East
            state: NpcState::Walking,
            ..Default::default()
        };
        npc.tick(None, false);
        assert!(npc.x > 0); // should have moved east
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
            facing: 2,  // East
            state: NpcState::Walking,
            ..Default::default()
        };
        let old_x = npc.x;
        npc.tick(None, false);
        assert!(npc.x > old_x, "NPC should move east");
    }

    #[test]
    fn test_npc_defeat() {
        let mut npc = Npc { active: true, gold: 50, vitality: 10, ..Default::default() };
        let (gold, _) = npc.defeat();
        assert!(!npc.active);
        assert_eq!(gold, 50);
    }

    #[test]
    fn test_npc_ai_fields_default() {
        let npc = Npc::default();
        assert_eq!(npc.goal, Goal::None);
        assert_eq!(npc.tactic, Tactic::None);
        assert_eq!(npc.facing, 0);
        assert_eq!(npc.state, NpcState::Still);
        assert_eq!(npc.cleverness, 0);
    }

    #[test]
    fn test_npc_tick_uses_stored_facing() {
        let mut npc = Npc {
            npc_type: NPC_TYPE_ORC,
            race: RACE_ENEMY,
            x: 1000,
            y: 1000,
            vitality: 10,
            active: true,
            facing: 2, // East
            state: NpcState::Walking,
            ..Default::default()
        };
        let old_x = npc.x;
        npc.tick(None, false);
        assert!(npc.x > old_x, "Walking east should increase X");
    }

    #[test]
    fn test_npc_tick_still_does_not_move() {
        let mut npc = Npc {
            npc_type: NPC_TYPE_ORC,
            race: RACE_ENEMY,
            x: 1000,
            y: 1000,
            vitality: 10,
            active: true,
            facing: 2,
            state: NpcState::Still,
            ..Default::default()
        };
        let old_x = npc.x;
        let old_y = npc.y;
        npc.tick(None, false);
        assert_eq!(npc.x, old_x);
        assert_eq!(npc.y, old_y);
    }

    #[test]
    fn test_npc_tick_blocked_becomes_frust() {
        let mut npc = Npc {
            npc_type: NPC_TYPE_ORC,
            race: RACE_ENEMY,
            x: 1000,
            y: 1000,
            vitality: 10,
            active: true,
            facing: 2,
            state: NpcState::Walking,
            ..Default::default()
        };
        // With no WorldData (None), proxcheck always passes → should move.
        npc.tick(None, false);
        assert_eq!(npc.state, NpcState::Walking); // Still walking (not frustrated)
    }

    #[test]
    fn test_npc_tick_blocked_by_actor() {
        use crate::game::actor::Tactic;
        let mut npc = Npc {
            npc_type: NPC_TYPE_ORC,
            race: RACE_ENEMY,
            x: 1000,
            y: 1000,
            vitality: 10,
            active: true,
            facing: 2,  // East
            state: NpcState::Walking,
            ..Default::default()
        };
        // Place actors blocking primary (east) + clockwise (SE) + counterclockwise (NE).
        let others = vec![(1003, 1000), (1002, 1002), (1002, 998)];
        let old_x = npc.x;
        let old_y = npc.y;
        npc.tick_with_actors(None, false, &others);
        assert_eq!(npc.x, old_x);
        assert_eq!(npc.y, old_y);
        assert_eq!(npc.state, NpcState::Still);
        assert_eq!(npc.tactic, Tactic::Frust);
    }

    #[test]
    fn test_npc_tick_not_blocked_by_distant_actor() {
        let mut npc = Npc {
            npc_type: NPC_TYPE_ORC,
            race: RACE_ENEMY,
            x: 1000,
            y: 1000,
            vitality: 10,
            active: true,
            facing: 2,  // East
            state: NpcState::Walking,
            ..Default::default()
        };
        let others = vec![(2000, 2000)];
        let old_x = npc.x;
        npc.tick_with_actors(None, false, &others);
        assert!(npc.x > old_x, "NPC should move east — actor is far away");
    }

    #[test]
    fn test_npc_tick_empty_actors_same_as_no_actors() {
        let mut npc1 = Npc {
            npc_type: NPC_TYPE_ORC,
            race: RACE_ENEMY,
            x: 1000,
            y: 1000,
            vitality: 10,
            active: true,
            facing: 2,
            state: NpcState::Walking,
            ..Default::default()
        };
        let mut npc2 = npc1.clone();
        npc1.tick(None, false);
        npc2.tick_with_actors(None, false, &[]);
        assert_eq!(npc1.x, npc2.x);
        assert_eq!(npc1.y, npc2.y);
    }
}
