//! Sprite blitting, actor rendering, and animation frame computation.
//! See `docs/spec/display-rendering.md` for specification.

use super::*;

impl GameplayScene {
    pub(super) fn render_hibar(&mut self, canvas: &mut Canvas<Window>, resources: &mut SceneResources<'_, '_>) {
        let brave    = self.state.brave;
        let luck     = self.state.luck;
        let kind     = self.state.kind;
        let vitality = self.state.vitality;
        let wealth   = self.state.wealth;
        let buttons = self.menu.print_options();
        let msg_count = self.messages.len().min(4);
        let msgs: Vec<&str> = self.messages.iter().collect();
        let msg_start = msgs.len().saturating_sub(4);
        let msgs_visible: Vec<&str> = msgs[msg_start..].to_vec();
        let textcolors = &self.textcolors;
        let compass_regions = &self.compass_regions;
        let input_comptable_dir = compass_dir_for_input(self.current_direction());
        let hiscreen_opt = resources.find_image("hiscreen");
        let amber_font = resources.amber_font;
        let topaz_font = resources.topaz_font;
        let compass_normal = resources.compass_normal;
        let compass_highlight = resources.compass_highlight;
        let cursor_active = self.menu_cursor.active;
        let cursor_col = self.menu_cursor.col;
        let cursor_row = self.menu_cursor.row;
        let topaz_baseline = topaz_font.get_font().baseline as i32;

        let tc = canvas.texture_creator();
        if let Ok(mut hibar_tex) = tc.create_texture_target(
            sdl2::pixels::PixelFormatEnum::RGBA32, 640, HIBAR_NATIVE_H,
        ) {
            let _ = canvas.with_texture_canvas(&mut hibar_tex, |hc| {
                hc.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
                hc.clear();

                if let Some(hiscreen) = hiscreen_opt {
                    hiscreen.draw_scaled(hc, sdl2::rect::Rect::new(0, 0, 640, HIBAR_NATIVE_H));
                } else {
                    hc.set_draw_color(sdl2::pixels::Color::RGB(80, 60, 20));
                    hc.fill_rect(sdl2::rect::Rect::new(0, 0, 640, HIBAR_NATIVE_H)).ok();
                }

                amber_font.set_color_mod(0xAA, 0x55, 0x00);
                amber_font.render_string(&format!("Brv:{:3}", brave),     hc, 14,  52);
                amber_font.render_string(&format!("Lck:{:3}", luck),      hc, 90,  52);
                amber_font.render_string(&format!("Knd:{:3}", kind),      hc, 168, 52);
                amber_font.render_string(&format!("Vit:{:3}", vitality),  hc, 245, 52);
                amber_font.render_string(&format!("Wlth:{:3}", wealth),   hc, 321, 52);

                for (i, msg) in msgs_visible.iter().enumerate() {
                    let line_from_bottom = (msg_count - 1 - i) as i32;
                    let y = 42 - line_from_bottom * 10;
                    amber_font.render_string(msg, hc, 16, y);
                }
                amber_font.set_color_mod(255, 255, 255);

                for btn in &buttons {
                    let col = btn.display_slot & 1;
                    let row = btn.display_slot / 2;
                    let btn_x = if col == 0 { 430i32 } else { 482i32 };
                    let btn_y = (row as i32) * 9 + 8;
                    let bg_rgba = textcolors[btn.bg_color as usize];
                    let bg = (((bg_rgba >> 16) & 0xFF) as u8, ((bg_rgba >> 8) & 0xFF) as u8, (bg_rgba & 0xFF) as u8);
                    let fg_rgba = textcolors[btn.fg_color as usize];
                    let fg = (((fg_rgba >> 16) & 0xFF) as u8, ((fg_rgba >> 8) & 0xFF) as u8, (fg_rgba & 0xFF) as u8);
                    topaz_font.render_string_with_bg("      ", hc, btn_x, btn_y, bg, fg);
                    topaz_font.set_color_mod(fg.0, fg.1, fg.2);
                    topaz_font.render_string(&btn.text, hc, btn_x + 4, btn_y);
                    topaz_font.set_color_mod(255, 255, 255);
                }

                const COMPASS_X: i32 = 567;
                const COMPASS_SRC_Y: i32 = 15;
                const COMPASS_SRC_W: u32 = 48;
                const COMPASS_SRC_H: u32 = 24;
                let compass_dest = sdl2::rect::Rect::new(COMPASS_X, COMPASS_SRC_Y, COMPASS_SRC_W, COMPASS_SRC_H);
                if let Some(normal_tex) = compass_normal {
                    hc.copy(normal_tex, None, compass_dest).ok();
                }
                if input_comptable_dir < compass_regions.len() {
                    let (rx, ry, rw, rh) = compass_regions[input_comptable_dir];
                    if rw > 1 || rh > 1 {
                        if let Some(highlight_tex) = compass_highlight {
                            let src = sdl2::rect::Rect::new(rx, ry, rw as u32, rh as u32);
                            let dst = sdl2::rect::Rect::new(COMPASS_X + rx, COMPASS_SRC_Y + ry, rw as u32, rh as u32);
                            hc.copy(highlight_tex, src, dst).ok();
                        }
                    }
                }

                // Controller menu cursor outline
                if cursor_active {
                    let cursor_x = if cursor_col == 0 { 430i32 } else { 482i32 };
                    let cursor_y = (cursor_row as i32) * 9 + 8 - topaz_baseline;
                    let cursor_w = 48u32; // button text width (6 chars × 8px)
                    let cursor_h = 9u32;  // row height
                    hc.set_draw_color(sdl2::pixels::Color::RGB(255, 255, 255));
                    hc.draw_rect(sdl2::rect::Rect::new(
                        cursor_x - 1, cursor_y - 1, cursor_w + 2, cursor_h + 2
                    )).ok();
                }
            });
            canvas.copy(
                &hibar_tex,
                sdl2::rect::Rect::new(0, 0, 640, HIBAR_NATIVE_H),
                sdl2::rect::Rect::new(0, HIBAR_Y, 640, HIBAR_H),
            ).ok();
        }; // semicolon: drops Result<Texture> temporary before tc is dropped
    }

    /// Clear and color the canvas according to the current viewstatus mode.
    pub(super) fn render_by_viewstatus(&mut self, canvas: &mut Canvas<Window>, resources: &mut SceneResources<'_, '_>) {
        match self.state.viewstatus {
            // Normal play or forced redraw
            0 | 98 | 99 => {
                canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
                canvas.clear();
                // Blit composed map framebuf to canvas (world-105).
                if let Some(ref mr) = self.map_renderer {
                    if !mr.framebuf.is_empty() {
                        // Apply current_palette: indexed u8 → RGBA32 bytes for SDL2.
                        let pal = &self.current_palette;
                        let mut rgb_buf: Vec<u8> = Vec::with_capacity(mr.framebuf.len() * 4);
                        for &idx in &mr.framebuf {
                            let rgba = pal[(idx & 31) as usize];
                            // ARGB8888 on little-endian: memory bytes are [B, G, R, A]
                            rgb_buf.push((rgba & 0xFF) as u8);
                            rgb_buf.push(((rgba >> 8) & 0xFF) as u8);
                            rgb_buf.push(((rgba >> 16) & 0xFF) as u8);
                            rgb_buf.push(0xFF);
                        }
                        let tc = canvas.texture_creator();
                        let surface_result = sdl2::surface::Surface::from_data(
                            &mut rgb_buf,
                            crate::game::map_renderer::MAP_DST_W,
                            crate::game::map_renderer::MAP_DST_H,
                            crate::game::map_renderer::MAP_DST_W * 4,
                            sdl2::pixels::PixelFormatEnum::ARGB8888,
                        );
                        if let Ok(surface) = surface_result {
                            if let Ok(tex) = tc.create_texture_from_surface(&surface) {
                                let src = sdl2::rect::Rect::new(0, 0, PLAYFIELD_LORES_W, PLAYFIELD_LORES_H);
                                let dst = sdl2::rect::Rect::new(
                                    PLAYFIELD_X, PLAYFIELD_Y,
                                    PLAYFIELD_CANVAS_W, PLAYFIELD_CANVAS_H,
                                );
                                let _ = canvas.copy(&tex, Some(src), Some(dst));
                            }
                        }
                    }
                }

                self.render_hibar(canvas, resources);

                // Tick witch visual effect (scanline warp, applied to map texture).
                self.witch_effect.tick();
            }
            // Map view (bird totem)
            1 => {
                canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
                canvas.clear();

                if let Some(ref world) = self.map_world {
                    let buf = crate::game::map_view::bigdraw(
                        self.state.hero_x, self.state.hero_y, world,
                    );
                    let mut pixels_u8: Vec<u8> = Vec::with_capacity(buf.len() * 4);
                    for &px in &buf {
                        pixels_u8.push((px & 0xFF) as u8);
                        pixels_u8.push(((px >> 8) & 0xFF) as u8);
                        pixels_u8.push(((px >> 16) & 0xFF) as u8);
                        pixels_u8.push(0xFF);
                    }
                    let tc = canvas.texture_creator();
                    let surface_result = sdl2::surface::Surface::from_data(
                        &mut pixels_u8,
                        crate::game::map_view::BIGDRAW_COLS as u32,
                        crate::game::map_view::BIGDRAW_ROWS as u32,
                        (crate::game::map_view::BIGDRAW_COLS * 4) as u32,
                        sdl2::pixels::PixelFormatEnum::ARGB8888,
                    );
                    if let Ok(surface) = surface_result {
                        if let Ok(tex) = tc.create_texture_from_surface(&surface) {
                            let dst = sdl2::rect::Rect::new(32, 40, 576, 144);
                            let _ = canvas.copy(&tex, None, Some(dst));
                        }
                    }
                }

                // Hero position marker (center of the map view)
                canvas.set_draw_color(sdl2::pixels::Color::RGB(255, 255, 255));
                let hero_px = 32 + 576 / 2;
                let hero_py = 40 + 144 / 2;
                let _ = canvas.draw_line(
                    sdl2::rect::Point::new(hero_px - 4, hero_py),
                    sdl2::rect::Point::new(hero_px + 4, hero_py),
                );
                let _ = canvas.draw_line(
                    sdl2::rect::Point::new(hero_px, hero_py - 4),
                    sdl2::rect::Point::new(hero_px, hero_py + 4),
                );

                self.render_hibar(canvas, resources);
            }
            // Message overlay
            2 => {
                canvas.set_draw_color(sdl2::pixels::Color::RGB(48, 48, 48));
                canvas.clear();
                // "MESSAGE" — text rendering pending font wiring
            }
            // Inventory screen (viewstatus=4): black play area with item sprites, normal HI bar.
            // Original: do_option() ITEMS hit=5 — clears playfield to black, blits item sprites
            // from seq_list[OBJECTS] using inv_list[] layout, then stillscreen() + viewstatus=4.
            4 => {
                canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
                canvas.clear();

                // Build a 320×200 lores canvas with item sprites at their inv_list positions.
                // Items use the objects sprite sheet (cfile 3, 16×16 frames).
                if let Some(ref obj_sheet) = self.object_sprites {
                    use crate::game::sprites::{INV_LIST, OBJ_SPRITE_H, SPRITE_W};
                    const LORES_W: usize = 320;
                    const LORES_H: usize = 200;
                    // Index 31 = transparent background.
                    let mut inv_indices = vec![31u8; LORES_W * LORES_H];
                    let stuff = *self.state.stuff();

                    for (j, item) in INV_LIST.iter().enumerate() {
                        let count = stuff[j] as usize;
                        if count == 0 { continue; }
                        let num = count.min(item.maxshown as usize);
                        let frame = item.image_number as usize;
                        if let Some(frame_pix) = obj_sheet.frame_pixels(frame) {
                            let mut dst_y = item.yoff as i32;
                            for _ in 0..num {
                                let dst_x = item.xoff as i32 + 20;
                                for row in 0..item.img_height as usize {
                                    let src_row = item.img_off as usize + row;
                                    if src_row >= OBJ_SPRITE_H { break; }
                                    let py = dst_y + row as i32;
                                    if py < 0 || py >= LORES_H as i32 { continue; }
                                    for col in 0..SPRITE_W {
                                        let px = dst_x + col as i32;
                                        if px < 0 || px >= LORES_W as i32 { continue; }
                                        let src_idx = frame_pix[src_row * SPRITE_W + col];
                                        if src_idx != 31 {
                                            inv_indices[py as usize * LORES_W + px as usize] = src_idx;
                                        }
                                    }
                                }
                                dst_y += item.ydelta as i32;
                            }
                        }
                    }

                    // Apply palette: indexed u8 → RGBA32 bytes for SDL2.
                    let pal = &self.current_palette;
                    let mut rgb_buf: Vec<u8> = Vec::with_capacity(LORES_W * LORES_H * 4);
                    for &idx in &inv_indices {
                        let rgba = if idx == 31 {
                            0u32 // transparent background → black
                        } else {
                            pal[(idx & 31) as usize]
                        };
                        rgb_buf.push((rgba & 0xFF) as u8);
                        rgb_buf.push(((rgba >> 8) & 0xFF) as u8);
                        rgb_buf.push(((rgba >> 16) & 0xFF) as u8);
                        rgb_buf.push(0xFF);
                    }
                    // Blit the lores inventory canvas to the playfield rect (clip x=16, scale 2×).
                    let tc = canvas.texture_creator();
                    if let Ok(surface) = sdl2::surface::Surface::from_data(
                        &mut rgb_buf,
                        LORES_W as u32, LORES_H as u32,
                        LORES_W as u32 * 4,
                        sdl2::pixels::PixelFormatEnum::ARGB8888,
                    ) {
                        if let Ok(tex) = tc.create_texture_from_surface(&surface) {
                            let src = sdl2::rect::Rect::new(
                                16, 0, PLAYFIELD_LORES_W, PLAYFIELD_LORES_H,
                            );
                            let dst = sdl2::rect::Rect::new(
                                PLAYFIELD_X, PLAYFIELD_Y,
                                PLAYFIELD_CANVAS_W, PLAYFIELD_CANVAS_H,
                            );
                            let _ = canvas.copy(&tex, Some(src), Some(dst));
                        }
                    }; // semicolon: drops Result<Surface> temporary before rgb_buf is dropped
                }

                self.render_hibar(canvas, resources);
            }
            _ => {
                canvas.set_draw_color(sdl2::pixels::Color::RGB(0, 0, 0));
                canvas.clear();
            }
        }
    }

    pub(super) fn blit_sprite_to_framebuf(
        frame_pixels: &[u8],
        rel_x: i32,
        rel_y: i32,
        max_rows: usize,
        framebuf: &mut [u8],
        fb_w: i32,
        fb_h: i32,
    ) {
        use crate::game::sprites::{SPRITE_W, SPRITE_H};
        let row_limit = max_rows.min(SPRITE_H) as i32;
        for row in 0..row_limit {
            let dst_y = rel_y + row;
            if dst_y < 0 || dst_y >= fb_h { continue; }
            for col in 0..SPRITE_W as i32 {
                let dst_x = rel_x + col;
                if dst_x < 0 || dst_x >= fb_w { continue; }
                let src_idx = frame_pixels[(row as usize) * SPRITE_W + col as usize];
                if src_idx == 31 { continue; } // transparent
                framebuf[(dst_y * fb_w + dst_x) as usize] = src_idx;
            }
        }
    }

    /// Blit an object sprite (16×obj_h) into the framebuf.
    pub(super) fn blit_obj_to_framebuf(
        frame_pixels: &[u8],
        rel_x: i32,
        rel_y: i32,
        obj_h: usize,
        framebuf: &mut [u8],
        fb_w: i32,
        fb_h: i32,
    ) {
        use crate::game::sprites::SPRITE_W;
        for row in 0..obj_h as i32 {
            let dst_y = rel_y + row;
            if dst_y < 0 || dst_y >= fb_h { continue; }
            for col in 0..SPRITE_W as i32 {
                let dst_x = rel_x + col;
                if dst_x < 0 || dst_x >= fb_w { continue; }
                let src_idx = frame_pixels[(row as usize) * SPRITE_W + col as usize];
                if src_idx == 31 { continue; }
                framebuf[(dst_y * fb_w + dst_x) as usize] = src_idx;
            }
        }
    }

    /// Compute the hero's weapon-overlay blit parameters for the current body
    /// `frame` and `hero_facing`. Mirrors `select_atype_inum` and the bow
    /// special-case in `reference/logic/sprite-rendering.md` (`fmain.c:2412-2425`).
    ///
    /// Returns `(frame_pixels_slice, wx, wy, height)` where the slice has been
    /// pre-trimmed to the correct OBJECTS half/full-height row band and bit-7
    /// has been stripped from the source `inum`.
    pub(super) fn compute_weapon_blit<'a>(
        frame: usize,
        hero_facing: u8,
        weapon_type: u8,
        obj_sheet: &'a crate::game::sprites::SpriteSheet,
        rel_x: i32,
        rel_y: i32,
    ) -> Option<(&'a [u8], i32, i32, usize)> {
        use crate::game::sprites::{
            STATELIST, SPRITE_W, BOW_X, BOW_Y,
            obj_frame_height, obj_frame_y_offset, obj_frame_index,
        };
        if !(weapon_type > 0 && weapon_type <= 5) { return None; }
        let entry = STATELIST.get(frame)?;

        // Resolve (x_off, y_off, raw_inum) per weapon class.
        // Weapon-class k offsets (fmain.c:2412-2418):
        //   bow=0 (special-cased), mace=32, sword=48, dirk=64.
        let (x_off, y_off, raw_inum): (i32, i32, u8) = if weapon_type == 5 {
            // Wand: inum = facing + 103; DIR_NE (=2) shifts Y by -6 (fmain.c:2418).
            let wy = if hero_facing == 2 { entry.wpn_y as i32 - 6 } else { entry.wpn_y as i32 };
            (entry.wpn_x as i32, wy, (hero_facing as u8).wrapping_add(103))
        } else if weapon_type == 4 && frame < 32 {
            // Bow walking pose: per-frame BOW_X/BOW_Y offsets and direction-dependent
            // bow inum derived from walk-cycle group (frame / 8) — fmain.c:2429-2433:
            //   group & 1 → 30 (east-west bow); group & 2 → 0x53 (north bow); else → 0x51 (south bow).
            //   0 = south(0..7)→0x51, 1 = west(8..15)→30, 2 = north(16..23)→0x53, 3 = east(24..31)→30.
            let bow_inum: u8 = match frame / 8 {
                0 => 0x51,
                1 => 30,
                2 => 0x53,
                _ => 30,
            };
            (BOW_X[frame] as i32, BOW_Y[frame] as i32, bow_inum)
        } else {
            // Hand weapons (and bow on non-walking frames): wpn_no + k.
            let k: u8 = match weapon_type { 1 => 64, 2 => 32, 3 => 48, _ => 0 };
            (entry.wpn_x as i32, entry.wpn_y as i32, entry.wpn_no.wrapping_add(k))
        };

        // Bit-7 dual role + half-height set: pick the right row band of the
        // OBJECTS frame (`compute_sprite_size` + `compute_shape_clip`).
        let h = obj_frame_height(raw_inum) as usize;
        let y_skip = obj_frame_y_offset(raw_inum) as usize;
        let frame_idx = obj_frame_index(raw_inum) as usize;
        let fp_full = obj_sheet.frame_pixels(frame_idx)?;
        let row_bytes = SPRITE_W;
        let start = y_skip * row_bytes;
        let end = start + h * row_bytes;
        if end > fp_full.len() { return None; }
        Some((&fp_full[start..end], rel_x + x_off, rel_y + y_off, h))
    }

    /// Map a facing direction (0=N…7=NW) to the sprite sheet frame base.
    /// Mirrors the diroffs[] group mapping from fmain.c:
    ///   southwalk=0-7, westwalk=8-15, northwalk=16-23, eastwalk=24-31.
    pub(super) fn facing_to_frame_base(facing: u8) -> usize {
        // diroffs[0..7] from fmain.c:1010 with original facing DIR_NW=0..DIR_W=7:
        //   [16,16,24,24,0,0,8,8] → NW=16,N=16,NE=24,E=24,SE=0,S=0,SW=8,W=8.
        // Mapped to Rust facing (0=N..7=NW): NE→east, SE→south, SW→west, NW→north.
        match facing {
            0 => 16, // N  → northwalk
            1 => 24, // NE → eastwalk
            2 => 24, // E  → eastwalk
            3 => 0,  // SE → southwalk
            4 => 0,  // S  → southwalk
            5 => 8,  // SW → westwalk
            6 => 8,  // W  → westwalk
            _ => 16, // NW → northwalk
        }
    }

    /// Map facing direction to fighting sprite frame base.
    /// diroffs[d+8] from fmain.c:1010 with original facing DIR_NW=0..DIR_W=7:
    ///   [56,56,68,68,32,32,44,44] → NW=56,N=56,NE=68,E=68,SE=32,S=32,SW=44,W=44.
    /// Mapped to Rust facing (0=N..7=NW): NE→east, SE→south, SW→west, NW→north.
    /// Frame ranges: southfight=32-43, westfight=44-55, northfight=56-67, eastfight=68-79.
    pub(super) fn facing_to_fight_frame_base(facing: u8) -> usize {
        match facing {
            0 => 56, // N  → northfight
            1 => 68, // NE → eastfight
            2 => 68, // E  → eastfight
            3 => 32, // SE → southfight
            4 => 32, // S  → southfight
            5 => 44, // SW → westfight
            6 => 44, // W  → westfight
            _ => 56, // NW → northfight
        }
    }

    /// Map (npc_type, race) → cfile index for enemy sprite rendering.
    /// Returns None for SetFig humans (rendered in a separate pass) and skipped types.
    /// cfile 7 covers ghost/wraith/skeleton per RESEARCH.md sprite assignments.
    pub(super) fn npc_type_to_cfile(npc_type: u8, race: u8) -> Option<usize> {
        use crate::game::npc::*;
        match npc_type {
            NPC_TYPE_NONE | NPC_TYPE_CONTAINER => None,
            NPC_TYPE_HUMAN if race == RACE_ENEMY => Some(6),
            NPC_TYPE_HUMAN => None,  // SetFig — handled in setfig pass
            NPC_TYPE_SWAN     => Some(11),
            NPC_TYPE_HORSE    => Some(5),
            NPC_TYPE_DRAGON   => Some(10),
            NPC_TYPE_GHOST    => Some(7),
            NPC_TYPE_ORC      => Some(6),
            NPC_TYPE_WRAITH   => Some(7),
            NPC_TYPE_SKELETON => Some(7),
            NPC_TYPE_SNAKE | NPC_TYPE_SPIDER | NPC_TYPE_DKNIGHT => Some(8),
            NPC_TYPE_LORAII | NPC_TYPE_NECROMANCER => Some(9),
            NPC_TYPE_RAFT     => Some(4),
            _                 => Some(6), // unknown enemy types default to ogre sheet
        }
    }

    /// Map (npc_type, race) → SETFIG_TABLE index for named NPC rendering.
    /// Returns None if the NPC is not a SetFig.
    /// SETFIG_TABLE indices: 0=wizard, 8=bartender, 13=beggar (see sprites.rs).
    pub(super) fn npc_to_setfig_idx(npc_type: u8, race: u8) -> Option<usize> {
        use crate::game::npc::*;
        if npc_type != NPC_TYPE_HUMAN { return None; }
        match race {
            RACE_SHOPKEEPER => Some(8),   // bartender
            RACE_BEGGAR     => Some(13),  // beggar
            RACE_NORMAL     => Some(0),   // wizard (default named NPC)
            _               => None,
        }
    }

    /// SPEC §21.4 / RESEARCH §2.6 (`fmain.c:2463-2464`): when the swan is not
    /// being ridden by the hero, render it using the RAFT sheet (cfile 4) at
    /// fixed frame 1 instead of the carrier sheet (cfile 11).
    ///
    /// Returns `Some((cfile_idx, frame))` if the override applies, else `None`.
    ///
    /// `state.flying != 0` indicates the hero is mounted on the swan (see
    /// `GameState::flying` and magic.rs swan-mount logic). While mounted, the
    /// normal carrier-sheet render path applies with facing-indexed frame.
    pub(super) fn swan_grounded_override(
        npc: &crate::game::npc::Npc,
        state: &GameState,
    ) -> Option<(usize, usize)> {
        use crate::game::npc::NPC_TYPE_SWAN;
        if npc.npc_type == NPC_TYPE_SWAN && state.flying == 0 {
            Some((4, 1))
        } else {
            None
        }
    }

    /// Compute the sprite frame index for an NPC, matching fmain.c:2076–2108.
    /// `npc_idx` is the NPC's index in the table (provides phase offset like original `cycle + i`).
    /// Returns the frame index clamped to `num_frames`.
    pub(super) fn npc_animation_frame(
        npc: &crate::game::npc::Npc,
        npc_idx: usize,
        cycle: u32,
        num_frames: usize,
    ) -> usize {
        use crate::game::npc::{NpcState, RACE_WRAITH, RACE_SNAKE};

        let frame_base = Self::facing_to_frame_base(npc.facing);

        let raw = match npc.state {
            NpcState::Walking => {
                if npc.race == RACE_WRAITH {
                    // Wraiths: no walk cycle (fmain.c:2079 — race 2 skips cycle offset)
                    frame_base
                } else if npc.race == RACE_SNAKE {
                    // Snakes walking: 2-frame, changes every 2 ticks (fmain.c:2081)
                    frame_base + ((cycle as usize / 2) & 1)
                } else {
                    // Default: 8-frame walk cycle with per-NPC phase offset (fmain.c:1863)
                    frame_base + ((cycle as usize + npc_idx) & 7)
                }
            }
            NpcState::Still => {
                if npc.race == RACE_SNAKE {
                    // Snakes still: 2-frame idle, every tick (fmain.c:2079)
                    frame_base + (cycle as usize & 1)
                } else {
                    // Default still: static frame (fmain.c:~1900 — diroffs[d] + 1)
                    frame_base + 1
                }
            }
            // Dying/Dead/Sinking/Fighting/Shooting: static base frame
            _ => frame_base,
        };

        raw % num_frames
    }

    /// Blit all visible actors (hero + enemy NPCs) onto the map framebuf (sprite-104).
    /// Called immediately after mr.compose() so actors appear on top of tiles.
    pub(super) fn blit_actors_to_framebuf(
        sprite_sheets: &[Option<crate::game::sprites::SpriteSheet>],
        obj_sprites: &Option<crate::game::sprites::SpriteSheet>,
        state: &GameState,
        npc_table: &Option<crate::game::npc::NpcTable>,
        map_x: u16,
        map_y: u16,
        framebuf: &mut Vec<u8>,
        _hero_submerged: bool,
    ) {
        use crate::game::map_renderer::{MAP_DST_W, MAP_DST_H};
        use crate::game::sprites::{SPRITE_H, SPRITE_W, STATELIST};
        let fb_w = MAP_DST_W as i32;
        let fb_h = MAP_DST_H as i32;

        // --- Hero sprite ---
        // cfiles[0]=Julian (brother=1), [1]=Phillip (brother=2), [2]=Kevin (brother=3)
        let hero_cfile = state.brother.saturating_sub(1) as usize;
        if let Some(Some(ref sheet)) = sprite_sheets.get(hero_cfile) {
            let (rel_x, mut rel_y) = Self::actor_rel_pos(state.hero_x, state.hero_y, map_x, map_y);
            let environ = state.actors.first().map_or(0i8, |a| a.environ);
            let body_rows: usize = if environ == 2 {
                SPRITE_H.saturating_sub(10)
            } else if environ > 2 {
                rel_y += environ as i32;
                SPRITE_H.saturating_sub(environ as usize)
            } else {
                SPRITE_H
            };
            if rel_x > -(SPRITE_W as i32) && rel_x < fb_w && rel_y > -(SPRITE_H as i32) && rel_y < fb_h {
                let hero_facing = state.actors.first().map_or(0u8, |a| a.facing);
                let is_moving = state.actors.first().map_or(false, |a| a.moving);
                // Sprite sheet layout (from fmain.c statelist[] and diroffs[]):
                //   southwalk=0-7, westwalk=8-15, northwalk=16-23, eastwalk=24-31
                // Original diroffs[] groups: NW+N→north, NE+E→east, SE+S→south, SW+W→west.
                // Rust facing: 0=N, 1=NE, 2=E, 3=SE, 4=S, 5=SW, 6=W, 7=NW.
                let hero_state = state.actors.first().map(|a| &a.state);
                // Frustration render override (SPEC §9.8, player-only).
                //   frustflag 0..=20   → normal sprite selection below.
                //   frustflag 21..=40  → head-shake: dex alternates 84/85 every 2 cycles (south-facing).
                //   frustflag 41..     → frozen south-facing pose, dex = 40.
                // Overrides normal walking/fight selection but not active Fighting animation.
                let frustflag = state.frustflag;
                let frust_render_frame: Option<usize> = if frustflag >= 41 {
                    Some(40)
                } else if frustflag >= 21 {
                    Some(84 + ((state.cycle as usize >> 1) & 1))
                } else {
                    None
                };
                let frame = if let Some(f) = frust_render_frame {
                    f
                } else if let Some(ActorState::Fighting(fight_state)) = hero_state {
                    // Fighting: use fight frame base + current animation state (0-8).
                    let fight_base = Self::facing_to_fight_frame_base(hero_facing);
                    fight_base + (*fight_state as usize).min(8)
                } else {
                    // Walking or still: existing logic.
                    let frame_base = Self::facing_to_frame_base(hero_facing);
                    if is_moving { frame_base + (state.cycle as usize) % 8 } else { frame_base + 1 }
                };
                // Body sprite frame: for fighting, use STATELIST figure field; for walking/still, frame is already correct
                let body_frame = if let Some(ActorState::Fighting(_)) = hero_state {
                    STATELIST[frame].figure as usize
                } else {
                    frame
                };
                // Weapon overlay (fmain.c passmode weapon blit).
                // Draw order depends on facing: weapon behind body for N,SW,W,NW.
                let weapon_type = state.actors.first().map_or(0u8, |a| a.weapon);
                let wpn_blit = if let Some(ref obj_sheet) = obj_sprites {
                    Self::compute_weapon_blit(frame, hero_facing, weapon_type, obj_sheet, rel_x, rel_y)
                } else { None };

                // Weapon behind body for N(0), SW(5), W(6), NW(7)
                let weapon_behind = matches!(hero_facing, 0 | 5 | 6 | 7);
                if weapon_behind {
                    if let Some((wfp, wx, wy, oh)) = wpn_blit {
                        Self::blit_obj_to_framebuf(wfp, wx, wy, oh, framebuf, fb_w, fb_h);
                    }
                }
                if let Some(fp) = sheet.frame_pixels(body_frame) {
                    Self::blit_sprite_to_framebuf(fp, rel_x, rel_y, body_rows, framebuf, fb_w, fb_h);
                }
                if !weapon_behind {
                    if let Some((wfp, wx, wy, oh)) = wpn_blit {
                        Self::blit_obj_to_framebuf(wfp, wx, wy, oh, framebuf, fb_w, fb_h);
                    }
                }
            }
        }

        // --- Enemy NPCs from npc_table ---
        if let Some(ref table) = npc_table {
            for (npc_idx, npc) in table.npcs.iter().enumerate().filter(|(_, n)| n.active) {
                let (cfile_idx, override_frame) =
                    if let Some((ovr_cfile, ovr_frame)) = Self::swan_grounded_override(npc, state) {
                        (ovr_cfile, Some(ovr_frame))
                    } else {
                        let Some(c) = Self::npc_type_to_cfile(npc.npc_type, npc.race) else { continue };
                        (c, None)
                    };
                let Some(Some(ref sheet)) = sprite_sheets.get(cfile_idx) else { continue };

                let (rel_x, rel_y) = Self::actor_rel_pos(npc.x as u16, npc.y as u16, map_x, map_y);
                let frame = override_frame
                    .map(|f| f.min(sheet.num_frames.saturating_sub(1)))
                    .unwrap_or_else(|| Self::npc_animation_frame(npc, npc_idx, state.cycle, sheet.num_frames));

                if let Some(fp) = sheet.frame_pixels(frame) {
                    Self::blit_sprite_to_framebuf(fp, rel_x, rel_y, crate::game::sprites::SPRITE_H, framebuf, fb_w, fb_h);
                }
            }
        }
    }
}
