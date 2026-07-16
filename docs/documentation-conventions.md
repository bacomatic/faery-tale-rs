# Documentation Conventions

Applies to all documentation work under `reference/`.

## Source Citations

- Format: `file.c:LINE` or `file.c:START-END` (e.g., `fmain.c:1609`, `narr.asm:251-347`)
- All file paths are relative to the source root — no directory prefixes
- Speech references: `speak(N)` where N is the `narr.asm` message table index

## Section Numbering

- RESEARCH sub-documents: `## N. Title` for top-level sections, `### N.M Subtitle` for subsections
- Section numbers are preserved across the split (e.g., §17 is always "Main Game Loop" in `RESEARCH-systems.md`)

## Cross-References

- Between reference docs: `[STORYLINE.md §5](STORYLINE.md#5-npc-dialogue-trees)`
- To RESEARCH sections: `[§6 Terrain](RESEARCH.md#6-terrain--collision)` — the hub `RESEARCH.md` has anchor stubs that redirect to sub-documents. Direct links to sub-documents also work.
- Diagrams use Mermaid syntax (flowcharts, state diagrams, sequence diagrams)

## Single Source of Truth

The `RESEARCH-*.md` sub-documents are the single source of truth for game mechanics. `RESEARCH.md` is their hub/index with anchor stubs for backward compatibility. `STORYLINE-*.md` sub-documents are the single source of truth for narrative content, with `STORYLINE.md` as the hub.

## Logic Docs Are the Normative Form

Pseudo-code lives only in `reference/logic/`. Do not add pseudo-code blocks to `reference/RESEARCH.md`, `reference/ARCHITECTURE.md`, or `reference/STORYLINE.md` — those remain prose + tables + Mermaid only. When a behavior has been captured in `reference/logic/<subsystem>.md`, link to its anchor from RESEARCH instead of paraphrasing the logic.

- The grammar is defined in [`reference/logic/STYLE.md`](../reference/logic/STYLE.md).
- Global identifiers, enums, structs, constants, and table refs are declared in [`reference/logic/SYMBOLS.md`](../reference/logic/SYMBOLS.md). SYMBOLS.md changes are orchestrator-reviewed; agents propose additions in their report rather than edit it directly.
- Run `tools/run.sh lint_logic.py` after any change under `reference/logic/`. A clean lint is required before the task is considered complete.

## Verification Workflow

When documenting a game mechanic:

1. **Read the source code** to extract the actual logic — never guess or infer from game behavior alone.
2. **Cite specific lines** using the `file:line` format.
3. **Cross-reference multiple code paths** when a system spans files (e.g., direction encoding verified via `fsubs.asm` movement vectors, `com2` table, and `fmain2.c` `set_course()`).
4. **Log unresolvable questions** in `reference/PROBLEMS.md` when something cannot be determined from source code alone (magic numbers, platform-dependent behavior, gameplay intent vs. bugs). Never guess — file a problem instead.

## Experiment Results

Files in `tools/results/` are transient (gitignored) and must **never** be linked from documentation. When an experiment produces findings relevant to a doc entry, inline the key results directly — include the reproduction command (`python tools/<script>.py`), a bullet summary of findings, and any data tables needed to support the conclusion. The reader must be able to understand the evidence without access to `tools/results/`.

## Anti-Drift Rules

These rules apply to ALL agents and all documentation work. They prevent the most common failure modes in reverse-engineering research: circular reasoning, unsupported claims, and scope creep.

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

### Don't Trust Summaries

When reviewing another agent's work, read the actual artifact (discovery file, experiment results, code), not just the summary. Summaries can be incomplete, overconfident, or wrong.
