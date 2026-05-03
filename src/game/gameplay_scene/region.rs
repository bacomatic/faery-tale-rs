//! Region transitions, map coordinate math, and palette computation.
//! See `docs/spec/world-structure.md` and `docs/spec/palettes-daynight-visuals.md`.

use super::*;

impl GameplayScene {
    /// Called when the hero transitions to a new region.
    /// Reloads world data and NPC table for the new region (npc-101, world-110).
    pub(crate) fn on_region_changed(&mut self, region: u8, game_lib: &GameLibrary) {
        self.log_buffer
            .push(format!("on_region_changed: region changed to {}", region));
        // Reset door interaction state: all opened doors and the locked-message dedup.
        self.opened_doors.clear();
        self.bumped_door = None;
        self.last_person = None;
        // Idempotent — populates all regions on first call, preserves
        // pickup/visibility state on subsequent transitions.
        self.state.populate_world_objects(game_lib);
        self.log_buffer.push(format!(
            "on_region_changed: {} world objects loaded",
            self.state.world_objects.len()
        ));
        if let Some(ref adf) = self.adf {
            let world_result = if let Some(cfg) = game_lib.find_region_config(region) {
                let map_blocks: Vec<u32> = if region < 8 {
                    Self::outdoor_map_blocks(game_lib)
                } else {
                    vec![cfg.map_block]
                };
                crate::game::world_data::WorldData::load(
                    adf,
                    region,
                    cfg.sector_block,
                    &map_blocks,
                    cfg.terra_block,
                    cfg.terra2_block,
                    &cfg.image_blocks,
                )
            } else {
                Err(anyhow::anyhow!("no region config for region {}", region))
            };
            match world_result {
                Ok(mut world) => {
                    // SPEC §15.10: Hidden City gate. If entering region 4 (desert) with
                    // fewer than 5 golden statues, overwrite 4 tiles at map offset
                    // (11 × 128) + 26 with impassable tile 254 to block the city entrance.
                    if region == 4 {
                        const ITEM_STATUE: usize = 25; // stuff[25] = gold statue count
                        if self.state.stuff()[ITEM_STATUE] < 5 {
                            let offset = (11 * 128) + 26;
                            if offset + 3 < world.map_mem.len() {
                                world.map_mem[offset] = 254;
                                world.map_mem[offset + 1] = 254;
                                world.map_mem[offset + 2] = 254;
                                world.map_mem[offset + 3] = 254;
                                self.log_buffer
                                    .push("Azal city entrance blocked (statues < 5)".to_string());
                            }
                        }
                    }

                    self.base_colors_palette = Self::build_base_colors_palette(game_lib, region);
                    self.current_palette = Self::region_palette(game_lib, region);
                    self.palette_dirty = true; // force recompute next cadence tick
                    self.map_renderer = Some(MapRenderer::new(&world, self.shadow_mem.clone()));
                    self.map_world = Some(world);
                    self.log_buffer.push(format!(
                        "on_region_changed: world reloaded for region {}",
                        region
                    ));
                }
                Err(e) => self
                    .log_buffer
                    .push(format!("on_region_changed: WorldData::load failed: {e}")),
            }
            self.npc_table = Some(crate::game::npc::NpcTable::load(adf, region));
            self.log_buffer.push(format!(
                "on_region_changed: NPC table loaded for region {}",
                region
            ));
        }
    }

    /// Collect the four y-band map_block values for the full overworld map (regions 0,2,4,6).
    /// All outdoor region pairs share a map file per y-band (F1/F2 share 160, F3/F4 share 168…).
    pub(crate) fn outdoor_map_blocks(
        game_lib: &crate::game::game_library::GameLibrary,
    ) -> Vec<u32> {
        [0u8, 2, 4, 6]
            .iter()
            .filter_map(|&r| game_lib.find_region_config(r))
            .map(|cfg| cfg.map_block)
            .collect()
    }

    /// Compute the outdoor region_num (0–7) from hero world-coordinates.
    ///
    /// Mirrors `gen_mini()` in `fmain.c`:
    ///   xs  = (hero_x + 7) >> 8          // sector column of viewport centre
    ///   ys  = (hero_y - 26) >> 8         // sector row of viewport centre
    ///   xr  = (xs >> 6) & 1              // 0 = west half, 1 = east half
    ///   yr  = (ys >> 5) & 3              // north→south band 0–3
    ///   region_num = xr + yr * 2
    pub(crate) fn outdoor_region_from_pos(hero_x: u16, hero_y: u16) -> u8 {
        let xs = (hero_x as u32 + 7) >> 8;
        let ys = (hero_y as u32).saturating_sub(26) >> 8;
        let xr = (xs >> 6) & 1;
        let yr = (ys >> 5) & 3;
        (xr + yr * 2) as u8
    }

    ///
    /// The base palette is `pagecolors[]` from fmain2.c — hardcoded in faery.toml.
    /// Only color index 31 varies by region (from `fade_page()` in fmain2.c:526-535):
    ///   - region 4 (desert):        0x0980
    ///   - region 9 (dungeons/caves): 0x0445
    ///   - all other regions:         0x0bdf  (already the default in pagecolors)
    pub(crate) fn region_palette(
        game_lib: &GameLibrary,
        region: u8,
    ) -> crate::game::palette::Palette {
        use crate::game::palette::{amiga_color_to_rgba, PALETTE_SIZE};
        let mut palette = [0xFF808080_u32; PALETTE_SIZE];
        if let Some(base) = game_lib.find_palette("pagecolors") {
            for (i, entry) in base.colors.iter().enumerate().take(PALETTE_SIZE) {
                palette[i] = amiga_color_to_rgba(entry.color);
            }
        }
        let color31: u16 = match region {
            4 => 0x0980, // F5 — desert area
            9 => 0x0445, // F10 — dungeons/caves (0x00f0 when secret_timer active)
            _ => 0x0bdf, // all other regions (already the default in pagecolors[31])
        };
        palette[31] = amiga_color_to_rgba(color31);
        palette
    }

    /// Build a base `colors::Palette` for a region from faery.toml pagecolors,
    /// with per-region color 31 override applied.
    pub(crate) fn build_base_colors_palette(
        game_lib: &GameLibrary,
        region: u8,
    ) -> Option<crate::game::colors::Palette> {
        let base = game_lib.find_palette("pagecolors")?;
        let mut cloned = base.clone();
        let color31: u16 = match region {
            4 => 0x0980,
            9 => 0x0445,
            _ => 0x0bdf,
        };
        if let Some(c) = cloned.colors.get_mut(31) {
            *c = crate::game::colors::RGB4::from(color31);
        }
        Some(cloned)
    }

    /// Recompute current_palette from base_colors_palette + lighting state.
    ///
    /// For outdoors (region < 8): applies fade_page() with per-channel percentages
    /// derived from lightlevel (0=midnight, 300=noon) and jewel light_on flag.
    /// For indoors (region >= 8): returns base palette at full brightness.
    ///
    /// In both cases, color 31 (sky) is set to a fixed per-region value after any
    /// fading (SPEC §17.6):
    ///   - region 4  (desert):             0x0980  orange-brown
    ///   - region 9, secret_active:        0x00f0  bright green
    ///   - region 9  (normal):             0x0445  dark grey-blue
    ///   - all others:                     0x0bdf  light blue
    pub(crate) fn compute_current_palette(
        base: &crate::game::colors::Palette,
        region_num: u8,
        lightlevel: u16,
        light_on: bool,
        secret_active: bool,
    ) -> crate::game::palette::Palette {
        use crate::game::palette::{amiga_color_to_rgba, PALETTE_SIZE};

        // Indoors: full brightness; still route through fade_page so that an
        // active light_timer (Green Jewel) applies the warm-red torch tint.
        // Reference: fmain2.c:1659 — `fade_page(100,100,100,True,pagecolors)` for region>=8.
        if region_num >= 8 {
            let faded = crate::game::palette_fader::fade_page(100, 100, 100, true, light_on, base);
            let mut pal = [0xFF808080_u32; PALETTE_SIZE];
            for (i, entry) in faded.colors.iter().enumerate().take(PALETTE_SIZE) {
                pal[i] = amiga_color_to_rgba(entry.color);
            }
            // SPEC §17.6: color 31 (sky) override for all indoor cases.
            pal[31] = amiga_color_to_rgba(match (region_num, secret_active) {
                (9, true) => 0x00f0,  // secret area active: bright green
                (9, false) => 0x0445, // dungeon normal: dark grey-blue
                _ => 0x0bdf,          // other indoor regions: light blue
            });
            return pal;
        }

        let ll = lightlevel as i32;
        let ll_boost = if light_on { 200i32 } else { 0 };
        let r_pct = (ll - 80 + ll_boost) as i16;
        let g_pct = (ll - 61) as i16;
        let b_pct = (ll - 62) as i16;

        let faded =
            crate::game::palette_fader::fade_page(r_pct, g_pct, b_pct, true, light_on, base);

        // Convert colors::Palette (RGB4) → [u32; 32] (ARGB8888).
        let mut out = [0xFF808080_u32; PALETTE_SIZE];
        for (i, entry) in faded.colors.iter().enumerate().take(PALETTE_SIZE) {
            out[i] = amiga_color_to_rgba(entry.color);
        }
        // SPEC §17.6: color 31 (sky) is a fixed per-region value, not subject to
        // day/night fading.  Apply the override after fade_page.
        out[31] = amiga_color_to_rgba(match region_num {
            4 => 0x0980, // desert sky: orange-brown
            _ => 0x0bdf, // all other outdoor regions: light blue
        });
        out
    }

    /// Camera follow from fsubs.asm:1360–1423.
    /// Dead zone (±20 px X / ±10 px Y): camera still, player moves in window.
    /// Creep zone (20–70 px X / 10–24/44 px Y): camera advances 1 px/tick toward player.
    /// Beyond threshold: camera tracks 1:1 with player, keeping player pinned at the edge.
    /// Immediately center the camera on the hero (used after teleports).
    pub(crate) fn snap_camera_to_hero(&mut self) {
        const CX: i32 = 144;
        const CY: i32 = 70;
        const WRAP: i32 = 0x8000;
        self.map_x = (self.state.hero_x as i32 - CX).rem_euclid(WRAP) as u16;
        self.map_y = (self.state.hero_y as i32 - CY).rem_euclid(WRAP) as u16;
    }

    pub(crate) fn map_adjust(hero_x: u16, hero_y: u16, map_x: u16, map_y: u16) -> (u16, u16) {
        const CX: i32 = 144;
        const CY: i32 = 70;
        const WRAP: i32 = 0x8000;

        // Ideal camera origin, wrapped into [0, WRAP).
        let ideal_x = (hero_x as i32 - CX).rem_euclid(WRAP);
        // Shortest-path signed delta in (-WRAP/2, WRAP/2].
        let dx = {
            let d = (ideal_x - map_x as i32).rem_euclid(WRAP);
            if d > WRAP / 2 {
                d - WRAP
            } else {
                d
            }
        };
        let new_map_x = (if dx > 70 {
            ideal_x - 70
        } else if dx < -70 {
            ideal_x + 70
        } else if dx > 20 {
            map_x as i32 + 1
        } else if dx < -20 {
            map_x as i32 - 1
        } else {
            map_x as i32
        })
        .rem_euclid(WRAP);

        let ideal_y = (hero_y as i32 - CY).rem_euclid(WRAP);
        let dy = {
            let d = (ideal_y - map_y as i32).rem_euclid(WRAP);
            if d > WRAP / 2 {
                d - WRAP
            } else {
                d
            }
        };
        let new_map_y = (if dy > 44 {
            ideal_y - 44
        } else if dy < -24 {
            ideal_y + 24
        } else if dy > 10 {
            map_y as i32 + 1
        } else if dy < -10 {
            map_y as i32 - 1
        } else {
            map_y as i32
        })
        .rem_euclid(WRAP);

        (new_map_x as u16, new_map_y as u16)
    }

    pub(crate) fn actor_rel_pos(abs_x: u16, abs_y: u16, map_x: u16, map_y: u16) -> (i32, i32) {
        Self::actor_rel_pos_offset(abs_x, abs_y, map_x, map_y, -8, -26)
    }

    /// Raft/Carrier/Dragon use (-16, -16) offsets (fmain.c:2152-2155).
    pub(crate) fn carrier_rel_pos(abs_x: u16, abs_y: u16, map_x: u16, map_y: u16) -> (i32, i32) {
        Self::actor_rel_pos_offset(abs_x, abs_y, map_x, map_y, -16, -16)
    }

    pub(crate) fn actor_rel_pos_offset(
        abs_x: u16,
        abs_y: u16,
        map_x: u16,
        map_y: u16,
        ox: i32,
        oy: i32,
    ) -> (i32, i32) {
        const WRAP: i32 = 0x8000;
        let dx = (abs_x as i32 - map_x as i32 + ox).rem_euclid(WRAP);
        let rel_x = if dx > WRAP / 2 { dx - WRAP } else { dx };
        let dy = (abs_y as i32 - map_y as i32 + oy).rem_euclid(WRAP);
        let rel_y = if dy > WRAP / 2 { dy - WRAP } else { dy };
        (rel_x, rel_y)
    }
}
