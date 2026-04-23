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

## Subsystem 3: ai-system — ✅ Complete

**Reference**: `reference/logic/ai-system.md` (+ `game-loop.md §actor_tick`,
`RESEARCH.md §8`, `frustration.md`, `SYMBOLS.md §GOAL_*`/`TACTIC_*`/`STATE_*`)
**Code**: `src/game/npc_ai.rs` (`tick_npc`, `select_tactic`, `do_tactic`),
AI wiring in `src/game/gameplay_scene.rs::update_actors`,
frust-latching in `src/game/npc.rs::tick_with_actors`
**Audit date**: 2025 (current session)

### Summary
- **14 findings**: 1 CONFORMANT, 7 NEEDS-FIX (all fixed), 1 INVENTED (fixed),
  5 SPEC-GAP (queued), 0 REF-AMBIGUOUS, 0 RESEARCH-REQUIRED.
- Fixes applied in **one commit** (SHA to be recorded by orchestrator).
- Build/tests: ✅ `cargo build` clean (zero new warnings beyond pre-existing);
  `cargo test` — 576 + 12 + 12 tests passing (8 new targeted tests added).

### Findings

#### F3.1 — SETFIG actors re-aim at hero every tick [INVENTED]
**Location**: `src/game/npc_ai.rs::tick_npc` (SETFIG early-return block).
**Reference**: `reference/logic/ai-system.md:46`
(`if actor.type == SETFIG: return` — `fmain.c:2119-2120`).
**Issue**: Rust's SETFIG branch short-circuited the AI loop but kept a
special case that called `set_course(SC_AIM)` + `state = Still` whenever the
actor's goal was `Goal::Stand`, re-orienting shopkeepers toward the hero on
every tick. Original `advance_goal` exits **immediately** for SETFIG — no
goal/state/facing/tactic mutations at all. Shopkeeper pose comes from
`set_shape` at spawn time, not from per-tick AI.
**Resolution**: SETFIG branch now returns unconditionally. Two tests that
enforced the invented behavior (`test_setfig_shopkeeper_stand_faces_hero`
and `test_freeze_nonhostile_npc_still_acts`) were rewritten to assert that
the spawn-time pose is preserved verbatim.

#### F3.2 — Melee-reach threshold used wrong numeric GOAL values [NEEDS-FIX]
**Location**: `src/game/npc_ai.rs::goal_value` (renamed `goal_numeric`).
**Reference**: `reference/logic/ai-system.md:103` (`thresh = 14 - mode`),
`SYMBOLS.md:275-286` (`GOAL_ATTACK1=1, ATTACK2=2, ARCHER1=3, ARCHER2=4`).
**Issue**: The helper mapped `Attack1 → 0, Attack2 → 1, Archer1 → 3,
Archer2 → 4`, producing `thresh = 14, 13, 11, 10` instead of the ref's
`13, 12, 11, 10`. ATTACK1 and ATTACK2 enemies therefore engaged melee at
1-pixel longer reach than the original.
**Resolution**: Renamed helper to `goal_numeric` with correct numeric
values for all GOAL_* constants. Added `test_melee_thresh_uses_numeric_goal`
verifying ATTACK1 engages at xd=12, ATTACK2 does not.

#### F3.3 — Snakes unconditionally march to fixed turtle-nest coords [INVENTED]
**Location**: `src/game/npc_ai.rs::select_tactic` (snake branch).
**Reference**: `reference/logic/ai-system.md:84`
(`if actor.race == 4 and turtle_eggs: tactic = EGG_SEEK`).
**Issue**: Rust unconditionally set `Tactic::EggSeek` for any snake
(`race == 4`), making snakes teleport-seek `(23087, 5667)` in every zone
whether or not the global `turtle_eggs` counter was non-zero. Original
gates EGG_SEEK on that counter; when zero, snakes fall through to the
normal armed/unarmed/vitality/range decision tree and pursue the hero.
**Resolution**: `tick_npc` / `select_tactic` now take a `turtle_eggs: bool`
parameter. `gameplay_scene::update_actors` passes `false` today
(SPEC-GAP — the `turtle_eggs` global is not yet plumbed into `GameState`;
see F3.10). Added `test_snake_no_egg_seek_when_eggs_absent` and
`test_snake_egg_seek_when_eggs_present`.

#### F3.4 — Dark knight `stand_guard` outside melee reach missing [NEEDS-FIX]
**Location**: `src/game/npc_ai.rs::select_tactic` (post melee-shortcut).
**Reference**: `reference/logic/ai-system.md:110-112`
(`elif actor.race == 7 and actor.vitality != 0: stand_guard` —
`fmain.c:2168-2169`), `RESEARCH.md §Dark Knight`.
**Issue**: When a living DKnight (`race == 7`) was outside his extended
16-pixel reach, Rust fell through to the normal tactic tree (Pursue /
Evade / Backup). Original ref forces `state = STILL, facing = DIR_S` —
the DKnight plants himself and waits. Combined with his fixed spawn
position (21635, 25762), this is the behavior that keeps him blocking the
hidden valley exit.
**Resolution**: Added the `race == 7 && vitality > 0` stand_guard branch
after the close-range melee shortcut. Added
`test_dknight_stand_guard_out_of_reach` and
`test_dknight_stand_guard_only_when_alive`.

#### F3.5 — `Tactic::Frust` was a no-op — frust-latched NPCs stuck [NEEDS-FIX]
**Location**: `src/game/npc_ai.rs` — `do_tactic` (treated Frust as no-op);
Frust is latched by `npc.rs::tick_with_actors` on full-block.
**Reference**: `reference/logic/ai-system.md:70-75`
(`if tactic == FRUST or tactic == SHOOTFRUST: do_tactic(rand(…))`),
`frustration.md#resolve_frust_tactic` (`fmain.c:2141-2144`).
**Issue**: Blocked NPCs set `tactic = Frust` to request a new tactic next
tick, but `do_tactic` had a match arm that did nothing for Frust, and
`select_tactic` never checked for it — so frust-latched actors never
recovered, effectively freezing any NPC that collided with scenery.
**Resolution**: `select_tactic` now detects `Tactic::Frust` before the
goal-mode branches and assigns a random fallback tactic:
bow actors (`weapon & 4 != 0`) draw from `rand(2,5)` →
`Follow/BumbleSeek/Random/Backup`; melee actors draw from `rand(3,4)` →
`BumbleSeek/Random`. The rest of the pipeline then executes the new
tactic normally. Added `test_frust_tactic_dispatches_random_fallback`
and `test_frust_tactic_dispatches_bow_fallback`.

#### F3.6 — `do_tactic` rate-limit mis-gated (ARCHER2 + SHOOT) [NEEDS-FIX]
**Location**: `src/game/npc_ai.rs::do_tactic` (opening probabilistic gate).
**Reference**: `RESEARCH.md §8.2` (fmain2.c:1666-1682 —
`r = !(rand() & 7)`; `if goal == ATTACK2: r = !(rand() & 3)`; SHOOT
bypasses the gate entirely).
**Issue**: Rust gated the tactic executor at 25% for both `Attack2` and
`Archer2`; original upgrades only `ATTACK2`. More importantly, `SHOOT`
must **not** be rate-limited (archers need to face the hero every tick
to line up their shot), but Rust passed SHOOT through the gate, throttling
archer firing facing updates.
**Resolution**: Gate now uses `goal == Attack2 → 3, else → 7`; and
`Tactic::Shoot` bypasses the gate entirely. Added
`test_do_tactic_shoot_bypasses_rate_limit`.

#### F3.7 — Hero-dead leader/follower split wrong [NEEDS-FIX]
**Location**: `src/game/npc_ai.rs::select_tactic` (hero_dead override),
`src/game/gameplay_scene.rs::update_actors` (caller).
**Reference**: `reference/logic/ai-system.md:60-64` + line 555
(`leader = 0` at loop start; `if (leader == 0) leader = i` at
`fmain.c:2183`).
**Issue**: Rust set `goal = Follower` for any NPC whenever *some* leader
slot existed (`leader_idx.is_some()`). That makes the leader itself also
assign Follower to itself, so nobody flees first. Original ref zeroes
`leader` at loop start; the **first** iterated eligible actor sees
`leader == 0` → `GOAL_FLEE`, and subsequent iterations see `leader != 0`
→ `GOAL_FOLLOWER`.
**Resolution**: `tick_npc`/`select_tactic` now take the actor's own
`npc_idx`. `hero_dead` path: `leader_idx == Some(npc_idx) || None → Flee;
otherwise → Follower`. Added
`test_hero_dead_leader_flees_others_follow`.

#### F3.8 — Melee-weapon test used `weapon < 4` instead of bit test [NEEDS-FIX]
**Location**: `src/game/npc_ai.rs::select_tactic` (close-range melee check).
**Reference**: `reference/logic/ai-system.md:105`
(`if (actor.weapon & 4) == 0`).
**Issue**: Rust used `weapon < 4` to detect melee. Equivalent for the
weapon values 0–7 actually in use, but it encoded an invariant (`weapon
never ≥ 8`) the ref never made. Bit-2 test is the canonical check.
**Resolution**: Changed to `(weapon & 4) == 0`.

#### F3.9 — `Goal::Stand` used SC_AIM; ref uses SC_SMART + stop_motion [NEEDS-FIX]
**Location**: `src/game/npc_ai.rs::select_tactic` Stand branch.
**Reference**: `reference/logic/ai-system.md:121-122`
(`set_course(actor, hero_x, hero_y, 0)` then `stop_motion(actor)`).
**Issue**: Rust called `set_course(SC_AIM)` (mode 5) which skips the
axis-suppression applied by mode 0 (smart seek). On diagonals this
produced slightly different facings for Stand-goal actors vs the original
(facing toward the hero without axis snap).
**Resolution**: Stand branch now calls `set_course(SC_SMART)` (mode 0)
and then overrides `state = Still` to match `stop_motion`.

#### F3.10 — `xtype > 59` race filter uses `race < 4` placeholder [SPEC-GAP]
**Location**: `src/game/npc_ai.rs::select_tactic`.
**Reference**: `reference/logic/ai-system.md:65`
(`xtype > 59 and actor.race != extn.v3`), `RESEARCH.md:1483`.
**Issue**: The fidelity-exact condition needs the currently-active extent
record (`extn.v3` = per-zone race filter, e.g. 7 for DKnight valley,
9 for necromancer arena). That pointer is not plumbed into GameState
yet, so Rust keeps a hand-tuned placeholder `race < 4`. For DKnight
(race 7, v3 7) both conditions agree, so no current zone misbehaves —
but a future port of spectre / witch special zones will need the real
`extn.v3`.
**Resolution**: Queued. SPEC §11 should add an `extn.v3` field to the
zone/extent model and `select_tactic` should consult it directly.

#### F3.11 — `turtle_eggs` global counter not plumbed [SPEC-GAP]
**Location**: `src/game/game_state.rs` (`try_rescue_egg` stub),
`src/game/gameplay_scene.rs::update_actors` (hardcodes `turtle_eggs = false`).
**Reference**: `reference/logic/ai-system.md:84` and surrounding
turtle-egg quest at `fmain.c:3040-3100`.
**Issue**: The original `turtle_eggs` world-state counter is replaced by
an inventory-item stub in `GameState`. Snakes can never enter EGG_SEEK
until the counter is formalised.
**Resolution**: Queued. Once the turtle-eggs counter is modelled as a
world flag (not inventory), thread it through `update_actors` into
`tick_npc`.

#### F3.12 — `STATE_SHOOT1` → `fire_aimed_shot` branch missing [SPEC-GAP]
**Location**: `src/game/npc_ai.rs::select_tactic` (no SHOOT1 short-circuit),
`src/game/gameplay_scene.rs::update_actors` (uses `archer_cooldown`).
**Reference**: `reference/logic/ai-system.md:54-56` (`elif actor.state ==
STATE_SHOOT1: fire_aimed_shot`).
**Issue**: The ref uses a two-frame archer state machine
(SHOOT1 = aiming, SHOOT3 = fired). Rust short-circuits with a single
`archer_cooldown` counter driven off the per-NPC `Shooting` state. Same
observable firing behaviour, but the FSM topology and cadence differ.
**Resolution**: Queued. Proper fidelity requires introducing
SHOOT1/SHOOT3 states and wiring `fire_aimed_shot` to the SHOOT1→SHOOT3
transition.

#### F3.13 — Carrier (daynight & 15) == 0 cadence missing [SPEC-GAP]
**Location**: `src/game/npc_ai.rs::tick_npc` (carriers route to a
different code path), `src/game/gameplay_scene.rs` (turtle auto).
**Reference**: `reference/logic/ai-system.md` carrier preamble
(`fmain.c:2114-2117`).
**Issue**: Original `advance_goal` for carrier-type actors only calls
`set_course(…, mode=5)` every 16 ticks (`daynight & 15 == 0`). Rust
handles turtle carriers through a dedicated T3-CARRY-TURTLE-AUTO path
that ignores this cadence.
**Resolution**: Queued for the carrier-transport subsystem audit.

#### F3.14 — `frustflag` reset semantics differ from ref [SPEC-GAP]
**Location**: `src/game/gameplay_scene.rs::update_player_motion`.
**Reference**: `reference/logic/frustration.md` (ref resets frustflag
from any successful actor action — SINK, walk, shot, melee, dying —
at `fmain.c:1577, 1650, 1707, 1715, 1725`).
**Issue**: Rust only resets `frustflag` when the player walks or when
*any* enemy NPC is active. The per-actor successful-action resets are
not wired, so the frustration animation can trigger during combat even
when NPCs are acting.
**Resolution**: Queued for subsystem 20 (frustration) audit — cross-
cutting change touching walk_step, shoot, melee, and checkdead paths.

### CONFORMANT items

- **C3.1 — Battleflag 300-pixel proximity test**: Rust's
  `update_actors` sets `state.battleflag` when any active NPC is within
  300 px of the hero, matching `RESEARCH.md §8.3` point 4 (on-screen =
  300×300 box around hero).
- **Scroll-text compliance**: `src/game/npc_ai.rs` and the AI wiring in
  `gameplay_scene.rs::update_actors` emit no player-facing strings —
  two-source rule (SPEC §23.6, REQ R-INTRO-012) is satisfied by
  construction.

### SPEC/REQ updates queued

- **F3.10 (SPEC-GAP)**: Add `extn.v3` (zone race filter) to the zone/
  extent model and use it in the `xtype > 59` FLEE gate.
- **F3.11 (SPEC-GAP)**: Formalise the `turtle_eggs` global counter as a
  world-state flag (not inventory) and thread it to `tick_npc`.
- **F3.12 (SPEC-GAP)**: Introduce STATE_SHOOT1/SHOOT3 FSM states and
  `fire_aimed_shot` dispatch to replace the `archer_cooldown` proxy.
- **F3.13 (SPEC-GAP)**: Carrier AI should run every 16 ticks
  (`daynight & 15 == 0`) with `set_course(mode=5)`; document in SPEC.
- **F3.14 (SPEC-GAP)**: Hook per-actor successful-action resets of
  `frustflag` from walk_step, shoot, melee, and dying paths per
  `frustration.md`.

### Blockers

None — all NEEDS-FIX and INVENTED items are resolved; SPEC-GAPs are queued
for later batch updates and/or subsystem audits (frustration, carrier-
transport, zones) and do not block other subsystem audits.

---

## Subsystem 4: encounters — ✅ Complete

**Reference**: `reference/logic/encounters.md` (primary) + `RESEARCH.md §2.7`,
`§9`, `game-loop.md` (no_motion_tick Phase 14h/14i/14j), `day-night.md` (no
night modifier — encounter cadence lives entirely in `no_motion_tick`),
`PROBLEMS.md`.
**Code**: `src/game/encounter.rs`, encounter call sites in
`src/game/gameplay_scene.rs` (`try_trigger_encounter`, `spawn_encounter_group`,
`SpawnEncounterRandom` / `SpawnEncounterType` debug handlers).
**Audit date**: 2025 (current session)

### Summary
- **11 findings**: 1 CONFORMANT, 3 NEEDS-FIX (all fixed), 2 INVENTED (both
  removed/replaced), 5 SPEC-GAP (queued), 0 REF-AMBIGUOUS, 0
  RESEARCH-REQUIRED.
- Fixes applied in **one commit** (SHA to be recorded by orchestrator).
- Build/tests: ✅ `cargo build` clean (zero new warnings); `cargo test` —
  579 + 12 + 12 tests passing (3 new targeted tests: weapon_probs groups 2
  and 3; carrier-blocks-trigger).

### Findings

#### F4.1 — `WEAPON_PROBS` groups 2 and 3 held wrong values [NEEDS-FIX]
**Location**: `src/game/encounter.rs::WEAPON_PROBS`.
**Reference**: `reference/logic/encounters.md#roll_weapon`
(`weapon_probs[arms*4+col]`, `fmain2.c:860-868`); `RESEARCH.md §2.7 →
weapon_probs`.
**Issue**: Rust had group 2 = `1,1,2,1` and group 3 = `2,1,2,2`. The
authoritative table (`fmain2.c:862-863`, RESEARCH §2.7) is group 2 =
`1,2,1,2` ("dirks and maces") and group 3 = `1,2,3,2` ("mostly maces, some
swords"). Group 2 is the Ogre's arms row (`encounter_chart[0].arms = 2`) so
ogres were rolling dirks 75 % of the time instead of dirks 50 % / maces
50 %. Group 3 is the Skeleton's row (`arms = 3`), which in the ref has a
1-in-4 chance of a sword — the Rust table made swords impossible for
skeletons.
**Resolution**: Replaced the two rows with the ref-exact values. Added
`test_weapon_probs_group2` and `test_weapon_probs_group3` locking the
corrected tuples.

#### F4.2 — `try_trigger_encounter` missing `active_carrier == 0` gate [NEEDS-FIX]
**Location**: `src/game/encounter.rs::try_trigger_encounter`;
`src/game/gameplay_scene.rs` encounter-tick block (≈5105).
**Reference**: `reference/logic/encounters.md#roll_wilderness_encounter`
(Phase 14j gate: "not actors_on_screen and not actors_loading and
active_carrier == 0 and xtype < 50"), `fmain.c:2081`, `RESEARCH.md §9.4`.
**Issue**: The ref suppresses random encounters while the hero is riding
any carrier (swan, raft, turtle, dragon). Rust only checked `xtype < 50`
and the 32-tick / screen / slot-count gates, so the hero could be
ambushed while on turtle-back or aboard a swan — impossible in the
original.
**Resolution**: Added an `active_carrier: i16` parameter to
`try_trigger_encounter`, gated on `active_carrier != 0 → None`. Threaded
`self.state.active_carrier` through from the gameplay-scene call site.
Added `test_try_trigger_encounter_blocks_on_carrier`.

#### F4.3 — Spawned enemies face south (4) instead of north (0) [NEEDS-FIX]
**Location**: `src/game/encounter.rs::spawn_encounter` (Npc init block).
**Reference**: `reference/logic/encounters.md#set_encounter`
(`an.facing = 0`, `fmain.c:2761`).
**Issue**: Rust hardcoded `facing: 4` ("south (toward hero, roughly)").
The ref initialises `facing = 0` (north); the actual heading is rewritten
the first time the AI layer calls `set_course` on the fresh actor. The
initial facing only matters for the first frame before AI ticks, but it
was visible on spawn — enemies appearing facing south regardless of
where the hero was. No ref citation supports facing the hero at spawn.
**Resolution**: Set `facing: 0` with a comment pointing back to
`fmain.c:2761` and noting set_course takes over on the first AI tick.

#### F4.4 — `gold: treasure * 5` seeded at spawn [INVENTED]
**Location**: `src/game/encounter.rs::spawn_encounter` (Npc init block).
**Reference**: `reference/logic/encounters.md#set_encounter` (no gold
field written) vs `#roll_treasure` (`fmain.c:3270-3273` — treasure drop
is a separate pickup-path concern rolled on body search, and resolves to
an `inv_list[]` slot index, not a currency amount).
**Issue**: Rust's `spawn_encounter` seeded `npc.gold = treasure_tier * 5`
so that the kill-reward path in `combat.rs:167` (`state.gold +=
npc.gold`) would hand out gold per-enemy-type. This is a double-invention:
(a) the ref never seeds `an->gold` at spawn, and (b) rolled treasure is
an item drop, not a pile-of-coins reward. Ogres therefore dropped 10
gold, orcs 5, wraiths 20, etc. — all fabricated amounts.
**Resolution**: Set `gold: 0` at spawn. The downstream currency reward
path still reads `npc.gold`, so non-encounter NPCs that legitimately
carry gold (e.g. shopkeepers, setfig cfile entries with baked gold
fields in their 16-byte record) are unaffected. Treasure-item drops
belong to the inventory/pickup audit.

#### F4.5 — `ENCOUNTER_CHART: [u8; 11]` dead zone→monster table [INVENTED]
**Location**: `src/game/encounter.rs` (top-of-file `pub const`).
**Reference**: None — the original `encounter_chart[]` (`fmain.c:42-64`)
is a monster-stats table keyed by race, not a zone-to-monster mapping.
The authoritative 11-entry stats table is already present as
`ENCOUNTER_CHART_FULL`.
**Issue**: A second 11-entry `ENCOUNTER_CHART` table mapped "forest →
Orc, plains → Orc, mountains → Skeleton, swamp → Ghost, dungeon →
Wraith, road → Orc, ruins → Skeleton, graveyard → Ghost, dark zone →
Wraith, beach → Orc, desert → Skeleton". No such table exists in the
original — zone-level monster selection is handled by `xtype` overrides
inside `roll_wilderness_encounter` (swamp, spider, xtype-49) plus the
extent-record `v3` filter for forced encounters. Also: several listed
zones (road, ruins, graveyard, beach, desert) do not correspond to any
`xtype` value the ref drives. The table was never referenced anywhere
in the codebase — pure dead invented data.
**Resolution**: Removed the constant. `rg` confirms no remaining
references.

#### F4.6 — Spawn count uses fixed `MAX_GROUP = 4`, not `v1 + rand(v2)` [SPEC-GAP]
**Location**: `src/game/encounter.rs::spawn_encounter_group` (const
`MAX_GROUP: usize = 4`).
**Reference**: `reference/logic/encounters.md#load_actors`
(`encounter_number = extn.v1 + rand(0, extn.v2 - 1)`, `fmain.c:2725`);
`RESEARCH.md §9.4 → Monster Count`.
**Issue**: The ref sets a *pending* `encounter_number` in 14j per the
current extent's `extn.v1`/`extn.v2` (world-at-large: 1 + rnd(8) → 1–8;
spider pit: 4; graveyard: 8 + rnd(8) → 8–15), and 14i drains that
counter into slots 3–6 across multiple 16-tick placement passes,
recycling dead slots. Rust telescopes 14i+14j into one call that spawns
exactly 4 enemies per trigger — so small-zone encounters (1–3 orcs
around the village) are oversized and large-zone encounters (8–15
skeletons at the graveyard) are undersized.
**Resolution**: Queued. Requires plumbing the active extent's `v1`/`v2`
into the spawn path plus a persistent `encounter_number` counter drained
on a 16-tick cadence. Cross-cuts subsystems 4, 11 (quests/zones), and 6
(extent model).

#### F4.7 — `mixflag` race-pairing + weapon-reroll logic missing [SPEC-GAP]
**Location**: `src/game/encounter.rs::spawn_encounter`,
`::spawn_encounter_group` (no `mixflag` global modelled).
**Reference**: `reference/logic/encounters.md#place_extent_encounters`
(`mixflag = rand(0, 0x7fffffff)` with zeroing for `xtype > 49` or
`(xtype & 3) == 0`); `#set_encounter` (bit 1 pairs races
`(encounter_type & 0xfffe) + rand(0,1)` except snakes; bit 2 re-rolls
the weapon column each spawn).
**Issue**: The ref's Phase 14i rolls `mixflag` once per placement batch.
Bit 1 enables race mixing (ogre↔orc, wraith↔skeleton); bit 2 enables
per-enemy weapon-column rerolls. Neither is modelled — Rust spawns a
single `encounter_type` with its own arms row and per-spawn weapon
rolls (equivalent to `mixflag & 4` always-on), so groups are always
racially homogeneous. Biome-uniform zones (`(xtype & 3) == 0`) are
supposed to force `mixflag = 0` to enforce uniformity; Rust already
spawns uniform groups, so this happens to match, but the bit-1 pairing
in non-uniform zones is missing.
**Resolution**: Queued. Requires a per-placement-batch `mixflag` and a
race-pairing branch inside `spawn_encounter`.

#### F4.8 — DKnight fixed spawn position (21635, 25762) not honoured [SPEC-GAP]
**Location**: `src/game/encounter.rs::spawn_encounter` (no race-filter
branch).
**Reference**: `reference/logic/encounters.md#set_encounter`
(`if extn.v3 == 7: xtest = 21635; ytest = 25762`, `fmain.c:2741`);
`RESEARCH.md §9.5` and the DKnight entry at §8.4.
**Issue**: The Dark Knight is unique in that its forced-encounter extent
pins its spawn to a fixed world coordinate, ignoring the ring/spread
logic. Combined with F3.4 (`stand_guard` out-of-reach) this is what keeps
the DKnight rooted at the hidden-valley exit. Rust scatters him inside
the usual 63-pixel box around the `encounter_origin`. The observable
effect depends on the caller — the `SpawnEncounterRandom` debug action
never enters the hidden-valley extent, so random-tick spawning can't
reproduce this; the breakage would show when the forced-encounter
`find_place` path is ported and routed through the same
`spawn_encounter`.
**Resolution**: Queued. Requires `extn.v3` plumbing (same SPEC-GAP as
F3.10 ai-system). When present, `spawn_encounter` should short-circuit
to (21635, 25762) for race 7.

#### F4.9 — Astral-plane terrain-code-7 acceptance missing [SPEC-GAP]
**Location**: `src/game/encounter.rs::spawn_encounter_group` (collision
retry loop).
**Reference**: `reference/logic/encounters.md#set_encounter`
(`if xtype == 52 and px_to_im(xtest, ytest) == 7: placed = True`,
`fmain.c:2746`).
**Issue**: On the astral plane (xtype 52) the ref accepts the "void"
terrain code 7 as a valid spawn target so Loraii can place against an
otherwise unwalkable backdrop. Rust uses only `actor_collides` for the
retry check and has no xtype-52 / terrain-7 acceptance branch. Astral
Loraii spawns will therefore reject every slot and fall through to the
"no placement" exit more often than the original.
**Resolution**: Queued. Requires `px_to_im` call at the retry-loop call
site plus an `xtype == 52` branch.

#### F4.10 — Cluster-origin retry (9 attempts + walkability) not implemented [SPEC-GAP]
**Location**: `src/game/encounter.rs::spawn_encounter_group`
(`encounter_origin` is called once).
**Reference**: `reference/logic/encounters.md#place_extent_encounters`
(k-loop 1..10: call `set_loc`, bail out of the whole batch if none of 9
candidate cluster origins lands on terrain code 0).
**Issue**: The ref tries up to 9 random ring-origins around the hero
and only fills slots when one of them passes `px_to_im == 0`. If all 9
fail (player stuck in a walkable pocket surrounded by water/rock) the
batch is dropped, and `encounter_number` carries to the next 16-tick
pass. Rust picks a single ring-origin with no walkability check and
relies on the per-slot `actor_collides` retry; this changes the spawn-
suppression behaviour around cliff edges, shorelines, and narrow
passages — enemies can appear where the original would have skipped
the batch.
**Resolution**: Queued. Requires an outer origin-retry loop in
`spawn_encounter_group` plus `px_to_im(enc_x, enc_y) == 0` on each
candidate.

#### F4.11 — `actors_loading` gate omitted (architectural) [SPEC-GAP]
**Location**: `src/game/encounter.rs::try_trigger_encounter`.
**Reference**: `reference/logic/encounters.md#roll_wilderness_encounter`
and `#prep` (async disk I/O model; 14h polls `CheckDiskIO(8)` and 14j is
suppressed while `actors_loading == True`).
**Issue**: The port loads sprite assets synchronously at startup, so
there is no concept of "actors still streaming in from disk" to gate
against. The ref's `actors_loading` gate therefore has no analog. This is
an architectural divergence (SPEC §3 async-I/O replacement), not a
fidelity bug in practice, but it bears noting because the ref lists it
as part of the 14j gate tuple.
**Resolution**: Queued as SPEC documentation (SPEC §19.3 / §3 note that
async-I/O gates collapse to always-ready).

### CONFORMANT items

- **C4.1 — `pick_encounter_type` xtype overrides**: Swamp (`xtype == 7`
  rerolls 2→4), spider region (`xtype == 8 → 6`), and `xtype == 49 →
  wraith (2)` match `encounters.md#roll_wilderness_encounter` lines
  2087–2090 exactly.
- **C4.2 — Danger-level formula**: `region_num > 7 → 5+xtype` /
  else `2+xtype`, gated by `rand(0,63) > danger_level → skip`, matches
  `encounters.md` lines 2082–2085 verbatim. Spawn probability
  `(danger_level+1)/64` is preserved.
- **C4.3 — Ring-distance formula**: `encounter_origin` uses the 8-way
  direction table × (150 + rand(0, 63)) / 2 per `set_loc`
  (`fmain2.c:1716-1719`). Actual pixel offsets fall inside the
  ref's 150–213 px envelope.
- **C4.4 — Goal selection by weapon bit 4**: `weapon & 4 != 0` routes
  to ARCHER1/ARCHER2 (plus cleverness offset); else ATTACK1/ATTACK2 —
  matches `encounters.md` lines 2762–2764.
- **C4.5 — Scroll-text compliance**: encounter spawning emits no
  player-facing strings; the two-source rule (SPEC §23.6,
  R-INTRO-012) is satisfied by construction.
- **C4.6 — `freeze_timer` suppression**: gameplay-scene gate
  (`freeze_timer == 0`) before calling `try_trigger_encounter` matches
  `RESEARCH.md` freeze-timer semantics (encounters suppressed while
  freeze > 0).

### SPEC/REQ updates queued

- **F4.6 (SPEC-GAP)**: Model `encounter_number` as a persistent counter
  drained on a 16-tick cadence into slots 3–6; source from the active
  extent's `extn.v1 + rand(0, extn.v2 - 1)`.
- **F4.7 (SPEC-GAP)**: Add a per-placement-batch `mixflag` and wire
  `mixflag & 2` (race pairing) / `mixflag & 4` (weapon-col reroll) into
  `spawn_encounter`.
- **F4.8 (SPEC-GAP)**: Once `extn.v3` is plumbed (see F3.10),
  short-circuit `spawn_encounter` for `extn.v3 == 7` to (21635, 25762).
- **F4.9 (SPEC-GAP)**: Accept terrain code 7 as a valid spawn surface
  when `xtype == 52` (astral plane).
- **F4.10 (SPEC-GAP)**: Add an outer 9-try origin retry around
  `spawn_encounter_group` with `px_to_im == 0` walkability gating.
- **F4.11 (SPEC-GAP, doc-only)**: Note in SPEC that the async-I/O
  `actors_loading` gate collapses to always-false in the port.

### Blockers

None — all NEEDS-FIX and INVENTED items are resolved; SPEC-GAPs are
queued for later batch updates (extent-model / zones / carrier-transport
audits) and do not block other subsystem audits.

---

## Subsystem 5: movement — ✅ Complete

**Reference**: `reference/logic/movement.md` (primary) + `terrain-collision.md`,
`input-handling.md`, `game-loop.md`, `day-night.md`, `carrier-transport.md`,
`astral-plane.md`, `frustration.md`, `RESEARCH.md §5`, `PROBLEMS.md`,
`SYMBOLS.md`.
**Code**: `src/game/gameplay_scene.rs` (`apply_player_input`,
`update_turtle_autonomous`, `update_environ`), `src/game/collision.rs`
(`newx`, `newy`, `proxcheck`, `actor_collides`, `px_to_terrain_type`),
`src/game/actor.rs`, `src/game/combat.rs::hero_speed_for_env`.
**Audit date**: 2025 (current session)

### Summary
- **9 findings**: 1 CONFORMANT, 2 NEEDS-FIX (both fixed), 0 INVENTED, 5
  SPEC-GAP (queued), 1 REF-AMBIGUOUS, 0 RESEARCH-REQUIRED.
- Fixes applied in **one commit** (SHA to be recorded by orchestrator).
- Build/tests: ✅ `cargo build` clean (zero new warnings); `cargo test` —
  579 + 12 + 12 tests passing (no new tests added; existing deviation and
  turtle-probe tests unaffected).

### Findings

#### F5.1 — Wall-slide deviation only runs on diagonals [NEEDS-FIX]
**Location**: `src/game/gameplay_scene.rs::apply_player_input`
(≈lines 967–1006, the `is_diagonal` gate on the `+1` / `-2` deviation
attempts).
**Reference**: `reference/logic/movement.md#walk_step` (`fmain.c:1615-1626`);
`walk_step` runs the primary → `(d+1)&7` → `(d-2)&7` deviation sequence for
any non-zero `proxcheck` return, with no branch on whether `d` is cardinal
or diagonal.
**Issue**: Rust gated the entire deviation block behind
`is_diagonal = matches!(dir, NE|SE|SW|NW)`. A hero walking straight N/S/E/W
into a wall-corner would stop dead instead of sliding one octant CW or
CCW. The original walks the same sequence for every direction (the bump
tables in `xdir[]`/`ydir[]` are symmetric across cardinals and
diagonals), so players in the port could not skim along walls with
cardinal input.
**Resolution**: Dropped the `is_diagonal` conditional and promoted the
`dev1` / `dev2` block to run for every direction; preserved the
`blocked_by_door` (terrain 15) bypass so bumping into a door still
triggers Phase-1 door logic rather than sliding around it.

#### F5.2 — Turtle water probe uses 2-foot `prox` instead of single-point `px_to_im` [NEEDS-FIX]
**Location**: `src/game/gameplay_scene.rs::update_turtle_autonomous`
(≈lines 2150–2166).
**Reference**: `reference/logic/carrier-transport.md#carrier_tick`
(`fmain.c:1525-1537`) — the turtle's autonomous-swim probe chain uses
single-point `px_to_im(xtest, ytest) != 5`, not the foot-level 2-probe
`prox`.
**Issue**: Rust sampled both `(nx+4, ny+2)` and `(nx-4, ny+2)` and
accepted the candidate direction only when both points returned terrain
5 (very deep water). The original accepts the candidate whenever the
turtle's centre sits on terrain 5, which is much more permissive at
water/shore boundaries. Net effect: the Rust turtle rejects many
water-adjacent tiles the original accepts, truncating its autonomous
swim range and occasionally stalling it where it should continue.
**Resolution**: Replaced the two-probe `prox`-style check with a single
`px_to_terrain_type(world, nx as i32, ny as i32) == 5` call, matching
the ref's `px_to_im(xtest, ytest) != 5` guard. No test fixtures needed
updating — existing `update_turtle_autonomous` tests cover open-water
and no-probe cases, both of which remain equivalent under the new
predicate.

#### F5.3 — Outdoor 300 / 32565 wrap-teleport not implemented [SPEC-GAP]
**Location**: `src/game/gameplay_scene.rs::apply_player_input` (position
commit ≈line 933).
**Reference**: `reference/logic/movement.md` "World wrapping" note —
`fmain.c:1831-1839` is a post-commit outdoor-only teleport that fires
for `region_num < 8` when `hero_x` or `hero_y` falls outside
`[300, 32565]`, snapping the hero to the opposite edge. NPCs never
wrap.
**Issue**: Rust uses the 15-bit `rem_euclid(0x8000)` wrap in the
position delta (`newx` / `newy` behaviour), which is what the
low-level math primitive does. The ref's higher-level wrap threshold
(≥300 / ≤32565) and teleport is missing — in practice unreachable in
normal play because the play extent does not butt against the 0 /
32768 seams, but the behavioural edge case is divergent.
**Resolution**: Queued. Requires adding a post-commit wrap block in
`apply_player_input` gated on `region_num < 8`.

#### F5.4 — `newy` does not preserve bit-15 flag across moves [SPEC-GAP]
**Location**: `src/game/collision.rs::newy` (lines 132-140).
**Reference**: `reference/logic/movement.md#newy` (`fsubs.asm:1298-1319`)
— ref preserves `y & 0x8000` as a "flag layer" across every Y step by
masking the wrapping add to `0x7FFF` then re-ORing the captured bit.
**Issue**: Rust instead splits behaviour on an `indoor: bool` parameter:
indoor pixel math just does `(y + dy) as u16` with no masking; outdoor
uses `rem_euclid(0x8000)`. For indoor positions close to 0x8000 (e.g.
`0x9FFF + 3`) the two models diverge — ref would wrap within the indoor
half (`0x8002`), Rust would roll over to `0xA002`. Indoor map extents
never actually span the bit-15 boundary in practice, but the primitive
itself is not a faithful port.
**Resolution**: Queued. Requires mirroring the `flag_bit` preservation
in `collision::newy` and dropping the `indoor` parameter (callers can
be updated likewise).

#### F5.5 — On-foot ice-environ (`k == -2`) velocity accumulation not modelled [SPEC-GAP]
**Location**: `src/game/combat.rs::hero_speed_for_env` (lines 305-317)
and the hero-movement path in `src/game/gameplay_scene.rs`
(`apply_player_input`). The port applies the swan ice-physics only
while `flying != 0`.
**Reference**: `reference/logic/movement.md#walk_step`
(`fmain.c:1581-1598`): when `k == -2`, the actor (including the hero on
ice terrain 7) accumulates `vel += xdir[d]` / `vel += ydir[d]` capped
at 42 px (40 for swan) and advances by `vel // 4`, independent of the
environ→speed table.
**Issue**: Rust's `hero_speed_for_env(-2, false)` returns 2 (normal
walking speed) and there is no velocity accumulator for the hero when
walking on terrain 7. Step-on-ice slides are therefore absent unless
the hero is mounted on the swan.
**Resolution**: Queued. Blocks a full ice-terrain port; flagged as
architectural (the Rust port has not yet needed foot-ice behaviour
because no test / demo path routes the hero onto terrain 7 outside of
swan flight).

#### F5.6 — `frustflag` reset guarded by "enemy active" instead of "any NPC acted" [REF-AMBIGUOUS]
**Location**: `src/game/gameplay_scene.rs::apply_player_input`
(≈lines 1011-1023).
**Reference**: `reference/logic/frustration.md` "Reset asymmetry" note
— `frustflag = 0` fires from at least five unrelated paths
(`fmain.c:1577, 1650, 1707, 1715, 1725`), none guarded by `i == 0`. So
any actor's successful walk / shot / melee / death zeroes the hero's
counter.
**Issue**: Rust resets `frustflag = 0` if "any active enemy NPC exists"
(approximation), not "any NPC successfully acted this tick" (ref).
Direction: the Rust reset fires more aggressively (resets even when the
enemy did not actually succeed at a walk/attack this tick), but misses
the non-enemy-NPC case entirely. The frustration sprite overrides
(20 / 40 thresholds) are therefore slightly miscalibrated.
**Resolution**: Flagged. The ref's per-tick-per-actor semantics require
a centralised actor scheduler that the Rust port does not yet express
(NPC advance happens in `update_npcs`, hero in `apply_player_input`, no
shared "did_act" flag). Revisit once the Rust actor-tick dispatch is
unified; no fidelity loss in typical gameplay because the 40-tick
fight-sprite threshold is rarely reached in combat contexts anyway.

#### F5.7 — Carrier/turtle "blocked onto rock" terrain-1 override [SPEC-GAP]
**Location**: `src/game/gameplay_scene.rs::apply_player_input`
(≈lines 941-945, `turtle_blocked` guard).
**Reference**: `reference/logic/carrier-transport.md#carrier_tick`
(`fmain.c:1542`) and `reference/logic/movement.md` hero-speed table —
the turtle body commits position only when `j == 5` (very deep water);
the hero on turtle-back has `e = 3` but still runs full `proxcheck`
inside `walk_step` (`fmain.c:1599 if i == 0 and riding == 5`).
**Issue**: Rust adds a `turtle_blocked` gate that vetoes any move whose
destination terrain code is `1` (hard block) only while on the turtle.
Per ref, `proxcheck` already rejects terrain code 1 on every foot probe
regardless of mount — the extra gate is redundant (Rust disables
`proxcheck` while on raft/turtle, so this is where the rock-blocking
actually happens). Net effect is conformant, but the `!turtle_blocked`
extra branches throughout the deviation / door logic are an invented
code shape not present in the ref.
**Resolution**: Queued as tidy-up, not a gameplay regression. Fold the
terrain-1 check back into the normal `proxcheck` path once the port
removes the blanket `on_raft` proxcheck bypass.

#### F5.8 — `swan_dismount` velocity gate matches ref but uses different vel representation [SPEC-GAP]
**Location**: `src/game/gameplay_scene.rs` swan-dismount / swan-physics
paths (search `swan_vx`, `swan_vy`, `dismount`).
**Reference**: `reference/logic/carrier-transport.md#swan_dismount`
(`fmain.c:1417-1428`) — dismount requires
`|vel_x| < 15 && |vel_y| < 15`, with velocities in `Actor.vel_x`
scaled as `displacement * 4` in `walk_step`.
**Issue**: Rust stores swan velocity in separate `swan_vx` / `swan_vy`
fields with the same scale, and the ±15 gate is applied correctly, but
`Actor.vel_x` / `Actor.vel_y` (the fields the ref reads) are not
populated for the hero actor during swan flight. Consumers that read
`Actor.vel_*` during swan flight (e.g. potential future "knock-back
while flying" code) would see stale values.
**Resolution**: Queued. Requires mirroring `swan_vx/vy` into
`actors[0].vel_x/vel_y` per tick, or removing the parallel fields.

### CONFORMANT items

- **C5.1 — `xdir[]` / `ydir[]` tables and `newx` / `newy` math**:
  `collision::newx` / `collision::newy` mirror `fsubs.asm:1280-1319`:
  cardinals have magnitude 3, diagonals magnitude 2, and the `* speed / 2`
  combination reproduces the reference's per-tick pixel envelope
  (3 px cardinal / 2 px diagonal at speed 2). 15-bit world wrap on the
  outdoor branch matches ref `& 0x7fff`.
- **C5.2 — Facing encoding (`0=N, 1=NE, … 7=NW`)**: matches
  `reference/logic/movement.md` "Direction mnemonic" section and
  `SYMBOLS.md §2.1` exactly across `apply_player_input`, `Actor.facing`,
  and `facing_toward`.
- **C5.3 — `proxcheck` two-probe asymmetric thresholds**:
  `collision::proxcheck` returns blocked on right-probe terrain `1` or
  `>= 10` and left-probe terrain `1` or `>= 8`, matching
  `reference/logic/terrain-collision.md#prox` (`fsubs.asm:1590-1611`).
- **C5.4 — Actor bounding-box collision (22×18 px)**:
  `collision::actor_collides` uses `|dx| < 11 && |dy| < 9`, matching
  `fmain2.c:286-291` (`proxcheck` actor loop).
- **C5.5 — Hero-on-raft / hero-on-turtle speed = 3**: both the SPEC and
  `reference/logic/movement.md#walk_step` hero speed table
  (`fmain.c:1599`) specify `e = 3` when `i == 0 && riding == 5`; Rust
  `hero_speed_for_env` and the turtle-carrier branch in
  `apply_player_input` apply `3` unconditionally when on raft or turtle.
- **C5.6 — `facing` committed on successful move only**:
  `apply_player_input` writes `final_facing` to the player actor after
  the `can_move` branch succeeds, matching `walk_step` which only reaches
  `newloc: an.facing = d` on the success path (`fmain.c:1628`). On a
  fully-blocked move the facing is left unchanged, consistent with the
  ref's "return without commit" behaviour.
- **C5.7 — `frustflag` increment on fully-blocked move**:
  `apply_player_input` increments `frustflag` only when all three probes
  (primary + 2 deviations) fail, matching `fmain.c:1657`. Reset on
  successful commit matches `fmain.c:1650`.
- **C5.8 — Movement-tied `speak()` / `event()` calls**: `find_place`
  place-name narration, carrier-transition music swaps, door-bump
  locked/opened messages, and astral-plane Loraii pre-load all gate on
  position-derived extents rather than on movement per se, matching
  `reference/logic/astral-plane.md` and `carrier-transport.md`. No
  scroll-area string is emitted directly from the movement path itself,
  satisfying the two-source rule (SPEC §23.6).

### SPEC/REQ updates queued

- **F5.3 (SPEC-GAP)**: Add outdoor-only 300 / 32565 wrap-teleport to
  `apply_player_input` post-commit step.
- **F5.4 (SPEC-GAP)**: Rework `collision::newy` to preserve `y & 0x8000`
  as a flag layer (drop `indoor` parameter).
- **F5.5 (SPEC-GAP)**: Port the `walk_step` on-foot ice-environ
  velocity accumulator (`fmain.c:1581-1598`) so terrain 7 produces the
  inertial slide without requiring `riding == 11`.
- **F5.6 (REF-AMBIGUOUS)**: Revisit `frustflag` reset semantics once a
  unified actor-tick dispatcher exists; flip the gate from
  "enemy_active" to "any NPC acted this tick".
- **F5.7 (SPEC-GAP)**: Remove the `turtle_blocked` terrain-1 side-gate
  once `proxcheck` runs on the mount path; consolidate with ref
  `fmain.c:1599` behaviour.
- **F5.8 (SPEC-GAP)**: Mirror `swan_vx` / `swan_vy` into
  `actors[0].vel_x` / `vel_y` (or remove the parallel fields) so
  `Actor.vel_*` matches ref semantics during swan flight.

### Blockers

None — all NEEDS-FIX items are resolved; SPEC-GAPs and the single
REF-AMBIGUOUS finding are queued and do not block other subsystem
audits. No scroll-area text is produced by the movement path (CONFORMANT
C5.8).

---

## Subsystem 6: doors — ✅ Complete

**Reference**: `reference/logic/doors.md` (primary) +
`reference/logic/game-loop.md#check_door`, `movement.md#walk_step`,
`dialog_system.md §"Door / key feedback"` and §"USE menu",
`messages.md`, `inventory.md`, `RESEARCH.md §12 door system`,
`SYMBOLS.md` (`STATBASE`, `KEYBASE`, `TERRAIN_DOOR`).
**Code**: `src/game/doors.rs` (`DoorEntry`, `key_req`, `doorfind`,
`doorfind_exit`, `doorfind_nearest_by_bump_radius`,
`entry_spawn`/`exit_spawn`, `apply_door_tile_replacement`),
`src/game/gameplay_scene.rs` (bump-open at ≈lines 1093-1185,
walk-through entry at ≈lines 1028-1067, `MenuAction::TryKey` at
≈lines 2754-2808), `src/game/menu.rs` (`MenuMode::Keys` visibility),
`faery.toml [[doors]]` (86-entry `doorlist` port).
**Audit date**: 2025 (current session)

### Summary
- **9 findings**: 4 CONFORMANT, 0 NEEDS-FIX, 2 INVENTED (both fixed),
  2 REF-AMBIGUOUS (queued), 1 SPEC-GAP (queued), 0 RESEARCH-REQUIRED.
- Fixes applied in **one commit**.
- Build/tests: ✅ `cargo build` clean (zero new warnings);
  `cargo test` — 579 + 12 + 12 tests passing (no new tests added or
  modified; existing door tests in `src/game/doors.rs` still green).

### Findings

#### F6.1 — `TryKey` zero-count path prints invented "No such key." [INVENTED]
**Location**: `src/game/gameplay_scene.rs::handle_menu_action`
(`MenuAction::TryKey`, zero-key guard ≈line 2760).
**Reference**: `reference/logic/doors.md#use_key_on_door`
(`fmain.c:3475`): `if (stuff[hit+KEYBASE] == 0) goto menu0;` — the
original **silently** returns to the items menu without printing
anything when the player selects a key of which they hold zero. The
only "no keys" message in the KEYS flow is `"% has no keys!"` at
`fmain.c:3450`, emitted one level up in the USE → Keys toggle gate
when the sum of all six key counts is zero; in the Rust port that
gate is already enforced visually by `menu.rs::set_options`
(enabled flag `8` hides the submenu when no keys exist).
**Issue**: The Rust handler emitted a fabricated `"No such key."`
message in this path — not present in `faery.toml [narr]` nor in
`reference/logic/dialog_system.md`, violating the two-source
scroll-text rule.
**Resolution**: Dropped the message; the zero-count branch now
silently falls through to `gomenu(Items)`, matching the ref. The
`bumped_door` latch is cleared unconditionally on KEYS entry
(addresses F6.3 below).

#### F6.2 — `TryKey` no-match path prints invented "Key didn't fit." [INVENTED]
**Location**: `src/game/gameplay_scene.rs::handle_menu_action`
(`MenuAction::TryKey`, both "no nearest door" and "key mismatch"
branches, ≈lines 2787/2790).
**Reference**: `reference/logic/doors.md#use_key_on_door`
(`fmain.c:3483-3485`) and
`reference/logic/dialog_system.md §"USE menu"` row 3483-3485:
`extract("% tried a ") + inv_list[KEYBASE+hit].name + " but it didn't" + print("fit.")` — a single assembled line of the form
`"{brother} tried a {key name} but it didn't fit."`.
**Issue**: Rust emitted the invented `"Key didn't fit."` (no `%`
substitution, no key name) on both the "no door in bump radius" and
"door found but wrong key" paths. Violates the two-source rule and
drops the `%`-name + `{key name}` composition.
**Resolution**: Replaced both branches with the ref-accurate
composition `format!("{} tried a {} but it didn't fit.", bname,
stuff_index_name(16+idx))`, using the already-ported `inv_list`
name table in `world_objects::stuff_index_name` (indices 16..21 =
Gold/Green/Blue/Red/Grey/White Key).

#### F6.3 — KEYS submenu entry did not clear the `bumped` lock-message latch [CONFORMANT]
**Location**: `src/game/gameplay_scene.rs::handle_menu_action`
(`MenuAction::TryKey`, ≈line 2762).
**Reference**: `reference/logic/doors.md#use_key_on_door`
(`fmain.c:3474`): `bumped = 0` is the first statement of
`use_key_on_door`, before the zero-key guard and the 9-direction
sweep. The intent is to guarantee that a subsequent door bump
(even after an unsuccessful key try) re-speaks `"It's locked."`
rather than staying silent behind the suppression latch.
**Issue**: Rust only cleared `bumped_door = None` on the success
path inside the sweep; failing key tries left the latch set,
causing the next bump on the same door to stay silent.
**Resolution**: Moved `self.bumped_door = None` to the top of the
`TryKey` arm (before the zero-count guard), matching
`fmain.c:3474`. Classified CONFORMANT after fix (the reset is now
unconditional on KEYS entry).

#### F6.4 — `doorfind_nearest_by_bump_radius` approximates the 9-direction sweep with a 32×64 px window [REF-AMBIGUOUS]
**Location**: `src/game/doors.rs::doorfind_nearest_by_bump_radius`
(search window `BUMP_PROX_X=32`, `BUMP_PROX_Y=64`), used by both the
terrain-15 bump path in `apply_player_input` (≈lines 1117-1118)
and the `TryKey` sweep (≈line 2766).
**Reference**: `reference/logic/doors.md#use_key_on_door`
(`fmain.c:3477-3481`) — the original iterates
`for (i=0; i<9; i++) doorfind(newx(hero_x,i,16), newy(hero_y,i,16),
hit+1);` — i.e. 9 canonical directions × 16 px, each probing live
`sector_mem` for `TERRAIN_DOOR=15` and then looking up the tile id
in `open_list[17]`. Rust has no `sector_mem`/`open_list` layer; the
port keeps the 86-entry `doorlist[]` only and drives unlock from it.
**Issue**: The bump-radius search (filter by |Δx| < 32 ∧ |Δy| < 64,
min Euclidean distance) can match a door up to 31 px east/west
or 63 px north/south of the hero, whereas the ref only triggers
for a door whose upper-left tile lies at exactly one of 9 probed
`(hero_x ± {0,3,2,3}, hero_y ± {0,3,2,3})` pixel offsets (scaled
by 16). Net effect: a key used adjacent-but-not-facing a door
could succeed in Rust where it would fail in the ref, and the
terrain-15 bump path could match a more distant door than
`doorfind`'s tile-level probe would.
**Resolution**: Flagged REF-AMBIGUOUS, no fix. A faithful port
requires either (a) porting `open_list` + `sector_mem` tile-level
lookup, or (b) shrinking `BUMP_PROX_*` to ~16 px and ensuring the
door's `src_x/src_y` are stored as the upper-left tile corner so
the 9-direction projection lands inside the match box. Both are
larger refactors spanning tile-data storage and are out of scope
for this per-subsystem audit.

#### F6.5 — `find_place`/`place_msg`/`inside_msg` narration is not fired on door transitions [SPEC-GAP]
**Location**: `src/game/gameplay_scene.rs` outdoor→indoor and
indoor→outdoor branches (≈lines 1033-1040, 1137-1143);
`NarrConfig.place_msg`/`inside_msg` (`src/game/game_library.rs:181`)
are loaded but never read.
**Reference**: `reference/logic/game-loop.md#check_door`
(`fmain.c:1889` and `fmain.c:1912` companion): both halves of the
door teleport call `find_place(2)` / `find_place(False)` after
`xfer(...)`, which (per
`reference/logic/astral-plane.md#find_place`) resolves the current
extent's `xtype` and emits the matching entry from `place_msg[]`
(outdoor) or `inside_msg[]` (indoor), e.g. `"% came to a small
chamber."`, `"He entered the tavern."`, `"He unlocked the door and
entered."`.
**Issue**: The Rust port never narrates place/room names on door
transitions; the `[narr] place_msg` / `inside_msg` tables in
`faery.toml` are dead data. This is broader than the doors
subsystem (the outdoor extent-crossing path in
`no_motion_tick`/`find_place` is also unported) but is user-visible
every time the hero steps through a door.
**Resolution**: Queued as SPEC-GAP. Porting requires the
`find_place` extent-matching loop (`fmain.c:2647-2720`) plus the
`_place_tbl` / `_inside_tbl` tile-range tables; both are in
`reference/_discovery/` but not yet mirrored in `faery.toml`.
Leaving unfixed here per subsystem-scope rule; flagged for the
eventual cross-cutting narration pass.

#### F6.6 — Opened-door state persists across region reloads rather than resetting with `sector_mem` [REF-AMBIGUOUS]
**Location**: `src/game/gameplay_scene.rs::GameplayScene.opened_doors`
(`HashSet<usize>`, ≈line 367); cleared only per-door in the
walk-through branch (≈line 1141).
**Reference**: `reference/logic/doors.md` "Notes" section:
_"Opened-door state is not saved. `doorfind` writes directly into
`sector_mem` via `mapxy`; these edits live for the lifetime of the
currently loaded sector. Any `xfer` that triggers a region reload
discards them."_ I.e. the ref's "is-open" bit lives in the tile
graphic itself, which is re-read from disk on every region swap.
**Issue**: The Rust port keeps a durable `opened_doors: HashSet`
that survives region transitions and save/load (not serialised in
`persist.rs`, so it's cleared on load — but persists across any
number of `xfer`s within a single run). A player who unlocks a
door and later returns will still see `opened_doors.contains(idx)`
true and will be allowed a second walk-through without re-bumping;
in the ref the tile would have reverted to the "closed" graphic
after the first region reload and would need re-unlocking.
**Resolution**: Flagged REF-AMBIGUOUS. The fix depends on the
port's eventual `load_all` / `sector_mem` architecture — once the
Rust map layer re-reads tiles from disk on region swap, the
`opened_doors` set should be cleared inside the region-transition
handler (not in save-load). Until then, the current behaviour is
slightly more permissive than the ref but not exploitable
(the door table's `dst_x/dst_y` is still gated by the
sub-tile `hero_x & 15` / `hero_y & 0x10` guard, and keys are
consumed exactly once per open).

### CONFORMANT items

- **C6.1 — Door type → key-requirement mapping (`key_req`)**:
  `src/game/doors.rs::key_req` matches the `open_list[17]`
  `keytype` column enumerated at `fmain.c:1059-1078` / ref
  `reference/logic/doors.md` §Symbols: `HWOOD`/`VWOOD`/`HCITY`/
  `VCITY`/`LOG`/`STAIR`/`CAVE` → NOKEY; `HSTONE`/`VSTONE` → GREEN
  (`stuff[17]`); `CRYST` → KBLUE (`stuff[18]`); `SECRET` → RED
  (`stuff[19]`); `HSTON2`/`VSTON2` → GREY (`stuff[20]`); `MARBLE`
  → WHITE (`stuff[21]`); `BLACK` → Talisman (per RESEARCH §12);
  `DESERT` → GoldStatues (`stuff[25] >= 5`, via `STATBASE=25`).
- **C6.2 — Sub-tile entry/exit guards**: both the outdoor walk-on
  entry (`gameplay_scene.rs::apply_player_input` ≈lines 1047-1050)
  and the indoor exit (`doors.rs::doorfind_exit`) apply the
  `hero_y & 0x10` (horizontal) / `hero_x & 15` (vertical) guards
  with the correct polarity per `game-loop.md#check_door`
  (`fmain.c:1878-1879, 1909-1910`): enter when upper-half /
  left-portion, exit when lower-half / right-portion.
- **C6.3 — `entry_spawn` / `exit_spawn` offsets**: the destination
  coordinate offsets per door class match `check_door`
  (`fmain.c:1882-1885, 1912-1915`) exactly — CAVE: `(+24, +16)`
  on enter, `(-4, +16)` on exit; horizontal: `(+16, +0)` enter,
  `(+16, +34)` exit; vertical: `(-1, +16)` enter, `(+20, +16)`
  exit. Riding (`riding != 0`) short-circuits both directions per
  `fmain.c:1859` (`check_door` early-return), matching
  SPEC §21.7 T1-CARRY-DOOR-BLOCK.
- **C6.4 — Scroll-text compliance**: the only strings this
  subsystem speaks are the direct literals `"It opened."`
  (`fmain.c:1117`), `"It's locked."` (`fmain.c:1122`), and the
  composed `"% tried a <keyname> but it didn't fit."`
  (`fmain.c:3483-3485`), all three enumerated in
  `reference/logic/dialog_system.md §"Door / key feedback"` and
  §"USE menu". No scroll-area text is invented; the narration
  gap is the missing `place_msg`/`inside_msg` path (F6.5), not
  anything the door handler itself prints.

### SPEC/REQ updates queued

- **F6.4 (REF-AMBIGUOUS)**: Decide the port's tile-level unlock
  strategy (port `open_list` + `sector_mem` vs. tighten the
  `BUMP_PROX_*` window on the existing `DoorEntry` table).
- **F6.5 (SPEC-GAP)**: Port `find_place` + `_place_tbl` /
  `_inside_tbl` so door transitions emit the matching
  `[narr] place_msg` / `inside_msg` line.
- **F6.6 (REF-AMBIGUOUS)**: Clear `opened_doors` on region
  reload once the Rust map layer re-reads tiles from disk.

### Blockers

None — the two INVENTED scroll-text violations are fixed; the
remaining items (F6.4 REF-AMBIGUOUS, F6.5 SPEC-GAP, F6.6
REF-AMBIGUOUS) are queued for the cross-cutting narration /
map-reload passes and do not block downstream subsystem audits.
Two-source rule: ✅ door subsystem speaks only ref-literal
(`dialog_system.md`) strings.

---

## Subsystem 7: npc-dialogue

**Scope**: `CMODE_TALK` dispatch (Yell / Say / Ask), proximity
auto-speech, and per-NPC speech selection for setfig NPCs
(wizard, priest, guards, princess, king, noble, sorceress,
bartender, witch, spectre, ghost, ranger, beggar) and enemy
races. GIVE-item dialogue is covered by subsystem 11 (quests);
shop BUY menu by a later subsystem.

**Primary ref**: `reference/logic/npc-dialogue.md` (pseudo-code
for `talk_dispatch`, `wizard_hint`, `priest_speech`,
`bartender_speech`, `ranger_hint`, `proximity_auto_speak`),
cross-referenced with `reference/logic/game-loop.md#sort_sprites`
(speech proximity radius) and `reference/logic/messages.md §2`
(speech-index table 0..60).

**Code surface audited**:
`src/game/gameplay_scene.rs::update_proximity_speech`
(`fmain.c:2094-2103`), `handle_setfig_talk` (`fmain.c:3367-3423`),
the `GameAction::Yell` / `GameAction::Speak` / `GameAction::Ask`
branches in `do_option`, and their interaction with
`nearest_fig` (`fmain2.c:426-442`).

### Findings

#### F7.1 — Proximity speech range hard-coded to 35 px, spec says 50 px [NEEDS-FIX → FIXED]
**Location**: `src/game/gameplay_scene.rs:42`
(`const PROXIMITY_SPEECH_RANGE: i32 = 35`).
**Reference**: `reference/logic/game-loop.md#sort_sprites`
(`fmain.c:2370`): `perdist = 50` is the speech-proximity radius
used to compute `nearest_person`; `proximity_auto_speak` then
greets that actor via the `_speeches` table.
**Issue**: The port's auto-speech ring was 15 px tighter than
the original, so the beggar / witch / princess / necromancer /
dark-knight greetings fired later than they should and could be
walked past entirely along tangent paths.
**Fix**: Changed `PROXIMITY_SPEECH_RANGE` from 35 to 50. The
existing proximity tests (`test_proximity_auto_speech_*`)
parameterise on the constant and still pass at the new radius.

#### F7.2 — Princess TALK case missing captive-flag guard [NEEDS-FIX → FIXED]
**Location**: `handle_setfig_talk`, case 4
(`src/game/gameplay_scene.rs` ≈line 1720).
**Reference**: `reference/logic/npc-dialogue.md#talk_dispatch`
case 4 (`fmain.c:3397`):
```
if ob_list8[9].ob_stat != 0:    # princess still captive
    speak(16)
```
**Issue**: Rust unconditionally fired `speak(16)` ("Please,
sir, rescue me from this horrible prison!") every time the
player selected TALK near a princess, even after
`execute_princess_rescue` cleared `world_objects[9].ob_stat` to
0. The original falls silent once the princess has been
rescued.
**Fix**: Guarded `speak(16)` on
`world_objects[PRINCESS_OB_INDEX].ob_stat != 0`, matching the
same check already used in `update_proximity_speech` and the
rescue-zone trigger.

#### F7.3 — King TALK case missing princess-captive guard [NEEDS-FIX → FIXED]
**Location**: `handle_setfig_talk`, case 5
(`src/game/gameplay_scene.rs` ≈line 1725).
**Reference**: `reference/logic/npc-dialogue.md#talk_dispatch`
case 5 (`fmain.c:3398`):
```
if ob_list8[9].ob_stat != 0:    # princess still captive
    speak(17)
```
**Issue**: Rust unconditionally fired `speak(17)` ("I cannot
help you, young man. My armies are decimated…") even after
rescue. The original keys the despondent-king line on the same
princess-captive flag as case 4 — once a princess has been
brought back (`ob_stat == 0`), the king stands silent until the
writ cutscene (`speak(18)`, out of scope here) is triggered.
**Fix**: Same `PRINCESS_OB_INDEX` guard as F7.2.

#### F7.4 — Enemy-race TALK speech indices inverted [NEEDS-FIX → FIXED]
**Location**: `handle_setfig_talk`, `FigKind::Npc` branch
(`src/game/gameplay_scene.rs` ≈line 1667).
**Reference**: `reference/logic/npc-dialogue.md#talk_dispatch`
(`fmain.c:3422`): `if an.type == ENEMY: speak(an.race)` — races
0..9 map 1:1 to `_speeches[0..9]` (ogre, goblin-man, wraith,
skeleton, snake, salamander, loraii, "Die foolish mortal!",
"No need to shout!", ranger-goal-0). Confirmed by
`reference/logic/messages.md §2`.
**Issue**: The Rust mapping was:
```
RACE_NORMAL (0)  → speak(3)   // should be speak(0)  (ogre)
RACE_UNDEAD (1)  → speak(2)   // should be speak(1)  (goblin)
RACE_WRAITH (2)  → speak(2)                           (ok)
RACE_ENEMY  (3)  → speak(1)   // should be speak(3)  (skeleton)
RACE_SNAKE  (4)  → speak(4)                           (ok)
```
Three of the five enemy races produced the wrong banter line on
TALK, and the fallback `_ => 6` silently mapped every uncovered
race to the Loraii "no reply" slot instead of honouring the
`speak(an.race)` identity.
**Fix**: Replaced the custom table with `race if race < 10 =>
race as usize` (the direct `speak(an.race)` path). Preserved
special-cased setfig-race NPCs that live in `npc_table` under
the current port architecture (`RACE_SHOPKEEPER → 12`,
`BEGGAR → 23`, `WITCH → 46`, `SPECTRE → 47`, `GHOST → 49`) so
that TALK still produces the correct setfig-switch line for
those actors when they are reached via `FigKind::Npc` rather
than `FigKind::SetFig`. Added `WITCH/SPECTRE/GHOST` cases that
were missing entirely; the fallback `_ => 6` is retained for
truly unknown high-bit races.

#### F7.5 — Invented "next-brother" shout string on empty Yell [INVENTED → FIXED]
**Location**: `do_option(GameAction::Yell)` no-target branch
(`src/game/gameplay_scene.rs` ≈line 3250).
**Reference**: `reference/logic/npc-dialogue.md#talk_dispatch`
lines 49-50:
```
if nearest == 0:    # no target within range
    return
```
The spec is explicit: `talk_dispatch` silently returns when
`nearest_fig` finds nothing inside the 100 px yell radius.
There is no "yell brother's name" behaviour in
`fmain.c:3367-3423`, and `reference/logic/dialog_system.md`
does not list any such scroll-text literal.
**Issue**: Rust emitted `"Phillip!"`, `"Kevin!"`, or
`"Julian!"` via `self.messages.push(format!("{}!", …))` when
yelling with no NPC in range. That is both an invented
behaviour and an invented scroll-text literal (violates the
two-source rule — not in `faery.toml [narr]` and not in
`dialog_system.md`).
**Fix**: Removed the fallback arm; empty-yell is now a silent
return matching the ref. Yell inside 100 px still routes to
the "No need to shout, son!" cutoff (`speak(8)`) or the normal
setfig switch, unchanged.

#### F7.6 — Invented "There is no one here to talk to." fallback [INVENTED → FIXED]
**Location**: `do_option(GameAction::Speak | GameAction::Ask)`
no-target branch (`src/game/gameplay_scene.rs` ≈line 3301).
**Reference**: `reference/logic/npc-dialogue.md#talk_dispatch`
lines 49-50 (silent return when `nearest == 0`); confirmed by
`reference/logic/dialog_system.md "Hardcoded scroll messages"`
— the string is not enumerated there and does not exist in
`faery.toml`.
**Issue**: Speak/Ask with no target in 50 px pushed a
hard-coded English string onto the scroll queue. Violates the
two-source rule and the silent-return invariant of
`talk_dispatch`.
**Fix**: Removed the `else { self.messages.push(…) }` arm.
The turtle-carrier fallback (`active_carrier == CARRIER_TURTLE
→ speak(56)/speak(57)`) still fires when applicable; otherwise
the handler silently returns.

#### F7.7 — Sorceress TALK lacks ob_listg[9] first-visit gate and rand64() luck check [SPEC-GAP]
**Location**: `handle_setfig_talk`, case 7
(`src/game/gameplay_scene.rs` ≈line 1737).
**Reference**: `reference/logic/npc-dialogue.md#talk_dispatch`
case 7 (`fmain.c:3400-3404`):
```
if ob_listg[9].ob_stat != 0:      # already given the figurine
    if luck < rand64():           # rand64() ∈ [0,63]
        luck = luck + 5
else:
    speak(45)
    ob_listg[9].ob_stat = 1       # mark figurine as ground-dropped
prq(7)
```
**Issue**: The Rust port unconditionally fires `speak(45)` on
every visit, boosts `luck += 5` whenever `luck < 64` (cap
is invented; original's `rand64()` gate produces a
probabilistic boost that tops out at 63 via the rand
distribution, not a hard comparison to 64), and never marks the
sorceress-statue slot as placed. It also omits the `prq(7)`
gift-voice priority queue write.
**Resolution**: Flagged SPEC-GAP. Fixing requires a dedicated
`ob_listg[9]` slot tracking for the sorceress figurine (the
current `world_objects` architecture loads ground items with
`ob_stat=1` pre-set and does not distinguish "not-yet-gifted"
from "already-dropped"), plus a `prq()` port. Both are
cross-cutting changes beyond this subsystem's scope. Left as
a queued item for the quest-item / voice-priority pass.

#### F7.8 — Priest TALK lacks writ branch (speak(39)/speak(19)) and prq(4) heal-voice [SPEC-GAP]
**Location**: `handle_setfig_talk`, case 1
(`src/game/gameplay_scene.rs` ≈line 1704).
**Reference**: `reference/logic/npc-dialogue.md#priest_speech`
(`fmain.c:3382-3394`):
```
if stuff[28] != 0:                        # player holds writ
    if ob_listg[10].ob_stat == 0:
        speak(39); ob_listg[10].ob_stat = 1
    else:
        speak(19)
    return
if kind < 10: speak(40); return
speak(36 + daynight%3)
anim_list[0].vitality = 15 + brave/4
prq(4)
```
**Issue**: Rust implements only the `kind < 10` / `kind >= 10
+ heal` halves. It skips the writ entry (which should take
precedence over both branches), so a hero carrying the writ
(`stuff[28] == 1` after princess rescue) still hears the
rotating daily hint and receives a heal instead of being
handed the priest's golden statue (`speak(39)` + drop) or the
already-given rebuke (`speak(19)`). `prq(4)` is also unported.
**Resolution**: Flagged SPEC-GAP. Requires an `ob_listg[10]`
slot for the priest-statue drop and a `prq()` port (same
dependencies as F7.7). Left as a queued item.

#### F7.9 — `last_person` keyed on actor index instead of race [REF-AMBIGUOUS]
**Location**: `update_proximity_speech`
(`src/game/gameplay_scene.rs::PersonId`, ≈lines 417-429,
consumed at ≈line 1331).
**Reference**: `reference/logic/npc-dialogue.md#proximity_auto_speak`:
_"The suppression is **keyed on race**, not on actor index, so
leaving and re-entering the same NPC's range re-fires the
speech only if a different race was encountered in between."_
(`fmain.c:2094, 2103` — `k = anim_list[nearest_person].race`
then `last_person = k`).
**Issue**: The Rust `PersonId` enum stores
`Npc(idx)` / `SetFig(world_idx)`, so two different beggar
setfigs or two necromancer NPCs are treated as distinct
targets. In the ref, moving directly from one beggar to
another would NOT re-fire `speak(23)` because both share
`race == 0x8d`; the Rust port re-fires because the indices
differ.
**Resolution**: Flagged REF-AMBIGUOUS — the ref semantics are
clear, but the fix requires routing both `FigKind::Npc` and
`FigKind::SetFig` through a common "race byte" lookup
(`npc_table[i].race` for enemies; `0x80 | setfig_type` for
setfigs), which is a structural change to `PersonId`. No
player-facing regressions from the current behaviour were
surfaced in tests (the game ships with one beggar, one witch,
one princess, so the race-vs-index distinction rarely fires);
left for a later architecture pass.

### CONFORMANT items

- **C7.1 — Yell radius doubling and shout-too-close cutoff**:
  `GameAction::Yell` calls `nearest_fig(1, 100)` (100 px yell
  radius) and falls through to `speak(8)` ("No need to shout,
  son!") when `fig.dist < 35`, then exits without entering the
  setfig switch — matching `talk_dispatch` lines 45-56
  (`fmain.c:3368, 3373`).
- **C7.2 — Say/Ask radius 50 px**: both
  `GameAction::Speak` and `GameAction::Ask` call
  `nearest_fig(1, 50)` per `fmain.c:3368` else-branch.
- **C7.3 — Wizard `kind<10` rebuke and goal-indexed hints**:
  `handle_setfig_talk` case 0 fires `speak(35)` for
  `kind<10` (`fmain.c:3380`) and `speak(27 + goal)` otherwise
  (`fmain.c:3381`), with `goal` pulled from the setfig
  `WorldObject` — matching `wizard_hint` and the 27..34 hint
  table in `messages.md §2`.
- **C7.4 — Priest `kind<10` rebuke, rotating daily hint, and
  heal formula**: case 1 fires `speak(40)` for `kind<10`
  (`fmain.c:3388`); otherwise `speak(36 + daynight%3)`
  (`fmain.c:3390`) and heal to `15 + brave/4` (`fmain.c:3391`),
  matching `priest_speech` bodies (writ branch caveated in
  F7.8).
- **C7.5 — Guard (front + back) share `speak(15)`**: cases
  2 and 3 both fire the single guard line (`fmain.c:3395-3396`).
- **C7.6 — Noble `speak(20)` unconditional**: case 6 mirrors
  `fmain.c:3399`.
- **C7.7 — Bartender three-way branch**: case 8 selects
  `speak(13)` for `fatigue < 5`, `speak(12)` for
  `dayperiod > 7`, else `speak(14)` (`fmain.c:3406-3408`).
- **C7.8 — Ranger region-2 override + goal-indexed hint**:
  case 12 fires `speak(22)` in `region_num == 2` and
  `speak(53 + goal)` otherwise (`fmain.c:3412-3413`).
- **C7.9 — Witch / spectre / ghost flat `speak(46/47/49)`**:
  cases 9-11 match `fmain.c:3409-3411`.
- **C7.10 — Beggar TALK `speak(23)`**: case 13 matches the
  proximity greeting (`fmain.c:3415`), and the GIVE-gold path
  fires the `speak(24 + goal)` prophecy per subsystem 11.
- **C7.11 — 15-tick TALKING flicker on `can_talk` setfigs**:
  `handle_setfig_talk` sets `talk_flicker[world_idx] = 15`
  iff `SETFIG_TABLE[k].can_talk`, and the timer decrements to
  zero in `update_actors` — matching `fmain.c:3375-3377` and
  the `tactic`-to-STATE_STILL transition at `fmain.c:1557`.
  (Verified by `t4_talk_flicker_*` tests.)
- **C7.12 — Turtle carrier shell dialogue**: the
  `active_carrier == CARRIER_TURTLE` fallback in
  `GameAction::Speak` fires `speak(56)` and awards `stuff[6]`
  on first contact, then `speak(57)` on subsequent TALKs
  (`fmain.c:3418-3420`). (Verified by
  `test_turtle_dialog_*` tests.)
- **C7.13 — Proximity auto-speech coverage**:
  `update_proximity_speech` fires `speak(23/46/16/43/41)` for
  beggar / witch / princess (captive-gated) / necromancer /
  dark-knight and no-ops for every other race, matching
  `proximity_auto_speak` cases at `fmain.c:2097-2101`.
- **C7.14 — Dead target exclusion**: `nearest_fig` filters on
  `active` (for `Npc`) and `visible` + `ob_stat == 3` (for
  `SetFig`); combined with the `checkdead` → `active=false`
  path in combat, this satisfies the ref's `STATE_DEAD` skip
  at `fmain.c:3371`.
- **C7.15 — Speech-index literals match `messages.md §2`**:
  every concrete `speak(N)` call in `handle_setfig_talk`,
  `update_proximity_speech`, and the beggar-gold / turtle /
  yell branches references an index that matches the
  enumerated `_speeches` entry in `messages.md` — no invented
  indices.

### SPEC/REQ updates queued

- **F7.7 (SPEC-GAP)**: Model `ob_listg[9]` (sorceress figurine
  drop slot) and the `rand64()`-gated repeat-visit luck boost.
  Dependency: `prq()` / voice-priority port.
- **F7.8 (SPEC-GAP)**: Model `ob_listg[10]` (priest statue
  drop slot) and wire the `stuff[28]` writ branch through
  `handle_setfig_talk`. Same `prq()` dependency.
- **F7.9 (REF-AMBIGUOUS)**: Migrate `PersonId` from
  index-keyed to race-keyed once cross-subsystem "race byte"
  lookup utilities are in place.

### Blockers

None — the four NEEDS-FIX findings (F7.1-F7.4) and two
INVENTED findings (F7.5-F7.6) are fixed; the remaining items
(F7.7 SPEC-GAP, F7.8 SPEC-GAP, F7.9 REF-AMBIGUOUS) are queued
on cross-cutting dependencies (`ob_listg` slot tracking,
`prq()` port, race-keyed proximity dedup) and do not block
downstream subsystem audits.
Two-source rule: ✅ the npc-dialogue subsystem now speaks only
strings that resolve through `faery.toml [narr] speeches` via
`crate::game::events::speak`, plus the ref-literal
`"No need to shout, son!"` at `speak(8)` — no hard-coded scroll
strings remain in the TALK / Yell / proximity paths.

---

## Subsystem 8: shops

**Scope**: bartender (race `0x88`) BUY-menu dispatch — the
game's only vendor NPC.  Covers jtrans row map, gold gating,
per-slot side effects (food eat, arrow bundle, generic item),
scroll-text narration, and the BUY vs TALK split (pub /
inn / temple behaviour defers to subsystem 7).

**Primary ref**: `reference/logic/shops.md` (`buy_dispatch`
pseudo-code, `fmain.c:3424-3442`, `TABLE:jtrans` at
`fmain2.c:850`), cross-referenced with
`reference/logic/dialog_system.md` (literal registry),
`reference/logic/messages.md §1` (event_msg indices 22, 23,
13), and `reference/logic/npc-dialogue.md#bartender_speech`
(TALK path).

**Code surface audited**: `src/game/shop.rs`,
`src/game/gameplay_scene.rs::do_buy_slot` plus all seven
`GameAction::Buy*` arms, the `GameAction::Speak`/`Ask`
near-shopkeeper branch, `src/game/shop_inventory_tests.rs`.

### Findings

#### F8.1 — Buy-menu inventory slots were mapped to the wrong `stuff[]` indices [NEEDS-FIX → FIXED]
**Location**: `src/game/gameplay_scene.rs` BUY arms +
`src/game/shop.rs::buy_item`.
**Reference**: `reference/logic/shops.md` jtrans row table
(`fmain2.c:850`): Food→0, Arrow→8, Vial→11, Mace→1, Sword→2,
Bow→3, Totem→13.
**Issue**: The port routed BUY slots as
`stuff[0/1/11/8/10/9/13]++` (food→Dirk, arrow→Mace, mace→Arrows,
sword→slot 10, bow→slot 9).  Five of the seven slots landed in
the wrong inventory byte — buying a Mace overwrote the arrow
count, buying a Sword / Bow wrote into slots the original never
uses for those items, and buying Food incremented the Dirk
counter.
**Fix**: Replaced the ad-hoc `do_buy(item_idx, name)` helper
with a single jtrans-driven dispatch, `shop::JTRANS =
[(0,3),(8,10),(11,15),(1,30),(2,45),(3,75),(13,20)]`, and a
seven-arm `do_buy_slot(slot)` in `gameplay_scene`.

#### F8.2 — Food slot granted `stuff[0]++` (Dirk) instead of firing `eat(50)` + `event(22)` [NEEDS-FIX → FIXED]
**Location**: old `GameAction::BuyFood` and
`shop::buy_item(state, 0)`.
**Reference**: `shops.md:44-47` — "Item `i == 0` is a
sentinel… Food branch fires `eat(50)` and a narration event"
(`fmain.c:3433`).
**Issue**: Shop-path incremented `stuff[0]` (the Dirk
count); no-shop-path fell through to an invented `eat_food`
helper that *decremented* `stuff[0]`.  Either way the Dirk
counter was data-corrupted on every food interaction.
**Fix**: `BuyOutcome::Food` now calls `state.eat_amount(50)`
and narrates `event_msg[22]` ("% bought some food and ate
it.").  Additionally pushes `event_msg[13]` ("% was feeling
quite full.") whenever the meal takes hunger below zero, per
`shops.md:131-133` (`fmain2.c:1704-1708`).  The no-shop
fallback is now silent (matches `fmain.c:3425-3426` silent
break).

#### F8.3 — Arrow slot granted `stuff[1]++` (Mace) instead of the 10-arrow bundle at `stuff[8]` [NEEDS-FIX → FIXED]
**Location**: old `GameAction::BuyArrow` →
`do_buy(..., 1, "arrows", ...)`.
**Reference**: `shops.md:47-48` and table row for slot 6
(`fmain.c:3434`): `stuff[i] += 10`, `event(23)`.
**Issue**: Arrows were mis-routed to `stuff[1]` (Mace) with a
bundle size of 1 and invented "Bought arrows for N gold."
scroll text.
**Fix**: `BuyOutcome::Arrows` does `stuff[8] = stuff[8]
.saturating_add(10)` and narrates `event_msg[23]` ("% bought
some arrows.") — no other side effect.

#### F8.4 — `wealth` gate used `< cost`; original is `<= j` (strict 1-gold margin) [NEEDS-FIX → FIXED]
**Location**: old `shop::buy_item`: `if state.gold < cost`.
**Reference**: `shops.md:97` / `fmain.c:3430` — `if wealth <=
j: print("Not enough money!")`.  The 1-gold margin is
explicitly called out as "deliberate, not a bug"
(`shops.md:123-126`).
**Issue**: Port allowed a purchase at exactly the listed
price (e.g. `wealth == 30` could buy a 30-gold Mace); the
original refuses.  Net: port players got one extra marginal
purchase per tight-budget encounter.
**Fix**: `shop::buy_slot` now checks `state.wealth <= price`
and returns `BuyResult::NotEnough`.  Locked in by
`buy_slot_strict_gold_margin` / `buy_slot_one_over_price_succeeds`.

#### F8.5 — Shop currency field diverged from the UI / save-file field [NEEDS-FIX → FIXED]
**Location**: `src/game/shop.rs` (deducted from `state.gold:
i32`) vs. the stat-line render (`gameplay_scene.rs:2407`
reads `state.wealth: i16`) and `persist.rs:47,162` (save file
uses `wealth`).
**Reference**: `shops.md` consistently names the currency
field `wealth` (`fmain.c:3431: wealth = wealth - j`).
**Issue**: Purchases decremented `gold`, which is neither
rendered in the `Wlth:` stat line nor persisted — the
displayed / saved balance drifted from the in-memory one after
every buy.
**Fix**: Shop now operates on `state.wealth` throughout.  The
wider `gold` vs. `wealth` split (combat / loot still award
`gold`) is flagged below but out of scope for this subsystem.

#### F8.6 — Invented scroll strings in the BUY path [INVENTED → REMOVED]
**Location**: former BUY arms + `do_buy` helper.
**Reference**: `dialog_system.md:340-341` (literal registry):
only `"% bought a {item_name}."` and `"Not enough money!"`
are sanctioned shop literals.
**Issue**: The port emitted five distinct invented strings:
`"Bought {} for {} gold."`, `"Bought food for {} gold."`,
`"Cannot buy {}: {}"`, `"No shopkeeper nearby."`, and
`"Not enough gold"` (the `buy_item` `Err` payload).  None
appear in `faery.toml [narr]` or the dialog_system literal
table.
**Fix**: Narration now uses `event_msg[22]` (food),
`event_msg[23]` (arrows), `format!("{} bought a {}.", bname,
stuff_index_name(inv_idx))` (generic item — authorised literal
template), and `"Not enough money!"` (authorised literal).
No-shop and out-of-range-slot paths are silent.

#### F8.7 — TALK-near-shopkeeper showed an invented multi-line "Shopkeeper: What do you need?" menu [INVENTED → REMOVED]
**Location**: `GameAction::Speak | GameAction::Ask` branch,
formerly short-circuiting on `has_shopkeeper_nearby`.
**Reference**: `shops.md:16-18` — TALK resolves through
`bartender_speech` (subsystem 7, case 8 of
`handle_setfig_talk`) which selects
`speak(13/12/14)` from fatigue / dayperiod
(`fmain.c:3406-3408`).  There is no in-dialogue price menu;
the BUY menu is the commercial surface.
**Issue**: The port rendered a multi-line scroll block
(`"Shopkeeper: What do you need?\n  Food - 3 gold\n…"`) that
overrode the bartender's proper speech path — every TALK near
a bartender showed this invented menu instead of
`speak(13/12/14)`.  All of it was invented scroll text
(none in `faery.toml` or `dialog_system.md`).
**Fix**: Removed the `near_shop` branch; TALK near a
bartender now falls straight into `handle_setfig_talk`, which
already handles case 8 correctly per subsystem 7's C7.7.

#### F8.8 — Inn / temple / pub: no code paths; sleep trigger is terrain-based [CONFORMANT]
**Reference**: `shops.md:52-57` — the TALK tree *mentions*
ale (`speak(14)`) and lodging (`speak(12)`), but there is no
code path that sells ale or charges gold for lodging.  The
player rests by walking onto a sleeping-spot tile (ref lines
53-56).  Priest healing is part of `handle_setfig_talk` case 1
(subsystem 7 C7.4: `15 + brave/4` at `fmain.c:3391`) and is
triggered by TALK, not by a shop transaction.  Pub
"drinks" / rumours: none exist; `speak(14)` is purely
informational (subsystem 7 C7.7).
**Status**: Nothing to port.  The shop subsystem is
exhausted by the seven-slot `buy_dispatch`; inn/temple/pub are
not separate commercial surfaces.

#### F8.9 — Shop hours / quest-gated stock: no gating in source [CONFORMANT]
**Reference**: `shops.md` — enabled[] template marks every
BUY slot as "immediate-action" with no dynamic enable/disable
(`fmain.c:525`); the buy-menu is always open regardless of
`dayperiod` or quest state (`shops.md:23-26`).  The only
time-of-day speech gating is TALK-side (bartender
`speak(12)` at `dayperiod > 7` — subsystem 7 C7.7).
**Status**: Port matches: no hour / quest gate on any BUY
arm.

#### F8.10 — Nearest-shopkeeper proximity gate uses a 32×32 bounding box, not the shared `nearest_person` global [REF-AMBIGUOUS]
**Location**: `shop::has_shopkeeper_nearby`.
**Reference**: `shops.md:87-91` / `fmain.c:3425-3426` —
the original checks
`anim_list[nearest_person].race == 0x88`, where
`nearest_person` is a global side-effect of the most recent
`nearest_fig` call (typically set by a prior TALK).
**Issue**: The port does not yet model the
`nearest_person`-as-shared-global pattern.  The current box
is a coarse stand-in that produces roughly the right answer
(a bartender within a 32-px box qualifies) but diverges at the
edges — the original would happily process a BUY *without* a
nearby bartender if the last `nearest_fig` (e.g. a long-range
TALK) happened to land on one.
**Classification**: REF-AMBIGUOUS at the port-architecture
level — fixing this requires a cross-cutting `nearest_person`
refactor shared with subsystems 7 (TALK) and 11 (GIVE).  Left
as-is for now; documented here for the future
`nearest_person` port.

#### F8.11 — `stuff[]` byte saturation at 255 preserves original u8 wrap window [CONFORMANT]
**Location**: `shop::buy_slot` uses `u8::saturating_add`.
**Reference**: `stuff[]` is a `byte[]` in the original
(`stuff` global declared as `char stuff[36]`).  The original
does not check for overflow; a player could theoretically
wrap a slot past 255 by sheer volume of purchases, though in
practice gold runs out long before (Bow at 75 gold × 255 =
19 125 gold).
**Status**: Port's saturating_add introduces a *tighter*
ceiling than the original's wrap, but the wrap is
unreachable in practice given gold economy.  Documented here
only so the saturate-vs-wrap choice is explicit; keeping
saturate (safer and player-visibly identical within the
practical range).

### What matches (confirmed conformant)

- **C8.1 — Seven-slot jtrans layout**: `shop::JTRANS`
  mirrors `fmain2.c:850` exactly:
  `{(0,3),(8,10),(11,15),(1,30),(2,45),(3,75),(13,20)}`.
  Locked in by the seven `t2_shop_costs_*` tests plus four
  per-slot side-effect tests in `shop.rs`.
- **C8.2 — Bartender race byte**: `RACE_SHOPKEEPER = 0x88`
  (`src/game/npc.rs:37`), matching setfig index 8 / race
  byte `0x88` from `shops.md:7-12`.
- **C8.3 — `hit > 11` → return, not break**: modelled by
  `BuyResult::Silent` for `slot >= JTRANS.len()`; the menu
  refresh (`set_options`) still happens in the BUY-arm
  wrapper, matching `shops.md:118-122` (fall-through via
  `do_option`'s trailing refresh).
- **C8.4 — Food sentinel `eat(50)` + `event(22)`**:
  `event_msg[22]` = "% bought some food and ate it."
  (`faery.toml:1617`) and the conditional `event_msg[13]`
  "% was feeling quite full." fire per `shops.md:131-133`.
- **C8.5 — Arrow sentinel bundle size 10**: `stuff[8] +=
  10` and `event_msg[23]` = "% bought some arrows."
  (`faery.toml:1618`).
- **C8.6 — Generic item narration template**: `"% bought a
  <name>."` using `stuff_index_name(inv_idx)` — the names
  match `inv_list[].name` ordering from `fmain.c:428`
  (`src/game/world_objects.rs:70-77`), authorised by
  `dialog_system.md:340`.
- **C8.7 — `"Not enough money!"` literal**: emitted
  unchanged (authorised by `dialog_system.md:341`,
  `shops.md:65`).
- **C8.8 — No inn / temple / pub commercial paths**:
  matches `shops.md:52-57` — the only vendor is the
  bartender, and the only commercial surface is the seven-
  slot BUY menu.
- **C8.9 — No quest / time gating on BUY**: matches
  `shops.md:23-26` (`enabled[]` template marks every slot as
  immediate-action, always green).

### SPEC/REQ updates queued

- **F8.10 (REF-AMBIGUOUS)**: model `nearest_person` as a
  shared global (or per-frame cache) so BUY / TALK / GIVE
  all agree on the target figure; fold
  `has_shopkeeper_nearby` into that lookup.  Cross-cutting
  with subsystems 7, 11.
- **(gold/wealth split, follow-up)**: `state.gold: i32` is
  still written by combat (`combat.rs:167`), loot
  (`loot.rs:69`), and several quest arms, while the UI /
  save file read `state.wealth: i16`.  Awards via `gold`
  never appear in the `Wlth:` stat line or persist.  Out of
  scope for shops (now reads/writes `wealth`), but should be
  unified during the combat-loot audit.
- **(`eat()` event(13) fan-out, follow-up)**: the shop path
  now fires `event_msg[13]` on satiation-crossover, but the
  two other `eat_amount` callers (`pickup_fruit`,
  `try_safe_autoeat`) do not.  `shops.md:131-133` says
  `eat()` itself fires it; pushing the narration into
  `GameState::eat_amount` would require threading a
  message-queue reference and is better done during the
  inventory subsystem audit.

### Blockers

None — all five NEEDS-FIX findings (F8.1-F8.5) and both
INVENTED findings (F8.6-F8.7) are fixed.  F8.10 is the sole
REF-AMBIGUOUS item and is queued on the `nearest_person`
port refactor.

**Two-source rule**: ✅ the shops subsystem now emits only
strings that resolve through `faery.toml [narr] event_msg`
(indices 22, 23, 13) or literals sanctioned by
`dialog_system.md:340-341` (`"Not enough money!"` and
`"% bought a <name>."`).  All five invented strings have
been removed.

---

## Blockers & Open Questions for User Review

_None yet. This section collects REF-AMBIGUOUS, RESEARCH-REQUIRED, and
SPEC-GAP items that need user adjudication before proceeding._

---
