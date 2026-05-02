# Porting Guide

This directory contains implementation-facing guidance for agents and developers building modern ports from the original 1987 source.

Scope:
- Convert reverse-engineering findings into actionable implementation checklists.
- Capture parity-critical behaviors that are easy to miss in direct code translation.
- Provide verification matrices for behavioral equivalence testing.

How to use these docs:
1. Start with a subsystem checklist file in this directory.
2. Implement the subsystem while preserving all mandatory behaviors.
3. Run the parity test matrix in the checklist.
4. If a behavior cannot be implemented exactly, document the deviation and rationale.

Rules for porting agents:
- Treat original source behavior as ground truth unless documented otherwise in `reference/PROBLEMS.md`.
- Do not infer behavior from gameplay memory; cite source lines.
- Preserve ordering-sensitive logic exactly (table order, branch order, scan order).
- Preserve caller-mode contracts (for example, function flags that alter behavior).
- Preserve known quirks unless the target project intentionally normalizes them.

Directory contents:
- `extent-system-checklist.md` — `find_place`, `extent_list`, `xtype`, `extn`-dependent behavior, flag mode contract.
- `doors-xfer-checklist.md` — `doorlist[86]`, `open_list[17]`, `doorfind`, `xfer`, key system, region transitions.
- `ai-goals-tactics-checklist.md` — Goal modes, tactic dispatch, `do_tactic`, `set_course`, DKnight, frustration cycle.
- `terrain-collision-checklist.md` — `_px_to_im`, `proxcheck`, terrain types 0–15, `environ` accumulation, item-gated terrain.
- `save-load-checklist.md` — Save file block order, 80-byte variable layout, extent persistence, post-load restoration.
- `inventory-checklist.md` — `stuff[]` array layout, per-item semantics, weapon system, magic guards, win condition trigger.
- `input-handling-checklist.md` — Interrupt handler, circular key buffer, event filtering, mouse-to-menu encoding, heartbeat.
- `day-night-checklist.md` — `daynight` counter, `lightlevel` formula, `dayperiod` events, `day_fade`, hunger/fatigue.

Cross-references:
- `reference/RESEARCH.md` — canonical mechanics reference.
- `reference/logic/*.md` — normative pseudo-code specifications.
- `reference/PROBLEMS.md` — unresolved questions and intentional uncertainty.
