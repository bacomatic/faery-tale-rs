# Faery Tale Adventure — Reverse-Engineering Research Project

## Project Purpose

This is a **research and documentation project** for reverse-engineering *The Faery Tale Adventure* (MicroIllusions, 1987 Amiga). The repository contains the original source code by Talin and a growing body of analysis documents.

## Critical Rule: Source Code Is Read-Only

**Do not edit any source file under any circumstances.** The C, assembly, header, procedure, and build files are original 1987 artifacts preserved for reference. This includes all `.c`, `.asm`, `.h`, `.i`, `.p` files in the repo root, plus `makefile`, `AztecC.Err`, `fta.br`, `notes`, and everything in `game/` and `ToArchive/`.

Agents may only read source files to verify or extract information for documentation.

## Repository Layout

```
(root)              Original source code (Aztec C + 68000 assembly) — READ ONLY
docs/
  ARCHITECTURE.md   System architecture overview, Mermaid diagrams, display geometry
  RESEARCH.md       Comprehensive mechanics reference (20 numbered sections)
  STORYLINE.md      Quest flows and NPC interactions as Mermaid state diagrams
  _discovery/       Raw findings from discovery agents — working notes, not final docs
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

## Documentation Conventions

- **Source citations**: `file.c:LINE` or `file.c:START-END` (e.g., `fmain.c:1609`, `narr.asm:251-347`). All paths are relative to the repo root.
- **Speech references**: `speak(N)` where N is the index into `narr.asm` message table.
- **Section numbering**: RESEARCH.md uses `## N. Title` / `### N.M Subtitle`. Sections files use matching numbers.
- **Cross-references**: Markdown links between docs (e.g., `[STORYLINE.md §5](STORYLINE.md#5-npc-dialogue-trees)`).
- **Diagrams**: Mermaid syntax for flowcharts, state diagrams, and sequence diagrams.

## Verification Workflow

When documenting a game mechanic:

1. **Read the source code** to extract the actual logic — never guess or infer from game behavior alone.
2. **Cite specific lines** using the `file:line` format.
3. **Cross-reference multiple code paths** when a system spans files (e.g., direction encoding verified via `fsubs.asm` movement vectors, `com2` table, and `fmain2.c` `set_course()`).
4. **Log unresolvable questions** in `docs/PROBLEMS.md` when something cannot be determined from source code alone (magic numbers, platform-dependent behavior, gameplay intent vs. bugs). Never guess — file a problem instead.

## Anti-Drift Rules

These rules apply to ALL agents and all documentation work in this project. They prevent the most common failure modes in reverse-engineering research: circular reasoning, unsupported claims, and scope creep.

### Evidence Before Claims

No mechanic may be documented without a source code citation. No citation may be reported without re-reading the actual line. The sequence is always: read code → cite line → verify citation → then document.

### Never Guess

If you cannot determine something from the source code, the correct response is to log it in `docs/PROBLEMS.md`. The incorrect response is to write "probably", "likely", "seems to", or "based on game behavior." There is no middle ground.

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

### Don't Trust Summaries

When reviewing another agent's work, read the actual artifact (discovery file, experiment results, code), not just the summary. Summaries can be incomplete, overconfident, or wrong.

## Key Source Files (Quick Reference)

| File | Domain |
|------|--------|
| `fmain.c` | Core game loop, actors, combat, physics, rendering, UI |
| `fmain2.c` | Quests, NPC dialogue, shops, brother succession, save/load, win condition |
| `fsubs.asm` | Movement vectors, joystick handler, direction tables, low-level subroutines |
| `fsupp.asm` | Assembly support routines |
| `gdriver.asm` | Graphics driver: bitplane compositing, sprite rendering, blitter ops |
| `narr.asm` | All in-game message text, indexed by speech number |
| `terrain.c` | Terrain data decoding, region names |
| `ftale.h` | Master header: struct definitions, motion states, goal modes, display constants |
| `iffsubs.c` | IFF/ILBM image parser |
| `hdrive.c` | Disk I/O and async asset loading |
| `text.c` | Text rendering, HUD display |
| `mtrack.c` | 4-channel music tracker/player |

## Technical Context

- **Language**: Aztec C (1987) + Motorola 68000 assembly
- **Platform**: Commodore Amiga (custom chipset: Agnus, Denise, Paula)
- **Display**: Non-interlaced 320×200 frame, mixed-resolution split (lo-res playfield + hi-res status bar)
- **Graphics**: 5-bitplane (32-color) double-buffered playfield
- **Direction encoding**: 0=NW, 1=N, 2=NE, 3=E, 4=SE, 5=S, 6=SW, 7=W
