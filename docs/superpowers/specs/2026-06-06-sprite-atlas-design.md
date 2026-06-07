# Design: Sprite Atlas Documentation

**Date:** 2026-06-06
**Branch:** research
**Status:** approved

## Problem

Every sprite sheet used by Faery Tale Adventure has frame coordinates, dimensions, and content that are currently undocumented as a whole. Frame indices (`inum` values) are referenced throughout `logic/sprite-rendering.md`, `logic/game-loop.md`, and `_discovery/sprite-compositing.md` but there is no single source that maps every frame index to a name/purpose for any sheet. This makes the sprite-rendering spec incomplete for porting purposes.

## Goal

Produce a sprite atlas — a per-frame registry of all sprite sheets — using source-code inference. Unknown frames are flagged for future visual inspection rather than left as silent gaps.

## Approach: Option B — Atlas files + `data/` tier

Create a `reference/data/` subdirectory that establishes a new documentation tier for dense tabular/machine-readable game data. Add sprite atlas files under `reference/data/sprites/`. Update `reference/README.md` to register the new tier.

## Directory Structure

```
reference/
  data/
    README.md                ← defines the data/ tier convention
    sprites/
      objects.md             ← OBJECTS sheet (116 frames)
      actors.md              ← PHIL + ENEMY sheets
      carriers.md            ← RAFT + CARRIER + DRAGON + SETFIG sheets
```

## `reference/data/README.md`

Explains:
- What belongs here: dense tabular or machine-readable data extracted and verified from source, distinct from behavioral pseudo-code (`logic/`) and raw trace notes (`_discovery/`).
- Trust level: normative, same as `logic/` — source-verified, not inferred narrative.
- Current contents: sprite atlases.
- Future candidates: terrain-type table, encounter probability tables, item/weapon stat table.

## Atlas File Format

Each file contains:

### 1. Header block

```
Sheet:       <name>
seq_list:    <enum slot>
Dimensions:  <W>×<H>px per frame (fixed) | variable — see Notes
Frame count: <N>
Source:      <file>:<line-range>
```

Plus a one-line note on any sheet-level rendering rule (e.g. OBJECTS half-height packing).

### 2. Frame table

| `inum` | Name / description | Size (px) | Source ref |
|--------|--------------------|-----------|------------|
| `0` | example | 16×32 | `fmain.c:154` |
| `0x3a` | *(unknown)* | 16×16 | — |

Column rules:
- **`inum`**: decimal for actor/carrier frames, hex for OBJECTS (matches code style).
- **Name**: from code symbol, comment, or contextual inference. `*(unknown)*` when no code reference exists. Unknown frames are also collected in a `## Unknown Frames` section at the end of the file for easy visual-inspection triage.
- **Size**: derived mechanically from sheet metadata (`cfiles[]`) + OBJECTS half-height rule (`compute_sprite_size` logic). Not re-verified visually.
- **Source ref**: earliest/most canonical code location naming or using this frame. `—` when inferred with no direct reference.

### 3. Notes section

Sheet-level rendering quirks that affect how frames are used:
- OBJECTS: bit-7 dual-row packing, half-height set list, `inv_list` indirection.
- Actors: `statelist[].figure` indirection, `diroffs[]` base-frame selection.
- Carriers: per-file variant loading.

## Per-Atlas Scope

### `objects.md` — OBJECTS sheet

- `seq_list[OBJECTS]` (enum slot 1), 116 frames, 16×16px or 16×8px (half-height set).
- Frame name sources:
  - `inv_list[0..30].image_number` → 31 inventory item frames named by `inv_list[].name`
  - `statelist[].wpn_no` + weapon-class `k` offsets → weapon overlay frames
  - Named special frames from code: `0x1b` (arrow shaft), `0x11–0x17` (arrow flight), `97–98` (drowning bubbles), `0x58` (fiery-death overlay), `30`/`0x51`/`0x53` (bow direction frames), `103–110` (wand frames by facing), `8–12` (small ground items), `25–26` (bones/scrap)
  - Remainder: `*(unknown)*`, collected in Unknown Frames section

### `actors.md` — PHIL + ENEMY sheets

- PHIL: `seq_list[PHIL]` (slot 0), 67 frames, 16×32px.
- ENEMY: `seq_list[ENEMY]` (slot 2), frame count varies by `actor_file`, 16×32px.
- Coverage: `statelist[87].figure` maps every animation `inum` to a body-frame index. `diroffs[16]` and `fallstates[24]` cover remaining reachable frames. Near-complete for PHIL; ENEMY coverage documented per `actor_file` where known.
- One combined file because both sheets share the same animation frame indexing scheme via `statelist`.

### `carriers.md` — RAFT + CARRIER + DRAGON + SETFIG sheets

- RAFT: `seq_list[RAFT]` (slot 3), 2 frames, 32×32px.
- CARRIER: `seq_list[CARRIER]` (slot 5), 8 frames, 64×64px.
- DRAGON: `seq_list[DRAGON]` (slot 6), 5 frames, 48×40px.
- SETFIG: `seq_list[SETFIG]` (slot 4), per-region load, 16×32px.
- Per-frame names from carrier transport and combat code references.

## `reference/README.md` change

Add a **"Data Tables"** entry to the Canonical Documentation table:

| `data/README.md` | Data tier hub — sprite atlases and other source-extracted tabular data. |

## Out of Scope

- Visual verification of unknown frames (flagged for follow-up, not done here).
- Migration of existing tables (e.g. `inv_list`, `statelist`, encounter tables) from `_discovery/` or `logic/` into `data/` — separate future pass.
- Pixel-coordinate extraction tooling — not needed for source-inference approach.
