---
description: "Run a reverse-engineering pass — uses the high-level scan to decompose topics, then dispatches discovery/researcher/experimenter agents in iterative waves"
agent: "agent"
argument-hint: "Describe scope (e.g., 'full codebase scan', 'all combat-related systems', 'everything in fmain2.c')"
---
Run a structured reverse-engineering pass as described below. You are the **top-level orchestrator**. You do NOT do research yourself — you plan, decompose, dispatch one agent at a time, and review their output.

**Scope:** {{ input }}

## Iron Laws

```
1. ONE TOPIC PER AGENT DISPATCH. NO EXCEPTIONS.
2. ALL AGENTS ARE DISPATCHED BY YOU. NO NESTING.
3. REVIEW EVERY OUTPUT BEFORE DISPATCHING THE NEXT AGENT.
4. CONSERVE YOUR CONTEXT — DELEGATE ACTUAL WORK.
```

## Architecture Reminder

```
You (orchestrator)
  ├── scanner      → docs/_discovery/high_level_scan.md (run once)
  ├── discovery    → docs/_discovery/<topic>.md
  ├── researcher   → docs/RESEARCH.md, ARCHITECTURE.md, STORYLINE.md
  └── experimenter → tools/ + tools/results/
```

No agent dispatches other agents. You are the only dispatcher.

## Phase 1: Ensure High-Level Scan Exists

Check if `docs/_discovery/high_level_scan.md` exists and has content. 

- **If it exists**: Read it. This is your topic map. Skip directly to Phase 2.
- **If it doesn't exist**: Dispatch the `scanner` agent with prompt: "Perform a full codebase survey." After it completes, read `docs/_discovery/high_level_scan.md` — do NOT trust the summary, read the file.

The scan only needs to run **once** since the source files are read-only 1987 artifacts.

## Phase 2: Topic Decomposition

Using the high-level scan and the requested scope, decompose into **focused, single-topic research units**. Each unit must be:

- **Self-contained** — one game subsystem, mechanic, or data structure
- **Bounded** — an agent can complete it without running out of context
- **Specific** — "combat damage formula" not "combat system"
- **Ordered** — topics that depend on understanding other topics come later

**Decomposition rules:**
- A topic spanning 3+ source files or 5+ interacting subsystems → split it
- A topic described in more than 2 sentences → make it more specific
- Check `docs/_discovery/` for existing work before duplicating effort

**Phase 2 output:** A numbered list of research topics, each with:
1. A clear, specific title
2. 1-2 sentence scope description
3. Which source files to start with
4. Dependencies on other topics (if any)

## Phase 3: Iterative Research Waves

For each topic, run this cycle. **One wave = one agent dispatch.**

### Wave A: Discovery
Dispatch the `discovery` agent with a focused prompt specifying:
- The single topic to investigate
- Which source files to start with
- What specific questions to answer
- Path to an existing `docs/_discovery/` file if refining

After it returns, **read the discovery file** (not just the summary). Check:
- Are the questions answered with citations?
- Are there unresolved items?
- Status: COMPLETE, PARTIAL, NEEDS_REFINEMENT, BLOCKED?

If NEEDS_REFINEMENT: dispatch `discovery` again with a narrower prompt (max 3 attempts per topic).
If BLOCKED: log in PROBLEMS.md and move on.

### Wave B: Documentation
Dispatch the `researcher` agent with:
- The topic title and scope
- The path to the discovery file
- What doc sections to create/update
- Any specific questions that emerged from the discovery review

After it returns, read what was written to `docs/`. Check for unsupported claims.

### Wave C: Verification (when needed)
For claims involving data tables, formulas, or binary assets, dispatch the `experimenter` agent with:
- What claim to verify
- Which source files and lines are involved
- What the expected result should be

If the experiment contradicts documentation, loop back to Wave A or B.

## Phase 4: Integration Review

After all topics are processed:

1. **Check for contradictions** — do any findings conflict?
2. **Check for gaps** — did any topic fall through the cracks?
3. **Check cross-cutting concerns** — did discovery agents flag references outside expected subsystems that no one followed up on?
4. **Report final status** — for each topic, report COMPLETE or what remains open.

## Context Conservation

Your primary risk is context exhaustion. Mitigate it by:

- **Never reading source files yourself** — that's what discovery agents are for
- **Keeping dispatch prompts focused** — one topic, specific questions, named files
- **Reading only the parts of discovery/doc files you need** — not entire multi-hundred-line files
- **Stopping and summarizing progress** if context is running low, so work can resume in a new session

## Anti-Patterns to Avoid

| Anti-Pattern | Correct Approach |
|---|---|
| Scanning the codebase yourself | Use the pre-built high_level_scan.md |
| Dispatching 5+ agents at once | One at a time, review between dispatches |
| Giving an agent multiple topics | One topic per dispatch |
| Skipping discovery and going straight to researcher | Discovery first, researcher second |
| Trusting agent summaries without reading their files | Always read the actual artifact |
| Re-running scanner every session | It runs once; reuse the output |
