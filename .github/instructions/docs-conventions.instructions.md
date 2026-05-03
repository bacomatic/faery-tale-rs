---
description: "Use when editing documentation files — enforces source citation format, section numbering, cross-references, and single-source-of-truth rules for RESEARCH.md"
applyTo: "reference/**"
---
# Documentation Conventions

## Source Citations

- Format: `file.c:LINE` or `file.c:START-END` (e.g., `fmain.c:1609`, `narr.asm:251-347`)
- All file paths are relative to the repo root — no directory prefixes
- Speech references: `speak(N)` where N is the narr.asm message table index

## Section Numbering

- RESEARCH sub-documents: `## N. Title` for top-level sections, `### N.M Subtitle` for subsections
- Section numbers are preserved across the split (e.g., §17 is always "Main Game Loop" in RESEARCH-systems.md)

## Cross-References

- Between reference docs: `[STORYLINE.md §5](STORYLINE.md#5-npc-dialogue-trees)`
- To RESEARCH sections: `[§6 Terrain](RESEARCH.md#6-terrain--collision)` — the hub RESEARCH.md has anchor stubs that redirect to sub-documents. Direct links to sub-documents (e.g., `RESEARCH-terrain-combat.md#6-terrain--collision`) also work.
- Diagrams use Mermaid syntax (flowcharts, state diagrams, sequence diagrams)

## Single Source of Truth

The RESEARCH-*.md sub-documents are the single source of truth for game mechanics. RESEARCH.md is their hub/index with anchor stubs for backward compatibility. STORYLINE-*.md sub-documents are the single source of truth for narrative content, with STORYLINE.md as the hub.

## Open Problems

When something cannot be determined from source code alone (magic numbers, platform-dependent behavior, gameplay intent vs. bugs), log it in `reference/PROBLEMS.md` using the entry template. Never guess or infer — file a problem for expert review.

## Experiment Results

Files in `tools/results/` are transient (gitignored) and must **never** be linked from documentation. When an experiment produces findings relevant to a doc entry, inline the key results directly — include the reproduction command (`python tools/<script>.py`), a bullet summary of findings, and any data tables needed to support the conclusion. The reader must be able to understand the evidence without access to `tools/results/`.

## Read-Only Source

Never edit `.c`, `.asm`, `.h`, `.i`, `.p` files, `makefile`, `AztecC.Err`, `fta.br`, `notes`, or anything in `game/` or `ToArchive/`. These are original 1987 artifacts — read only to extract information for documentation.
