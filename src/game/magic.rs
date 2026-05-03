//! Magic item system: 7 consumable items with timer-based effects.
//! Ports MAGIC menu (fmain.c case MAGIC, MAGICBASE=9) verbatim.
//!
//! Items occupy stuff[9..=15]; timers (light_timer, secret_timer, freeze_timer)
//! live in GameState and are decremented each tick there.

use crate::game::actor::ActorKind;
use crate::game::game_state::GameState;

/// Magic item indices in stuff[] (MAGICBASE = 9 in fmain.c).
/// hit=5..=11 in the MAGIC menu; item = stuff[4 + hit].
pub const ITEM_STONE_RING: usize = 9; // hit=5: teleport via stone ring
pub const ITEM_LANTERN: usize = 10; // hit=6: light_timer += 760
pub const ITEM_VIAL: usize = 11; // hit=7: heal (vitality += rand8() + 4)
pub const ITEM_ORB: usize = 12; // hit=8: secret_timer += 360
pub const ITEM_TOTEM: usize = 13; // hit=9: show world map
pub const ITEM_RING: usize = 14; // hit=10: freeze_timer += 100
pub const ITEM_SKULL: usize = 15; // hit=11: kill all on-screen enemies

/// Timer increments ported verbatim from fmain.c.
pub const LIGHT_TIMER_INCREMENT: i16 = 760;
pub const SECRET_TIMER_INCREMENT: i16 = 360;
pub const FREEZE_TIMER_INCREMENT: i16 = 100;

/// Heal vitality cap formula from fmain.c: `15 + brave / 4`.
pub fn heal_cap(brave: i16) -> i16 {
    15 + brave / 4
}

/// Simple pseudo-random 0-7 for magic effects (ports rand8() pattern).
/// Uses system time nanos similar to combat.rs melee_rand().
fn rand8() -> i16 {
    use std::time::SystemTime;
    let nanos = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    (nanos & 7) as i16
}

/// Stone ring sector coordinates from fmain.c stone_list[].
/// 11 pairs of (x_sector, y_sector) for teleport destinations.
const STONE_RINGS: [(u8, u8); 11] = [
    (54, 43),
    (71, 77),
    (78, 102),
    (66, 121),
    (12, 85),
    (79, 40),
    (107, 38),
    (73, 21),
    (12, 26),
    (26, 53),
    (84, 60),
];

/// Stone ring activation sector (fmain.c: hero_sector == 144).
const STONE_RING_SECTOR: u16 = 144;

/// Find the index of the stone ring the hero is currently standing at,
/// based on their sector coordinates matching a ring in STONE_RINGS.
fn find_current_ring(hero_x: u16, hero_y: u16) -> Option<usize> {
    let sx = (hero_x >> 8) as u8;
    let sy = (hero_y >> 8) as u8;
    STONE_RINGS
        .iter()
        .position(|&(rx, ry)| rx == sx && ry == sy)
}

/// Structured outcome of a MAGIC submenu cast.
///
/// Mirrors the control-flow side-effects of `magic_dispatch`
/// (`fmain.c:3300-3365`); messages are the caller's responsibility and must
/// come from `faery.toml [narr]` tables or documented `dialog_system.md`
/// literals — never invented here.
#[derive(Debug, Clone, PartialEq)]
pub enum MagicResult {
    /// Slot empty (`stuff[4+hit] == 0` at `fmain.c:3303`). Caller emits
    /// `event(21)` — "% does not have that item." — and does **not** decrement.
    NoOwned,
    /// Precondition failed (wrong sector / not on stone / region>7 /
    /// riding>1). Original `fmain.c` returns silently without consuming the
    /// charge — no scroll text.
    Suppressed,
    /// Effect applied; no scroll text in the original.
    /// Covers Green Jewel (`light_timer += 760`), Crystal Orb
    /// (`secret_timer += 360`), Gold Ring (`freeze_timer += 100`), and
    /// Bird Totem (sets `viewstatus = 1`).
    Applied,
    /// Glass Vial heal branch (`fmain.c:3348-3354`). When `capped == false`
    /// the original prints the hardcoded literal `"That feels a lot
    /// better!"` (dialog_system.md:339). When capped it prints nothing.
    Healed { capped: bool },
    /// Blue Stone teleport (`case 5`). Falls through into the Glass Vial
    /// heal block (`fmain.c:3347` has no `break;`), so the `capped` flag
    /// governs the same heal-message condition.
    StoneTeleport { capped: bool },
    /// Jade Skull mass-kill (`case 11`). `slain` are actor-slot indices
    /// that were transitioned to vitality 0. `in_battle` reflects
    /// `battleflag` at cast time; when true the caller emits `event(34)`
    /// — `"They're all dead!" he cried.`
    MassKill { slain: Vec<usize>, in_battle: bool },
}

/// Apply one MAGIC submenu cast. Mirrors `magic_dispatch` at
/// `fmain.c:3300-3365`. The `extn.v3 == 9` arena gate (`fmain.c:3304`) is
/// handled by the caller because the extent lookup is scene-local.
///
/// Charge consumption follows the original: Blue Stone / Bird Totem /
/// Gold Ring precondition misses return `Suppressed` **without** decrement
/// (`fmain.c:3365` epilogue is only reached on successful branches).
pub fn use_magic(state: &mut GameState, item_idx: usize) -> MagicResult {
    if item_idx < ITEM_STONE_RING || item_idx > ITEM_SKULL {
        return MagicResult::NoOwned;
    }
    if state.stuff()[item_idx] == 0 {
        return MagicResult::NoOwned;
    }

    let result = match item_idx {
        ITEM_STONE_RING => {
            // fmain.c:3327 — hero_sector gate.
            if state.hero_sector != STONE_RING_SECTOR {
                return MagicResult::Suppressed;
            }
            // fmain.c:3328 — sub-cell centring gate.
            let hx_frac = (state.hero_x & 255) / 85;
            let hy_frac = (state.hero_y & 255) / 64;
            if hx_frac != 1 || hy_frac != 1 {
                return MagicResult::Suppressed;
            }
            let current = match find_current_ring(state.hero_x, state.hero_y) {
                Some(c) => c,
                None => return MagicResult::Suppressed,
            };
            // fmain.c:3333 — step facing+1 stones forward, wrap 11.
            let dest = (current + state.facing as usize + 1) % STONE_RINGS.len();
            let (dx, dy) = STONE_RINGS[dest];
            state.hero_x = ((dx as u16) << 8) | (state.hero_x & 255);
            state.hero_y = ((dy as u16) << 8) | (state.hero_y & 255);
            // fmain.c:3338 — drag mount along with hero (SPEC §21.7).
            state.sync_carrier_to_hero();
            // Fall through into Glass Vial heal (fmain.c:3347 — no `break`).
            let capped = apply_vial_heal(state);
            MagicResult::StoneTeleport { capped }
        }
        ITEM_LANTERN => {
            // fmain.c:3306 — Green Jewel. No scroll text.
            state.light_timer = state.light_timer.saturating_add(LIGHT_TIMER_INCREMENT);
            MagicResult::Applied
        }
        ITEM_VIAL => {
            // fmain.c:3348-3354 — Glass Vial heal.
            let capped = apply_vial_heal(state);
            MagicResult::Healed { capped }
        }
        ITEM_ORB => {
            // fmain.c:3307 — Crystal Orb. No scroll text.
            state.secret_timer = state.secret_timer.saturating_add(SECRET_TIMER_INCREMENT);
            MagicResult::Applied
        }
        ITEM_TOTEM => {
            // fmain.c:3310 — regions 8,9 locked without cheat1. Silent.
            if state.region_num > 7 && !state.cheat1 {
                return MagicResult::Suppressed;
            }
            // fmain.c:3322 — viewstatus = 1 (VIEWSTATUS_MAP).
            // The "+" marker blit / bigdraw / stillscreen / prq(5) are
            // rendering-side effects not yet wired here (see SPEC-GAP).
            state.viewstatus = 1;
            MagicResult::Applied
        }
        ITEM_RING => {
            // fmain.c:3308 — while mounted on swan/dragon: silent no-op, no consume.
            if state.riding > 1 {
                return MagicResult::Suppressed;
            }
            state.freeze_timer = state.freeze_timer.saturating_add(FREEZE_TIMER_INCREMENT);
            MagicResult::Applied
        }
        ITEM_SKULL => {
            // fmain.c:3355-3363 — mass-kill race < 7 live ENEMYs.
            // checkdead(i, 0) at fmain.c:3359 contributes brave += 1; the
            // explicit brave -= 1 at fmain.c:3359 cancels that. Net: brave
            // unchanged, kind unchanged (filter excludes SETFIG). The
            // STATE_DYING / loot / race-specific drops that checkdead +
            // actor_tick normally run are not wired from this path — see
            // the Subsystem 2 audit SPEC-GAP.
            let mut slain: Vec<usize> = Vec::new();
            let anix = state.anix;
            for i in 1..anix {
                let a = &mut state.actors[i];
                if a.vitality > 0 && a.kind == ActorKind::Enemy && a.race < 7 {
                    a.vitality = 0;
                    slain.push(i);
                }
            }
            MagicResult::MassKill {
                slain,
                in_battle: state.battleflag,
            }
        }
        _ => return MagicResult::NoOwned,
    };

    // fmain.c:3365 — `if (!--stuff[4+hit]) set_options();` epilogue. Only
    // branches that did NOT early-return reach here.
    state.stuff_mut()[item_idx] -= 1;
    result
}

/// Glass Vial heal block (`fmain.c:3348-3354`).
/// Returns `true` if the heal was clamped at the cap (`vitality > 15 + brave/4`)
/// — in which case the original prints **no** message. Returns `false` if the
/// heal landed under the cap, in which case the caller should emit
/// `"That feels a lot better!"` (dialog_system.md:339).
fn apply_vial_heal(state: &mut GameState) -> bool {
    let heal = rand8() + 4;
    let cap = heal_cap(state.brave);
    let raw = state.vitality + heal;
    if raw > cap {
        state.vitality = cap;
        true
    } else {
        state.vitality = raw;
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game::game_state::GameState;

    #[test]
    fn test_lantern_adds_to_light_timer() {
        let mut state = GameState::new();
        state.stuff_mut()[ITEM_LANTERN] = 1;
        let result = use_magic(&mut state, ITEM_LANTERN);
        assert_eq!(result, MagicResult::Applied);
        assert_eq!(state.light_timer, LIGHT_TIMER_INCREMENT);
        assert_eq!(state.stuff()[ITEM_LANTERN], 0);
    }

    #[test]
    fn test_orb_adds_to_secret_timer() {
        let mut state = GameState::new();
        state.stuff_mut()[ITEM_ORB] = 2;
        let _ = use_magic(&mut state, ITEM_ORB);
        assert_eq!(state.secret_timer, SECRET_TIMER_INCREMENT);
        // Second use stacks.
        let _ = use_magic(&mut state, ITEM_ORB);
        assert_eq!(state.secret_timer, SECRET_TIMER_INCREMENT * 2);
    }

    #[test]
    fn test_ring_adds_to_freeze_timer() {
        let mut state = GameState::new();
        state.stuff_mut()[ITEM_RING] = 1;
        let _ = use_magic(&mut state, ITEM_RING);
        assert_eq!(state.freeze_timer, FREEZE_TIMER_INCREMENT);
    }

    #[test]
    fn test_vial_heals_vitality() {
        let mut state = GameState::new();
        state.vitality = 5;
        state.brave = 40;
        state.stuff_mut()[ITEM_VIAL] = 1;
        let result = use_magic(&mut state, ITEM_VIAL);
        assert!(matches!(result, MagicResult::Healed { .. }));
        assert!(state.vitality > 5);
        assert!(state.vitality <= heal_cap(40));
    }

    #[test]
    fn test_use_item_no_stock() {
        let mut state = GameState::new();
        assert_eq!(use_magic(&mut state, ITEM_LANTERN), MagicResult::NoOwned);
    }

    #[test]
    fn test_heal_cap() {
        assert_eq!(heal_cap(40), 25); // 15 + 40/4 = 25
        assert_eq!(heal_cap(0), 15);
    }

    #[test]
    fn test_timers_decrement_in_tick() {
        let mut state = GameState::new();
        state.light_timer = 5;
        state.secret_timer = 3;
        state.freeze_timer = 1;
        state.tick(1);
        assert_eq!(state.light_timer, 4);
        assert_eq!(state.secret_timer, 2);
        assert_eq!(state.freeze_timer, 0);
    }

    #[test]
    fn test_vial_heal_randomness() {
        // fmain.c:3349 — heal = rand8() + 4 (yields 4-11), capped at 15 + brave/4.
        let mut state = GameState::new();
        state.vitality = 5;
        state.brave = 40; // cap = 15 + 40/4 = 25
        state.stuff_mut()[ITEM_VIAL] = 10;

        for _ in 0..10 {
            let before = state.vitality;
            let _ = use_magic(&mut state, ITEM_VIAL);
            let gained = state.vitality - before;
            let expected_max = (11).min(25 - before);
            assert!(
                gained >= 4.min(25 - before) && gained <= expected_max,
                "Heal gained {gained} outside expected range"
            );
            assert!(
                state.vitality <= 25,
                "Vitality {0} exceeded cap 25",
                state.vitality
            );
        }
    }

    #[test]
    fn test_vial_heal_cap_enforcement() {
        // fmain.c:3350 — heal is clamped at `15 + brave/4`; capped branch is silent.
        let mut state = GameState::new();
        state.brave = 20; // cap = 15 + 20/4 = 20
        state.vitality = 18;
        state.stuff_mut()[ITEM_VIAL] = 1;
        let _ = use_magic(&mut state, ITEM_VIAL);
        assert!(state.vitality <= 20, "Vitality exceeded cap");
    }

    #[test]
    fn test_vial_heal_capped_flag() {
        // When vitality would exceed the cap, heal is clamped and capped=true.
        let mut state = GameState::new();
        state.brave = 20; // cap = 20
        state.vitality = 19; // raw = 19 + [4..11] = [23..30] → always clamped.
        state.stuff_mut()[ITEM_VIAL] = 1;
        let r = use_magic(&mut state, ITEM_VIAL);
        assert_eq!(r, MagicResult::Healed { capped: true });
        assert_eq!(state.vitality, 20);
    }

    #[test]
    fn test_jade_skull_no_brave_change() {
        // fmain.c:3359 — checkdead(i,0) does `brave += 1` for each kill, then
        // the explicit `brave -= 1` at the same line cancels it. Net: brave
        // unchanged. (The full checkdead STATE_DYING transition / loot drops
        // are not wired from this path — see Subsystem 2 audit SPEC-GAP.)
        let mut state = GameState::new();
        state.brave = 50;
        state.stuff_mut()[ITEM_SKULL] = 1;

        state.anix = 4;
        state.actors[1].vitality = 10;
        state.actors[1].kind = ActorKind::Enemy;
        state.actors[1].race = 3;
        state.actors[2].vitality = 15;
        state.actors[2].kind = ActorKind::Enemy;
        state.actors[2].race = 5;
        state.actors[3].vitality = 20;
        state.actors[3].kind = ActorKind::Enemy;
        state.actors[3].race = 6;

        let r = use_magic(&mut state, ITEM_SKULL);
        match r {
            MagicResult::MassKill { slain, .. } => {
                assert_eq!(slain, vec![1, 2, 3]);
            }
            other => panic!("expected MassKill, got {:?}", other),
        }

        assert_eq!(state.actors[1].vitality, 0);
        assert_eq!(state.actors[2].vitality, 0);
        assert_eq!(state.actors[3].vitality, 0);
        // Net zero brave change (ref: +1 per checkdead, −1 per magic epilogue).
        assert_eq!(state.brave, 50);
    }

    #[test]
    fn test_jade_skull_battleflag_reported() {
        // fmain.c:3362 — `if (battleflag) event(34);`
        let mut state = GameState::new();
        state.stuff_mut()[ITEM_SKULL] = 1;
        state.anix = 1;
        state.battleflag = true;
        let r = use_magic(&mut state, ITEM_SKULL);
        assert_eq!(
            r,
            MagicResult::MassKill {
                slain: vec![],
                in_battle: true
            }
        );
    }

    #[test]
    fn test_jade_skull_skips_race_7_plus() {
        // fmain.c:3358 — `race < 7` filter spares Dark Knight (7), Loraii (8),
        // Necromancer (9), and every SETFIG (race bit 7 set).
        let mut state = GameState::new();
        state.stuff_mut()[ITEM_SKULL] = 1;
        state.anix = 4;
        state.actors[1].vitality = 10;
        state.actors[1].kind = ActorKind::Enemy;
        state.actors[1].race = 7;
        state.actors[2].vitality = 10;
        state.actors[2].kind = ActorKind::Enemy;
        state.actors[2].race = 9;
        state.actors[3].vitality = 10;
        state.actors[3].kind = ActorKind::Enemy;
        state.actors[3].race = 0x89;
        let _ = use_magic(&mut state, ITEM_SKULL);
        assert_eq!(state.actors[1].vitality, 10);
        assert_eq!(state.actors[2].vitality, 10);
        assert_eq!(state.actors[3].vitality, 10);
    }

    #[test]
    fn test_totem_blocked_underground() {
        // fmain.c:3310 — region>7 without cheat1 ⇒ silent early return, no consume.
        let mut state = GameState::new();
        state.stuff_mut()[ITEM_TOTEM] = 1;
        state.region_num = 8;
        state.cheat1 = false;

        let r = use_magic(&mut state, ITEM_TOTEM);
        assert_eq!(r, MagicResult::Suppressed);
        assert_eq!(
            state.stuff()[ITEM_TOTEM],
            1,
            "Charge must be preserved on suppressed"
        );
    }

    #[test]
    fn test_totem_allowed_overworld() {
        let mut state = GameState::new();
        state.stuff_mut()[ITEM_TOTEM] = 1;
        state.region_num = 7;

        let r = use_magic(&mut state, ITEM_TOTEM);
        assert_eq!(r, MagicResult::Applied);
        assert_eq!(state.viewstatus, 1);
        assert_eq!(state.stuff()[ITEM_TOTEM], 0);
    }

    #[test]
    fn test_totem_cheat1_bypass() {
        let mut state = GameState::new();
        state.stuff_mut()[ITEM_TOTEM] = 1;
        state.region_num = 9;
        state.cheat1 = true;

        let r = use_magic(&mut state, ITEM_TOTEM);
        assert_eq!(r, MagicResult::Applied);
        assert_eq!(state.viewstatus, 1);
        assert_eq!(state.stuff()[ITEM_TOTEM], 0);
    }

    #[test]
    fn test_ring_blocked_on_turtle() {
        // fmain.c:3308 — riding > 1 ⇒ silent early return, no consume.
        let mut state = GameState::new();
        state.stuff_mut()[ITEM_RING] = 1;
        state.riding = 5;

        let r = use_magic(&mut state, ITEM_RING);
        assert_eq!(r, MagicResult::Suppressed);
        assert_eq!(state.freeze_timer, 0);
        assert_eq!(state.stuff()[ITEM_RING], 1);
    }

    #[test]
    fn test_ring_blocked_on_swan() {
        let mut state = GameState::new();
        state.stuff_mut()[ITEM_RING] = 1;
        state.riding = 11;

        let r = use_magic(&mut state, ITEM_RING);
        assert_eq!(r, MagicResult::Suppressed);
        assert_eq!(state.freeze_timer, 0);
        assert_eq!(state.stuff()[ITEM_RING], 1);
    }

    #[test]
    fn test_ring_allowed_on_foot() {
        let mut state = GameState::new();
        state.stuff_mut()[ITEM_RING] = 1;
        state.riding = 0;

        let r = use_magic(&mut state, ITEM_RING);
        assert_eq!(r, MagicResult::Applied);
        assert_eq!(state.freeze_timer, FREEZE_TIMER_INCREMENT);
        assert_eq!(state.stuff()[ITEM_RING], 0);
    }

    #[test]
    fn test_ring_allowed_on_raft() {
        let mut state = GameState::new();
        state.stuff_mut()[ITEM_RING] = 1;
        state.riding = 1;

        let r = use_magic(&mut state, ITEM_RING);
        assert_eq!(r, MagicResult::Applied);
        assert_eq!(state.freeze_timer, FREEZE_TIMER_INCREMENT);
        assert_eq!(state.stuff()[ITEM_RING], 0);
    }

    // ── stone-circle teleport tests (SPEC §21.7) ─────────────────────────────

    fn stone_ring_state() -> GameState {
        let mut state = GameState::new();
        state.hero_x = (54u16 << 8) | 85;
        state.hero_y = (43u16 << 8) | 64;
        state.hero_sector = STONE_RING_SECTOR;
        state.facing = 0;
        state.stuff_mut()[ITEM_STONE_RING] = 1;
        state
    }

    #[test]
    fn test_stone_ring_teleports_hero() {
        let mut state = stone_ring_state();
        state.vitality = 5;
        state.brave = 40; // cap = 25, heal will land uncapped.
        let r = use_magic(&mut state, ITEM_STONE_RING);
        assert!(matches!(r, MagicResult::StoneTeleport { .. }));
        assert_eq!(state.hero_x >> 8, 71, "hero_x sector after teleport");
        assert_eq!(state.hero_y >> 8, 77, "hero_y sector after teleport");
    }

    #[test]
    fn test_stone_ring_falls_through_to_heal() {
        // fmain.c:3347 has no `break;` — case 5 falls through into case 7 heal.
        let mut state = stone_ring_state();
        state.vitality = 5;
        state.brave = 40;
        let before = state.vitality;
        let _ = use_magic(&mut state, ITEM_STONE_RING);
        assert!(
            state.vitality > before,
            "stone ring teleport must also heal (case 5 fall-through)"
        );
    }

    #[test]
    fn test_stone_ring_wrong_sector_suppressed() {
        let mut state = stone_ring_state();
        state.hero_sector = 0;
        let r = use_magic(&mut state, ITEM_STONE_RING);
        assert_eq!(r, MagicResult::Suppressed);
        assert_eq!(state.stuff()[ITEM_STONE_RING], 1, "Charge preserved");
    }

    #[test]
    fn test_stone_ring_teleports_turtle_carrier() {
        let mut state = stone_ring_state();
        state.on_raft = true;
        state.wcarry = 3;
        state.actors[3].abs_x = 0;
        state.actors[3].abs_y = 0;

        let _ = use_magic(&mut state, ITEM_STONE_RING);
        assert_eq!(state.actors[3].abs_x, state.hero_x);
        assert_eq!(state.actors[3].abs_y, state.hero_y);
        assert!(state.on_raft);
        assert_eq!(state.wcarry, 3);
    }

    #[test]
    fn test_stone_ring_teleports_raft_carrier() {
        let mut state = stone_ring_state();
        state.on_raft = true;
        state.wcarry = 1;
        state.actors[1].abs_x = 0;
        state.actors[1].abs_y = 0;

        let _ = use_magic(&mut state, ITEM_STONE_RING);
        assert_eq!(state.actors[1].abs_x, state.hero_x);
        assert_eq!(state.actors[1].abs_y, state.hero_y);
    }

    #[test]
    fn test_stone_ring_teleports_swan_carrier() {
        let mut state = stone_ring_state();
        state.flying = 1;
        state.actors[3].abs_x = 0;
        state.actors[3].abs_y = 0;

        let _ = use_magic(&mut state, ITEM_STONE_RING);
        assert_eq!(state.actors[3].abs_x, state.hero_x);
        assert_eq!(state.actors[3].abs_y, state.hero_y);
        assert_eq!(state.flying, 1);
    }

    #[test]
    fn test_stone_ring_unmounted_no_carrier_move() {
        let mut state = stone_ring_state();
        state.actors[1].abs_x = 9999;
        state.actors[1].abs_y = 8888;
        state.actors[3].abs_x = 7777;
        state.actors[3].abs_y = 6666;

        let _ = use_magic(&mut state, ITEM_STONE_RING);
        assert_eq!(state.actors[1].abs_x, 9999);
        assert_eq!(state.actors[1].abs_y, 8888);
        assert_eq!(state.actors[3].abs_x, 7777);
        assert_eq!(state.actors[3].abs_y, 6666);
    }
}
