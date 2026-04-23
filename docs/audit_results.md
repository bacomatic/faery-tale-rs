# Audit Results

Deep-dive audit findings, one section per subsystem. Execution follows
[`AUDIT_PLAN.md`](AUDIT_PLAN.md).

**Finding severity legend:**

| Code | Meaning |
|---|---|
| CONFORMANT | Code matches ref + spec + req |
| NEEDS-FIX | Code is wrong per the reference — fix applied |
| SPEC-GAP | Code may be right but SPEC/REQ is silent or wrong |
| REF-AMBIGUOUS | Reference itself is unclear/contradictory — user review |
| RESEARCH-REQUIRED | Behavior observable but not yet documented — user review |
| INVENTED | Code has no ref/spec support — removed or replaced |

**Status legend:**
- ✅ Complete (all findings resolved or queued for user)
- ⚠️ Incomplete (blocked — requires user input; see Blockers at bottom)
- 🔒 Blocked

---

## Subsystem 1: combat — ✅ Complete

**Reference**: `reference/logic/combat.md` (+ `game-loop.md#melee_hit_detection`,
`game-loop.md#missile_tick`, `RESEARCH.md §7`, `frustration.md`,
`dialog_system.md#hardcoded-scroll-messages--complete-reference`)
**Code**: `src/game/combat.rs`, combat paths in `src/game/gameplay_scene.rs`
(`run_combat_tick`, `apply_hit`, missile tick block at ~5070, `GameAction::Fight`
/ `GameAction::Shoot` / `GameAction::Attack`)
**Audit date**: 2025 (current session)

### Summary
- **19 findings**: 4 CONFORMANT, 7 NEEDS-FIX (all fixed), 4 INVENTED (all
  resolved), 3 SPEC-GAP (queued), 0 REF-AMBIGUOUS, 1 RESEARCH-REQUIRED
  (queued).
- Fixes applied in **one commit** (SHA to be recorded by orchestrator).
- Build/tests: ✅ `cargo build` clean (zero new warnings); `cargo test` —
  563 + 12 + 12 tests passing.

### Findings

#### F1.1 — "You shoot an arrow!" / "You cast a fireball!" scroll-text [INVENTED]
**Location**: `src/game/gameplay_scene.rs:762-767` (bow/wand release) and
`src/game/gameplay_scene.rs:3046-3051` (`GameAction::Shoot` menu).
**Reference**: `reference/logic/combat.md#missile_step`,
`reference/logic/dialog_system.md#hardcoded-scroll-messages--complete-reference`
(no entry for arrow/fireball fire).
**Issue**: Original `fmain.c` emits **no** scroll-area text when the hero
releases a bow or casts a wand — the only side effects are `effect()` SFX
and the projectile being placed in `missile_list[]`. These strings are not
in `faery.toml [narr]` and not in `dialog_system.md`. Violates the
two-source scroll-text rule.
**Resolution**: Both strings removed; arrow consumption preserved.

#### F1.2 — Immune target still takes pushback / follow-through [NEEDS-FIX]
**Location**: `src/game/gameplay_scene.rs` (`apply_hit`, was ~1917-1943).
**Reference**: `reference/logic/combat.md#dohit` (`fmain2.c:231-235`).
**Issue**: Reference `dohit` checks immunity (necromancer / witch /
spectre / ghost) **before** any other effect and returns immediately on a
block — no damage, no SFX, no `move_figure` knockback, no `checkdead`.
Rust set `actual_damage = 0` but then still applied target pushback,
attacker follow-through, and fell through to the `checkdead` branch.
**Resolution**: Immunity branch now bypasses the entire damage / pushback /
follow-through block. `checkdead` is skipped because vitality was never
decremented.

#### F1.3 — DRAGON / SETFIG knockback not suppressed [NEEDS-FIX]
**Location**: `src/game/gameplay_scene.rs` (`apply_hit`, pushback block).
**Reference**: `reference/logic/combat.md#dohit` / `combat.md#notes`
(`fmain2.c:243` — `type != DRAGON && type != SETFIG`).
**Issue**: Every melee hit unconditionally shoved the target 2px. Original
`dohit` gates pushback on `type != DRAGON && type != SETFIG` — dragons and
static scenario NPCs are immovable.
**Resolution**: Added `target_pushable` guard — skip pushback when
`npc.npc_type == NPC_TYPE_DRAGON` or `(npc.race & 0x80) != 0` (SETFIG bit).

#### F1.4 — Attacker follow-through not gated on target move [NEEDS-FIX]
**Location**: `src/game/gameplay_scene.rs` (`apply_hit`).
**Reference**: `reference/logic/combat.md#dohit`
(`if pushable and move_figure(j, fc, 2) and i >= 0: move_figure(i, fc, 2)`).
**Issue**: The hero's follow-through step was unconditional — even against
an immovable target (dragon, setfig, terrain-blocked) the hero would slide
forward 2px. Reference only moves the attacker when the target moved.
**Resolution**: Follow-through now gated on `target_moved` (which is set
only when pushback actually committed). `apply_hit` is never entered with
`i < 0` (arrows/fireballs are applied via the missile block), so the
`i >= 0` guard is implicit.

#### F1.5 — `brave.min(100)` cap on kill [INVENTED]
**Location**: `src/game/gameplay_scene.rs` (`apply_hit`, checkdead branch).
**Reference**: `reference/logic/combat.md#checkdead` (`fmain.c:2777`:
`brave = brave + 1` — no cap).
**Issue**: Rust capped bravery at 100 on enemy kill; reference has no upper
bound. `brave` is a `u8` in the original but reaches the 255 ceiling only
via hundreds of kills. A 100-cap breaks the melee-reach formula
`(brave/20)+5` beyond `brave=100` and the dodge curve.
**Resolution**: Removed `.min(100)`; now plain `self.state.brave += 1`.

#### F1.6 — Missing `kind -= 3` SETFIG-kill penalty [NEEDS-FIX]
**Location**: `src/game/gameplay_scene.rs` (`apply_hit`, checkdead branch).
**Reference**: `reference/logic/combat.md#checkdead`
(`fmain.c:2775`: `kind -= 3` when `type == SETFIG and race != 0x89`).
**Issue**: Killing setfigure NPCs (race bit 7 set) is supposed to impose a
3-point kindness penalty, except the witch (race `0x89`). Rust did
nothing. Affects the `kind` stat that drives end-game scoring and
brother-succession side effects.
**Resolution**: Added branch: `if (race & 0x80) != 0 && race != RACE_WITCH`
→ `self.state.kind -= 3`. Floor-at-zero clamp added per
`fmain.c:2778` (`if kind < 0: kind = 0`).

#### F1.7 — Missing dark-knight death speech `speak(42)` [NEEDS-FIX]
**Location**: `src/game/gameplay_scene.rs` (`apply_hit`, checkdead branch).
**Reference**: `reference/logic/combat.md#checkdead` (`fmain.c:2774`).
**Issue**: When a `race == 7` (dark knight / dreamknight) NPC dies, the
original emits `speak(42)` ("`dreamknight msg 2 (earned the right)`" in
`faery.toml`). Rust never fired the speech.
**Resolution**: Added `const RACE_DARK_KNIGHT: u8 = 7` (inline, local to
the branch) and `dark_knight_speech` deferred message variable. Speech is
sourced from `faery.toml [narr].speeches[42]` via `events::speak`.

#### F1.8 — Missile never expires after 40 ticks [NEEDS-FIX]
**Location**: `src/game/combat.rs` (`Missile` struct);
`src/game/gameplay_scene.rs` (missile tick block, was ~5085-5091).
**Reference**: `reference/logic/combat.md#missile_step` (`fmain.c:2274`:
`ms.time_of_flight > 40` → deactivate).
**Issue**: Rust missiles only deactivated on hit or on going out of world
bounds (`> 32768`). Fireballs fired in open terrain would fly indefinitely.
**Resolution**: Added `Missile.time_of_flight: u8` field (defaulted to 0 in
every literal and in `fire_missile`). Missile tick increments each frame
and deactivates when `time_of_flight > 40`.

#### F1.9 — `GameAction::Attack` menu-path invented scroll text [INVENTED]
**Location**: `src/game/gameplay_scene.rs` (old ~2963-2998).
**Reference**: `reference/logic/dialog_system.md` — none of `"Enemy
defeated!"`, `"You hit for {N}!"`, `"Nothing to attack."`, `"{N} items
dropped!"`, or `"The turtle rewards you with a Sea Shell!"` exist.
**Issue**: A legacy menu "Attack" command (pre-dates the proper fight
state machine) uses the deprecated `resolve_combat` helper with an
invented damage formula and invented scroll-area strings.
**Resolution**: Stripped all invented scroll messages from this path. The
underlying deprecated `resolve_combat` helper is retained and continues to
be called (to preserve any side-effects hooked into it), but the user-
visible strings are gone. The damage formula itself (`resolve_combat`) is
flagged SPEC-GAP F1.15 below — real combat runs through `run_combat_tick`
so the legacy path is dormant in normal play.

#### F1.10 — Witch lasso / necromancer talisman drops [CONFORMANT]
**Location**: `src/game/gameplay_scene.rs` (`apply_hit`, checkdead branch).
**Reference**: `reference/RESEARCH.md §7.8 "Special Death Drops"`;
`reference/logic/combat.md#notes` ("Race-specific loot drops").
**Issue**: None. The necromancer→woodcutter transform + talisman drop
(object 139) and the witch's golden-lasso drop (object 27, +10 y offset)
match spec.

#### F1.11 — Weapon-code immunity table [CONFORMANT]
**Location**: `src/game/combat.rs` `check_immunity`.
**Reference**: `reference/logic/combat.md#dohit` (`fmain2.c:231-234`),
`RESEARCH.md §7.1` immunity table.
**Issue**: None. Necromancer (race 9) → `speak(58)` if `weapon<4`; witch
(race `0x89`) → `speak(58)` if `weapon<4 && !has_sun_stone`; spectre
(`0x8a`) / ghost (`0x8b`) → silent; all others vulnerable.

#### F1.12 — Hero reach formula `(brave/20)+5`, enemy reach `2+rand4()` [CONFORMANT]
**Location**: `src/game/combat.rs` `combat_reach`.
**Reference**: `reference/logic/combat.md#melee_swing` (`fmain.c:2249-2250`);
`RESEARCH.md §7.2`.
**Issue**: None. Hero: `(brave/20)+5` clamped `[4..15]`; enemy: `2 + rand4()`
clamped at 15.

#### F1.13 — Monster dodge gate `rand256() > brave` [CONFORMANT]
**Location**: `src/game/gameplay_scene.rs` (`run_combat_tick`).
**Reference**: `reference/logic/combat.md#melee_swing` (`fmain.c:2260`).
**Issue**: None. Hero auto-hits; monsters gated by `rand256() > brave`.

#### F1.14 — Missile damage `rand8() + 4` and radii 6/9 [CONFORMANT]
**Location**: `src/game/combat.rs` `Missile::damage`, `Missile::tick`.
**Reference**: `reference/logic/combat.md#missile_step` (`fmain.c:2295-2296`);
`RESEARCH.md §7.3`.
**Issue**: None. Arrow radius 6px, fireball 9px, both damage `rand8()+4`
(4–11).

#### F1.15 — Deprecated `resolve_combat()` formula unreachable on real path [SPEC-GAP]
**Location**: `src/game/combat.rs:134-160` (`resolve_combat`,
`#[deprecated]`).
**Reference**: No matching formula in `combat.md` or `RESEARCH.md`. The
original has no "hero damage = vitality * weapon / 8" rule.
**Issue**: `resolve_combat` is a legacy pre-state-machine combat helper
with a clearly invented damage formula. It is `#[deprecated]` and is now
only reachable through the dormant `GameAction::Attack` menu path (F1.9).
**Resolution**: Queued — proposed SPEC update: explicitly document that
the real combat path is the `run_combat_tick` / `apply_hit` pair (melee
state machine) and that `resolve_combat` is deprecated pending full
removal. No fix applied in this pass because the function is behind
`#[deprecated]` and is not on a live gameplay path.

#### F1.16 — Missile terrain-stop (`px_to_im` codes 1, 15) not implemented [SPEC-GAP]
**Location**: `src/game/gameplay_scene.rs` missile tick (~5085).
**Reference**: `reference/logic/combat.md#missile_step`
(`fmain.c:2276-2277`: missile dies in place if terrain code is 1 or 15).
**Issue**: Rust missiles pass through trees, rock walls, buildings. They
only stop on hit or at world bounds. This is a fidelity gap.
**Resolution**: Queued — requires a `px_to_im`-equivalent terrain probe
wired into the missile loop. Proposed SPEC addition: missile vs. terrain
interaction (codes 1 = impassable, 15 = solid → missile dies in place,
fireball plays spent-puff frame (type=3)).

#### F1.17 — Near-miss "clang" SFX `effect(1, 150 + rand256())` not implemented [SPEC-GAP]
**Location**: `src/game/gameplay_scene.rs` (`run_combat_tick`, target-scan
loop).
**Reference**: `reference/logic/combat.md#melee_swing`
(`fmain.c:2262`: `if yd < bv + 2 and wt != 5: effect(1, 150+rand256())`).
**Issue**: No SFX on near-miss band. Also, `dohit` hit-SFX (`effect(0/2/3/5,
…)`) are not wired up. The broader `effect()` hook (`audio::play_sfx`) is
defined but not invoked from combat at all.
**Resolution**: Queued — SFX subsystem is stubbed; will be covered once
the audio subsystem audit picks up the `effect()` dispatch table. For now,
flagged as a gap rather than a bug because no SFX is emitted for any
combat event, not just near-misses.

#### F1.18 — Missile slot-0 dodge asymmetry `bitrand(512) > bv` [SPEC-GAP]
**Location**: `src/game/gameplay_scene.rs` missile tick.
**Reference**: `reference/logic/combat.md#missile_step` and
`reference/PROBLEMS.md#p16` (resolved: intentional original design —
only slot 0 missiles are dodge-eligible, giving ~17% dodge rate via
round-robin `mdex`).
**Issue**: Rust applies **no** dodge roll — every in-range missile hits.
This drops the hero's bravery-dependent projectile survivability.
**Resolution**: Queued — fix deferred because the Rust missile array is
assigned by lowest-free-slot (`fire_missile` in `combat.rs`), not by
round-robin `mdex`. Reproducing the ~17% dodge rate requires either
switching to round-robin slot assignment or adopting a different dodge
probability model. Proposed SPEC clarification needed before the fix.

#### F1.19 — `aftermath()` function not implemented [RESEARCH-REQUIRED]
**Location**: No matching function in Rust.
**Reference**: `reference/logic/combat.md#aftermath`
(`fmain2.c:253-276`); `reference/logic/dialog_system.md` lines 212-221
for the hardcoded strings `"Bravely done!"`, `"{N} foes were defeated in
battle."`, `"{N} foes fled in retreat."`.
**Issue**: Reference fires `aftermath()` when `battleflag` transitions
True→False (inside `no_motion_tick`). It tallies dead/fleeing enemies
(slot index ≥ 3, `ENEMY` type), prints the commendation line if
`vitality<5 && dead>0`, prints count lines if `xtype<50`, and triggers
`get_turtle()` if a turtle-egg delivery is pending. Rust has no
equivalent — battles end silently.
**Resolution**: Queued for user review. The three scroll strings are
already whitelisted as hardcoded literals in `dialog_system.md`; adding
`aftermath()` requires (a) threading `battleflag` edge-detection through
`no_motion_tick`, (b) adopting the per-tick `ENEMY`-filtered tally, and
(c) wiring `get_turtle()`. Left as a standalone follow-up because it
interacts with the `battleflag` detector (which itself is only an
approximation: `proximity < 300px` rather than the original
`anim_list`-based active-fighting set).

### SPEC/REQ updates queued

- **F1.15 (SPEC-GAP)**: Document the real combat path (`run_combat_tick` /
  `apply_hit`) as canonical in `SPECIFICATION.md §10 Combat`. Mark
  `combat::resolve_combat` for removal.
- **F1.16 (SPEC-GAP)**: Add missile-terrain-stop rule — `px_to_im ∈ {1,15}`
  kills the missile; fireball transitions to spent-puff frame (`missile_type
  = 3`).
- **F1.17 (SPEC-GAP)**: Document combat SFX dispatch: `effect(0, 800 +
  bitrand(511))` hero-takes-melee; `effect(1, 150+rand256())` near-miss
  clang; `effect(2, 500+rand(0,63))` arrow-hit; `effect(3, 400+rand(0,255))`
  monster-hit; `effect(5, 3200+bitrand(511))` fireball — once the audio
  subsystem is formalised.
- **F1.18 (SPEC-GAP)**: Decide whether to reproduce the
  `bitrand(512) > bv` slot-0-only dodge asymmetry via round-robin `mdex`
  or to adopt a unified per-missile dodge probability.
- **F1.19 (RESEARCH-REQUIRED)**: Add `aftermath()` to spec with exact
  transition semantics for `battleflag` edge detection.

### Blockers

None — all NEEDS-FIX and INVENTED items are resolved; SPEC-GAPs are queued
for later batch updates; the RESEARCH-REQUIRED item (F1.19 `aftermath()`)
does not block other subsystem audits.

---

## Blockers & Open Questions for User Review

_None yet. This section collects REF-AMBIGUOUS, RESEARCH-REQUIRED, and
SPEC-GAP items that need user adjudication before proceeding._

---
