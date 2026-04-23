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
/// Ports do_tactic from fmain2.c:1664-1700 (see RESEARCH §8.2).
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
    // CONFUSED: no AI processing on any tick (§11.9). First-tick random walk was already
    // executed directly inside select_tactic when the goal was assigned.
    if npc.goal == Goal::Confused {
        return;
    }

    let r = ai_rand(tick, npc.x as u32 ^ npc.y as u32);

    // Per RESEARCH §8.2 (fmain2.c:1666-1669): SHOOT bypasses the rate-limit; all
    // other tactics are gated at 12.5% default, upgraded to 25% only when
    // goal == ATTACK2 (clever melee). ARCHER2 is NOT in the 25% bucket here;
    // the 25% fast-reconsider for ATTACK1/ARCHER2 lives in select_tactic's
    // re-plan gate (a separate clock per ai-system.md#advance_goal).
    if npc.tactic != Tactic::Shoot {
        let mask: u32 = if npc.goal == Goal::Attack2 { 3 } else { 7 };
        if (r & mask) != 0 {
            return; // No re-aim this tick.
        }
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

/// Advance one NPC's AI by one tick, with a freeze gate.
///
/// Per SPEC §19.3: when `freeze` is true (i.e. `freeze_timer > 0`), hostile NPCs
/// (`race < 7`) skip all AI — they do not change tactic, facing, or movement state.
/// Non-hostile NPCs (`race >= 7`, e.g. shopkeepers, villagers) are always processed.
///
/// Per SPEC §11.5 step 3 and ref ai-system.md:46 (`if actor.type == SETFIG: return`):
/// SETFIG actors (`race >= 0x80`) short-circuit the entire goal FSM. No facing,
/// state, goal, or tactic changes occur on any tick. Their spawn-time pose is
/// whatever `set_shape` / init code assigned.
pub fn tick_npc(
    npc: &mut Npc,
    npc_idx: usize,
    hero_x: i32,
    hero_y: i32,
    hero_dead: bool,
    leader_idx: Option<usize>,
    npcs: &[(i32, i32)],
    tick: u32,
    xtype: u16,
    turtle_eggs: bool,
    freeze: bool,
) {
    // SETFIG actors (race >= 0x80): return unconditionally — ref ai-system.md:46.
    if npc.race >= 0x80 {
        return;
    }

    if freeze && npc.race < 7 {
        return;
    }
    select_tactic(npc, npc_idx, hero_x, hero_y, hero_dead, leader_idx, xtype, turtle_eggs, tick);
    do_tactic(npc, hero_x, hero_y, leader_idx, npcs, tick);
}

/// Numeric GOAL_* value matching SYMBOLS.md:275-286. Used for melee-reach
/// threshold `thresh = 14 - mode` at ref ai-system.md:103.
fn goal_numeric(goal: &Goal) -> i32 {
    match goal {
        Goal::User => 0,
        Goal::Attack1 => 1,
        Goal::Attack2 => 2,
        Goal::Archer1 => 3,
        Goal::Archer2 => 4,
        Goal::Flee => 5,
        Goal::Stand => 6,
        // GOAL_DEATH = 7 (no Rust equivalent in this enum)
        // GOAL_WAIT = 8 (no Rust equivalent)
        Goal::Follower => 9,
        Goal::Confused => 10,
        _ => 0,
    }
}

/// Select tactic for this NPC based on goal, distance, state.
/// Ports `advance_goal` from fmain.c:2109-2183 (see `reference/logic/ai-system.md`).
///
/// `npc_idx`: this NPC's slot index in the actor table — used for the hero-dead
///   leader/follower split (ref ai-system.md:60-64: first iterated actor becomes
///   `leader`, later ones become FOLLOWERS).
/// `hero_dead`: true if hero is dead/falling (STATE_DEAD or STATE_FALL).
/// `leader_idx`: pre-computed first-active-hostile slot index.
/// `xtype`: special-encounter terrain classifier.
/// `turtle_eggs`: true when the global turtle-eggs counter is non-zero (ref
///   ai-system.md:84). When false, snake race falls through to the normal AI.
/// `tick`: current game tick for RNG.
pub fn select_tactic(
    npc: &mut Npc,
    npc_idx: usize,
    hero_x: i32,
    hero_y: i32,
    hero_dead: bool,
    leader_idx: Option<usize>,
    xtype: u16,
    turtle_eggs: bool,
    tick: u32,
) {
    let r = ai_rand(tick, npc.x as u32 ^ (npc.y as u32).wrapping_mul(3));

    // === Forced mode overrides (ref ai-system.md:58-67) ===

    // Hero dead/fall: first active hostile takes FLEE, rest take FOLLOWER
    // (ref fmain.c:2133-2136, 2183 — leader starts at 0, set after each iteration).
    if hero_dead {
        let is_first_leader = match leader_idx {
            Some(li) => li == npc_idx,
            None => true,
        };
        npc.goal = if is_first_leader { Goal::Flee } else { Goal::Follower };
    }

    // Vitality critically low → FLEE (ref ai-system.md:65).
    // SPEC-GAP: original gate is `xtype > 59 && race != extn.v3`; we use the
    // placeholder `race < 4` until `extn` is plumbed. For DKnight (race 7,
    // v3 = 7) both conditions agree — DKnight does not flee in his zone.
    if npc.vitality < 2 || (xtype > 59 && npc.race < 4) {
        npc.goal = Goal::Flee;
    }

    // === Frust dispatch (ref ai-system.md:70-75, fmain.c:2141-2144) ===
    // Must run BEFORE the mode branches: a frust-latched actor in any goal
    // (including FLEE/FOLLOWER/STAND/WAIT/CONFUSED) receives a random fallback
    // tactic. See frustration.md#resolve_frust_tactic.
    if npc.tactic == Tactic::Frust {
        let rr = r >> 16;
        npc.tactic = if (npc.weapon & 4) != 0 {
            // bow/wand: rand(2, 5) → FOLLOW, BUMBLE_SEEK, RANDOM, BACKUP
            match rr & 3 {
                0 => Tactic::Follow,
                1 => Tactic::BumbleSeek,
                2 => Tactic::Random,
                _ => Tactic::Backup,
            }
        } else {
            // melee: rand(3, 4) → BUMBLE_SEEK, RANDOM
            if rr & 1 == 0 { Tactic::BumbleSeek } else { Tactic::Random }
        };
        return;
    }

    // === Non-hostile goal modes (bypass tactic tree) ===
    match npc.goal {
        Goal::Flee => {
            npc.tactic = Tactic::Backup;
            return;
        }
        Goal::Follower => {
            npc.tactic = Tactic::Follow;
            return;
        }
        Goal::Stand => {
            // Ref ai-system.md:121-122: set_course(mode=0 smart seek) + stop_motion.
            // SC_SMART applies axis suppression; then state is forced Still.
            set_course(npc, hero_x, hero_y, SC_SMART);
            npc.state = NpcState::Still;
            return;
        }
        Goal::None | Goal::User | Goal::Leader => {
            npc.state = NpcState::Still;
            return;
        }
        // CONFUSED: goal already set on first tick (with random walk executed). Subsequent
        // ticks bypass all tactic processing — actor continues last trajectory (§11.9).
        Goal::Confused => return,
        _ => {} // Attack/Archer goals continue to tactic tree.
    }

    // === Close-range melee check + DKnight stand_guard (ref ai-system.md:102-114) ===
    let xd = (hero_x - npc.x as i32).abs();
    let yd = (hero_y - npc.y as i32).abs();
    // Ref: thresh = 14 - mode (numeric GOAL_*); dark knight (race 7) uses 16.
    let thresh = if npc.race == 7 { 16 } else { 14 - goal_numeric(&npc.goal) };
    // Ref: `(weapon & 4) == 0` — bit 2 clear means melee (non-bow/wand).
    let is_melee = (npc.weapon & 4) == 0;
    if is_melee && xd < thresh && yd < thresh {
        // Inside melee range with a non-bow weapon: aim directly and fight.
        set_course(npc, hero_x, hero_y, SC_DIRECT);
        npc.state = NpcState::Fighting;
        return;
    }
    if npc.race == 7 && npc.vitality > 0 {
        // Living dark knight outside melee reach: stand_guard — state = STILL,
        // facing = DIR_S (south). Ref ai-system.md:110-112 (fmain.c:2168-2169).
        npc.state = NpcState::Still;
        npc.facing = 4;
        return;
    }

    // === Re-plan gate (ref ai-system.md:80-81, fmain.c:2132, 2148) ===
    // `r = chance(1, 16)` baseline; `(mode & 2) == 0` upgrades to `chance(1, 4)`.
    // Numeric values: ATTACK1=1 (bit1=0, 1/4), ATTACK2=2 (bit1=1, 1/16),
    // ARCHER1=3 (bit1=1, 1/16), ARCHER2=4 (bit1=0, 1/4).
    let gate_mask = match npc.goal {
        Goal::Attack1 | Goal::Archer2 => 3, // ~25%
        _ => 15,                             // ~6.25%
    };
    if (r & gate_mask) != 0 {
        return; // Keep current tactic this tick.
    }

    // === Tactic decision tree (ref ai-system.md:83-101) ===

    // Snake + turtle_eggs global: march to nest coords (ref line 84).
    if npc.race == 4 && turtle_eggs {
        npc.tactic = Tactic::EggSeek;
        return;
    }

    // Disarmed → CONFUSED + first-tick random walk.
    if npc.weapon == 0 {
        npc.goal = Goal::Confused;
        npc.tactic = Tactic::Random;
        npc.facing = ((r >> 4) & 7) as u8;
        npc.state = NpcState::Walking;
        return;
    }

    // Wounded evade: vitality < 6 AND coin flip (ref line 90).
    if npc.vitality < 6 && (r >> 8) & 1 == 0 {
        npc.tactic = Tactic::Evade;
        return;
    }

    // Archer range bands (ref lines 94-99).
    let is_archer = matches!(npc.goal, Goal::Archer1 | Goal::Archer2);
    if is_archer {
        if xd < 40 && yd < 30 {
            npc.tactic = Tactic::Backup;
            return;
        }
        if xd < 70 && yd < 70 {
            npc.tactic = Tactic::Shoot;
            return;
        }
        npc.tactic = Tactic::Pursue;
        return;
    }

    // Default melee → pursue.
    npc.tactic = Tactic::Pursue;
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

    #[test]
    fn test_select_tactic_dead_hero_causes_flee() {
        let mut npc = make_npc(100, 100);
        npc.goal = Goal::Attack1;
        npc.tactic = Tactic::Pursue;
        select_tactic(&mut npc, 0, 200, 100, true, None, 0, false, 42);
        assert_eq!(npc.goal, Goal::Flee);
    }

    #[test]
    fn test_select_tactic_low_vitality_flees() {
        let mut npc = make_npc(100, 100);
        npc.goal = Goal::Attack1;
        npc.vitality = 1;
        select_tactic(&mut npc, 0, 200, 100, false, None, 0, false, 42);
        assert_eq!(npc.goal, Goal::Flee);
    }

    #[test]
    fn test_select_tactic_archer_close_backups() {
        let mut backed_up = false;
        for tick in 0..200u32 {
            let mut npc = make_npc(100, 100);
            npc.goal = Goal::Archer1;
            npc.weapon = 4;
            npc.tactic = Tactic::Pursue;
            select_tactic(&mut npc, 0, 120, 110, false, None, 0, false, tick);
            if npc.tactic == Tactic::Backup {
                backed_up = true;
                break;
            }
        }
        assert!(backed_up, "Archer should select Backup when hero is close");
    }

    #[test]
    fn test_select_tactic_archer_in_range_shoots() {
        let mut shooting = false;
        for tick in 0..200u32 {
            let mut npc = make_npc(100, 100);
            npc.goal = Goal::Archer1;
            npc.weapon = 4;
            npc.tactic = Tactic::Pursue;
            select_tactic(&mut npc, 0, 160, 140, false, None, 0, false, tick);
            if npc.tactic == Tactic::Shoot {
                shooting = true;
                break;
            }
        }
        assert!(shooting, "Archer should select Shoot when hero is in range");
    }

    #[test]
    fn test_select_tactic_melee_close_range_fighting() {
        let mut fighting = false;
        for tick in 0..200u32 {
            let mut npc = make_npc(100, 100);
            npc.goal = Goal::Attack1;
            npc.weapon = 1;
            select_tactic(&mut npc, 0, 105, 105, false, None, 0, false, tick);
            if npc.state == NpcState::Fighting {
                fighting = true;
                break;
            }
        }
        assert!(fighting, "Melee NPC should enter Fighting at close range");
    }

    #[test]
    fn test_select_tactic_no_weapon_confused() {
        let mut confused = false;
        for tick in 0..200u32 {
            let mut npc = make_npc(100, 100);
            npc.goal = Goal::Attack1;
            npc.weapon = 0;
            select_tactic(&mut npc, 0, 200, 100, false, None, 0, false, tick);
            if npc.tactic == Tactic::Random {
                // Updated: goal must also be Confused (§11.9).
                assert_eq!(npc.goal, Goal::Confused, "weapon=0 NPC must have Goal::Confused");
                confused = true;
                break;
            }
        }
        assert!(confused, "Weaponless NPC should get tactic=Random (confused)");
    }

    #[test]
    fn test_select_tactic_flee_goal_stays_backup() {
        let mut npc = make_npc(100, 100);
        npc.goal = Goal::Flee;
        npc.tactic = Tactic::Pursue;
        select_tactic(&mut npc, 0, 200, 100, false, None, 0, false, 42);
        assert_eq!(npc.tactic, Tactic::Backup);
    }

    #[test]
    fn test_select_tactic_stand_goal_stays_still() {
        let mut npc = make_npc(100, 100);
        npc.goal = Goal::Stand;
        select_tactic(&mut npc, 0, 200, 100, false, None, 0, false, 42);
        assert_eq!(npc.state, NpcState::Still);
    }

    #[test]
    fn test_select_tactic_reconsider_probabilities() {
        // Test that all four goal types have correct reconsider probabilities per SPEC §11.7:
        // - ATTACK1 and ARCHER2 should reconsider at 25% (gate_mask = 3)
        // - ATTACK2 and ARCHER1 should reconsider at 6.25% (gate_mask = 15)
        
        const ITERATIONS: u32 = 2000;
        let mut reconsider_attack1 = 0u32;
        let mut reconsider_attack2 = 0u32;
        let mut reconsider_archer1 = 0u32;
        let mut reconsider_archer2 = 0u32;
        
        for tick in 0..ITERATIONS {
            // For archers, place hero close enough to trigger Backup tactic when reconsidering
            // (xd < 40 && yd < 30). Start with Pursue tactic.
            // If it stays Pursue, the gate blocked reconsideration.
            // If it changes to Backup, reconsideration happened.
            
            // Test ARCHER1 (should be ~6.25%)
            let mut npc = make_npc(100, 100);
            npc.goal = Goal::Archer1;
            npc.weapon = 4; // bow
            npc.tactic = Tactic::Pursue;
            select_tactic(&mut npc, 0, 120, 110, false, None, 0, false, tick); // xd=20, yd=10 → Backup
            if npc.tactic == Tactic::Backup {
                reconsider_archer1 += 1;
            }
            
            // Test ARCHER2 (should be ~25%)
            let mut npc = make_npc(100, 100);
            npc.goal = Goal::Archer2;
            npc.weapon = 4;
            npc.tactic = Tactic::Pursue;
            select_tactic(&mut npc, 0, 120, 110, false, None, 0, false, tick); // xd=20, yd=10 → Backup
            if npc.tactic == Tactic::Backup {
                reconsider_archer2 += 1;
            }
            
            // For melee, we need a condition that changes tactic.
            // Use low vitality (< 6) to potentially trigger Evade.
            // The evade check also has a 50% chance (r >> 8) & 1 == 0.
            // So we expect: reconsider_rate * 0.5 = observed_evade_rate
            // ATTACK1 at 25% * 0.5 = 12.5%
            // ATTACK2 at 6.25% * 0.5 = 3.125%
            
            // Test ATTACK1 (should be ~25%, so ~12.5% will trigger Evade)
            let mut npc = make_npc(100, 100);
            npc.goal = Goal::Attack1;
            npc.weapon = 1;
            npc.vitality = 3; // < 6
            npc.tactic = Tactic::Pursue;
            select_tactic(&mut npc, 0, 200, 100, false, None, 0, false, tick);
            if npc.tactic == Tactic::Evade {
                reconsider_attack1 += 1;
            }
            
            // Test ATTACK2 (should be ~6.25%, so ~3.125% will trigger Evade)
            let mut npc = make_npc(100, 100);
            npc.goal = Goal::Attack2;
            npc.weapon = 1;
            npc.vitality = 3;
            npc.tactic = Tactic::Pursue;
            select_tactic(&mut npc, 0, 200, 100, false, None, 0, false, tick);
            if npc.tactic == Tactic::Evade {
                reconsider_attack2 += 1;
            }
        }
        
        // Allow ±5% margin for probabilistic tests
        
        // ARCHER1: 6.25% of 2000 = 125 ± 50
        assert!(
            reconsider_archer1 > 75 && reconsider_archer1 < 175,
            "ARCHER1 should reconsider at ~6.25%: got {}/2000 = {}%",
            reconsider_archer1,
            reconsider_archer1 * 100 / ITERATIONS
        );
        
        // ARCHER2: 25% of 2000 = 500 ± 100
        assert!(
            reconsider_archer2 > 400 && reconsider_archer2 < 600,
            "ARCHER2 should reconsider at ~25%: got {}/2000 = {}%",
            reconsider_archer2,
            reconsider_archer2 * 100 / ITERATIONS
        );
        
        // ATTACK1: 25% * 50% = 12.5% of 2000 = 250 ± 75
        assert!(
            reconsider_attack1 > 175 && reconsider_attack1 < 325,
            "ATTACK1 should reconsider at ~25% (evade at ~12.5%): got {}/2000 = {}%",
            reconsider_attack1,
            reconsider_attack1 * 100 / ITERATIONS
        );
        
        // ATTACK2: 6.25% * 50% = 3.125% of 2000 = 62.5 ± 40
        assert!(
            reconsider_attack2 > 20 && reconsider_attack2 < 100,
            "ATTACK2 should reconsider at ~6.25% (evade at ~3.125%): got {}/2000 = {}%",
            reconsider_attack2,
            reconsider_attack2 * 100 / ITERATIONS
        );
        
        // Verify the 4x relationship between high and low reconsider rates
        assert!(
            reconsider_archer2 > reconsider_archer1 * 3,
            "ARCHER2 ({}) should reconsider ~4x more than ARCHER1 ({})",
            reconsider_archer2,
            reconsider_archer1
        );
        assert!(
            reconsider_attack1 > reconsider_attack2 * 3,
            "ATTACK1 ({}) should reconsider ~4x more than ATTACK2 ({})",
            reconsider_attack1,
            reconsider_attack2
        );
    }

    // T3-COMBAT-FREEZE-CAST: freeze spell tests (SPEC §19.2, §19.3).

    #[test]
    fn test_freeze_hostile_npc_does_not_act() {
        // SPEC §19.3: freeze_timer > 0 → hostile enemies (race < 7) skip all AI.
        // (b) frozen actors do not move on tick.
        let mut npc = make_npc(100, 100);
        npc.goal = Goal::Attack1;
        npc.tactic = Tactic::Pursue;
        npc.weapon = 1;
        npc.state = NpcState::Still;
        let initial_facing = npc.facing;

        for tick in 0..100u32 {
            tick_npc(&mut npc, 0, 200, 100, false, None, &[], tick, 0, false, true);
            assert_eq!(
                npc.state, NpcState::Still,
                "Frozen hostile NPC changed state at tick {tick}"
            );
            assert_eq!(
                npc.facing, initial_facing,
                "Frozen hostile NPC changed facing at tick {tick}"
            );
        }
    }

    #[test]
    fn test_freeze_nonhostile_npc_still_acts() {
        // SPEC §19.3: NPCs with race >= 7 (e.g. shopkeepers) are not frozen enemies.
        // Ref ai-system.md:46 `if actor.type == SETFIG: return` — SETFIG actors short-
        // circuit advance_goal unconditionally. Shopkeeper (race 0x88, SETFIG) therefore
        // does NOT re-aim at the hero per tick; its spawn-time pose is preserved.
        use crate::game::npc::RACE_SHOPKEEPER;
        let mut npc = Npc {
            npc_type: 1,
            race: RACE_SHOPKEEPER, // 0x88 ≥ 7 — not a combat enemy
            x: 100,
            y: 100,
            vitality: 10,
            active: true,
            goal: Goal::Stand,
            facing: 4, // spawn facing south
            ..Default::default()
        };
        // SETFIG short-circuit: state/facing/goal must not be touched, even with freeze=true.
        tick_npc(&mut npc, 0, 200, 100, false, None, &[], 0, 0, false, true);
        assert_eq!(npc.state, NpcState::Still);
        assert_eq!(npc.facing, 4, "SETFIG shopkeeper spawn facing must be preserved");
    }

    #[test]
    fn test_unfrozen_hostile_npc_can_act() {
        // When not frozen, hostile NPCs receive AI updates normally.
        let mut acted = false;
        for tick in 0..200u32 {
            let mut npc = make_npc(100, 100);
            npc.goal = Goal::Attack1;
            npc.tactic = Tactic::Pursue;
            npc.weapon = 1;
            npc.facing = 0;
            npc.state = NpcState::Still;
            tick_npc(&mut npc, 0, 200, 100, false, None, &[], tick, 0, false, false);
            if npc.state != NpcState::Still || npc.facing != 0 {
                acted = true;
                break;
            }
        }
        assert!(acted, "Unfrozen hostile NPC should eventually change state or facing");
    }

    #[test]
    fn test_freeze_expires_after_duration() {
        // SPEC §19.2, §19.3: freeze_timer decrements by 1 each tick; expires at 0.
        // (c) freeze expires after spec'd duration (FREEZE_TIMER_INCREMENT = 100 ticks).
        use crate::game::game_state::GameState;
        use crate::game::magic::FREEZE_TIMER_INCREMENT;
        let mut state = GameState::new();
        state.freeze_timer = FREEZE_TIMER_INCREMENT;
        // Tick exactly FREEZE_TIMER_INCREMENT times.
        state.tick(FREEZE_TIMER_INCREMENT as u32);
        assert_eq!(state.freeze_timer, 0, "Freeze should expire after {} ticks", FREEZE_TIMER_INCREMENT);
    }

    #[test]
    fn test_freeze_cast_sets_nonzero_timer() {
        // SPEC §19.2: Gold Ring cast increments freeze_timer by FREEZE_TIMER_INCREMENT.
        // (a) cast applies freeze effect to game state.
        use crate::game::game_state::GameState;
        use crate::game::magic::{use_magic, ITEM_RING, FREEZE_TIMER_INCREMENT};
        let mut state = GameState::new();
        state.stuff_mut()[ITEM_RING] = 1;
        let _ = use_magic(&mut state, ITEM_RING);
        assert!(state.freeze_timer > 0, "freeze_timer must be > 0 after cast");
        assert_eq!(state.freeze_timer, FREEZE_TIMER_INCREMENT);
    }

    // ── T3-COMBAT-CONFUSED: CONFUSED goal tests (§11.9) ────────────────────

    /// (a) A hostile actor with weapon=0 is assigned Goal::Confused by select_tactic.
    #[test]
    fn test_confused_goal_assigned_when_unarmed() {
        // select_tactic must reliably set Goal::Confused on an unarmed hostile; the recalc
        // gate fires probabilistically so we run several ticks and check at least one hits.
        let mut got_confused = false;
        for tick in 0..200u32 {
            let mut npc = make_npc(100, 100);
            npc.goal = Goal::Attack1;
            npc.weapon = 0; // unarmed
            select_tactic(&mut npc, 0, 200, 100, false, None, 0, false, tick);
            if npc.goal == Goal::Confused {
                got_confused = true;
                break;
            }
        }
        assert!(got_confused, "unarmed hostile must receive Goal::Confused");
    }

    /// (b) On the first CONFUSED tick, actor walks one step in a random direction.
    /// select_tactic must set state=Walking and a valid facing (0–7).
    #[test]
    fn test_confused_first_tick_random_walk() {
        // Force the gate to fire by iterating ticks; check Walking + valid facing.
        let mut found = false;
        for tick in 0..200u32 {
            let mut npc = make_npc(100, 100);
            npc.goal = Goal::Attack1;
            npc.weapon = 0;
            npc.state = NpcState::Still;
            select_tactic(&mut npc, 0, 200, 100, false, None, 0, false, tick);
            if npc.goal == Goal::Confused {
                assert_eq!(npc.state, NpcState::Walking, "first CONFUSED tick must set Walking");
                assert!(npc.facing <= 7, "facing must be 0–7, got {}", npc.facing);
                // do_tactic must be a no-op for Confused — confirm it doesn't touch state.
                let facing_before = npc.facing;
                do_tactic(&mut npc, 200, 100, None, &[], tick);
                assert_eq!(
                    npc.state, NpcState::Walking,
                    "do_tactic must not change state for Confused actor"
                );
                assert_eq!(
                    npc.facing, facing_before,
                    "do_tactic must not change facing for Confused actor"
                );
                found = true;
                break;
            }
        }
        assert!(found, "CONFUSED assignment not observed within 200 ticks");
    }

    /// (c) On subsequent ticks with Goal::Confused, neither select_tactic nor do_tactic
    /// modifies the actor (no new direction, no attack, no tactic change).
    #[test]
    fn test_confused_subsequent_tick_no_processing() {
        // Bootstrap: get actor into Confused state on tick T.
        let seed_tick = (0u32..200)
            .find(|&t| {
                let mut npc = make_npc(100, 100);
                npc.goal = Goal::Attack1;
                npc.weapon = 0;
                select_tactic(&mut npc, 0, 200, 100, false, None, 0, false, t);
                npc.goal == Goal::Confused
            })
            .expect("could not bootstrap Confused state");

        let mut npc = make_npc(100, 100);
        npc.goal = Goal::Attack1;
        npc.weapon = 0;
        select_tactic(&mut npc, 0, 200, 100, false, None, 0, false, seed_tick);
        assert_eq!(npc.goal, Goal::Confused);

        // Snapshot state after first tick.
        let facing_after_first = npc.facing;
        let state_after_first = npc.state.clone();
        let tactic_after_first = npc.tactic.clone();

        // Simulate 10 subsequent ticks: neither facing nor tactic should change.
        for tick in (seed_tick + 1)..(seed_tick + 11) {
            select_tactic(&mut npc, 0, 200, 100, false, None, 0, false, tick);
            do_tactic(&mut npc, 200, 100, None, &[], tick);

            assert_eq!(npc.goal, Goal::Confused, "Confused goal must persist on tick {tick}");
            assert_eq!(
                npc.facing, facing_after_first,
                "facing must not change on subsequent Confused ticks (tick {tick})"
            );
            assert_eq!(
                npc.state, state_after_first,
                "state must not change on subsequent Confused ticks (tick {tick})"
            );
            assert_eq!(
                npc.tactic, tactic_after_first,
                "tactic must not change on subsequent Confused ticks (tick {tick})"
            );
        }
    }

    /// (d) Restoring a weapon: if the caller resets the goal back to a hostile mode and
    /// calls select_tactic with weapon > 0, re-evaluation assigns a pursuit tactic (not Random).
    /// This tests that CONFUSED machinery only fires when weapon == 0.
    #[test]
    fn test_confused_lifted_when_weapon_restored() {
        // An actor was Confused; its weapon is restored and goal is reset to Attack1
        // (simulating external re-spawn or loot pickup).
        let mut got_pursue = false;
        for tick in 0..200u32 {
            let mut npc = make_npc(100, 100);
            npc.goal = Goal::Attack1; // goal reset by caller
            npc.weapon = 1;           // weapon restored
            npc.tactic = Tactic::Random; // leftover from confused period
            // Hero is far → no melee engage.
            select_tactic(&mut npc, 0, 200, 100, false, None, 0, false, tick);
            // Should NOT assign Confused; when gate fires, should choose Pursue.
            assert_ne!(
                npc.goal,
                Goal::Confused,
                "armed actor must not receive Goal::Confused"
            );
            if npc.tactic == Tactic::Pursue {
                got_pursue = true;
                break;
            }
        }
        assert!(got_pursue, "re-armed actor must eventually select Pursue tactic");
    }

    // ── T4-NPC-SETFIG: SETFIG AI exclusion tests (§11.5 step 3) ────────────

    /// (a) SETFIG actor (race=0x89 Witch) does NOT move when ticked with generic AI.
    /// tick_npc must return early for race >= 0x80 without changing state or tactic.
    #[test]
    fn test_setfig_witch_does_not_move() {
        use crate::game::npc::RACE_WITCH;
        let mut npc = Npc {
            npc_type: 9,
            race: RACE_WITCH, // 0x89 — SETFIG
            x: 100,
            y: 100,
            vitality: 10,
            weapon: 2,
            active: true,
            goal: Goal::Attack1, // hostile goal — must be suppressed
            tactic: Tactic::Pursue,
            state: NpcState::Still,
            facing: 0,
            ..Default::default()
        };
        for tick in 0..200u32 {
            tick_npc(&mut npc, 0, 200, 100, false, None, &[], tick, 0, false, false);
            assert_eq!(
                npc.state,
                NpcState::Still,
                "SETFIG Witch must not change state at tick {tick}"
            );
            // Tactic and goal must remain unchanged — no hostile AI.
            assert_eq!(
                npc.tactic,
                Tactic::Pursue,
                "SETFIG Witch tactic must not change at tick {tick}"
            );
            assert_eq!(
                npc.goal,
                Goal::Attack1,
                "SETFIG Witch goal must not change at tick {tick}"
            );
        }
    }

    /// (b) Regular hostile (race < 7) still moves normally after SETFIG gate.
    #[test]
    fn test_regular_hostile_still_moves() {
        let mut acted = false;
        for tick in 0..200u32 {
            let mut npc = make_npc(100, 100); // race = RACE_ENEMY (3 < 7)
            npc.goal = Goal::Attack1;
            npc.weapon = 1;
            npc.state = NpcState::Still;
            npc.facing = 0;
            tick_npc(&mut npc, 0, 200, 100, false, None, &[], tick, 0, false, false);
            if npc.state != NpcState::Still || npc.facing != 0 {
                acted = true;
                break;
            }
        }
        assert!(acted, "Regular hostile NPC must still receive AI updates");
    }

    /// (c) SETFIG shopkeeper with Stand goal: per ref ai-system.md:46
    /// (`if actor.type == SETFIG: return`), advance_goal is a no-op for SETFIG
    /// actors — their spawn pose (state, facing, tactic) is preserved verbatim.
    #[test]
    fn test_setfig_shopkeeper_stand_preserves_spawn_pose() {
        use crate::game::npc::RACE_SHOPKEEPER;
        let mut npc = Npc {
            npc_type: 1,
            race: RACE_SHOPKEEPER, // 0x88 — SETFIG
            x: 100,
            y: 100,
            vitality: 10,
            active: true,
            goal: Goal::Stand,
            facing: 4, // spawn facing south
            state: NpcState::Still,
            ..Default::default()
        };
        // Hero is due East at (200, 100); SETFIG must NOT re-orient.
        tick_npc(&mut npc, 0, 200, 100, false, None, &[], 0, 0, false, false);
        assert_eq!(
            npc.state,
            NpcState::Still,
            "SETFIG shopkeeper must stay Still"
        );
        assert_eq!(
            npc.facing, 4,
            "SETFIG shopkeeper spawn facing must be preserved (ref short-circuits AI)"
        );
    }

    // ── Subsystem 3 (ai-system) audit — targeted regression tests ──────────

    /// F3.4: a living dark knight (race 7) that is OUT of melee reach with a
    /// melee weapon stands still facing south — ref ai-system.md:110-112.
    #[test]
    fn test_dknight_stand_guard_out_of_reach() {
        let mut npc = Npc {
            npc_type: 6,
            race: 7, // DKnight
            x: 100,
            y: 100,
            vitality: 10,
            weapon: 2, // melee (bit 2 = 0)
            active: true,
            goal: Goal::Attack1,
            tactic: Tactic::Pursue,
            state: NpcState::Walking,
            facing: 2, // east before tick
            ..Default::default()
        };
        // Hero far away (300 away) — beyond DKnight thresh of 16.
        select_tactic(&mut npc, 0, 400, 100, false, None, 0, false, 42);
        assert_eq!(npc.state, NpcState::Still, "DKnight out of reach → STILL");
        assert_eq!(npc.facing, 4, "DKnight stand_guard faces south");
    }

    /// F3.4: a dead dark knight (vitality == 0) does NOT stand_guard — falls
    /// through to the normal AI pipeline (ref: `vitality != 0` gate).
    #[test]
    fn test_dknight_stand_guard_only_when_alive() {
        let mut npc = Npc {
            npc_type: 6,
            race: 7,
            x: 100,
            y: 100,
            vitality: 0, // dead
            weapon: 2,
            active: true,
            goal: Goal::Attack1,
            ..Default::default()
        };
        // vitality < 2 forces FLEE override, which pre-empts stand_guard.
        select_tactic(&mut npc, 0, 400, 100, false, None, 0, false, 42);
        assert_eq!(npc.goal, Goal::Flee);
        assert_eq!(npc.tactic, Tactic::Backup);
    }

    /// F3.2: melee-reach threshold `thresh = 14 - mode` with numeric GOAL values.
    /// ATTACK1 (mode 1) → thresh 13; ATTACK2 (mode 2) → thresh 12.
    #[test]
    fn test_melee_thresh_uses_numeric_goal() {
        // At (xd, yd) = (12, 0): ATTACK1 (thresh 13) engages; ATTACK2 (thresh 12) does NOT.
        let mut a1 = make_npc(100, 100);
        a1.goal = Goal::Attack1;
        a1.weapon = 1; // melee (bit 2 = 0)
        select_tactic(&mut a1, 0, 112, 100, false, None, 0, false, 0);
        assert_eq!(a1.state, NpcState::Fighting, "ATTACK1 at xd=12 must engage");

        let mut a2 = make_npc(100, 100);
        a2.goal = Goal::Attack2;
        a2.weapon = 1;
        // Force the reconsider gate closed so we don't accidentally pick Pursue.
        // We only care that Fighting is NOT entered; at xd=12 thresh=12, 12 < 12 is false.
        select_tactic(&mut a2, 0, 112, 100, false, None, 0, false, 0);
        assert_ne!(a2.state, NpcState::Fighting, "ATTACK2 at xd=12 must NOT engage");
    }

    /// F3.5: a frust-latched NPC receives a randomized fallback tactic, not a no-op.
    #[test]
    fn test_frust_tactic_dispatches_random_fallback() {
        let mut changed = false;
        for tick in 0..200u32 {
            let mut npc = make_npc(100, 100);
            npc.goal = Goal::Attack1;
            npc.tactic = Tactic::Frust;
            npc.weapon = 1; // melee → BUMBLE_SEEK or RANDOM
            select_tactic(&mut npc, 0, 200, 100, false, None, 0, false, tick);
            if matches!(npc.tactic, Tactic::BumbleSeek | Tactic::Random) {
                changed = true;
                break;
            }
        }
        assert!(changed, "Frust must dispatch to BUMBLE_SEEK/RANDOM for melee actors");
    }

    #[test]
    fn test_frust_tactic_dispatches_bow_fallback() {
        let mut seen: Vec<Tactic> = Vec::new();
        for tick in 0..1000u32 {
            let mut npc = make_npc(100, 100);
            npc.goal = Goal::Archer1;
            npc.tactic = Tactic::Frust;
            npc.weapon = 4; // bow, bit 2 set
            select_tactic(&mut npc, 0, 200, 100, false, None, 0, false, tick);
            if !seen.contains(&npc.tactic) {
                seen.push(npc.tactic.clone());
            }
        }
        // Bow frust fallback: rand(2,5) → FOLLOW/BUMBLE_SEEK/RANDOM/BACKUP.
        let allowed = [Tactic::Follow, Tactic::BumbleSeek, Tactic::Random, Tactic::Backup];
        for t in &seen {
            assert!(allowed.contains(t), "bow frust dispatch produced illegal tactic {:?}", t);
        }
        assert!(seen.len() >= 3, "bow frust dispatch must randomize: seen={:?}", seen);
    }

    /// F3.7: on hero death, the NPC at `leader_idx` flees; others follow.
    #[test]
    fn test_hero_dead_leader_flees_others_follow() {
        let mut leader = make_npc(100, 100);
        leader.goal = Goal::Attack1;
        // Leader slot = 2; this NPC is at idx 2.
        select_tactic(&mut leader, 2, 400, 400, true, Some(2), 0, false, 0);
        assert_eq!(leader.goal, Goal::Flee, "leader flees when hero is dead");

        let mut follower = make_npc(150, 150);
        follower.goal = Goal::Attack1;
        select_tactic(&mut follower, 3, 400, 400, true, Some(2), 0, false, 0);
        assert_eq!(follower.goal, Goal::Follower, "non-leader follows when hero is dead");
    }

    /// F3.6: SHOOT tactic bypasses the do_tactic rate limit (ref RESEARCH §8.2,
    /// fmain2.c:1666-1682 — SHOOT case runs every tick).
    #[test]
    fn test_do_tactic_shoot_bypasses_rate_limit() {
        // If the gate were active, facing would only update ~12.5% of ticks.
        // SHOOT must run every tick → over 100 ticks we expect near-100% updates.
        let mut updates = 0u32;
        for tick in 0..100u32 {
            let mut npc = make_npc(100, 100);
            npc.tactic = Tactic::Shoot;
            npc.goal = Goal::Archer1;
            npc.facing = 7; // starting facing NW; hero east → should change
            do_tactic(&mut npc, 200, 100, None, &[], tick);
            if npc.facing != 7 { updates += 1; }
        }
        // With the SHOOT-bypass, should be very close to 100.
        assert!(updates >= 90, "SHOOT must not be rate-limited: updates={updates}/100");
    }

    /// F3.3: snakes do NOT unconditionally march to the turtle nest — only when
    /// the global turtle_eggs counter is non-zero (ref ai-system.md:84).
    #[test]
    fn test_snake_no_egg_seek_when_eggs_absent() {
        let mut npc = make_npc(100, 100);
        npc.race = 4; // snake
        npc.goal = Goal::Attack1;
        npc.weapon = 1;
        let mut saw_egg_seek = false;
        for tick in 0..500u32 {
            npc.tactic = Tactic::Pursue;
            select_tactic(&mut npc, 0, 500, 500, false, None, 0, /*turtle_eggs*/ false, tick);
            if npc.tactic == Tactic::EggSeek {
                saw_egg_seek = true;
                break;
            }
        }
        assert!(!saw_egg_seek, "snake must not enter EGG_SEEK when turtle_eggs == 0");
    }

    #[test]
    fn test_snake_egg_seek_when_eggs_present() {
        let mut npc = make_npc(100, 100);
        npc.race = 4;
        npc.goal = Goal::Attack1;
        npc.weapon = 1;
        let mut saw_egg_seek = false;
        for tick in 0..500u32 {
            npc.tactic = Tactic::Pursue;
            select_tactic(&mut npc, 0, 500, 500, false, None, 0, /*turtle_eggs*/ true, tick);
            if npc.tactic == Tactic::EggSeek {
                saw_egg_seek = true;
                break;
            }
        }
        assert!(saw_egg_seek, "snake must select EGG_SEEK when turtle_eggs is set");
    }
}
