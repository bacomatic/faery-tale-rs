# AGENTS.md

This file is the compact agent contract for this repository. Keep it stable and concise.

## Project constraints

- Rust port of "The Faery Tale Adventure" (1987 Amiga game by MicroIllusions).
- Personal learning project; PRs are not accepted.
- **Fidelity first**: be true to the original game. Do not add enhancements or modernizations.
- Do not intentionally "fix" original behavior unless reproducing the original bug would require extra work.
- Original C/ASM sources in `original/` are reference material and may be non-buildable.

## Timing invariant (critical)

- **NTSC-only at 30 fps** (interlaced frame rate; the audio VBL interrupt is 60 Hz but gameplay ticks at 30 Hz).
- There was no PAL release; treat all gameplay/audio/animation timing as NTSC.
- Ignore any PAL/50Hz comments in `original/` as incorrect for this project.

## Agent working rules

- Make minimal, surgical changes consistent with existing code style.
- Preserve original variable names/magic values when they reflect source behavior.
- Prefer root-cause fixes over surface patches.
- Avoid unrelated refactors while touching gameplay-critical code.
- Validate changed behavior with targeted commands/tests when feasible.
- Always call `font.set_color_mod(r, g, b)` before every `render_string()` call. The canonical white/default is `set_color_mod(255, 255, 255)`. SDL2 color mod is stateful; failing to reset it causes text to render in the previous scene's tint color.
- When creating a commit to fix a bug, add `Closes: #<issue>` on its own line at the end of the commit message (e.g. `Closes: #111`).

## Canonical sources by topic

- Build/run commands and developer setup: `README.md`
- Reverse-engineering/file formats (`songs`, `v6`, etc.): `RESEARCH.md`
- Roadmap/progress and task state: `PLAN.md` + `plan_status.toml`
- Current architecture and implementation details: source under `src/` (especially `src/main.rs` and `src/game/`)

## Planning files contract

- `PLAN.md` is the human-readable roadmap and progress log.
- `plan_status.toml` is the machine-readable task state used by agents.
- On task state changes, update both files in the same edit.
- Validate consistency after edits with `bash scripts/plan_sync_check.sh`.

## State update do/don't

- Do treat `plan_status.toml` + `PLAN.md` as canonical project state.
- Do update matching GitHub rollup issue references when task state meaningfully changes.
- Do run `bash scripts/plan_sync_check.sh` before finishing state edits.
- Don't update only one of the planning files when changing state.

## Issue tracking memory

- GitHub Issues are the live tracker for active rollup tasks (`*-001`).
- For completed work that predates issue tracking, keep `issue = "pre-issues"`.
- Do not invent/backfill synthetic issue numbers for historical completed tasks.
- Refresh/sync workflow:
	- `bash scripts/sync_plan_from_github.sh` (preferred one-liner)
	- or manually: `sync_rollup_issue_states.sh --strict-open` → `refresh_issue_map.sh` → `plan_sync_check.sh`

