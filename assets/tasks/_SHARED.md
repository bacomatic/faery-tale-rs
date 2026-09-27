# Shared conventions — read before any task

This file holds the rules common to every task so individual task files stay small.
Read **this file + your one task file**. Read [`../plan.md`](../plan.md) only if your task
file points you to a specific section.

## The product is the assets, not the tools
This is a **one-time extraction**. The deliverable is the committed `assets/` bundle. Any script
you write under `tools/` is a **byproduct kept for provenance** — it is not maintained, is not
required to re-run, and its repeatability is **not** an acceptance criterion. Do not build
orchestration/drivers/regeneration harnesses for their own sake.

**Agents extract; scripts only decode.** Read the source and write the data yourself, citing every
value (`file:line`) and re-reading each citation. Write a script only when the input is **binary**
(bitplanes, samples, map/sector files) or **too large to transcribe reliably** (e.g. an 87-row C
array). A script must genuinely parse the original input — never embed hand-typed data in one. The
review app's citations are the check either way.

## Producer rules
- Scripts (where the rule above allows one) are **Python** under `tools/`. Never hardcode
  absolute or external paths. Honor `--game-dir` (default `src/assets`) and `--src-dir` (default
  `src/`). All original data lives in-repo under `src/`.
- **Sensory content only, converted losslessly** (pixels, colors, samples, glyphs, text). No
  gameplay/engine/rendering/creative changes. Behavior and its constants are not assets: they are
  specified in `reference/logic/` (see `plan.md`, "Behavior is not an asset").
- Output goes under `assets/<subdir>/` per the plan's Output layout — the emitted files are the
  real output.
- **Ship manual verification instructions as data.** Each task adds its items to a
  `verify.json` in its `assets/` subdir (one file per subdir; items tagged with the task ID;
  schema in [`tools/review/PLAN.md`](../../tools/review/PLAN.md)). The review app renders them for
  the human reviewer. Content must suit the asset type: for **images**, pair each file with a short
  `look_for` description **derived from source/reference**; for **JSON tables/text**, expected
  `counts` + spot values with source `citations`; for **audio**, expected counts + a one-line
  “what you should hear” cue; for **palettes**, a few `rgb4`→`rgba8` values with citations.
  `verify.json` may be hand-written or emitted by the extractor; the app validates it either way.
  It ships with the bundle and is listed in the manifest.
- `pytest` cases under `tools/tests/` are **optional aids** the implementer may add while
  extracting; they are not a deliverable and not the acceptance gate.
- **`.gitkeep` cleanup:** the `assets/` subdirs ship with `.gitkeep` placeholders. When your
  task writes real files into a directory, **`git rm` that directory's `.gitkeep`** in the same
  change — a `.gitkeep` must exist only in dirs that are still empty. The Reviewer confirms no
  `.gitkeep` remains alongside real output.
- JSON: stable key ordering and clean, readable byte output. (Determinism across re-runs is a
  nicety, not a requirement — the shipped file is what matters, not its reproducibility.)
- Color conversion: `rgb4` (the Amiga OCS 12-bit `0x0RGB` value, 4 bits/channel) → `rgba8`
  by nibble-replication (`0xF → 0xFF`). The palette JSON key is `rgb4`; the helper is
  `asset_common.rgb4_to_rgba8`.
- Transparency convention: sprite/tile **index 31** = transparent.
- Highlight mask: 1 bit/pixel, set where source palette index ∈ **16–24**; transparency
  follows index 31.

## Roles — IMPORTANT
Verification is **human-in-the-loop**. Every task has two roles:
1. **Implementer** (agent) — does the "Implementation" section, produces the assets, and authors
   the task's `verify.json` items. The "Implementer self-check" (round-trips, spot decodes, regression
   diffs) is an **optional aid** run while extracting — not a gate.
2. **Reviewer** (the human) — verifies the **final** assets **by perception only**: open the
   images, play the audio, eyeball tables/JSON (against a cited source line only when it's a
   quick glance), working through the task's page in the **review app** (`tools/review/`) and
   marking each item OK/Problem. The Reviewer never re-extracts, hand-decodes, or writes code to
   verify. `verify.json` items must be written for this: minutes per task, file ↔ "what you
   should see/hear", stated expected counts. Verdicts are appended to
   `assets/tasks/review_results.json`.

Anything perception misses will surface during port implementation and can be revisited then —
this is a labor of love, not a AAA production gate. A task is **done** when the Reviewer accepts
the assets; on REJECT the findings go back to an Implementer.

## Previews (for non-perceivable assets)
Where an asset can't be judged by looking at its JSON (music event streams, world/terra grids,
envelopes), the Implementer also emits **preview artifacts** — e.g. a rendered audio preview per
track, a colored PNG render of each region map — under the resource's `assets/<subdir>/previews/`.
Previews are **non-authoritative convenience artifacts**: they ship with the bundle (they are
useful to porting efforts too, human ones especially), but the extracted data remains the source
of truth. List them in the manifest like any other shipped file.

## Python environment — IMPORTANT
This repo runs tools via the **`.toolenv` venv**, not system Python.
- Run tools/tests with `.toolenv/bin/python` (e.g. `.toolenv/bin/python -m pytest tools/tests/...`).
  Plain `pytest` is **not** on PATH.
- `tools/run.sh` auto-provisions `.toolenv` from `tools/requirements.txt` — add new deps there.
- The venv has **no `pip` binary**; install with **`uv`** (`uv pip install ...`), do not call `pip` directly.
- Pillow + numpy are already installed in `.toolenv`.

## Do not commit
Leave all changes staged/untracked for human review. No git commits, no attribution lines.
