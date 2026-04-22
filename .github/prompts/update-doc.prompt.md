---
description: "Apply verified findings to RESEARCH.md, maintaining citation format and section numbering conventions"
agent: "agent"
argument-hint: "Describe the change to apply (e.g., 'fix key names in §8 rescue sequence', 'add Crystal Shard terrain bypass to §6')"
---
Apply the documented change described below to the project documentation. Follow these rules strictly.

**Change to apply:** {{ input }}

## Rules

1. **Read before writing.** Read the relevant section in [RESEARCH.md](../../reference/RESEARCH.md) before making any edit.
2. **Check other reference docs.** Scan [ARCHITECTURE.md](../../reference/ARCHITECTURE.md) and [STORYLINE.md](../../reference/STORYLINE.md) for related text that may also need updating (e.g., Mermaid diagrams referencing the same mechanic, summary tables with the same data).
3. **Verify all citations.** Before writing any `file.c:LINE` reference into documentation, re-read that line to confirm it contains the code described. Never copy citations from another source without re-verifying.
4. **Preserve conventions.** Use `file.c:LINE` or `file.c:START-END` for source citations. Use `speak(N)` for narr.asm references. Maintain existing section numbering (`## N. Title` / `### N.M Subtitle`).
5. **Do not edit source code.** All `.c`, `.asm`, `.h`, `.i`, `.p` files and everything in `game/` and `ToArchive/` are read-only.
6. **No unsupported claims.** Every factual statement about game mechanics must have a source citation. If you cannot cite it, do not write it. Log unknowns in [PROBLEMS.md](../../reference/PROBLEMS.md).
7. **Verify after editing.** After all edits, confirm the change is consistent across every file that was modified. Re-read each edited section to check for introduced errors.

## Workflow

1. Identify which RESEARCH.md section(s) are affected
2. Read current content of all affected files
3. Apply the change to RESEARCH.md
4. Check ARCHITECTURE.md and STORYLINE.md for any text, tables, or diagrams that reference the same data — update if needed
5. Report what was changed and where
