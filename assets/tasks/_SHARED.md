# Shared conventions — read before any task

This file holds the rules common to every task so individual task files stay small.
Read **this file + your one task file**. Read [`../plan.md`](../plan.md) only if your task
file points you to a specific section.

## Producer rules
- Producer is **Python** under `tools/`. Never hardcode the sibling path. Honor
  `--game-dir` (default `../faery-tale-rs/game`) and `--src-dir` (default `src/`).
- **Pixel-/byte-exact conversion only.** No gameplay/engine/rendering/creative changes.
- Every new extractor ships with `pytest` cases under `tools/tests/`.
- Output goes under `assets/<subdir>/` per the plan's Output layout.
- JSON: stable key ordering, deterministic byte output (re-runs must be identical).
- Amiga color conversion: 12-bit `0x0RGB` → `rgba8` by nibble-replication (`0xF → 0xFF`).
- Transparency convention: sprite/tile **index 31** = transparent.
- Highlight mask: 1 bit/pixel, set where source palette index ∈ **16–24**; transparency
  follows index 31.

## Roles — IMPORTANT
Every task has two roles, performed by **two different agents**:
1. **Implementer** — does the "Implementation" section, runs the "Implementer self-check".
2. **Verifier** — a *separate* agent that performs the "Verification" section. The Verifier
   must **not** trust the Implementer's claims or reuse the Implementer's verification code.
   Re-derive results independently (fresh decode, fresh checksums, manual byte inspection,
   oracle diff), then report PASS/FAIL with evidence.

A task is **not done** until the Verifier reports PASS. If FAIL, the Verifier files specific
findings and the task returns to an Implementer.

## Rust oracle
Where a Rust decoder exists in the sibling `../faery-tale-rs` checkout
(`tile_atlas.rs`, `iff_image.rs`, `palette.rs`, `songs.rs`, `audio.rs`, `font.rs`,
`world_data.rs`), its output is the canonical reference. Verifiers diff Python output
against the Rust oracle (via a small Rust harness or existing tests).

## Python environment — IMPORTANT
This repo runs tools via the **`.toolenv` venv**, not system Python.
- Run tools/tests with `.toolenv/bin/python` (e.g. `.toolenv/bin/python -m pytest tools/tests/...`).
  Plain `pytest` is **not** on PATH.
- `tools/run.sh` auto-provisions `.toolenv` from `tools/requirements.txt` — add new deps there.
- The venv has **no `pip` binary**; install with **`uv`** (`uv pip install ...`), do not call `pip` directly.
- Pillow + numpy are already installed in `.toolenv`.

## Do not commit
Leave all changes staged/untracked for human review. No git commits, no attribution lines.
