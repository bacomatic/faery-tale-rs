# The Faery Tale Adventure — Reverse-Engineering Research

A research and documentation project for reverse-engineering *The Faery Tale Adventure* (MicroIllusions, 1987 Amiga). The repository contains Talin's original source code alongside a growing body of analysis documents produced by reading and tracing the code.

> **Original README by Talin:** [README-Talin.md](README-Talin.md) — copyright status, historical context, and active forks.

## Repository Layout

```
(root)              Original source code (Aztec C + 68000 assembly) — READ ONLY
reference/
  ARCHITECTURE.md   System architecture overview, Mermaid diagrams, display geometry
  RESEARCH.md       Comprehensive mechanics reference (20 numbered sections)
  STORYLINE.md      Quest flows and NPC interactions as Mermaid state diagrams
  PROBLEMS.md       Open questions that can't be answered from source code alone
  world_db.json     Unified spatial database: objects, doors, extents, terrain by region/sector
  _discovery/       Raw findings from discovery agents — working notes, not final reference docs
game/               Runtime binary assets (images, fonts, music, map sectors) — READ ONLY
ToArchive/          Original distribution package — READ ONLY
tools/              Verification scripts and 68k assembly testing
  run.sh            Venv wrapper — runs any tool script via .toolenv
  verify_asm.py     Assemble & execute 68k snippets (GNU as + machine68k)
  validate_citations.py   Check doc citations against source files
  extract_table.py        Pull data tables from source
  requirements.txt  Python dependencies for .toolenv
AGENTS.md           Canonical agent contract (behavioral guidelines, critical directives, doc map)
docs/               Agent-facing task docs linked from AGENTS.md
  project-overview.md          Purpose, layout, read-only rules, source-file map, spatial DB
  research-workflow.md         Flat iterative agent model, roles, wave workflow
  documentation-conventions.md Citation format, numbering, anti-drift rules
  tools-conventions.md         Conventions for scripts under tools/
  agents/                      Agent role definitions (scanner, discovery, researcher, …)
  workflows/                   Orchestration procedures (reverse-engineer, verify-mechanic, …)
```

## Documentation

The documentation is three-tiered:

| Document | Purpose |
|----------|---------|
| [ARCHITECTURE.md](reference/ARCHITECTURE.md) | High-level system overview: 19 subsystems, data flow, game loop, display geometry |
| [RESEARCH.md](reference/RESEARCH.md) | Ground truth: 20 numbered sections covering every game mechanic with formulas, data tables, and source citations |
| [STORYLINE.md](reference/STORYLINE.md) | Narrative layer: quest progression, NPC dialogue trees, event sequences |
| [PROBLEMS.md](reference/PROBLEMS.md) | Open questions that can't be answered from source code alone |

## Prerequisites

- **Python 3.10+** — for verification tools
- **binutils-m68k-linux-gnu** — GNU cross-assembler for 68000, used by `verify_asm.py`
  ```bash
  sudo apt-get install binutils-m68k-linux-gnu
  ```
- **Python dependencies** are installed automatically into `.toolenv/` on first use of `tools/run.sh`

## Agent Architecture: Flat Iterative Model

Research is driven by AI agents (in any tool). Because subagents cannot dispatch further subagents, all agents are dispatched **directly by the orchestrator** (the user or top-level agent). No agent can dispatch another agent. The full model, roles, and workflow live in [docs/research-workflow.md](docs/research-workflow.md); the agent contract is [AGENTS.md](AGENTS.md).

```
User / Orchestrator
  ├── scanner      (one-time broad survey → _discovery/high_level_scan.md)
  ├── discovery    (traces code paths, writes to _discovery/)
  ├── researcher   (reviews discovery files, writes final reference docs)
  └── experimenter (writes/runs verification scripts under tools/)
```

Work proceeds in iterative waves — dispatch one agent, review its output, then decide the next step. All agents enforce anti-drift rules: no guessing, no unsupported claims, structured escalation when stuck, and mandatory source citations for every documented fact.

### Agents

Full role definitions live in [docs/agents/](docs/agents/).

| Agent | Role | Scope |
|-------|------|-------|
| [scanner](docs/agents/scanner.md) | Broad codebase survey — shallow scan of all source files, produces a topic inventory | Reads source code, writes `reference/_discovery/high_level_scan.md`. Runs once. |
| [discovery](docs/agents/discovery.md) | Deep code exploration — traces mechanics across files, follows all references | Reads source code, writes raw findings to `reference/_discovery/`. Does NOT write final documentation. |
| [researcher](docs/agents/researcher.md) | Synthesizes discovery findings into final documentation | Reviews `reference/_discovery/` files, does lightweight verification reads, writes to `reference/`. Does NOT do systematic code exploration or write to `_discovery/`. |
| [experimenter](docs/agents/experimenter.md) | Experimental verification — writes and runs scripts that mechanically validate claims | Reads source code, writes scripts and results under `tools/`. Does NOT write documentation. |

### Workflows

Step-by-step orchestration procedures live in [docs/workflows/](docs/workflows/).

| Workflow | Purpose |
|----------|---------|
| [reverse-engineer](docs/workflows/reverse-engineer.md) | Top-level orchestration — scans codebase, decomposes into topics, dispatches one agent per topic sequentially |
| [verify-mechanic](docs/workflows/verify-mechanic.md) | Structured verification of a specific game mechanic — traces code, cross-references paths, compares against existing reference docs |
| [run-experiment](docs/workflows/run-experiment.md) | Dispatches the experimenter agent to write and run a verification script |
| [update-doc](docs/workflows/update-doc.md) | Applies approved changes to documentation files with citation enforcement |

### Conventions

| Convention doc | Applies to | Enforces |
|----------------|-----------|----------|
| [documentation-conventions](docs/documentation-conventions.md) | `reference/**` | Source citation format (`file.c:LINE`), speech references (`speak(N)`), section numbering, read-only source protection |
| [tools-conventions](docs/tools-conventions.md) | `tools/**` | Naming conventions, result format, source read-only constraint, tool reuse policy |

### Iterative Wave Workflow

Research on any topic follows this cycle. The user drives every step.

1. **Scan** (once): Dispatch the `scanner` agent → produces `reference/_discovery/high_level_scan.md`. Reuse across all topics.
2. **Discover**: Dispatch the `discovery` agent for a specific topic → produces/updates a `reference/_discovery/<topic>.md` file.
3. **Review**: Read the discovery file. If gaps remain, dispatch `discovery` again with a narrower prompt.
4. **Document**: Dispatch the `researcher` agent with the discovery file path → researcher reviews and writes to `reference/`.
5. **Verify**: Dispatch the `experimenter` agent to validate specific claims → produces results in `tools/results/`.
6. **Correct**: If verification finds issues, loop back to step 2 or 4.

Each agent dispatch handles **exactly one topic** (e.g., "combat damage formula", not "combat system"). If a topic is too broad, decompose it into smaller topics before dispatching.

### Monitoring PROBLEMS.md

Agents log questions in [PROBLEMS.md](reference/PROBLEMS.md) whenever they encounter something that can't be determined from source code alone — magic numbers, ambiguous variable names, platform-specific Amiga behavior, or cases where gameplay intent is unclear (intentional design vs. bug).

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
| `fsupp.asm` | Assembly versions of colorplay, stillscreen, skipint (superseded) |
| `gdriver.asm` | Audio driver: VBlank music interrupt, score/sample playback, tempo |
| `narr.asm` | All in-game message text, indexed by speech number |
| `terrain.c` | Extracts terrain data from IFF landscape images into `terra` binary (build tool) |
| `ftale.h` | Master header: structs, motion states, goal modes, constants |
| `iffsubs.c` | IFF/ILBM image parser |
| `hdrive.c` | Disk I/O and async asset loading |
| `text.c` | Standalone font test program by Talin (not game-related) |
| `mtrack.c` | Disk track writer: game assets to disk 1 (build tool) |
