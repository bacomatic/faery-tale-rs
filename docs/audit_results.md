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

## Subsystem 2: magic — ✅ Complete

**Reference**: `reference/logic/magic.md` (+ `messages.md` event 21 / event 34
/ speech 59, `dialog_system.md` line 339 `"That feels a lot better!"`,
`combat.md#checkdead` for the Jade Skull brave accounting).
**Code**: `src/game/magic.rs`, `try_cast_spell` + `GameAction::CastSpell1..7`
dispatch in `src/game/gameplay_scene.rs` (~645, ~3037).
**Audit date**: 2025 (current session)

### Summary
- **10 findings**: 1 CONFORMANT, 3 NEEDS-FIX (all fixed), 1 INVENTED
  (resolved — spans seven invented scroll-text strings consolidated into one
  finding), 3 SPEC-GAP (queued), 0 REF-AMBIGUOUS, 2 RESEARCH-REQUIRED
  (queued; both cosmetic rendering).
- Fixes applied in **one commit** (`13100609`).
- Build/tests: ✅ `cargo build` clean (no new warnings); `cargo test` —
  567 + 12 + 12 tests passing (+4 new magic tests).

### Findings

#### F2.1 — Invented scroll-text for every spell outcome [INVENTED]
**Location**: `src/game/magic.rs:79-153` (old `use_magic`).
**Reference**: `reference/logic/magic.md#magic_dispatch` (`fmain.c:3300-3365`);
`reference/logic/dialog_system.md` (no entries for any of the strings below).
**Issue**: `use_magic` returned `Result<&'static str, &'static str>`, and
every branch fabricated a player-facing line. None of these come from
`faery.toml [narr]` or `dialog_system.md`'s hardcoded-literal list:
- `"You have none of that."` (should be `event(21)`)
- `"Not a magic item"` (unreachable in ref)
- `"You must stand on a stone ring to use this."` (ref is silent)
- `"Move to the center of the stone ring."` (ref is silent)
- `"The stone ring transports you!"` (ref is silent)
- `"The stone ring glows but nothing happens here."` (ref is silent)
- `"A warm light surrounds you."` (Green Jewel — ref is silent)
- `"You feel unseen."` (Crystal Orb — ref is silent)
- `"The bird totem does not work indoors."` (ref is silent)
- `"The bird totem shows the way."` (ref is silent)
- `"You cannot use the ring while riding."` (ref is silent)
- `"Time slows around you."` (Gold Ring — ref is silent)
- `"Death takes them all!"` / `"No enemies to claim."` (Jade Skull — ref:
  `event(34)` only on `battleflag`)
- `"That feels a lot better!"` emitted **unconditionally** after heal
  (ref: only in the non-capped branch at `fmain.c:3352`)

Violates the two-source scroll-text rule (SPEC §23.6, REQ R-INTRO-012).
**Resolution**: Replaced `use_magic`'s `Result<&str, &str>` with a
structured `MagicResult` enum (`NoOwned` / `Suppressed` / `Applied` /
`Healed { capped }` / `StoneTeleport { capped }` /
`MassKill { slain, in_battle }`) that carries **no** prose. Caller
`try_cast_spell` in `gameplay_scene.rs` now emits:
- `event_msg(21)` on `NoOwned` (ref `fmain.c:3303`);
- silent on `Suppressed` (ref silent return, charge preserved);
- silent on `Applied` (ref silent);
- `"That feels a lot better!"` (dialog_system.md:339 literal) only when
  `capped == false`, per `fmain.c:3352`;
- `event_msg(34)` only when Jade Skull kills with `battleflag == true`.

#### F2.2 — Blue Stone missing fall-through heal [NEEDS-FIX]
**Location**: `src/game/magic.rs:80-105` (old stone-ring branch).
**Reference**: `reference/logic/magic.md#magic_dispatch` notes
("Blue Stone fall-through") — `fmain.c:3326` `case 5:` has no `break;`
before `case 7:` at `fmain.c:3348`, so every successful teleport **also**
runs `vitality += rand8() + 4` clamped at `15 + brave/4`. Confirmed in
[RESEARCH §10](../reference/RESEARCH.md#10-inventory--items).
**Issue**: Rust teleported but never healed. Players lost the intended
free Glass Vial effect on every stone-circle use.
**Resolution**: Extracted the Glass Vial heal into a shared
`apply_vial_heal(state) -> bool` helper (returns `capped`); both the
stone-ring branch and the vial branch call it. Stone ring now returns
`MagicResult::StoneTeleport { capped }` and `try_cast_spell` emits the
heal message when `!capped` — mirroring the original C fall-through.
Added `test_stone_ring_falls_through_to_heal`.

#### F2.3 — Heal message printed even when vitality was capped [NEEDS-FIX]
**Location**: `src/game/magic.rs` Glass Vial branch (old ~110-116).
**Reference**: `reference/logic/magic.md#magic_dispatch` (`fmain.c:3350-3352`):
```
if vitality > cap:
    vitality = cap
else:
    print("That feels a lot better!")   # fmain.c:3352 — only in non-capped branch
```
**Issue**: Rust pushed the message on every successful heal, including
when the rolled heal overshot the cap and was clamped. Ref only prints
when there was room to heal.
**Resolution**: `apply_vial_heal` returns a `capped` flag; caller only
emits the string when `!capped`. Added `test_vial_heal_capped_flag`.

#### F2.4 — Jade Skull net-brave decrement is a fidelity bug [NEEDS-FIX]
**Location**: `src/game/magic.rs` Jade Skull branch (old ~151).
**Reference**: `reference/logic/magic.md#magic_dispatch`
(`fmain.c:3357-3359`) and `reference/logic/combat.md#checkdead`
(`fmain.c:2777` — `brave += 1` on every `i != 0` kill).
**Issue**: Ref loop body is
```
an.vitality = 0
checkdead(i, 0)      # brave += 1 (i != 0 branch)
brave = brave - 1    # explicit magic-dispatch penalty
```
Net effect: **`brave` unchanged per kill** (the +1 and −1 cancel). Rust
only did the `-1` and never called `checkdead`, producing a net −N on
N kills — a hidden cowardice penalty nowhere in the original.
**Resolution**: Removed the `state.brave -= killed` line. Net brave is
now 0 per kill, matching the ref. Updated
`test_jade_skull_no_brave_change` (formerly asserted −3; now asserts
no change) and added `test_jade_skull_battleflag_reported`,
`test_jade_skull_skips_race_7_plus`. The full `checkdead`
STATE_DYING / loot drop consequence for Jade Skull kills is flagged
below as F2.8 SPEC-GAP — the brave arithmetic is now correct, but the
death-transition plumbing (drops, `STATE_DYING`, `actor_tick` death
step) is still missing.

#### F2.5 — `event(21)` / `event(34)` / `speak(59)` ordering [NEEDS-FIX]
**Location**: `src/game/gameplay_scene.rs` `try_cast_spell` (was ~645-668).
**Reference**: `reference/logic/magic.md#magic_dispatch`
(`fmain.c:3303-3304`): not-owned check **precedes** the `extn.v3 == 9`
arena gate.
**Issue**: Old Rust checked the arena first (`speak(59)`) and only
called `use_magic` when outside the arena; inside the arena, an
attempt to cast an unowned spell would fire `speak(59)` instead of
`event(21)`. Additionally, `event(21)` and `event(34)` were not wired
at all — the NoOwned path used an invented string (F2.1) and the mass-
kill path never fired `event(34)` regardless of `battleflag`.
**Resolution**: Reordered: (1) not-owned ⇒ `event(21)`, (2) arena ⇒
`speak(59)`, (3) `use_magic` dispatch. `event(34)` now fires from the
`MassKill { in_battle: true, .. }` arm via
`events::event_msg(narr, 34, bname)`. All three strings resolve via
`faery.toml [narr].event_msg` / `[narr].speeches`.

#### F2.6 — `extn.v3 == 9` gate lives in caller, not magic_dispatch [CONFORMANT]
**Location**: `src/game/gameplay_scene.rs:650-660` (`try_cast_spell`
arena check) vs `src/game/magic.rs`.
**Reference**: `reference/logic/magic.md#magic_dispatch` (`fmain.c:3304`
— `if extn.v3 == 9: speak(59); return`).
**Issue**: The gate is inside `magic_dispatch` in the original but is
split into `try_cast_spell` in Rust because the extent lookup needs
zone data that lives on `GameplayScene`, not `GameState`. Behavior is
equivalent: identical `speak(59)` emission, identical no-consume
semantics, and the ordering relative to the not-owned check now
matches the ref (see F2.5).

#### F2.7 — Magic Wand fireball dispatches through combat `missile_step` [CONFORMANT]
**Location**: `src/game/gameplay_scene.rs:3058-3083`
(`GameAction::Shoot`); `src/game/combat.rs` `fire_missile`.
**Reference**: `reference/logic/magic.md` Notes ("Magic Wand vs the
Necromancer/Witch", `fmain.c:1693`); `reference/logic/combat.md#missile_step`.
**Issue**: None. Wand `weapon == 5` fires a fireball missile (mt=9) at
no ammo cost — Rust's `Shoot` branch only decrements `stuff[8]` when
`weapon == 4` (bow). `stuff[4]` is treated as a binary equip flag and
is never decremented per shot. Immunity bypass (`weapon >= 4` for
necromancer, `weapon >= 4 && stuff[7] != 0` for masked witch) already
covered in Subsystem 1 F1.11.

#### F2.8 — Jade Skull skips STATE_DYING / loot / race-drop pipeline [SPEC-GAP]
**Location**: `src/game/magic.rs` Jade Skull branch.
**Reference**: `reference/logic/magic.md#magic_dispatch`
(`fmain.c:3359` `checkdead(i, 0)`); `reference/logic/combat.md#checkdead`;
`reference/logic/combat.md` Notes on race-specific drops (emitted in
`actor_tick` STATE_DYING branch).
**Issue**: Ref Jade Skull routes each kill through `checkdead`, which
transitions the actor to `STATE_DYING` with `tactic = 7`; the
subsequent `actor_tick` frames then run race-specific loot drops and
the dying-animation countdown. Rust Jade Skull only sets `vitality = 0`
on the matching actors. No loot rolls fire, no dying animation plays,
and the actors sit inert. Since the `race < 7` filter excludes
Necromancer (9), masked Witch (`0x89`), Spectre (`0x8a`), and all
SETFIGs, this is purely about standard-enemy loot / death animation
— not about quest-critical drops.
**Resolution**: Queued — requires either (a) extracting Rust's
apply_hit death-transition block into a shared helper callable from
`magic::use_magic`, or (b) relocating Jade Skull's per-actor kill into
`try_cast_spell` where the loot + death-transition machinery is
already available. Proposed SPEC addition: explicitly document that
Jade Skull routes each kill through the same `checkdead` / death-
transition pipeline as a melee kill, including `loot::roll_treasure`
and the STATE_DYING → STATE_DEAD animation countdown.

#### F2.9 — `colorplay()` 32-frame palette strobe on stone-ring teleport [SPEC-GAP]
**Location**: `src/game/magic.rs` stone-ring branch.
**Reference**: `reference/logic/magic.md#magic_dispatch`
(`fmain.c:3336`: `colorplay()` — 32-frame palette strobe before `xfer`).
**Issue**: Rust teleports instantly with no palette cue. Not a gameplay
bug but a visible-feedback fidelity loss.
**Resolution**: Queued. Proposed SPEC addition under §19.2: stone-ring
teleport must run a 32-frame palette strobe (documented in
`reference/logic/visual-effects.md`) before changing `hero_x/y`.

#### F2.10 — Bird Totem "+" marker not rendered on map bitmap [RESEARCH-REQUIRED]
**Location**: `src/game/magic.rs` totem branch (`state.viewstatus = 1`).
**Reference**: `reference/logic/magic.md#magic_dispatch`
(`fmain.c:3311-3325`): `bigdraw` blits the world map, then the block
computes hero pixel coords `(i, j)` and draws `"+"` at palette pen 31
with `JAM1` draw-mode; clips to `0 < i < 320` and `0 < j < 143`.
**Issue**: Rust only sets `viewstatus = 1`. The map overlay path has
to be wired to the actual map-bitmap surface and a `"+"` glyph plotted
at the hero's scaled position. No corresponding Rust rendering code
exists yet.
**Resolution**: Queued for user review. The marker geometry
(`(hero_x >> 4) - ((secx + xreg) << 4) - 4`, pen 31, `JAM1`) is well-
specified in ref; implementation requires map-overlay hooks that are
not yet present in `src/scenes/map_scene.rs` (or wherever map
rendering lives).

### SPEC/REQ updates queued

- **F2.8 (SPEC-GAP)**: `SPECIFICATION.md §19.2` should state that Jade
  Skull routes each killed actor through the standard `checkdead` /
  STATE_DYING pipeline — including `loot::roll_treasure` and the
  dying-animation countdown — not just `vitality = 0`.
- **F2.9 (SPEC-GAP)**: `SPECIFICATION.md §19.2` should add a
  "32-frame palette strobe (`colorplay()`) runs immediately before the
  teleport" note for the Blue Stone branch.
- **F2.10 (RESEARCH-REQUIRED)**: Add Bird Totem map-overlay "+" marker
  rendering requirement with the exact geometry and pen index (31,
  JAM1) once the map-overlay rendering path is formalised.

### Blockers

None — all NEEDS-FIX and INVENTED items are resolved; SPEC-GAPs and the
single RESEARCH-REQUIRED item (F2.10, cosmetic map marker) are queued
for later batch updates and do not block other subsystem audits.

---

## Blockers & Open Questions for User Review

_None yet. This section collects REF-AMBIGUOUS, RESEARCH-REQUIRED, and
SPEC-GAP items that need user adjudication before proceeding._

---
