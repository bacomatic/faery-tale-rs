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
  _discovery/       Raw findings from discovery agents (working notes)
game/               Runtime binary assets (images, fonts, music, map sectors)
ToArchive/          Original distribution package
tools/              Verification scripts and 68k assembly testing
  run.sh            Venv wrapper — runs any tool script via .toolenv
  verify_asm.py     Assemble & execute 68k snippets (GNU as + machine68k)
  validate_citations.py   Check doc citations against source files
  extract_table.py        Pull data tables from source
  requirements.txt  Python dependencies for .toolenv
.github/
  copilot-instructions.md   Workspace instructions and anti-drift rules
  agents/                   Agent definitions (researcher, discovery, experimenter)
  prompts/                  Task prompts (verify-mechanic, reverse-engineer, etc.)
  instructions/             File-scoped conventions (docs, tools)
```

## Documentation

The documentation is three-tiered:

| Document | Purpose |
|----------|---------|
| [ARCHITECTURE.md](docs/ARCHITECTURE.md) | High-level system overview: 19 subsystems, data flow, game loop, display geometry |
| [RESEARCH.md](docs/RESEARCH.md) | Ground truth: 20 numbered sections covering every game mechanic with formulas, data tables, and source citations |
| [STORYLINE.md](docs/STORYLINE.md) | Narrative layer: quest progression, NPC dialogue trees, event sequences |
| [PROBLEMS.md](docs/PROBLEMS.md) | Open questions that can't be answered from source code alone |

## Prerequisites

- **Python 3.10+** — for verification tools
- **binutils-m68k-linux-gnu** — GNU cross-assembler for 68000, used by `verify_asm.py`
  ```bash
  sudo apt-get install binutils-m68k-linux-gnu
  ```
- **Python dependencies** are installed automatically into `.toolenv/` on first use of `tools/run.sh`

## Agent Hierarchy

Research is driven by AI agents in VS Code Copilot, organized in a strict delegation chain:

```
Orchestrator (/reverse-engineer prompt or user)
  └── Researcher (one per topic — plans, reviews, writes docs)
        ├── Discovery Agent (traces code, writes to _discovery/)
        └── Experimenter Agent (runs verification scripts)
```

Each level has a defined role and must not do the work of levels below it. All agents enforce anti-drift rules: no guessing, no unsupported claims, structured escalation when stuck, and mandatory source citations for every documented fact.

### Agents

| Agent | Role | Scope |
|-------|------|-------|
| `@researcher` | Coordinates research on a single topic — plans investigation, dispatches sub-agents, reviews findings, writes final docs | Reads source code, writes to `docs/`. Does NOT do systematic code exploration or write to `_discovery/`. |
| `@discovery` | Deep code exploration — traces mechanics across files, follows all references | Reads source code, writes raw findings to `docs/_discovery/`. Does NOT write final documentation. |
| `@experimenter` | Experimental verification — writes and runs scripts that mechanically validate claims | Reads source code, writes scripts and results under `tools/`. Does NOT write documentation. |

**Example prompts:**
- `@researcher How does the save/load system serialize actor state?`
- `@researcher Trace the complete NPC dialogue flow for the turtle (race 7)`

### Prompts

| Prompt | Purpose |
|--------|---------|
| `/reverse-engineer` | Top-level orchestration — scans codebase, decomposes into topics, dispatches one researcher per topic sequentially |
| `/verify-mechanic` | Structured verification of a specific game mechanic — traces code, cross-references paths, compares against existing docs |
| `/run-experiment` | Dispatches the experimenter agent to write and run a verification script |
| `/update-doc` | Applies approved changes to documentation files with citation enforcement |

**Example invocations:**
- `/reverse-engineer full codebase scan`
- `/verify-mechanic combat hit calculation and bravery scaling`
- `/run-experiment extract direction vectors from fsubs.asm`
- `/update-doc fix key names in §8 rescue sequence`

### Instructions

| Instruction | Auto-attaches to | Enforces |
|-------------|-------------------|----------|
| `docs-conventions` | `docs/**` | Source citation format (`file.c:LINE`), speech references (`speak(N)`), section numbering, read-only source protection |
| `tools-conventions` | `tools/**` | Naming conventions, result format, source read-only constraint, tool reuse policy |

### Typical Workflow

1. **Investigate** — use `@researcher` or `/verify-mechanic` to trace a mechanic in the source code
2. **Verify** — use `/run-experiment` to mechanically validate findings
3. **Review** — examine the findings and approve corrections
4. **Apply** — use `/update-doc` to write changes across all affected documentation files

### Monitoring PROBLEMS.md

Agents log questions in [PROBLEMS.md](docs/PROBLEMS.md) whenever they encounter something that can't be determined from source code alone — magic numbers, ambiguous variable names, platform-specific Amiga behavior, or cases where gameplay intent is unclear (intentional design vs. bug).

**Check this file periodically.** Your play-testing experience and domain knowledge is needed to resolve these entries.

## Verification Tools

All tools run via `tools/run.sh`, which manages a shared `.toolenv/` virtual environment:

```bash
# Validate source citations in documentation
tools/run.sh validate_citations.py

# Extract data tables from source
tools/run.sh extract_table.py fsubs.asm xdir ydir

# Assemble and execute 68k code to verify assembly logic
tools/run.sh verify_asm.py -c "moveq #42,d0; moveq #10,d1; add.l d1,d0" --trace
```

`verify_asm.py` uses `m68k-linux-gnu-as` (GNU cross-assembler) and `machine68k` (Musashi-based CPU emulator) to assemble and execute 68000 code snippets. It accepts Motorola syntax matching the FTA source files, supports labels, data directives, step-by-step tracing, initial register/memory setup, and JSON output. See [tools/README.md](tools/README.md) for full usage.

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
