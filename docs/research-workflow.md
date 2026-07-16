# Research Workflow

How reverse-engineering work is organized in this project. The individual agent role definitions live in [`agents/`](agents/); the step-by-step orchestration procedures live in [`workflows/`](workflows/).

## Agent Architecture: Flat Iterative Model

Subagents **cannot dispatch other subagents**. All agents are dispatched directly by the orchestrator. Work proceeds in iterative waves — each wave dispatches one agent, reviews its output, then decides the next step.

```
Orchestrator (top-level agent or user)
  ├── scanner      (one-time broad survey → _discovery/high_level_scan.md)
  ├── discovery    (traces code paths, writes to _discovery/)
  ├── researcher   (reviews discovery files, writes final reference docs)
  └── experimenter (writes/runs verification scripts under tools/)
```

## Agent Roles

Full definitions in [`agents/`](agents/):

- **Orchestrator** reads existing reference docs and discovery files, decomposes topics, dispatches agents one at a time, and reviews all output. It does NOT read source code in detail or do systematic exploration.
- **[Scanner](agents/scanner.md)** performs a broad, shallow scan of all source files and writes a structured topic inventory to `reference/_discovery/high_level_scan.md`. It runs **once** — the output is durable as long as the source code hasn't changed. It does NOT trace mechanics or write final documentation.
- **[Discovery](agents/discovery.md)** traces mechanics across source files and writes raw findings to `reference/_discovery/`. It does NOT write final documentation or dispatch other agents.
- **[Researcher](agents/researcher.md)** reviews discovery files in `reference/_discovery/`, synthesizes findings, and writes final documentation to `reference/`. It does NOT do systematic code exploration, dispatch agents, or write to `reference/_discovery/`.
- **[Experimenter](agents/experimenter.md)** writes and runs verification scripts under `tools/`. It does NOT write documentation or discovery files.
- **[Research assistant](agents/research.md)** answers natural-language questions by reasoning over the reference documentation via a locally-hosted LLM server.

## Iterative Wave Workflow

Research on any topic follows this cycle. The orchestrator drives every step.

1. **Scan** (once): Dispatch `scanner` → produces `reference/_discovery/high_level_scan.md`. Reuse this across all topics.
2. **Discover**: Dispatch `discovery` agent for a specific topic → produces/updates a `reference/_discovery/<topic>.md` file.
3. **Review**: Orchestrator reads the discovery file. If gaps remain, dispatch `discovery` again with a narrower prompt.
4. **Document**: Dispatch `researcher` agent with the discovery file path → researcher reads it and writes to `reference/`.
5. **Verify**: Dispatch `experimenter` agent to validate specific claims → produces results in `tools/results/`.
6. **Correct**: If verification finds issues, loop back to step 2 or 4.

Waves are sequential per topic. Independent topics may overlap, but no more than 2–3 concurrent dispatches. The full orchestration procedure is in [`workflows/reverse-engineer.md`](workflows/reverse-engineer.md).

## Single-Topic Rule

Each agent dispatch handles **exactly one topic** (e.g., "combat damage formula", not "combat system"). If a topic is too broad, decompose it into smaller topics before dispatching.

**Decomposition rules:**
- A topic spanning 3+ source files or 5+ interacting subsystems → split it.
- A topic described in more than 2 sentences → make it more specific.
- Check `reference/_discovery/` for existing work before duplicating effort.

## Context Conservation

The orchestrator's primary risk is context exhaustion. Mitigate it by:

- **Never reading source files yourself** — that's what discovery agents are for.
- **Keeping dispatch prompts focused** — one topic, specific questions, named files.
- **Reading only the parts of discovery/doc files you need** — not entire multi-hundred-line files.
- **Stopping and summarizing progress** if context is running low, so work can resume in a new session.

## Anti-Patterns to Avoid

| Anti-Pattern | Correct Approach |
|---|---|
| Scanning the codebase yourself | Use the pre-built `high_level_scan.md` |
| Dispatching 5+ agents at once | One at a time, review between dispatches |
| Giving an agent multiple topics | One topic per dispatch |
| Skipping discovery and going straight to researcher | Discovery first, researcher second |
| Trusting agent summaries without reading their files | Always read the actual artifact |
| Re-running scanner every session | It runs once; reuse the output |
