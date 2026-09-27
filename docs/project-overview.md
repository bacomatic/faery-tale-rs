# Project Overview

## Purpose

This is a **research and documentation project** for reverse-engineering *The Faery Tale Adventure* (MicroIllusions, 1987 Amiga). The repository contains the original source code by Talin and a growing body of analysis documents. The deliverable is accurate, citable knowledge — not working software.

## Source Code Is Read-Only

**Do not edit any source file under any circumstances.** The C, assembly, header, procedure, and build files are original 1987 artifacts preserved for reference. This includes all `.c`, `.asm`, `.h`, `.i`, `.p` files, plus `makefile`, `AztecC.Err`, `fta.br`, `notes`, and everything in `src/`, `game/`, and `ToArchive/`.

Agents may only read source files to verify or extract information for documentation.

## Repository Layout

```
src/                Original source code (Aztec C + 68000 assembly) — READ ONLY
reference/
  README.md         Documentation index — the entry point for all reference docs
  ARCHITECTURE.md   System architecture overview, Mermaid diagrams, display geometry
  RESEARCH.md       Comprehensive mechanics reference (hub linking RESEARCH-*.md)
  STORYLINE.md      Quest flows and NPC interactions (hub linking STORYLINE-*.md)
  logic/            Normative pseudo-code specifications for branching functions
  world_db.json     Unified spatial database: objects, doors, extents, terrain by region/sector
  _discovery/       Raw findings from discovery agents — working notes, not final reference docs
tools/              Verification scripts and 68k assembly testing (run via mise; see mise.toml)
assets/tasks/       Self-contained asset-extraction pipeline (own conventions — see its README)
docs/               Agent-facing contract docs (this directory)
```

## Documentation Structure

The reference documentation is tiered. Start at [`reference/README.md`](../reference/README.md) for the full index. In brief:

1. **ARCHITECTURE.md** — high-level system overview: subsystems, data flow, game-loop tick structure, display geometry.
2. **RESEARCH.md** (+ `RESEARCH-*.md`) — ground truth: numbered sections covering every game mechanic with formulas, data tables, and code citations.
3. **STORYLINE.md** (+ `STORYLINE-*.md`) — narrative layer: quest progression, NPC dialogue trees, event sequences as Mermaid diagrams.
4. **PROBLEMS.md** — open questions that cannot be answered from source code alone, awaiting expert input.
5. **world_db.json** / **quest_db.json** — machine-readable data lookup (see Spatial Database below).
6. **reference/logic/\*\*.md** — normative pseudo-code specifications for every non-trivial branching function. The source of truth for porters.

## Spatial Database

`reference/world_db.json` is a machine-readable spatial index generated from the game's binary map data and hardcoded source tables. **Discovery agents must consult this file when investigating any location-dependent mechanic** — doors, encounters, quest triggers, object placement, terrain features, or reachability questions.

The database contains:
- **objects** (129): Every world object with pixel coords, region, sector, grid position, type, and place name
- **doors** (86): All door/stair/gate transitions with outside and inside endpoints resolved to regions and sectors
- **extents** (23): Encounter trigger zones (rectangles) with type codes and resolution to region/sector
- **zones** (3): Hardcoded special zones (desert gate, fiery death box, astral plane)
- **sector_terrain** (996): Per-sector terrain composition summaries across all 10 regions
- **region_grids** (10): Full 64×32 tile grids per region, each tile classified by terrain type

**How to use it**: Load the JSON and filter by region, sector, or coordinate range. To find what's near a location, filter objects/doors/extents whose `region` matches, then check `grid_col`/`grid_row` proximity. The `place_name` field gives the in-game location name from `narr.asm`.

**Regeneration**: Run `python tools/decode_map_data.py --export-world-db` to regenerate from `game/image` and the hardcoded tables.

## Technical Context

- **Language**: Aztec C (1987) + Motorola 68000 assembly
- **Platform**: Commodore Amiga (custom chipset: Agnus, Denise, Paula)
- **Display**: Non-interlaced 320×200 frame, mixed-resolution split (lo-res playfield + hi-res status bar)
- **Graphics**: 5-bitplane (32-color) double-buffered playfield
- **Direction encoding**: 0=NW, 1=N, 2=NE, 3=E, 4=SE, 5=S, 6=SW, 7=W

## Key Source Files (Quick Reference)

### Game Executable (`fmain`)

Built from 8 object files linked by the makefile:

| File | Domain |
|------|--------|
| `fmain.c` | Core game loop, actors, combat, physics, rendering, UI |
| `fmain2.c` | Quests, NPC dialogue, shops, brother succession, save/load, visual effects |
| `fsubs.asm` | Movement vectors, joystick handler, direction tables, low-level subroutines |
| `gdriver.asm` | Audio driver: VBlank music interrupt server, score/sample playback, tempo |
| `narr.asm` | All in-game message text, indexed by speech number |
| `iffsubs.c` | IFF/ILBM image parser and ByteRun1 decompressor |
| `hdrive.c` | Dual-path disk I/O (floppy raw sectors / hard drive file) |
| `MakeBitMap.asm` | Bitplane allocation/deallocation |

### Shared Headers

| File | Domain |
|------|--------|
| `ftale.h` | Master header: struct definitions, motion states, goal modes, display constants |
| `ftale.i` | Assembly struct definitions mirroring `ftale.h` |
| `fincludes.c` | Precompiled header aggregator for Aztec C (`-hi amiga39.pre`) |

### Offline Tools (not part of game executable)

| File | Domain |
|------|--------|
| `terrain.c` | Extracts terrain data from IFF landscape images into `terra` binary |
| `mtrack.c` | Writes game assets to disk 1 at specific block offsets |
| `rtrack.c` | Writes game assets to disk 2 (subset of disk 1 data) |
| `copyimage.c` | Raw disk sector copy utility (device → file) |
| `text.c` | Standalone font test program by Talin (not game-related) |
| `form.c` | Standalone form/screen editor "edform" by Talin (not game-related) |

### Not Linked

| File | Domain |
|------|--------|
| `fsupp.asm` | Assembly versions of `colorplay`, `stillscreen`, `skipint` — superseded by C versions in `fmain2.c` |
