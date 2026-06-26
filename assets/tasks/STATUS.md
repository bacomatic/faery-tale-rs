# Task status — Asset Pipeline

Live progress tracker. **Read this first when resuming.** Update it on every state change.
See [`README.md`](README.md) for how to dispatch Implementer + Verifier subagents, and
[`_SHARED.md`](_SHARED.md) for conventions and env notes.

States: `TODO` · `IN PROGRESS` · `IMPLEMENTED (awaiting verify)` · `DONE` (verified PASS) · `BLOCKED`

| Task | State | Deps | Notes |
|---|---|---|---|
| [T0.1](T0.1-scaffolding.md) Scaffolding & shared helpers | **DONE** ✅ | — | Verified PASS. `tools/asset_common.py` + tests (16 pass), `assets/` tree, `game` symlink (gitignored). |
| [T0.2](T0.2-carray-baseline.md) C-array baseline | **DONE** ✅ | — | Verified PASS. Extended `tools/extract_table.py` (N-D + char-literal parse, `--json`); `diroffs` fixture. |
| [T1.1](T1.1-palettes.md) Palettes | **DONE** ✅ | T0.1, T0.2 | Verified PASS. `tools/extract_palettes.py` → 6 palette JSONs; Rust `palette.rs` oracle confirmed. |
| [T1.2](T1.2-tables.md) Gameplay tables | **DONE** ✅ | T0.2 | Verified PASS. 11 tables → `assets/tables/`; enum/symbol resolution confirmed; `extract_table.py` matcher improved (newline-tolerant). |
| [T1.3](T1.3-item-quest.md) Item/quest fold-in | **TODO** | T0.1 | Deps met — ready. |
| [T1.4](T1.4-text.md) Narrative text | **TODO** | T0.1 | Deps met — ready. |
| [T2.1](T2.1-sprites.md) Sprites | **TODO** | T0.1 | Deps met — ready. |
| [T2.2](T2.2-tiles.md) Tile atlas | TODO | T0.1 | |
| [T2.3](T2.3-masks.md) Shadow/collision masks | TODO | T0.1 | |
| [T2.4](T2.4-screens.md) IFF screens | TODO | T0.1 | |
| [T2.5](T2.5-world.md) World data | TODO | T0.1 | |
| [T2.6](T2.6-music.md) Music + instruments | TODO | T0.1 | |
| [T2.7](T2.7-sfx.md) SFX | TODO | T0.1 | |
| [T2.8](T2.8-fonts.md) Fonts | TODO | T0.1 | |
| [T3.1](T3.1-shaders.md) Reference shaders | TODO | T2.1, T2.2 | |
| [T3.2](T3.2-formats.md) Format spec | TODO | Wave 1 + Wave 2 | |
| [T4.1](T4.1-manifest.md) Manifest + driver | TODO | Wave 1 + Wave 2 | |
| [T4.2](T4.2-verification.md) Verification harness | TODO | T4.1 | |

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
  JSON ({index, amiga12, rgba8}). Verified PASS: `pagecolors` 0/16/24/31 hand-transcribed from `src/fmain2.c`;
  counts (32/20/32/53/32) confirmed; region overrides (4=0x0980, 9=0x0445, default 0x0bdf) hand-converted;
  Rust `palette.rs` `(r<<4)|r` == Python `*17` nibble-replication; pytest 19/19; `.gitkeep` removed.
  Doc nit: `pagecolors`/`sun_colors` actually live in `src/fmain2.c` (not fmain.c); extractor scans both, so no impact.

- **T1.2** — `tools/extract_tables.py` emits all 11 gameplay tables to `assets/tables/`. Also improved
  `tools/extract_table.py`'s declaration matcher (whole-file, newline-tolerant, type-agnostic; first-def-wins) —
  T0.2 tests stay green. Verified PASS: counts exact; statelist/setfig_table/file_index first+last rows
  hand-transcribed; `enum obytes` resolved independently (RED_KEY=242, WHITE_KEY=154 confirmed in `src/fmain2.c`);
  file_index sectors (32 outdoor / 96 indoor) and 40-block image groups confirmed; determinism + `.gitkeep` removal OK.

## Next
Wave 0, T1.1, T1.2 complete. Remaining Wave 1: T1.3, T1.4. Wave 2 (T2.1–T2.8) unblocked.
