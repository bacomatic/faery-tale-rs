//! NPC AI decision system — ports do_tactic/set_course/select_tactic from fmain.c/fmain2.c.

use crate::game::actor::{Goal, Tactic};
use crate::game::npc::{Npc, NpcState};

/// Simple deterministic RNG for AI decisions.
/// Uses the same LCG family as encounter.rs.
fn ai_rand(tick: u32, salt: u32) -> u32 {
    tick.wrapping_mul(2246822519)
        .wrapping_add(salt)
        .wrapping_mul(1664525)
        .wrapping_add(1013904223)
}

/// Execute the current tactic — gates set_course behind a probabilistic check.
/// Ports do_tactic from fmain2.c:2075.
///
/// `leader_idx`: if Some(i), the index of the leader NPC in `npcs` for Follow/Evade.
/// `npcs`: read-only snapshot of NPC positions for Follow/Evade targets.
pub fn do_tactic(
    npc: &mut Npc,
    hero_x: i32,
    hero_y: i32,
    leader_idx: Option<usize>,
    npcs: &[(i32, i32)],
    tick: u32,
) {
    let r = ai_rand(tick, npc.x as u32 ^ npc.y as u32);

    // Probabilistic gate: ~12.5% for most goals, ~25% for Attack2/Archer2.
    let mask = match npc.goal {
        Goal::Attack2 | Goal::Archer2 => 3, // !(rand & 3) → 25%
        _ => 7,                              // !(rand & 7) → 12.5%
    };
    if (r & mask) != 0 {
        return; // No re-aim this tick.
    }

    match npc.tactic {
        Tactic::Pursue => {
            set_course(npc, hero_x, hero_y, SC_SMART);
        }
        Tactic::Shoot => {
            // Check if aligned on cardinal/diagonal axis for firing.
            let xd = (hero_x - npc.x as i32).abs();
            let yd = (hero_y - npc.y as i32).abs();
            let aligned = xd < 8 || yd < 8 || (xd > yd.saturating_sub(5) && xd < yd + 7);
            if aligned && (r >> 8) & 1 == 0 {
                set_course(npc, hero_x, hero_y, SC_AIM);
                npc.state = NpcState::Shooting;
            } else {
                set_course(npc, hero_x, hero_y, SC_SMART);
            }
        }
        Tactic::Random => {
            npc.facing = ((r >> 4) & 7) as u8;
            npc.state = NpcState::Walking;
        }
        Tactic::BumbleSeek => {
            set_course(npc, hero_x, hero_y, SC_BUMBLE);
        }
        Tactic::Backup => {
            set_course(npc, hero_x, hero_y, SC_FLEE);
        }
        Tactic::Follow => {
            if let Some(li) = leader_idx {
                if li < npcs.len() {
                    let (lx, ly) = npcs[li];
                    set_course(npc, lx, ly + 20, SC_SMART);
                } else {
                    npc.facing = ((r >> 4) & 7) as u8;
                    npc.state = NpcState::Walking;
                }
            } else {
                npc.facing = ((r >> 4) & 7) as u8;
                npc.state = NpcState::Walking;
            }
        }
        Tactic::Evade => {
            if let Some(li) = leader_idx {
                let neighbor = if li + 1 < npcs.len() {
                    li + 1
                } else if li > 0 {
                    li - 1
                } else {
                    li
                };
                if neighbor < npcs.len() {
                    let (nx, ny) = npcs[neighbor];
                    set_course(npc, nx, ny + 20, SC_DEVIATE2);
                }
            }
        }
        Tactic::EggSeek => {
            set_course(npc, 23087, 5667, SC_SMART);
            npc.state = NpcState::Walking;
        }
        Tactic::Frust | Tactic::None => {
            // Frustrated or idle — do nothing this tick.
        }
    }
}

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
    use crate::game::actor::{Goal, Tactic};
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

    #[test]
    fn test_do_tactic_pursue_gates_reaim() {
        // do_tactic with Pursue should only re-aim ~12.5% of the time.
        // Run 1000 iterations, count how many change facing.
        let mut reaim_count = 0u32;
        for tick in 0..1000u32 {
            let mut npc = make_npc(100, 100);
            npc.tactic = Tactic::Pursue;
            npc.goal = Goal::Attack1;
            npc.facing = 0; // facing North
            do_tactic(&mut npc, 200, 100, None, &[], tick);
            if npc.facing != 0 {
                reaim_count += 1;
            }
        }
        // ~12.5% = 125 ± margin. Should be between 50 and 250.
        assert!(reaim_count > 50, "too few re-aims: {reaim_count}");
        assert!(reaim_count < 250, "too many re-aims: {reaim_count}");
    }

    #[test]
    fn test_do_tactic_random_sets_random_facing() {
        let mut triggered = false;
        for tick in 0..100u32 {
            let mut npc = make_npc(100, 100);
            npc.tactic = Tactic::Random;
            npc.goal = Goal::Attack1;
            npc.state = NpcState::Still;
            do_tactic(&mut npc, 200, 100, None, &[], tick);
            if npc.state == NpcState::Walking {
                triggered = true;
                break;
            }
        }
        assert!(triggered, "Random tactic should trigger within 100 ticks");
    }

    #[test]
    fn test_do_tactic_backup_flees() {
        let mut triggered_away = false;
        for tick in 0..100u32 {
            let mut npc = make_npc(100, 100);
            npc.tactic = Tactic::Backup;
            npc.goal = Goal::Archer1;
            npc.facing = 2; // East (toward hero)
            do_tactic(&mut npc, 200, 100, None, &[], tick);
            if npc.facing == 6 {
                // West = away from hero at (200,100)
                triggered_away = true;
                break;
            }
        }
        assert!(triggered_away, "Backup should eventually face away from hero");
    }

    #[test]
    fn test_do_tactic_attack2_higher_reaim_rate() {
        let mut reaim_a1 = 0u32;
        let mut reaim_a2 = 0u32;
        for tick in 0..1000u32 {
            let mut npc1 = make_npc(100, 100);
            npc1.tactic = Tactic::Pursue;
            npc1.goal = Goal::Attack1;
            npc1.facing = 0;
            do_tactic(&mut npc1, 200, 100, None, &[], tick);
            if npc1.facing != 0 {
                reaim_a1 += 1;
            }

            let mut npc2 = make_npc(100, 100);
            npc2.tactic = Tactic::Pursue;
            npc2.goal = Goal::Attack2;
            npc2.facing = 0;
            do_tactic(&mut npc2, 200, 100, None, &[], tick);
            if npc2.facing != 0 {
                reaim_a2 += 1;
            }
        }
        assert!(
            reaim_a2 > reaim_a1,
            "Attack2 should re-aim more often: a1={reaim_a1}, a2={reaim_a2}"
        );
    }
}
