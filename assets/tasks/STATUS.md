# Task status — Asset Extraction

Live progress tracker. **Read this first when resuming.** Update it on every state change.
See [`README.md`](README.md) for how to dispatch Implementer + Reviewer subagents, and
[`_SHARED.md`](_SHARED.md) for conventions and env notes.

States: `TODO` · `IN PROGRESS` · `REOPENED` (gaps vs current model) · `IMPLEMENTED (awaiting review)` · `DONE` (assets accepted) · `BLOCKED`

| Task | State | Deps | Notes |
|---|---|---|---|
| [T0.1](T0.1-scaffolding.md) Scaffolding & shared helpers | **DONE** ✅ | — | Verified PASS. `tools/asset_common.py` + tests (16 pass), `assets/` tree. |
| [T0.2](T0.2-carray-baseline.md) C-array baseline | **DONE** ✅ | — | Verified PASS. Extended `tools/extract_table.py` (N-D + char-literal parse, `--json`); `diroffs` fixture. |
| [T1.1](T1.1-palettes.md) Palettes | **REOPENED** | T0.1, T0.2 | JSON extracted (old-model PASS). Missing `palettes/verify.json` + swatch previews — see T1.5. |
| [T1.2](T1.2-tables.md) Gameplay tables | **REOPENED** | T0.2 | 11 tables extracted (old-model PASS). Missing `tables/verify.json` — see T1.5. |
| [T1.3](T1.3-item-quest.md) Item/quest fold-in | **REOPENED** | T0.1 | JSON extracted (old-model PASS). Missing T1.3 items in `tables/verify.json`; duplicate narrative text vs T1.4 — see T1.5. |
| [T1.4](T1.4-text.md) Narrative text | **TODO** | T0.1 | Deps met — ready. |
| [T1.5](T1.5-retrofit-done.md) Retrofit done tasks | **TODO** | T1.1–T1.3; Part C: T1.4 | Parts A–B ready now. |
| [T2.1](T2.1-sprites.md) Sprites | **TODO** | T0.1, T1.1 | Deps met — ready. |
| [T2.2](T2.2-tiles.md) Tile atlas | TODO | T0.1, T1.1, T1.2 | Deps met — ready. |
| [T2.3](T2.3-masks.md) Shadow/collision masks | TODO | T0.1 | |
| [T2.4](T2.4-screens.md) IFF screens | TODO | T0.1 | |
| [T2.5](T2.5-world.md) World data | TODO | T0.1 | |
| [T2.6](T2.6-music.md) Music + instruments | TODO | T0.1 | |
| [T2.7](T2.7-sfx.md) SFX | TODO | T0.1 | |
| [T2.8](T2.8-fonts.md) Fonts | TODO | T0.1 | |
| [T3.1](T3.1-shaders.md) Reference shaders + light-level renders | TODO | T2.1, T2.2 | GLSL reference-only (port validates); perception deliverable = `shaders/previews/` renders. |
| [T3.2](T3.2-formats.md) Format spec | TODO | Wave 1 + Wave 2 | |
| [T4.1](T4.1-manifest.md) Bundle index (manifest) | TODO | Wave 1 + Wave 2 | |
| [T4.2](T4.2-verification.md) Bundle acceptance (human) | TODO | T4.1 | |

## Log
- **T0.1** — Implemented by subagent (dir tree, `tools/asset_common.py`, `tools/tests/test_asset_common.py`,
  added pillow/numpy to `tools/requirements.txt`). Independently verified PASS by a separate subagent
  (hand-computed palette conversion, indexed-PNG round-trip, highlight-mask bits, JSON determinism, pytest 16/16).
  Surfaced env facts now recorded in `_SHARED.md`: use `.toolenv/bin/python`; install deps with `uv`.
- **T0.2** — Extended `tools/extract_table.py` (brace-aware N-D parse, char/hex/octal literals, `--json`
  deterministic output); added `tools/tests/test_extract_table.py` (18 pass) + `tools/tests/fixtures/diroffs.json`.
  Independently verified PASS: `diroffs` hand-transcribed from `src/fmain.c:1010` matches fixture exactly;
  `fallstates` (24 entries) extracted & hex-checked; synthetic N-D/char parsing confirmed.
- **Known unrelated failures:** `test_lint_logic::test_check_file_header_passes_on_valid_fixture` and
  `test_research_agent::TestConfig::test_default_values` (config default 60 vs expected 15) fail on a clean
  tree — **pre-existing**, not from this work. Worth fixing separately.

- **T1.1** — `tools/extract_palettes.py` emits `pagecolors/textcolors/introcolors/sun_colors/blackcolors/region_overrides`
  JSON ({index, rgb4, rgba8}). Verified PASS: `pagecolors` 0/16/24/31 hand-transcribed from `src/fmain2.c`;
  counts (32/20/32/53/32) confirmed; region overrides (4=0x0980, 9=0x0445, default 0x0bdf) hand-converted;
  nibble-replication `(r<<4)|r` == Python `*17` confirmed by hand; pytest 19/19; `.gitkeep` removed.
  Doc nit: `pagecolors`/`sun_colors` actually live in `src/fmain2.c` (not fmain.c); extractor scans both, so no impact.

- **T1.2** — `tools/extract_tables.py` emits all 11 gameplay tables to `assets/tables/`. Also improved
  `tools/extract_table.py`'s declaration matcher (whole-file, newline-tolerant, type-agnostic; first-def-wins) —
  T0.2 tests stay green. Verified PASS: counts exact; statelist/setfig_table/file_index first+last rows
  hand-transcribed; `enum obytes` resolved independently (RED_KEY=242, WHITE_KEY=154 confirmed in `src/fmain2.c`);
  file_index sectors (32 outdoor / 96 indoor) and 40-block image groups confirmed; determinism + `.gitkeep` removal OK.

- **T1.3** — `tools/fold_item_quest_tables.py` (thin adapter reusing `extract_item_effects.py` +
  `extract_quest_data.py`) emits `assets/tables/item_effects.json` + `quest_data.json`. Verified PASS:
  valid JSON, no name collision, item_effects `stuff[N]` citations spot-checked against `src/`, quest speeches/
  quest_items matched source, deterministic, pytest 7/7.
  **Finding handled:** the verifier found `quest_data`'s embedded `encounter_chart` copy contradicts
  `src/fmain.c:52-63` / the byte-exact T1.2 `encounter_chart.json` (a pre-existing error in
  `tools/extract_quest_data.py`). Fix: the fold tool now drops the redundant `encounter_chart`/`setfig_table`
  copies so T1.2 remains the single source of truth.
- **Follow-up (resolved):** `tools/extract_quest_data.py` ENCOUNTER_CHART rows 0–4 (Ogre/Orc/Wraith/Skeleton/Snake)
  hand-transcribed wrong vs `src/fmain.c:52-63`. Corrected the literals in place; rows 5–10 were already correct.
  Regenerated `reference/quest_db.json` (`--validate` passes; only the 5 rows + timestamp changed).
  `assets/tables/quest_data.json` unaffected — T1.3's fold tool already drops the redundant `encounter_chart` copy.

- **Plan review (2026-08-30)** — in-depth review against source; fixes applied to `plan.md` + tasks:
  - **T2.3**: masks are **192** entries (not 256) — `SHADOW_SZ`=12,288 B (`fmain.c:642`), ADF
    blocks 896–919 (`fmain.c:1222`, `mtrack.c` diskmap[21–23]); block location added to the task.
  - **T2.4**: enumerated all **9** ILBM screens (`p1a p1b p2a p2b p3a p3b page0 winpic hiscreen`);
    dimensions vary — take from `BMHD` (`hiscreen` is 640×57).
  - **T2.7**: no fixed SFX rate exists — playback period is randomized per trigger
    (`fmain.c:3617-3619`, call sites `fmain2.c:238-241`, `fmain.c:1488/1680/1690/2262`); samples
    are 4-byte-BE-length-prefixed (`fmain.c:1033-1041`). WAVs get base-period rates + new `sfx.json`
    with per-trigger period expressions.
  - **T1.4**: scope extended to `_question` (8), `_placard_text` (20), `_place_tbl` (29),
    `_inside_tbl` (37) for bundle self-sufficiency; parsing rules added for empty-string entries
    (`dc.b 0` slots at `place_msg[0..1]`/`inside_msg[0..1]`) and `;` comments (trap: `narr.asm:197`).
  - **T2.1/T2.2**: now consume the accepted `assets/palettes/` + `assets/tables/file_index.json`
    instead of re-reading `src/` (T1.1/T1.2 added as deps; both already DONE).
  - **T2.8**: there are **two** game fonts — `Amber/9` (`LoadSeg("fonts/Amber/9")`, `fmain.c:774`)
    and topaz-8, which the game opens from **ROM** (`OpenFont(&topaz_ta)`, `fmain.c:650,778`);
    `Topaz/8` copied in from the sibling `faery-tale-rs/game/fonts` as a stand-in for the ROM font
    (user-provided; amber files verified identical between checkouts; unused `Topaz/9e` not kept).
    `.font` headers are never opened by the game — extract only `amber_9`, `topaz_8`.

- **Reviewer model finalized (2026-08-30)** — the Reviewer is **the human, verifying final assets
  by perception only** (look/listen/glance); no re-extraction, hand-decoding, or verification
  code. Re-derivation steps moved into the (optional) Implementer self-checks. New **previews**
  rule for non-perceivable assets: T2.5 emits region-map render PNGs, T2.6 emits rough audio
  renders per track — non-authoritative, but **shipped** (useful to porting efforts too).
  T3.1 verification clarified: GLSL is verified via the Python reference (`fade_page.py`/
  `compare.py`), never by building a GPU harness. Anything perception misses is caught during
  port implementation. Updated: `_SHARED.md`, `README.md`, `plan.md`, T1.4, T2.1–T2.8, T3.1,
  T3.2, T4.1, T4.2 (completed tasks T0.x/T1.1–T1.3 left as historical record).

- **T3.1 rescoped + site planned (2026-08-30)** — T3.1 GLSL is now **reference-only, validated by
  the port**; its perception deliverable is broad **light-level reference renders**
  (`assets/shaders/previews/`: all region atlases + several actor sheets × light levels +
  moonlight/green-jewel, via the Python `fade_page()` port). Separately, a **static docs+asset
  browser site** (MkDocs Material) was decided and planned in [`site/PLAN.md`](../../site/PLAN.md)
  — 5 small phases, deps already installed in `.toolenv` (`mkdocs-material` added to
  `tools/requirements.txt`). Built site will be committed.

- **Done-task audit (2026-09-26)** — re-checked T0.1–T1.3 against the perception/VERIFY/previews
  model. T0.1/T0.2 ship nothing (tooling) → stay DONE; only their task-file verification text is stale.
  T1.1–T1.3 → REOPENED: no `VERIFY.md` anywhere, no palette swatch previews, stale citations
  (`pagecolors`/`sun_colors` are in `fmain2.c`), `blackcolors.json` undocumented, and
  `quest_data.json` duplicates T1.4's speeches/placards/riddles. All gaps tracked in **T1.5**.

- **Review app + `verify.json` convention (2026-09-26)** — `VERIFY.md` files are replaced by
  per-subdir `verify.json` data (items tagged by task ID) rendered by a local review app under
  `tools/review/` (FastAPI + React/Vite/TS; plan in [`tools/review/PLAN.md`](../../tools/review/PLAN.md)).
  Verdicts are appended to `assets/tasks/review_results.json` (with file hashes for staleness); drafts
  go to gitignored `tools/review/.drafts/`. No top-level `assets/VERIFY.md`; `verify.json` files are
  listed in the manifest. Phase 0 (docs switch) done: `plan.md`, `_SHARED.md`, `README.md`, every
  task file (T1.1–T4.2, T1.5). T3.1/T3.2/T4.1/T4.2 gained `verify.json` items so they can be reviewed
  in the app (root-level items live in `assets/verify.json`). Next: Phases 1–3 (backend, frontend,
  viewers); T1.5's human review needs them.

## Next
Wave 0 complete. T1.1–T1.3 reopened → **T1.5** Parts A–B (ready now). Remaining Wave 1: **T1.4**
(narrative text, scope extended — see Plan review log entry), then T1.5 Part C. Wave 2 (T2.1–T2.8) unblocked. Side track: build the browser site in small
bursts per [`site/PLAN.md`](../../site/PLAN.md) (Phase 1 next). **Everything above is uncommitted**
— commit (with user consent) before dispatching subagents.
