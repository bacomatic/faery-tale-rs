//! NPC AI decision system — ports do_tactic/set_course/select_tactic from fmain.c/fmain2.c.

use crate::game::npc::{Npc, NpcState};

/// set_course modes (from fmain2.c).
pub const SC_SMART: u8 = 0; // Smart seek — suppress minor axis
pub const SC_DEVIATE1: u8 = 1; // Smart + ±1 deviation when close
pub const SC_DEVIATE2: u8 = 2; // Smart + ±2 deviation when close
pub const SC_FLEE: u8 = 3; // Flee — negate direction
pub const SC_BUMBLE: u8 = 4; // Bumble — skip axis suppression
pub const SC_AIM: u8 = 5; // Aim only — set facing, don't walk
pub const SC_DIRECT: u8 = 6; // Direct — target is raw delta, not position

/// Compute facing and state from position delta.
/// Ports set_course from fmain2.c.
///
/// Modes 0–3: smart seek with axis suppression (if one axis > 2× the other, zero the minor).
/// Mode 0: plain smart seek.
/// Mode 1: + deviation ±1 when dist < 40.
/// Mode 2: + deviation ±2 when dist < 30.
/// Mode 3: flee (negate direction signs before lookup).
/// Mode 4: bumble (skip axis suppression — true diagonals).
/// Mode 5: aim only (set facing but do NOT set state to Walking).
/// Mode 6: direct (target_x/y are raw deltas, not world positions).
pub fn set_course(npc: &mut Npc, target_x: i32, target_y: i32, mode: u8) {
    let (dx, dy) = if mode == SC_DIRECT {
        (target_x, target_y)
    } else {
        (target_x - npc.x as i32, target_y - npc.y as i32)
    };

    if dx == 0 && dy == 0 {
        npc.state = NpcState::Still;
        return;
    }

    // Flee mode: negate direction.
    let (dx, dy) = if mode == SC_FLEE {
        (-dx, -dy)
    } else {
        (dx, dy)
    };

    let adx = dx.abs();
    let ady = dy.abs();

    // Axis suppression: modes 0–3 suppress minor axis if major > 2× minor.
    // Mode 4 (bumble) skips this entirely.
    let (eff_dx, eff_dy) = if mode <= SC_FLEE {
        let sx = if ady > adx * 2 { 0 } else { dx };
        let sy = if adx > ady * 2 { 0 } else { dy };
        (sx, sy)
    } else {
        (dx, dy)
    };

    // com2 lookup: map (xsign, ysign) to compass direction 0–7.
    // Index: signum + 1 → 0/1/2.
    const COM2: [[u8; 3]; 3] = [
        [7, 0, 1], // dy < 0: NW, N, NE
        [6, 9, 2], // dy = 0: W, STILL, E
        [5, 4, 3], // dy > 0: SW, S, SE
    ];
    let xi = (eff_dx.signum() + 1) as usize;
    let yi = (eff_dy.signum() + 1) as usize;
    let mut facing = COM2[yi][xi];

    if facing == 9 {
        npc.state = NpcState::Still;
        return;
    }

    // Deviation: modes 1 and 2 add random ±N when close to target.
    if mode == SC_DEVIATE1 && (adx + ady) < 40 {
        let coin = (npc.x.wrapping_mul(7).wrapping_add(npc.y)) & 1;
        if coin == 0 {
            facing = (facing.wrapping_add(1)) & 7;
        } else {
            facing = (facing.wrapping_sub(1)) & 7;
        }
    } else if mode == SC_DEVIATE2 && (adx + ady) < 30 {
        let coin = (npc.x.wrapping_mul(13).wrapping_add(npc.y)) & 1;
        if coin == 0 {
            facing = (facing.wrapping_add(2)) & 7;
        } else {
            facing = (facing.wrapping_sub(2)) & 7;
        }
    }

    npc.facing = facing;
    if mode != SC_AIM {
        npc.state = NpcState::Walking;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::npc::RACE_ENEMY;

    fn make_npc(x: i16, y: i16) -> Npc {
        Npc {
            npc_type: 6,
            race: RACE_ENEMY,
            x,
            y,
            vitality: 10,
            active: true,
            ..Default::default()
        }
    }

    #[test]
    fn test_set_course_smart_east() {
        let mut npc = make_npc(100, 100);
        set_course(&mut npc, 200, 100, SC_SMART);
        assert_eq!(npc.facing, 2); // East
        assert_eq!(npc.state, NpcState::Walking);
    }

    #[test]
    fn test_set_course_smart_north() {
        let mut npc = make_npc(100, 100);
        set_course(&mut npc, 100, 50, SC_SMART);
        assert_eq!(npc.facing, 0); // North
        assert_eq!(npc.state, NpcState::Walking);
    }

    #[test]
    fn test_set_course_smart_axis_suppression() {
        // Target far east, slightly south — should suppress Y axis → pure East.
        let mut npc = make_npc(100, 100);
        set_course(&mut npc, 300, 110, SC_SMART);
        assert_eq!(npc.facing, 2); // East (Y suppressed)
    }

    #[test]
    fn test_set_course_smart_diagonal() {
        // Target equally far NE — should get NE (facing 1).
        let mut npc = make_npc(100, 100);
        set_course(&mut npc, 200, 0, SC_SMART);
        assert_eq!(npc.facing, 1); // NE
    }

    #[test]
    fn test_set_course_flee_reversal() {
        // Flee mode: target east → NPC should face west.
        let mut npc = make_npc(100, 100);
        set_course(&mut npc, 200, 100, SC_FLEE);
        assert_eq!(npc.facing, 6); // West (flee from east)
        assert_eq!(npc.state, NpcState::Walking);
    }

    #[test]
    fn test_set_course_aim_only() {
        // Aim mode: set facing but don't change state to Walking.
        let mut npc = make_npc(100, 100);
        npc.state = NpcState::Still;
        set_course(&mut npc, 200, 100, SC_AIM);
        assert_eq!(npc.facing, 2); // East
        assert_eq!(npc.state, NpcState::Still); // NOT Walking
    }

    #[test]
    fn test_set_course_direct_mode() {
        // Direct mode: target_x/y are raw deltas, not world positions.
        let mut npc = make_npc(100, 100);
        set_course(&mut npc, 1, 0, SC_DIRECT); // raw delta: +X, 0Y → East
        assert_eq!(npc.facing, 2);
    }

    #[test]
    fn test_set_course_same_position() {
        // Same position → state should be Still (com2 returns 9).
        let mut npc = make_npc(100, 100);
        set_course(&mut npc, 100, 100, SC_SMART);
        assert_eq!(npc.state, NpcState::Still);
    }

    #[test]
    fn test_set_course_bumble_no_axis_suppression() {
        // Bumble mode: should NOT suppress minor axis, allowing true diagonals.
        let mut npc = make_npc(100, 100);
        // Far east, slightly south — in SMART mode this suppresses Y.
        // In BUMBLE mode it should keep the diagonal.
        set_course(&mut npc, 300, 130, SC_BUMBLE);
        assert_eq!(npc.facing, 3); // SE (not suppressed to E)
    }
}
