//! Debug command dispatch (DebugCommand handler).
//! See `docs/DEBUG_SPECIFICATION.md` for specification.

use super::*;

impl GameplayScene {
    pub fn apply_command(&mut self, cmd: DebugCommand) {
        use DebugCommand::*;
        match cmd {
            SetStat { stat, value } => {
                let field = Self::stat_field_mut(&mut self.state, stat);
                *field = value;
            }
            AdjustStat { stat, delta } => {
                let field = Self::stat_field_mut(&mut self.state, stat);
                *field = field.saturating_add(delta);
            }
            SetInventory { index, value } => {
                let stuff = self.state.stuff_mut();
                if (index as usize) < stuff.len() {
                    stuff[index as usize] = value;
                }
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            AdjustInventory { index, delta } => {
                let stuff = self.state.stuff_mut();
                if (index as usize) < stuff.len() {
                    stuff[index as usize] = stuff[index as usize].saturating_add_signed(delta);
                }
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            TeleportSafe => {
                self.state.hero_x = self.state.safe_x;
                self.state.hero_y = self.state.safe_y;
                self.snap_camera_to_hero();
            }
            TeleportCoords { x, y } => {
                self.state.hero_x = x;
                self.state.hero_y = y;
                self.snap_camera_to_hero();
            }
            TeleportStoneRing { index } => {
                self.dlog(format!(
                    "debug command not yet wired: TeleportStoneRing {{ index: {} }}",
                    index
                ));
            }
            ToggleMagicEffect { effect } => match effect {
                MagicEffect::Light => self.state.light_sticky = !self.state.light_sticky,
                MagicEffect::Secret => self.state.secret_sticky = !self.state.secret_sticky,
                MagicEffect::Freeze => self.state.freeze_sticky = !self.state.freeze_sticky,
            },
            SetGodMode { flags } => {
                self.state.god_mode = flags;
            }
            SetDayPhase { phase } => {
                self.state.daynight = phase;
                let raw = self.state.daynight / 40;
                self.state.lightlevel = if raw >= 300 { 600 - raw } else { raw };
                self.state.dayperiod =
                    crate::game::game_state::dayperiod_from_daynight(self.state.daynight);
            }
            SetGameTime { hour, minute } => {
                // Each hour = 1000 daynight ticks; each minute ≈ 1000/60
                let ticks =
                    (hour as u16).saturating_mul(1000) + (minute as u16).saturating_mul(1000) / 60;
                self.state.daynight = ticks % 24000;
                let raw = self.state.daynight / 40;
                self.state.lightlevel = if raw >= 300 { 600 - raw } else { raw };
                self.state.dayperiod =
                    crate::game::game_state::dayperiod_from_daynight(self.state.daynight);
            }
            HoldTimeOfDay { hold } => {
                self.state.freeze_sticky = hold;
            }
            TriggerWitchEffect => {
                self.witch_effect.start();
            }
            TriggerTeleportEffect => {
                self.teleport_effect.start();
            }
            TriggerPaletteTransition { to_black } => {
                self.dlog(format!("TriggerPaletteTransition: to_black={}", to_black));
            }
            InstaKill => {
                let mut killed = 0usize;
                for actor in self.state.actors.iter_mut().skip(1) {
                    if matches!(actor.kind, ActorKind::Enemy | ActorKind::Dragon)
                        && !matches!(actor.state, ActorState::Dead | ActorState::Dying)
                    {
                        actor.vitality = 0;
                        actor.state = ActorState::Dying;
                        killed += 1;
                    }
                }
                self.dlog(format!("InstaKill: killed {} enemies", killed));
            }
            HeroPack => {
                // Fill a sensible selection: full weapon set, all magic, all keys, arrows
                let stuff = self.state.stuff_mut();
                // Weapons: dirk(0), mace(1), sword(2), bow(3), magic wand(4), golden lasso(5)
                for i in 0..=5 {
                    stuff[i] = 1;
                }
                // Arrows: slot 8
                stuff[8] = 99;
                // Magic items: slots 9-15
                for i in 9..=15 {
                    stuff[i] = 1;
                }
                // Keys: slots 16-21
                for i in 16..=21 {
                    stuff[i] = 1;
                }
                self.dlog("HeroPack: weapons, magic, and keys filled".to_string());
                let wealth = self.state.wealth;
                self.menu.set_options(self.state.stuff(), wealth);
            }
            SummonSwan => {
                use crate::game::actor::{Goal, Tactic};
                use crate::game::npc::*;
                let hero_x = self.state.hero_x as i16;
                let hero_y = self.state.hero_y as i16;
                if let Some(ref mut table) = self.npc_table {
                    if let Some(slot) = table.npcs.iter_mut().find(|n| n.slot_free()) {
                        *slot = Npc {
                            npc_type: NPC_TYPE_SWAN,
                            race: RACE_NORMAL,
                            x: hero_x + 48,
                            y: hero_y,
                            vitality: 70,
                            gold: 0,
                            speed: 1,
                            weapon: 0,
                            active: true,
                            goal: Goal::None,
                            tactic: Tactic::None,
                            facing: 0,
                            state: NpcState::Still,
                            cleverness: 0,
                            looted: false,
                        };
                        self.dlog(
                            "summoned swan near hero (grounded, requires Golden Lasso to mount)"
                                .to_string(),
                        );
                    } else {
                        self.dlog("summon swan: no free NPC slots".to_string());
                    }
                } else {
                    self.dlog("summon swan: no npc_table loaded".to_string());
                }
            }
            RestartAsBrother { brother } => {
                let b = match brother {
                    BrotherId::Julian => 1u8,
                    BrotherId::Phillip => 2,
                    BrotherId::Kevin => 3,
                };
                self.state.brother = b;
                self.update_brother_substitution();
                self.dlog(format!("RestartAsBrother: switched to brother {}", b));
            }
            QueryTerrain => {
                let x = self.state.hero_x as i32;
                let y = self.state.hero_y as i32;
                let lines: Vec<String> = match &self.map_world {
                    None => vec![
                        format!("terrain: hero=({}, {})", x, y),
                        "terrain: map_world not loaded".to_string(),
                    ],
                    Some(world) => {
                        let terra_head = format!(
                            "terrain: terra_mem[0..16] = {:02x?}",
                            &world.terra_mem[..16]
                        );
                        let probes: Vec<String> = [
                            ("right_foot", x + 4, y + 2),
                            ("left_foot",  x - 4, y + 2),
                        ].iter().map(|&(label, px, py)| {
                            let p = collision::terrain_probe(world, px, py);
                            format!(
                                "terrain: {}  pos=({},{})  d4=0x{:02x}  imx={} imy={}  xs={} ys={}  map[{}]=sec{}  sec_off={}  tile_idx={}  terra=[{:02x},{:02x},{:02x},{:02x}]  tiles&d4=0x{:02x}  type={}",
                                label, p.x, p.y, p.d4, p.imx, p.imy,
                                p.xs, p.ys, p.map_offset, p.sec_num,
                                p.sector_offset, p.tile_idx,
                                p.terra_bytes[0], p.terra_bytes[1],
                                p.terra_bytes[2], p.terra_bytes[3],
                                p.tiles_and_d4, p.terrain_type,
                            )
                        }).collect();
                        std::iter::once(format!("terrain: hero=({}, {})", x, y))
                            .chain(std::iter::once(terra_head))
                            .chain(probes)
                            .collect()
                    }
                };
                for line in lines {
                    self.dlog(line);
                }
            }
            QueryActors => {
                let count = self.state.actors.len();
                let lines: Vec<String> = std::iter::once(format!("Actors: {} total", count))
                    .chain(self.state.actors.iter().enumerate().map(|(i, actor)| {
                        format!(
                            "  [{:2}] {:?} race={} vit={} @({},{}) {:?}",
                            i,
                            actor.kind,
                            actor.race,
                            actor.vitality,
                            actor.abs_x,
                            actor.abs_y,
                            actor.state
                        )
                    }))
                    .collect();
                for line in lines {
                    self.dlog(line);
                }
                let npc_lines: Vec<String> = if let Some(ref table) = self.npc_table {
                    let mut v = vec![format!("NpcTable ({} slots):", table.npcs.len())];
                    for (i, npc) in table.npcs.iter().enumerate() {
                        if npc.npc_type != crate::game::npc::NPC_TYPE_NONE {
                            v.push(format!(
                                "  [npc{:2}] type={} race={} vit={} @({},{}) {:?} goal:{:?} tac:{:?}",
                                i, npc.npc_type, npc.race, npc.vitality,
                                npc.x, npc.y, npc.state, npc.goal, npc.tactic
                            ));
                        }
                    }
                    v
                } else {
                    vec![]
                };
                for line in npc_lines {
                    self.dlog(line);
                }
            }
            QuerySongs => {
                self.dlog("QuerySongs: song library info is in main loop; use /songs".to_string());
            }
            DumpAdfBlock { block, count } => match &self.adf {
                None => self.dlog("DumpAdfBlock: ADF not loaded".to_string()),
                Some(adf) => {
                    let total = adf.num_blocks() as u32;
                    let end = block + count;
                    if end > total {
                        self.dlog(format!(
                            "DumpAdfBlock: range [{}, {}) exceeds ADF size ({} blocks)",
                            block, end, total
                        ));
                    } else {
                        let data = adf.load_blocks(block, count).to_vec();
                        self.dlog(format!(
                            "ADF block(s) {}..{} ({} bytes):",
                            block,
                            end,
                            data.len()
                        ));
                        for (row_i, chunk) in data.chunks(16).enumerate() {
                            let offset = block as usize * 512 + row_i * 16;
                            let hex: String = chunk
                                .iter()
                                .map(|b| format!("{:02X}", b))
                                .collect::<Vec<_>>()
                                .join(" ");
                            let ascii: String = chunk
                                .iter()
                                .map(|&b| {
                                    if b >= 0x20 && b < 0x7F {
                                        b as char
                                    } else {
                                        '.'
                                    }
                                })
                                .collect();
                            self.dlog(format!("{:06X}: {}  {}", offset, hex, ascii));
                        }
                    }
                }
            },
            SpawnEncounterRandom => {
                let zone_idx = self.state.region_num as usize;
                let hero_x = self.state.hero_x as i16;
                let hero_y = self.state.hero_y as i16;
                if let Some(ref mut table) = self.npc_table {
                    let spawned = crate::game::encounter::spawn_encounter_group(
                        table,
                        zone_idx,
                        hero_x,
                        hero_y,
                        self.state.tick_counter,
                    );
                    self.dlog(format!("forced encounter: {} enemies", spawned));
                } else {
                    self.dlog("forced encounter: no npc_table loaded".to_string());
                }
            }
            SpawnEncounterType(npc_type) => {
                use crate::game::npc::*;
                let zone_idx = self.state.region_num as usize;
                let hero_x = self.state.hero_x as i16;
                let hero_y = self.state.hero_y as i16;
                let requested_type = npc_type;
                if let Some(ref mut table) = self.npc_table {
                    if let Some(slot) = table.npcs.iter_mut().find(|n| n.slot_free()) {
                        let mut npc = crate::game::encounter::spawn_encounter(
                            zone_idx,
                            hero_x + 48,
                            hero_y,
                            self.state.tick_counter,
                        );
                        npc.npc_type = requested_type;
                        npc.race = match requested_type {
                            NPC_TYPE_WRAITH => RACE_WRAITH,
                            NPC_TYPE_GHOST | NPC_TYPE_SKELETON => RACE_UNDEAD,
                            _ => RACE_ENEMY,
                        };
                        *slot = npc;
                        self.dlog(format!("spawned enemy type={}", requested_type));
                    } else {
                        self.dlog("spawn enemy: no free NPC slots".to_string());
                    }
                } else {
                    self.dlog("spawn enemy: no npc_table loaded".to_string());
                }
            }
            ClearEncounters => {
                if let Some(ref mut table) = self.npc_table {
                    let n = table.active_count();
                    for npc in table.npcs.iter_mut() {
                        npc.active = false;
                    }
                    self.dlog(format!("cleared {} NPCs", n));
                } else {
                    self.dlog("clear encounters: no npc_table loaded".to_string());
                }
            }
            ScatterItems { count, item_id } => {
                use crate::game::game_state::WorldObject;
                use crate::game::sprites::INV_LIST;
                use crate::game::world_objects::stuff_index_to_ob_id;
                const TALISMAN_IDX: usize = 22;

                if count == 0 {
                    self.dlog("scattered 0 items".to_string());
                    return;
                }

                let region = self.state.region_num;
                let hero_x = self.state.hero_x as i32;
                let hero_y = self.state.hero_y as i32;
                let mut dropped = 0usize;

                if let Some(id) = item_id {
                    // Drop `count` copies of one specific item in a ring.
                    let radius = if count == 1 { 16.0f32 } else { 80.0f32 };
                    for i in 0..count {
                        let angle = if count == 1 {
                            0.0f32
                        } else {
                            2.0 * std::f32::consts::PI * (i as f32) / (count as f32)
                        };
                        let x = (hero_x + (radius * angle.cos()) as i32).clamp(0, 0x7FFF) as u16;
                        let y = (hero_y + (radius * angle.sin()) as i32).clamp(0, 0x7FFF) as u16;
                        let ob_id_val = stuff_index_to_ob_id(id).unwrap_or(id as u8);
                        self.state.world_objects.push(WorldObject {
                            ob_id: ob_id_val,
                            ob_stat: 1,
                            region,
                            x,
                            y,
                            visible: true,
                            goal: 0,
                        });
                        dropped += 1;
                    }
                    self.dlog(format!(
                        "scattered {} x item {} ({})",
                        dropped,
                        id,
                        if id == TALISMAN_IDX {
                            "TALISMAN — end-of-game item"
                        } else {
                            ""
                        }
                    ));
                } else {
                    // Drop `count` items from the safe pool (no talisman), in a ring.
                    let safe_pool: Vec<usize> =
                        (0..INV_LIST.len()).filter(|&i| i != TALISMAN_IDX).collect();
                    let n = count.min(safe_pool.len() * 4); // allow cycling
                    for i in 0..n {
                        let item_id = safe_pool[i % safe_pool.len()];
                        let angle = 2.0 * std::f32::consts::PI * (i as f32) / (n as f32);
                        let x = (hero_x + (80.0f32 * angle.cos()) as i32).clamp(0, 0x7FFF) as u16;
                        let y = (hero_y + (80.0f32 * angle.sin()) as i32).clamp(0, 0x7FFF) as u16;
                        let ob_id_val = stuff_index_to_ob_id(item_id).unwrap_or(item_id as u8);
                        self.state.world_objects.push(WorldObject {
                            ob_id: ob_id_val,
                            ob_stat: 1,
                            region,
                            x,
                            y,
                            visible: true,
                            goal: 0,
                        });
                        dropped += 1;
                    }
                    self.dlog(format!("scattered {} items", dropped));
                }
            }
            KillActorSlot { slot } => {
                let idx = slot as usize;
                if idx == 0 {
                    self.dlog("KillActorSlot: slot 0 is the hero; use /die instead".to_string());
                } else if let Some(actor) = self.state.actors.get_mut(idx) {
                    if matches!(actor.state, ActorState::Dead | ActorState::Dying) {
                        self.dlog(format!("KillActorSlot: slot {} already dead/dying", slot));
                    } else {
                        actor.vitality = 0;
                        actor.state = ActorState::Dying;
                        self.dlog(format!("KillActorSlot: slot {} killed", slot));
                    }
                } else {
                    self.dlog(format!("KillActorSlot: slot {} out of range", slot));
                }
            }
            SetCheat1 { enabled } => {
                self.state.cheat1 = enabled;
                self.dlog(format!("cheat1 = {}", if enabled { "on" } else { "off" }));
            }
            TeleportNamedLocation { name } => {
                let needle = name.to_ascii_lowercase();
                let matches: Vec<(usize, String, u16, u16, u16, u16)> = self
                    .zones
                    .iter()
                    .enumerate()
                    .filter(|(_, z)| z.label.to_ascii_lowercase().contains(&needle))
                    .map(|(i, z)| (i, z.label.clone(), z.x1, z.y1, z.x2, z.y2))
                    .collect();
                match matches.len() {
                    0 => self.dlog(format!("TeleportNamedLocation: no zone matches '{}'", name)),
                    1 => {
                        let (idx, label, x1, y1, x2, y2) = matches.into_iter().next().unwrap();
                        let cx = (x1 as u32 + x2 as u32) / 2;
                        let cy = (y1 as u32 + y2 as u32) / 2;
                        self.state.hero_x = cx.min(0x7FFF) as u16;
                        self.state.hero_y = cy.min(0x7FFF) as u16;
                        self.snap_camera_to_hero();
                        // DBG-LOG-04: proof-of-pattern — emit via the new
                        // categorized channel instead of the legacy string
                        // log_buffer path.
                        self.pending_log.push(crate::debug_log!(
                            General,
                            "teleport: '{}' (zone {}, center {},{})",
                            label,
                            idx,
                            self.state.hero_x,
                            self.state.hero_y
                        ));
                    }
                    n => {
                        self.dlog(format!(
                            "TeleportNamedLocation: '{}' is ambiguous ({} matches):",
                            name, n
                        ));
                        for (i, label, _, _, _, _) in matches.into_iter().take(8) {
                            self.dlog(format!("  [{}] {}", i, label));
                        }
                    }
                }
            }
            QueryDoors => {
                let keys: Vec<u8> = (16..=21)
                    .map(|i| self.state.stuff().get(i).copied().unwrap_or(0))
                    .collect();
                let total = self.doors.len();
                let opened = self.opened_doors.len();
                let region = self.state.region_num;
                let rows: Vec<(
                    usize,
                    u8,
                    u16,
                    u16,
                    u8,
                    u16,
                    u16,
                    crate::game::doors::KeyReq,
                )> = self
                    .doors
                    .iter()
                    .enumerate()
                    .filter(|(_, d)| d.src_region == region)
                    .map(|(i, d)| {
                        (
                            i,
                            d.door_type,
                            d.src_x,
                            d.src_y,
                            d.dst_region,
                            d.dst_x,
                            d.dst_y,
                            crate::game::doors::key_req(d.door_type),
                        )
                    })
                    .collect();
                self.dlog(format!("── Doors ── total: {}", total));
                self.dlog(format!(
                    "  Keys held (slots 16-21): gold={} silver={} ruby={} skull={} iron={} crystal={}",
                    keys[0], keys[1], keys[2], keys[3], keys[4], keys[5]
                ));
                self.dlog(format!("  Opened door tiles: {}", opened));
                if rows.is_empty() {
                    self.dlog(format!("  (no doors in current region {})", region));
                } else {
                    let shown = rows.len().min(20);
                    for (i, dt, sx, sy, dr, dx, dy, kr) in rows.iter().take(20) {
                        self.dlog(format!(
                            "  [{}] type={} at ({},{}) -> r{}:({},{}) req={:?}",
                            i, dt, sx, sy, dr, dx, dy, kr
                        ));
                    }
                    if rows.len() > shown {
                        self.dlog("  ... (truncated)".to_string());
                    }
                }
            }
            QueryExtent => {
                let x = self.state.hero_x;
                let y = self.state.hero_y;
                match crate::game::zones::find_zone(&self.zones, x, y) {
                    None => self.dlog(format!(
                        "── Extent ── hero at ({},{}): no matching zone",
                        x, y
                    )),
                    Some(idx) => {
                        let (label, etype, x1, y1, x2, y2, v1, v2, v3) = {
                            let z = &self.zones[idx];
                            (
                                z.label.clone(),
                                z.etype,
                                z.x1,
                                z.y1,
                                z.x2,
                                z.y2,
                                z.v1,
                                z.v2,
                                z.v3,
                            )
                        };
                        self.dlog(format!("── Extent ── hero at ({},{})", x, y));
                        self.dlog(format!(
                            "  [{}] '{}' etype={} ({:?})  bounds=({},{})-({},{})",
                            idx,
                            label,
                            etype,
                            crate::game::zones::ZoneType::from_etype(etype),
                            x1,
                            y1,
                            x2,
                            y2
                        ));
                        self.dlog(format!("  v1={}  v2={}  v3={}", v1, v2, v3));
                    }
                }
            }
            SetTickRate { .. } => {
                // Intercepted in main.rs before reaching here.
            }
        }
    }
}
