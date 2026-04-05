---
description: "Verify a game mechanic by reading original source code, citing specific lines, and updating documentation with findings"
agent: "agent"
argument-hint: "Describe the mechanic or system to verify (e.g., 'direction encoding', 'lava damage', 'door system')"
---
Verify the game mechanic described below by following this strict workflow. Do NOT guess or infer from game behavior — all claims must be traced to source code.

**Mechanic to verify:** {{ input }}

## Iron Law

```
NO VERIFICATION CLAIMS WITHOUT RE-READING THE SOURCE LINE
```

If you haven't read the actual source line in this session, you cannot cite it. If you cannot find the source line, you cannot claim the mechanic works a certain way.

## Step 1: Identify relevant source files

Search the original source files for code implementing this mechanic. Start with the key files listed in [copilot-instructions.md](../copilot-instructions.md) and expand as needed. Remember: source files are READ-ONLY — do not edit them.

## Step 2: Read and trace the logic

Read the actual source code implementing the mechanic. Follow the logic across files when the system spans multiple sources. Extract:
- The exact algorithm or formula used
- Relevant constants, arrays, and data tables
- Edge cases and boundary conditions
- Any dead code or vestigial features related to this mechanic

## Step 3: Cross-reference multiple code paths

Verify findings against at least two independent code paths when possible (e.g., a data table AND the code that reads it, or an input handler AND an AI routine using the same values). Flag any contradictions.

## Step 4: Compare against existing documentation

Check what [RESEARCH.md](../../docs/RESEARCH.md), [ARCHITECTURE.md](../../docs/ARCHITECTURE.md), and [STORYLINE.md](../../docs/STORYLINE.md) currently say about this mechanic. Identify:
- **Correct claims** — cite the source lines that confirm them
- **Incorrect claims** — cite the source lines that contradict them
- **Missing information** — note what the code reveals that the docs don't cover

## Step 5: Self-verify before reporting

Before presenting findings, re-check:
1. Every `file:line` citation — re-read the line and confirm it says what you claim
2. Every formula or constant — re-read the source, don't rely on memory
3. Every cross-reference — confirm both sides of every "X calls Y" or "X uses value from Y"
4. Scan your findings for "probably", "likely", "seems to" — replace with citations or remove

## Step 6: Report findings

Present a summary with:
- What the source code actually does (with `file:line` citations)
- What the documentation gets right and wrong
- Proposed corrections or additions, if any
- **Status**: COMPLETE | PARTIAL | NEEDS_REFINEMENT | BLOCKED

If anything about the mechanic **cannot be determined from source code alone** (magic numbers without comments, platform-dependent behavior, gameplay intent vs. bugs), log it in [PROBLEMS.md](../../docs/PROBLEMS.md) using the template in that file. Never guess — file a problem instead.

**Do not edit other documentation yet.** Wait for approval before making changes. When approved, update RESEARCH.md directly.
