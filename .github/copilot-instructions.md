# Faery Tale Adventure — Reverse-Engineering Research Project

## Project Purpose

This is a **research and documentation project** for reverse-engineering *The Faery Tale Adventure* (MicroIllusions, 1987 Amiga). The repository contains the original source code by Talin and a growing body of analysis documents.

## Critical Rule: Source Code Is Read-Only

**Do not edit any source file under any circumstances.** The C, assembly, header, procedure, and build files are original 1987 artifacts preserved for reference. This includes all `.c`, `.asm`, `.h`, `.i`, `.p` files in the repo root, plus `makefile`, `AztecC.Err`, `fta.br`, `notes`, and everything in `game/` and `ToArchive/`.

Agents may only read source files to verify or extract information for documentation.

## Repository Layout

```
(root)              Original source code (Aztec C + 68000 assembly) — READ ONLY
reference/
  ARCHITECTURE.md   System architecture overview, Mermaid diagrams, display geometry
  RESEARCH.md       Comprehensive mechanics reference (20 numbered sections)
  STORYLINE.md      Quest flows and NPC interactions as Mermaid state diagrams
  world_db.json     Unified spatial database: objects, doors, extents, terrain by region/sector
  _discovery/       Raw findings from discovery agents — working notes, not final reference docs
  superpowers/      Plans and specs for the reverse-engineering project itself
game/               Runtime binary assets (images, fonts, music, map sectors) — READ ONLY
ToArchive/          Original distribution package — READ ONLY
```

## Documentation Structure

Documentation is three-tiered:

1. **ARCHITECTURE.md** — High-level system overview: 19 subsystems, data flow, game loop tick structure, display geometry
2. **RESEARCH.md** — Ground truth: 20 numbered sections covering every game mechanic with formulas, data tables, and code citations
3. **STORYLINE.md** — Narrative layer: quest progression, NPC dialogue trees, event sequences as Mermaid diagrams
4. **PROBLEMS.md** — Open questions that cannot be answered from source code alone, awaiting expert input
5. **world_db.json** — Pre-computed spatial database cross-referencing all location-dependent game data (see [Spatial Database](#spatial-database) below)
6. **reference/logic/\*\*.md** — Normative pseudo-code specifications for every non-trivial branching function. The source of truth for porters.

## Spatial Database

`reference/world_db.json` is a machine-readable spatial index generated from the game's binary map data and hardcoded source tables. **Discovery agents must consult this file when investigating any location-dependent mechanic** — doors, encounters, quest triggers, object placement, terrain features, or reachability questions.

The database contains:
- **objects** (129): Every world object with pixel coords, region, sector, grid position, type, and place name
- **doors** (86): All door/stair/gate transitions with outside and inside endpoints resolved to regions and sectors
- **extents** (23): Encounter trigger zones (rectangles) with type codes and resolution to region/sector
- **zones** (3): Hardcoded special zones (desert gate, fiery death box, astral plane)
- **sector_terrain** (996): Per-sector terrain composition summaries across all 10 regions
- **region_grids** (10): Full 64×32 tile grids per region, each tile classified by terrain type

**How to use it**: Load the JSON and filter by region, sector, or coordinate range. For example, to find what's near a specific location, filter objects/doors/extents whose `region` matches, then check `grid_col`/`grid_row` proximity. The `place_name` field on objects and door endpoints gives the in-game location name from `narr.asm`.

**Regeneration**: Run `python tools/decode_map_data.py --export-world-db` to regenerate from `game/image` and the hardcoded tables.

## Agent Architecture: Flat Iterative Model

Subagents **cannot dispatch other subagents**. All agents are dispatched directly by the orchestrator. Work proceeds in iterative waves — each wave dispatches one agent, reviews its output, then decides the next step.

```
Orchestrator (top-level agent or user)
  ├── scanner      (one-time broad survey → _discovery/high_level_scan.md)
  ├── discovery    (traces code paths, writes to _discovery/)
  ├── researcher   (reviews discovery files, writes final reference docs)
  └── experimenter (writes/runs verification scripts under tools/)
```

### Agent Roles

- **Orchestrator** reads existing reference docs and discovery files, decomposes topics, dispatches agents one at a time, and reviews all output. It does NOT read source code in detail or do systematic exploration.
- **Scanner Agent** performs a broad, shallow scan of all source files and writes a structured topic inventory to `reference/_discovery/high_level_scan.md`. It runs **once** — the output is a durable reference as long as the source code hasn't changed. It does NOT trace mechanics or write final documentation.
- **Discovery Agent** traces mechanics across source files and writes raw findings to `reference/_discovery/`. It does NOT write final documentation or dispatch other agents.
- **Researcher Agent** reviews discovery files in `reference/_discovery/`, synthesizes findings, and writes final documentation to `reference/`. It does NOT do systematic code exploration, dispatch agents, or write to `reference/_discovery/`.
- **Experimenter Agent** writes and runs verification scripts under `tools/`. It does NOT write documentation or discovery files.

### Iterative Wave Workflow

Research on any topic follows this cycle. The orchestrator drives every step.

1. **Scan** (once): Dispatch `scanner` → produces `reference/_discovery/high_level_scan.md`. Reuse this across all topics.
2. **Discover**: Dispatch `discovery` agent for a specific topic → produces/updates a `reference/_discovery/<topic>.md` file.
3. **Review**: Orchestrator reads the discovery file. If gaps remain, dispatch `discovery` again with a narrower prompt.
4. **Document**: Dispatch `researcher` agent with the discovery file path → researcher reads it and writes to `reference/`.
5. **Verify**: Dispatch `experimenter` agent to validate specific claims → produces results in `tools/results/`.
6. **Correct**: If verification finds issues, loop back to step 2 or 4.

Waves are sequential per topic. Independent topics may overlap, but no more than 2–3 concurrent dispatches.

### Single-Topic Rule

Each agent dispatch handles **exactly one topic** (e.g., "combat damage formula", not "combat system"). If a topic is too broad, decompose it into smaller topics before dispatching.

## Documentation Conventions

- **Source citations**: `file.c:LINE` or `file.c:START-END` (e.g., `fmain.c:1609`, `narr.asm:251-347`). All paths are relative to the repo root.
- **Speech references**: `speak(N)` where N is the index into `narr.asm` message table.
- **Section numbering**: RESEARCH.md uses `## N. Title` / `### N.M Subtitle`. Sections files use matching numbers.
- **Cross-references**: Markdown links between reference docs (e.g., `[STORYLINE.md §5](STORYLINE.md#5-npc-dialogue-trees)`).
- **Diagrams**: Mermaid syntax for flowcharts, state diagrams, and sequence diagrams.

## Verification Workflow

When documenting a game mechanic:

1. **Read the source code** to extract the actual logic — never guess or infer from game behavior alone.
2. **Cite specific lines** using the `file:line` format.
3. **Cross-reference multiple code paths** when a system spans files (e.g., direction encoding verified via `fsubs.asm` movement vectors, `com2` table, and `fmain2.c` `set_course()`).
4. **Log unresolvable questions** in `reference/PROBLEMS.md` when something cannot be determined from source code alone (magic numbers, platform-dependent behavior, gameplay intent vs. bugs). Never guess — file a problem instead.

## Anti-Drift Rules

These rules apply to ALL agents and all documentation work in this project. They prevent the most common failure modes in reverse-engineering research: circular reasoning, unsupported claims, and scope creep.

### Evidence Before Claims

No mechanic may be documented without a source code citation. No citation may be reported without re-reading the actual line. The sequence is always: read code → cite line → verify citation → then document.

### Never Guess

If you cannot determine something from the source code, the correct response is to log it in `reference/PROBLEMS.md`. The incorrect response is to write "probably", "likely", "seems to", or "based on game behavior." There is no middle ground.

### Structured Escalation

When stuck, agents must report their status honestly:
- **COMPLETE**: All questions answered with citations
- **PARTIAL**: Some questions answered, gaps remain
- **NEEDS_REFINEMENT**: Found leads but need more investigation
- **BLOCKED**: Cannot proceed — state what's blocking

Never silently produce uncertain work. Escalate rather than guess.

### Repetition Limit

If the same question has been investigated 3+ times without resolution, one of these is true:
1. The scope is too broad — decompose the question
2. The answer isn't in the source code — log it in PROBLEMS.md
3. The approach is wrong — try an experiment instead of more code reading

Do not dispatch a 4th investigation without changing the approach.

### Logic Docs Are the Normative Form

Pseudo-code lives only in `reference/logic/`. Do not add pseudo-code blocks to `reference/RESEARCH.md`, `reference/ARCHITECTURE.md`, or `reference/STORYLINE.md` — those remain prose + tables + Mermaid only. When a behavior has been captured in `reference/logic/<subsystem>.md`, link to its anchor from RESEARCH instead of paraphrasing the logic.

- The grammar is defined in [`reference/logic/STYLE.md`](../reference/logic/STYLE.md).
- Global identifiers, enums, structs, constants, and table refs are declared in [`reference/logic/SYMBOLS.md`](../reference/logic/SYMBOLS.md). SYMBOLS.md changes are orchestrator-reviewed; agents propose additions in their report rather than edit it directly.
- Run `tools/run.sh lint_logic.py` after any change under `reference/logic/`. A clean lint is required before the task is considered complete.

### Don't Trust Summaries

When reviewing another agent's work, read the actual artifact (discovery file, experiment results, code), not just the summary. Summaries can be incomplete, overconfident, or wrong.

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

## Technical Context

- **Language**: Aztec C (1987) + Motorola 68000 assembly
- **Platform**: Commodore Amiga (custom chipset: Agnus, Denise, Paula)
- **Display**: Non-interlaced 320×200 frame, mixed-resolution split (lo-res playfield + hi-res status bar)
- **Graphics**: 5-bitplane (32-color) double-buffered playfield
- **Direction encoding**: 0=NW, 1=N, 2=NE, 3=E, 4=SE, 5=S, 6=SW, 7=W
