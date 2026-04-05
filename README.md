# The Faery Tale Adventure — Reverse-Engineering Research

A research and documentation project for reverse-engineering *The Faery Tale Adventure* (MicroIllusions, 1987 Amiga). The repository contains Talin's original source code alongside a growing body of analysis documents produced by reading and tracing the code.

> **Original README by Talin:** [README-Talin.md](README-Talin.md) — copyright status, historical context, and active forks.

## Repository Layout

```
(root)              Original source code (Aztec C + 68000 assembly) — READ ONLY
docs/
  ARCHITECTURE.md   System architecture overview with Mermaid diagrams
  RESEARCH.md       Comprehensive mechanics reference (20 sections)
  STORYLINE.md      Quest flows and NPC interactions as state diagrams
  PROBLEMS.md       Open questions needing expert input
  _sections/        Deep-dive files, one per RESEARCH.md section
  superpowers/      Plans and specs for the research project itself
game/               Runtime binary assets (images, fonts, music, map sectors)
ToArchive/          Original distribution package
.github/
  copilot-instructions.md   Workspace instructions for AI agents
  agents/                   Custom agent definitions
  prompts/                  Reusable task prompts
  instructions/             File-scoped conventions
```

## Documentation

The documentation is three-tiered:

| Document | Purpose |
|----------|---------|
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | High-level system overview: 19 subsystems, data flow, game loop, display geometry |
| [RESEARCH.md](docs/RESEARCH.md) | Ground truth: 20 numbered sections covering every game mechanic with formulas, data tables, and source citations |
| [STORYLINE.md](docs/STORYLINE.md) | Narrative layer: quest progression, NPC dialogue trees, event sequences |
| [PROBLEMS.md](docs/PROBLEMS.md) | Open questions that can't be answered from source code alone |

Each section in RESEARCH.md has a corresponding deep-dive in `docs/_sections/` (e.g., `section_03_characters.md` expands §3).

## Research Tools

This project includes AI agent customizations for VS Code Copilot to support structured reverse-engineering research.

### Agent: `@researcher`

A specialized agent for source code archaeology. Select it from the agent picker in Copilot Chat.

**What it does:** Reads original source code, traces game mechanics across files, cross-references multiple code paths, and reports findings with precise `file:line` citations. Has no terminal access and cannot edit source files.

**Example prompts:**
- `@researcher How does the save/load system serialize actor state?`
- `@researcher Trace the complete NPC dialogue flow for the turtle (race 7)`
- `@researcher What terrain types does prox() block, and at what thresholds?`

### Prompt: `/verify-mechanic`

A structured verification workflow for investigating a specific game mechanic.

**What it does:** Searches the source for all code paths implementing a mechanic, reads and traces the logic, cross-references at least two independent code paths, then compares findings against existing documentation. Reports what's correct, incorrect, and missing — and waits for approval before editing.

**Example invocations:**
- `/verify-mechanic direction encoding`
- `/verify-mechanic lava damage and rose protection`
- `/verify-mechanic combat hit calculation and bravery scaling`

### Prompt: `/update-doc`

Applies approved changes to documentation files.

**What it does:** Takes a described change and applies it to RESEARCH.md and the matching `_sections/` file together. Also checks ARCHITECTURE.md and STORYLINE.md for related content that may need updating. Enforces citation format and section numbering conventions.

**Example invocations:**
- `/update-doc fix key names in §8 rescue sequence to Green, Blue, Red, Grey, White`
- `/update-doc add Crystal Shard terrain bypass subsection to §6`

### Instruction: `docs-conventions`

Auto-attaches whenever any file under `docs/` is being edited. Enforces:
- Source citation format (`file.c:LINE` or `file.c:START-END`)
- Speech reference format (`speak(N)`)
- Section numbering conventions
- Dual-update rule (RESEARCH.md + matching `_sections/` file)
- Read-only source file protection

### Typical Workflow

1. **Investigate** — use `@researcher` or `/verify-mechanic` to trace a mechanic in the source code
2. **Review** — examine the findings and approve corrections
3. **Apply** — use `/update-doc` to write changes across all affected documentation files

### Monitoring PROBLEMS.md

Agents are instructed to log questions in [PROBLEMS.md](docs/PROBLEMS.md) whenever they encounter something that can't be determined from source code alone — magic numbers, ambiguous variable names, platform-specific Amiga behavior, or cases where gameplay intent is unclear (intentional design vs. bug).

**Check this file periodically.** Your play-testing experience and domain knowledge is needed to resolve these entries. When you can answer a question, add your resolution and move the entry to the Resolved section. Agents will incorporate your answers into the documentation on the next relevant edit.

## Source Code Reference

All source files are **read-only** — original 1987 artifacts preserved for reference.

| File | Domain |
|------|--------|
| `fmain.c` | Core game loop, actors, combat, physics, rendering, UI |
| `fmain2.c` | Quests, NPC dialogue, shops, brother succession, save/load |
| `fsubs.asm` | Movement vectors, joystick handler, direction tables |
| `fsupp.asm` | Assembly support routines |
| `gdriver.asm` | Graphics driver: bitplane compositing, sprite rendering |
| `narr.asm` | All in-game message text, indexed by speech number |
| `terrain.c` | Terrain data decoding, region names |
| `ftale.h` | Master header: structs, motion states, goal modes, constants |
| `iffsubs.c` | IFF/ILBM image parser |
| `hdrive.c` | Disk I/O and async asset loading |
| `text.c` | Text rendering, HUD display |
| `mtrack.c` | 4-channel music tracker/player |
