//! Carrier vehicle AI — turtle autonomous movement, facing helpers.
//! See `docs/spec/carriers.md` for specification.

use super::*;

impl GameplayScene {
    pub(crate) fn update_turtle_autonomous(&mut self) {
        use crate::game::collision::{newx, newy, px_to_terrain_type};
        use crate::game::game_state::CARRIER_TURTLE;

        if self.state.wcarry != 3
            || self.state.riding == 5
            || self.state.active_carrier != CARRIER_TURTLE
        {
            return;
        }

        let slot = self.state.wcarry as usize;
        if slot >= self.state.actors.len() {
            return;
        }

        // --- 16-tick hero-seeking facing update (CARRIER AI path,
        //     set_course mode 5 = SC_AIM: facing only, no state change). ---
        if self.state.tick_counter % 16 == 0 {
            let tx = self.state.actors[slot].abs_x as i32;
            let ty = self.state.actors[slot].abs_y as i32;
            let hx = self.state.hero_x as i32;
            let hy = self.state.hero_y as i32;
            if let Some(dir) = Self::facing_toward(tx, ty, hx, hy) {
                self.state.actors[slot].facing = dir;
            }
        }

        // --- Per-tick water-direction probe. ---
        let turtle_x = self.state.actors[slot].abs_x;
        let turtle_y = self.state.actors[slot].abs_y;
        let facing = self.state.actors[slot].facing;

        const DIR_OFFSETS: [i8; 4] = [0, 1, -1, -2];
        const TURTLE_SPEED: i32 = 3;

        let result: Option<(u16, u16)> = if let Some(ref world) = self.map_world {
            let mut found = None;
            for &off in &DIR_OFFSETS {
                let probe_dir = facing.wrapping_add(off as u8) & 7;
                let nx = newx(turtle_x, probe_dir, TURTLE_SPEED);
                let ny = newy(turtle_y, probe_dir, TURTLE_SPEED);
                // Ref carrier_tick (fmain.c:1525-1537): single-point `px_to_im(xtest, ytest) != 5`
                // guards each probe — not the 2-foot `prox` test. Turtle may straddle a
                // water/non-water boundary as long as its centre sits on terrain 5.
                if px_to_terrain_type(world, nx as i32, ny as i32) == 5 {
                    found = Some((nx, ny));
                    break;
                }
            }
            found
        } else {
            None
        };

        match result {
            Some((nx, ny)) => {
                self.state.actors[slot].abs_x = nx;
                self.state.actors[slot].abs_y = ny;
                self.state.actors[slot].moving = true;
            }
            None => {
                // No valid water direction — stay put. Do NOT mutate facing;
                // that is the CARRIER AI path's job at 16-tick cadence.
                self.state.actors[slot].moving = false;
            }
        }
    }

    pub(crate) fn facing_toward(x0: i32, y0: i32, x1: i32, y1: i32) -> Option<u8> {
        let dx = x1 - x0;
        let dy = y1 - y0;
        if dx == 0 && dy == 0 {
            return None;
        }
        // atan2(dx, -dy): north is -y, so angle 0 = up. Then divide into 8 octants.
        let angle = (dx as f32).atan2(-dy as f32); // radians, (-π, π]
        let two_pi = std::f32::consts::TAU;
        let normalized = (angle + two_pi) % two_pi; // [0, 2π)
        let octant = ((normalized / two_pi) * 8.0 + 0.5).floor() as i32 & 7;
        Some(octant as u8)
    }
}
