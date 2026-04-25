# AGENTS.md

This file is the compact agent contract for this repository. Keep it stable and concise.

## Project constraints

- Rust port of "The Faery Tale Adventure" (1987 Amiga game by MicroIllusions).
- Personal learning project; PRs are not accepted.
- **Fidelity first**: be true to the original game. Do not add enhancements or modernizations.
- Fidelity means matching the documented original behavior and player experience, even if the implementation structure differs.
- Do not intentionally "fix" original behavior unless reproducing the original bug would require extra work.
- This repository is now **specification-driven**: rely on the reference docs hosted on the `research` branch (see "Reference docs (remote)" below) and the owned specs in `docs/`, not on any historical source dump.

## Timing invariant (critical)

- **NTSC-only at 30 fps** for gameplay (the audio VBL interrupt is 60 Hz).
- There was no PAL release; treat all gameplay, audio, and animation timing as NTSC.

## Agent working rules

- **Always start by indexing `reference/README.md` from the research branch** (see "Reference docs (remote)" below). It is the entry point for all reference documentation.
- **Always follow `docs/GUIDELINES.md`** when writing, reviewing, or refactoring Rust code in this repository. If there is any conflict, follow `AGENTS.md` and the project reference docs first.
- Make minimal, surgical changes consistent with existing code style.
- Prefer root-cause fixes over surface patches.
- Avoid unrelated refactors while touching gameplay-critical code.
- Prefer the reference documents over guesswork; if the docs disagree, align implementation with `SPECIFICATION.md` and note the discrepancy.
- **Use test-driven development for new feature work**: start by writing tests that fail based on `SPECIFICATION.md` and `REQUIREMENTS.md`, then implement the code against those tests.
- **Do not modify tests just to make them pass** unless there is a strong project reason, such as a real change in the specification or requirements.
- Validate changed behavior with targeted commands/tests when feasible.
- Always call `font.set_color_mod(r, g, b)` before every `render_string()` call. The canonical white/default is `set_color_mod(255, 255, 255)`. SDL2 color mod is stateful; failing to reset it causes text to render in the previous scene's tint color.
- When creating a commit to fix a bug, add `Closes: #<issue>` on its own line at the end of the commit message (e.g. `Closes: #111`).
- **Fail Fast** If you cannot find an answer, say you don't know rather than guessing. This gives the user the opportunity to clarify or provide more information, and prevents implementation errors. Often this indicates a gap in the reference documentation that should be filled.
- **Do not invent player-facing strings.** Any message shown to the user must come from one of two authoritative sources: (1) `faery.toml` (`[narr]` tables such as `event_msg`, `speeches`, `place_msg`, `inside_msg`) via `crate::game::events`, or (2) the hardcoded string literals exhaustively enumerated in `reference/logic/dialog_system.md` on the research branch ("Hardcoded scroll messages — complete reference"). No other source of scroll-area text is permitted — never hardcode new narrative prose in Rust code. See `docs/SPECIFICATION.md` §23.6 and `docs/REQUIREMENTS.md` R-INTRO-012/013/014.

## Document ownership

- **Owned by this project (editable with user approval), checked in here:** `docs/SPECIFICATION.md`, `docs/REQUIREMENTS.md`, `docs/DEBUG_SPECIFICATION.md`, `docs/GUIDELINES.md`.
- **READ-ONLY reference docs (live on the `research` branch; agents must NEVER modify):** everything under `reference/` on that branch, including `RESEARCH.md`, `ARCHITECTURE.md`, `STORYLINE.md`, `PROBLEMS.md`, all files under `reference/logic/` and `reference/_discovery/`, plus `world_db.json`, `quest_db.json`, and the `region_*.png` / `overworld.png` map images.
- Always ask the user before editing any owned doc. Reference docs cannot be edited from this branch at all — surface desired changes to the user so they can update the `research` branch directly.

## Reference docs (remote)

Reference material lives on the `research` branch of this same repo. Agents fetch and index it on demand via `ctx_fetch_and_index`; it is not checked into the porting branches.

**Pinning policy:** track HEAD of `research` branch (always latest). Do not pin to commit SHAs unless the user explicitly requests it for a specific task.

**URL prefixes:**

- Raw (for indexing / fetching content):
  `https://raw.githubusercontent.com/bacomatic/faery-tale-rs/research/reference/`
- Browse (for human-readable links in markdown):
  `https://github.com/bacomatic/faery-tale-rs/blob/research/reference/`

**Standard fetch recipe (markdown / JSON):**

```
ctx_fetch_and_index(
  url: "https://raw.githubusercontent.com/bacomatic/faery-tale-rs/research/reference/<path>",
  source: "research:reference/<path>"
)
```
Then use `ctx_search` against the indexed content. For follow-ups across many docs, reuse the same `source:` label so results can be filtered cleanly.

**Binary assets (PNG region maps, `overworld.png`):** cannot be FTS-indexed. Fetch with `web_fetch` (raw URL) or `gh api` only when an image is genuinely needed.

**Reference doc inventory (paths under `reference/` on the research branch):**

- Top-level: `README.md`, `RESEARCH.md`, `ARCHITECTURE.md`, `STORYLINE.md`, `PROBLEMS.md`, `world_db.json`, `quest_db.json`, `overworld.png`, `region_0.png` … `region_9.png`
- `logic/`: `README.md`, `STYLE.md`, `SYMBOLS.md`, `messages.md`, `dialog_system.md`, `placard.md`, `magic.md`, `game-loop.md`, `input-handling.md`, `movement.md`, `terrain-collision.md`, `encounters.md`, `ai-system.md`, `combat.md`, `inventory.md`, `menu-system.md`, `doors.md`, `day-night.md`, `carrier-transport.md`, `astral-plane.md`, `quests.md`, `npc-dialogue.md`, `shops.md`, `save-load.md`, `brother-succession.md`, `frustration.md`, `visual-effects.md`
- `_discovery/`: raw trace artifacts (supporting context only, not authoritative). See `reference/README.md` on the research branch for the full list when needed.

When this list drifts (research branch adds/removes files), update it here in the same change that introduces a new dependency, or fetch `reference/README.md` from the research branch to re-derive it.

## Canonical sources by topic

- Entry point for reference documentation: `reference/README.md` on the `research` branch (fetch via the recipe above).
- Build/run commands and developer setup: `README.md` (repo root).
- **Authoritative reference docs (READ-ONLY, remote — see "Reference docs (remote)" above):** `reference/RESEARCH.md`, `reference/ARCHITECTURE.md`, `reference/STORYLINE.md`, `reference/PROBLEMS.md`, `reference/logic/**`, `reference/_discovery/**` on the `research` branch.
- **Project-owned docs (editable with user approval, local):** `docs/SPECIFICATION.md`, `docs/REQUIREMENTS.md`, `docs/DEBUG_SPECIFICATION.md`, `docs/GUIDELINES.md`.
- Current Rust implementation details: source under `src/` (especially `src/main.rs` and `src/game/`). Doc comments in `src/` that mention `reference/...` paths refer to files on the research branch — prepend the URL prefix above to view them.

## Game mechanics research order

When investigating any game mechanic (combat, movement, AI, timings, formulas, etc.):
1. **First**: index `reference/RESEARCH.md` from the research branch — it is the authoritative source of truth for verified mechanics.
2. Index `reference/ARCHITECTURE.md` for subsystem structure/data flow and `reference/STORYLINE.md` for quest/scenario flow.
3. Use `docs/SPECIFICATION.md` (local) to resolve implementation details and keep the port internally consistent.
4. Do not create competing source-of-truth documents unless the user explicitly requests it.

