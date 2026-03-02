# AGENTS.md

This file is the compact agent contract for this repository. Keep it stable and concise.

## Project constraints

- Rust port of "The Faery Tale Adventure" (1987 Amiga game by MicroIllusions).
- Personal learning project; PRs are not accepted.
- **Fidelity first**: be true to the original game. Do not add enhancements or modernizations.
- Do not intentionally "fix" original behavior unless reproducing the original bug would require extra work.
- Original C/ASM sources in `original/` are reference material and may be non-buildable.

## Timing invariant (critical)

- **NTSC-only at 60 Hz**.
- There was no PAL release; treat all gameplay/audio/animation timing as NTSC.
- Ignore any PAL/50Hz comments in `original/` as incorrect for this project.

## Agent working rules

- Make minimal, surgical changes consistent with existing code style.
- Preserve original variable names/magic values when they reflect source behavior.
- Prefer root-cause fixes over surface patches.
- Avoid unrelated refactors while touching gameplay-critical code.
- Validate changed behavior with targeted commands/tests when feasible.

## Canonical sources by topic

- Build/run commands and developer setup: `README.md`
- Reverse-engineering/file formats (`songs`, `v6`, etc.): `DECODE.md`
- Roadmap/progress and task state: `PLAN.md` + `plan_status.toml`
- Current architecture and implementation details: source under `src/` (especially `src/main.rs` and `src/game/`)

## Planning files contract

- `PLAN.md` is the human-readable roadmap and progress log.
- `plan_status.toml` is the machine-readable task state used by agents.
- On task state changes, update both files in the same edit.
- Validate consistency after edits with `bash scripts/plan_sync_check.sh`.

