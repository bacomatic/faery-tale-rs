# AGENTS.md

This file is the compact agent contract for this repository. Keep it stable and concise.

## Project constraints

- Rust port of "The Faery Tale Adventure" (1987 Amiga game by MicroIllusions).
- Personal learning project; PRs are not accepted.
- **Fidelity first**: be true to the original game. Do not add enhancements or modernizations.
- Fidelity means matching the documented original behavior and player experience, even if the implementation structure differs.
- Do not intentionally "fix" original behavior unless reproducing the original bug would require extra work.
- This repository is now **specification-driven**: rely on the checked-in reference docs in `docs/`, not on any historical source dump.

## Timing invariant (critical)

- **NTSC-only at 30 fps** for gameplay (the audio VBL interrupt is 60 Hz).
- There was no PAL release; treat all gameplay, audio, and animation timing as NTSC.

## Agent working rules

- **Always start with `docs/README.md`** — it is the entry point for all project documentation and lists exactly which docs are owned by this project (editable) versus read-only upstream reference material.
- **Always follow `docs/GUIDELINES.md`** when writing, reviewing, or refactoring Rust code in this repository. If there is any conflict, follow `AGENTS.md` and the project reference docs first.
- Make minimal, surgical changes consistent with existing code style.
- Prefer root-cause fixes over surface patches.
- Avoid unrelated refactors while touching gameplay-critical code.
- Prefer the reference documents over guesswork; if the docs disagree, align implementation with `docs/SPECIFICATION.md` and note the discrepancy.
- **Use test-driven development for new feature work**: start by writing tests that fail based on `docs/SPECIFICATION.md` and `docs/REQUIREMENTS.md`, then implement the code against those tests.
- **Do not modify tests just to make them pass** unless there is a strong project reason, such as a real change in the specification or requirements.
- Validate changed behavior with targeted commands/tests when feasible.
- Always call `font.set_color_mod(r, g, b)` before every `render_string()` call. The canonical white/default is `set_color_mod(255, 255, 255)`. SDL2 color mod is stateful; failing to reset it causes text to render in the previous scene's tint color.
- When creating a commit to fix a bug, add `Closes: #<issue>` on its own line at the end of the commit message (e.g. `Closes: #111`).
- **Fail Fast** If you cannot find an answer, say you don't know rather than guessing. This gives the user the opportunity to clarify or provide more information, and prevents implementation errors. Often this indicates a gap in the reference documentation that should be filled.
- **Do not invent player-facing strings.** Any message shown to the user must come from `faery.toml` (`[narr]` tables such as `event_msg`, `speeches`, `place_msg`, etc.) via `crate::game::events`. Never hardcode narrative prose in Rust code.

## Document ownership

The authoritative list lives in `docs/README.md`. Summary for quick reference:

- **Owned by this project (editable with user approval):** `docs/SPECIFICATION.md`, `docs/REQUIREMENTS.md`, `docs/DEBUG_SPECIFICATION.md`, `docs/GUIDELINES.md`.
- **READ-ONLY (maintained upstream; agents must never modify):** everything else under `docs/`, including `RESEARCH.md`, `ARCHITECTURE.md`, `STORYLINE.md`, `PROBLEMS.md`, and all files under `docs/logic/` and `docs/_discovery/`.
- Always ask the user before editing any owned doc; surface desired changes to read-only docs as discussion points only.

## Canonical sources by topic

- Entry point for all documentation: `docs/README.md`
- Build/run commands and developer setup: `README.md` (repo root)
- **Authoritative reference docs (READ-ONLY — agents must never modify; only the user may edit):** `docs/RESEARCH.md`, `docs/ARCHITECTURE.md`, `docs/STORYLINE.md`, `docs/PROBLEMS.md`, `docs/logic/**`, `docs/_discovery/**`
- **Project-owned docs (editable with user approval):** `docs/SPECIFICATION.md`, `docs/REQUIREMENTS.md`, `docs/DEBUG_SPECIFICATION.md`, `docs/GUIDELINES.md`
- Current Rust implementation details: source under `src/` (especially `src/main.rs` and `src/game/`)

## Game mechanics research order

When investigating any game mechanic (combat, movement, AI, timings, formulas, etc.):
1. **First**: check `docs/RESEARCH.md` — it is the authoritative source of truth for verified mechanics.
2. Use `docs/ARCHITECTURE.md` for subsystem structure/data flow and `docs/STORYLINE.md` for quest/scenario flow.
3. Use `docs/SPECIFICATION.md` to resolve implementation details and keep the port internally consistent.
4. Do not create competing source-of-truth documents unless the user explicitly requests it.

