---
description: "Use when editing documentation files — enforces source citation format, section numbering, cross-references, and single-source-of-truth rules for RESEARCH.md"
applyTo: "docs/**"
---
# Documentation Conventions

## Source Citations

- Format: `file.c:LINE` or `file.c:START-END` (e.g., `fmain.c:1609`, `narr.asm:251-347`)
- All file paths are relative to the repo root — no directory prefixes
- Speech references: `speak(N)` where N is the narr.asm message table index

## Section Numbering

- RESEARCH.md: `## N. Title` for top-level sections, `### N.M Subtitle` for subsections

## Cross-References

- Between docs: `[STORYLINE.md §5](STORYLINE.md#5-npc-dialogue-trees)`
- To sections within RESEARCH.md: `[§6 Inventory](RESEARCH.md#6-inventory--items)`
- Diagrams use Mermaid syntax (flowcharts, state diagrams, sequence diagrams)

## Single Source of Truth

RESEARCH.md is the single source of truth for game mechanics documentation. Edit it directly. There is no secondary file to keep in sync.

## Open Problems

When something cannot be determined from source code alone (magic numbers, platform-dependent behavior, gameplay intent vs. bugs), log it in `docs/PROBLEMS.md` using the entry template. Never guess or infer — file a problem for expert review.

## Read-Only Source

Never edit `.c`, `.asm`, `.h`, `.i`, `.p` files, `makefile`, `AztecC.Err`, `fta.br`, `notes`, or anything in `game/` or `ToArchive/`. These are original 1987 artifacts — read only to extract information for documentation.
