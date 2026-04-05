---
description: "Use when creating or editing experiment scripts — enforces naming conventions, result format, source read-only constraint, and tool reuse policy"
applyTo: "tools/**"
---
# Tools Directory Conventions

## Source Files Are Read-Only

Never edit `.c`, `.asm`, `.h`, `.i`, `.p` files, `makefile`, `AztecC.Err`, `fta.br`, `notes`, or anything in `game/` or `ToArchive/`. Scripts may only **read** source files to extract or verify information.

## Write Scope

Scripts and results must stay within `tools/`:
- Scripts: `tools/*.py`, `tools/*.sh`, `tools/*.js`, etc.
- Results: `tools/results/<name>.txt` or `tools/results/<name>.json`
- Do not write files outside of `tools/`.

## Reuse Before Creating

Before creating a new script, check what already exists in `tools/`. If an existing script covers the same verification type, extend it (add arguments, new modes) rather than creating a duplicate.

## Naming Conventions

| Prefix | Purpose | Example |
|--------|---------|---------|
| `validate_` | Check documentation accuracy | `validate_citations.py` |
| `extract_` | Pull data from source files | `extract_table.py` |
| `verify_` | Test formulas/algorithms | `verify_combat_damage.py` |
| `decode_` | Parse binary assets | `decode_map_sector.py` |

## Result Output Format

Results in `tools/results/` should use this structure:

```
Experiment: <name>
Date: <ISO date>
Command: <how to reproduce>
Status: PASS | FAIL | PARTIAL | NEEDS_HUMAN_REVIEW

Findings:
- <finding 1>
- <finding 2>

Details:
<detailed output>
```

## Script Requirements

- Scripts must be runnable from the repo root: `python tools/<script>.py`
- Use `argparse` or equivalent for configurable scripts
- Print a clear summary to stdout; write detailed results to `tools/results/`
- Exit code 0 for PASS, 1 for FAIL, 2 for PARTIAL/NEEDS_HUMAN_REVIEW
