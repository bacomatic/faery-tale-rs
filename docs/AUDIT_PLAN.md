# Non-Conformance Deep-Dive Audit — Plan

## Problem

The prior message-only audit flagged 85 dialog-rule violations (71 in
`gameplay_scene.rs`, 14 in `magic.rs`) where scroll-area text is invented in
Rust rather than sourced from `faery.toml [narr]` or `dialog_system.md`. These
flags are **symptoms**, not the underlying disease. Each flagged site exists
because the surrounding code invented behavior — and wherever behavior was
invented, other spec/requirements violations likely exist too (wrong damage
formulas, wrong AI decisions, wrong timing, etc.).

We now have 22 authoritative `reference/logic/*.md` docs. This is the first
time the project has had prescriptive, source-cited behavior specs for every
major subsystem. The flagged sites become **entry points** for a broader,
subsystem-by-subsystem deep-dive against these references.

## Approach

**Subsystem-by-subsystem audit, prioritized by fidelity risk.** For each
subsystem, perform a structured cross-check between:

1. `reference/logic/<subsystem>.md` (prescriptive behavior)
2. `reference/RESEARCH.md` (verified mechanics, especially formulas)
3. `docs/SPECIFICATION.md` (project spec — may need updates)
4. `docs/REQUIREMENTS.md` (project reqs — may need updates)
5. `faery.toml` data (if the subsystem is data-driven)
6. `src/game/<subsystem>.rs` (+ its call paths in `gameplay_scene.rs`)
7. Prior message-audit flag sites that touch this subsystem

Findings are classified and tracked, then fixed in-subsystem before moving on.

## Methodology

### Audit procedure (applied per subsystem)

1. **Read the authoritative source**: `reference/logic/<sub>.md` end-to-end.
2. **Pull cross-refs**: follow every `[cross-ref]` in that doc.
3. **Catalog the code surface**: `grep`/code-intel to find every Rust function
   that implements or calls into this subsystem.
4. **Line-by-line behavioral cross-check**: compare each symbol/function in the
   ref doc against its Rust counterpart. Record divergences.
5. **Data cross-check**: if the subsystem reads `faery.toml`, verify keys,
   ranges, and semantics against the ref.
6. **Message cross-check**: verify every scroll-area message path in this
   subsystem against `faery.toml [narr]` + `dialog_system.md`. (Absorbs the
   relevant portion of the prior 85-site flag list.)
7. **Classify findings** (see below).
8. **Apply fixes** in order of severity. Commit per-subsystem.

### Findings classification

| Code | Meaning | Action |
|---|---|---|
| **CONFORMANT** | Code matches ref + spec + req | No action |
| **NEEDS-FIX** | Code is wrong per the reference | Fix the code |
| **SPEC-GAP** | Code may be right but SPEC/REQ is silent or wrong | Propose SPEC/REQ update for user review |
| **REF-AMBIGUOUS** | Reference itself is unclear or contradicts itself | Flag for user (do not invent) |
| **RESEARCH-REQUIRED** | Behavior observable but not yet documented anywhere | Flag for user |
| **INVENTED** | Code behavior has no ref/spec support (includes the message flags) | Remove or replace with ref-compliant version |

### Working artifacts

- **Per-subsystem audit notes** tracked in SQL (`audit_findings` table).
  No markdown audit docs in the repo (per AGENTS.md).
- **plan.md** (this file) tracks overall progress and classification.
- **Commit scope**: one commit per subsystem for fixes; a separate commit for
  any SPEC/REQ updates that came out of the audit.

## Subsystem Priority Order

Ordered by fidelity risk × user-visibility × prior-flag density.

### Tier 1 — High risk, prior-flag dense

| # | Subsystem | Rust files | Ref doc | Prior flags |
|---|---|---|---|---|
| 1 | **combat** | `combat.rs`, `gameplay_scene.rs` combat paths | `combat.md` + RESEARCH §7 | ~30 |
| 2 | **magic** | `magic.rs` | `magic.md` | 14 |
| 3 | **ai-system** | `gameplay_scene.rs` NPC AI paths | `ai-system.md` | ~10 |
| 4 | **encounters** | `encounter.rs` | `encounters.md` | ~8 |

### Tier 2 — Medium risk

| # | Subsystem | Rust files | Ref doc |
|---|---|---|---|
| 5 | **movement** | `gameplay_scene.rs` movement, `actor.rs` | `movement.md` |
| 6 | **doors** | `doors.rs` | `doors.md` |
| 7 | **npc-dialogue** | `gameplay_scene.rs` dialog, `events.rs` | `npc-dialogue.md` + `dialog_system.md` |
| 8 | **shops** | `gameplay_scene.rs` shop paths | `shops.md` |
| 9 | **inventory** | `gameplay_scene.rs` inventory, `game_state.rs` | `inventory.md` |
| 10 | **day-night** | `game_clock.rs`, `gameplay_scene.rs` time paths | `day-night.md` |
| 11 | **quests** | `gameplay_scene.rs` quest triggers, `game_state.rs` | `quests.md` |

### Tier 3 — Lower risk / contained

| # | Subsystem | Rust files | Ref doc |
|---|---|---|---|
| 12 | **astral-plane** | `gameplay_scene.rs` astral paths | `astral-plane.md` |
| 13 | **brother-succession** | `game_state.rs` | `brother-succession.md` |
| 14 | **carrier-transport** | boat/unicorn paths | `carrier-transport.md` |
| 15 | **terrain-collision** | `collision.rs` | `terrain-collision.md` |
| 16 | **visual-effects** | `gfx_effects.rs` | `visual-effects.md` |
| 17 | **save-load** | save/load paths | `save-load.md` |
| 18 | **menu-system** | `menu.rs` | `menu-system.md` |
| 19 | **input-handling** | input paths | `input-handling.md` |
| 20 | **frustration** | combat/AI frustration | `frustration.md` |
| 21 | **game-loop** | `gameplay_scene.rs` outer loop | `game-loop.md` |

## Execution Plan

### Phase 0 — Setup (this session)

- [x] Write plan
- [ ] Create SQL tracking tables (`audit_subsystems`, `audit_findings`)
- [ ] Seed subsystems table with all 21 items
- [ ] Begin Phase 1

### Phase 1 — Tier 1 (combat, magic, ai-system, encounters)

For each Tier 1 subsystem:
- Full ref-vs-code audit per the methodology above.
- Findings logged in SQL with severity.
- NEEDS-FIX and INVENTED findings fixed immediately.
- SPEC-GAP findings queued for a batch SPEC/REQ update commit at end of phase.
- Single commit per subsystem (or two: code fixes + SPEC/REQ updates).
- Build + existing test suite passes after each commit.

Checkpoint: user-visible fidelity audit summary after Tier 1 complete.

### Phase 2 — Tier 2 (movement, doors, dialog, shops, inventory, day-night, quests)

Same methodology. Expect heavier dialog/scroll work here (absorbs most of the
remaining prior-audit flag sites).

### Phase 3 — Tier 3 (remaining 10 subsystems)

Same methodology. Most are expected to be mostly conformant (contained scope,
less invented-behavior risk).

### Phase 4 — Cross-cutting sweep

After all subsystems: one final pass to verify no scroll-area strings remain
that violate the two-source rule (SPEC §23.6 / R-INTRO-012). This is the
original 85-site audit's descendant — by Phase 4 most should already be fixed
incidentally during subsystem audits.

## Per-Subsystem Checklist (copy for each)

```
[ ] Read reference/logic/<sub>.md end-to-end
[ ] Follow all cross-refs; read RESEARCH section if cited
[ ] Grep/code-intel: enumerate Rust functions implementing this subsystem
[ ] Behavioral cross-check: each ref symbol → Rust counterpart
[ ] Data cross-check: faery.toml keys/ranges (if applicable)
[ ] Message cross-check: scroll-text paths vs faery.toml/dialog_system.md
[ ] File findings in SQL with severity + fix-approach note
[ ] Apply NEEDS-FIX / INVENTED fixes
[ ] Queue SPEC-GAP items for batch SPEC/REQ commit
[ ] cargo build + cargo test clean
[ ] Commit code fixes (scoped to this subsystem)
[ ] Update plan.md: mark subsystem done
```

## Risks & Notes

- **`magic.md` now exists** (275 lines) — removed prior SPEC-GAP risk. All
  Tier 1 subsystems now have authoritative reference docs.
- **Some ref docs may have errors**. When code disagrees with ref AND ref
  disagrees with RESEARCH, flag as REF-AMBIGUOUS and do not silently fix.
- **Scope creep risk**: discovering a non-conformant behavior may implicate
  multiple subsystems (e.g., combat damage affects AI retreat logic). Stay in
  current subsystem; cross-reference the finding for later subsystems.
- **Test coverage may be thin for some behavior paths**. Prefer adding
  fidelity tests against RESEARCH formulas when fixing core math (combat
  damage, spell effects, AI rolls). Per TDD policy in AGENTS.md.
- **User is intermittently available**. Apply conservative fidelity-first
  decisions autonomously. Flag anything truly ambiguous for review rather
  than inventing.

## Next Action

Set up SQL tracking (`audit_subsystems` + `audit_findings`), seed with the 21
subsystems, and begin **Subsystem 1: combat**.
