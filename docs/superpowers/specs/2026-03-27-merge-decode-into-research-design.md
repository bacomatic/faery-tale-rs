# Merge DECODE.md Into RESEARCH.md

**Date:** 2026-03-27
**Status:** Draft

## Problem

The project has two overlapping reverse-engineering reference documents:
`RESEARCH.md` (1,318 lines, 64 KB — game systems) and `DECODE.md`
(1,100 lines, 48 KB — binary formats + architecture). They share coverage
of terrain collision, sprite layout, input handling, and sound effects.
Having two files creates confusion about which to consult and leads to
duplicated or inconsistent information.

## Approach

Merge all DECODE.md content into RESEARCH.md using intelligent deduplication:
keep the more complete version of each overlapping topic, add new sections for
content unique to DECODE.md. Preserve RESEARCH.md's existing flat-section
organization. Delete DECODE.md and update all references.

## Section-by-Section Merge Map

Each DECODE.md section gets one of three treatments: **merge** (fold into an
existing RESEARCH.md section, keeping the more detailed version), **new**
(add as a new ## section), or **drop** (content already fully covered).

| DECODE.md Section | Lines | Action | RESEARCH.md Target |
|---|---|---|---|
| Input & Command Reference | 12–128 | **Merge** | "Key Bindings: Original game key map" — augment with any manual details not already present |
| `game/songs` — Music Score Data | 129–280 | **New section** | Insert after existing "setmood()" section (~line 605) |
| `game/v6` — Voice/Waveform Data | 281–313 | **New section** | Insert after songs section |
| `game/samples` — Sound Effects | 314–341 | **Merge** | "Sound Effects (`game/samples`)" — DECODE has trigger/speed details to add |
| `game/image` ADF — Sprite Shape Data | 342–424 | **Merge** | "Sprite / Shape File Layout (ADF)" — deduplicate, keep more complete of the two |
| Terrain Collision | 425–595 | **Replace** | "Terrain Collision System" — DECODE's 172-line version is far more detailed than RESEARCH's 42 lines; replace entirely |
| Compass Rose | 596–696 | **New section** | Insert near "Screen Layout" section |
| Input Decoding | 697–809 | **New section** | Insert after "Key Bindings: Design and compatibility notes" |
| Menu System | 810–1074 | **New section** | Insert after Input Decoding (largest new addition, 265 lines) |
| Known Original Exploits | 1075–1100 | **New section** | Append at end of document (before region diagrams) |

## Deduplication Rules

For overlapping topics, compare both versions side-by-side:

1. If DECODE.md is strictly more detailed → use DECODE's version, discard
   RESEARCH's.
2. If RESEARCH.md has unique context (e.g., game-logic rationale) that DECODE
   lacks → merge the unique parts from RESEARCH into DECODE's structure.
3. If they cover different facets of the same topic → concatenate, with a
   logical ordering (overview → data format → algorithm → implementation notes).

## Preamble Update

RESEARCH.md's current preamble references PLAN.md (which is being deleted per
the previous spec). Update the preamble to:

- Remove the PLAN.md reference.
- Describe RESEARCH.md as the single canonical reverse-engineering reference
  (covering both game systems and binary file formats).
- Keep the `research_index.toml` maintenance workflow note.

## DECODE.md Preamble Content

DECODE.md's preamble also references PLAN.md. This content is discarded along
with the file — no migration needed.

## Files to Delete

| File | Reason |
|------|--------|
| `DECODE.md` | All content merged into RESEARCH.md |

## Files to Update

### `RESEARCH.md`

- Update preamble (remove PLAN.md reference, describe as unified reference).
- Merge/add sections per the merge map above.
- Preserve all existing content that doesn't overlap with DECODE.md.

### `research_index.toml`

Two entries currently point to `DECODE.md`:

- `decode.songs` (`doc = "DECODE.md"`, anchor = songs heading) → change `doc`
  to `"RESEARCH.md"`, update `anchor` to match new heading slug.
- `decode.v6` (`doc = "DECODE.md"`, anchor = v6 heading) → same treatment.

All other entries already point to RESEARCH.md — no changes needed.

### `AGENTS.md`

"Canonical sources by topic" section:
- Current: separate bullets for DECODE.md and implicit RESEARCH.md coverage.
- New: single bullet for RESEARCH.md covering reverse-engineering, file formats,
  and game mechanics.

### `CLAUDE.md`

"Canonical sources by topic" section:
- Current: `**always check DECODE.md before guessing at binary format details**`
- New: point to RESEARCH.md with same guidance about checking before guessing.

### `README.md`

"Canonical Sources" section:
- Current: `Reverse-engineering and asset format notes: DECODE.md`
- New: point to RESEARCH.md.

### `scripts/check_docs_links.sh`

Remove `DECODE.md` from the `required_files` array.

## Validation

After all changes:

1. `bash scripts/check_docs_links.sh` passes.
2. No remaining references to `DECODE.md` in tracked files
   (verify with `git grep -l 'DECODE\.md'`).
3. All `research_index.toml` entries point to `RESEARCH.md`.
4. All anchors in `research_index.toml` resolve to actual headings in
   RESEARCH.md.
5. The merged RESEARCH.md has no duplicate sections covering the same topic.
