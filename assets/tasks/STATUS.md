# Task status — Asset Extraction

Live progress tracker. **Read this first when resuming.** Update it on every state change.
See [`README.md`](README.md) for how to dispatch Implementer + Reviewer subagents, and
[`_SHARED.md`](_SHARED.md) for conventions and env notes.

States: `TODO` · `IN PROGRESS` · `REOPENED` (gaps vs current model) · `IMPLEMENTED (awaiting review)` · `DONE` (assets accepted) · `BLOCKED`

| Task | State | Deps | Notes |
|---|---|---|---|
| [T0.1](T0.1-scaffolding.md) Scaffolding & shared helpers | **DONE** ✅ | — | Verified PASS. `tools/asset_common.py` + tests (16 pass), `assets/` tree. |
| [T0.2](T0.2-carray-baseline.md) C-array baseline | **DONE** ✅ | — | Verified PASS. Extended `tools/extract_table.py` (N-D + char-literal parse, `--json`); `diroffs` fixture. |
| [T1.1](T1.1-palettes.md) Palettes | **DONE** ✅ | T0.1, T0.2 | ACCEPTED 2026-09-29 (6/6 OK). T1.5 A–B: `palettes/verify.json`, region-9 `secret_timer` variant added (swatch previews later removed). |
| [T1.2](T1.2-tables.md) Gameplay tables | **DROPPED** | — | Not assets: constants stay in `reference/`, art bindings go into T2.1, `file_index` is used by T2.2/T2.5. `assets/tables/` deleted. |
| T1.3 Item/quest data | **DROPPED** | — | Behavior, not an asset: already spec'd in `reference/logic/magic.md`, `menu-system.md`, `brother-succession.md`. |
| [T1.4](T1.4-text.md) Narrative text | **TODO** | T0.1 | Deps met — ready. |
| [T1.5](T1.5-retrofit-done.md) Retrofit done tasks | **DONE** ✅ | T1.1 | Accepted with T1.1 (2026-09-29). Parts A–B done; Part C cancelled. T1.2 part moot (dropped). Reviewed via the T1.1 page. |
| [T2.1](T2.1-sprites.md) Sprites | **DONE** ✅ | T0.1, T1.1 | ACCEPTED 2026-09-29 (round 3, 41/41 OK). Per-actor sets, objects, 40 item PNGs, weapon/effect sheets, raw sheets; `sprites/verify.json` (41 items). |
| [T2.2](T2.2-tiles.md) Tile atlas | TODO | T0.1, T1.1 | Deps met — ready. |
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
- **Known unrelated failures (resolved 2026-09-26):** `test_lint_logic::test_check_file_header_passes_on_valid_fixture`
  (the linter looked for sources at the repo root instead of `src/`; fixed) and
  `test_research_agent::TestConfig::test_default_values` (deleted with `research_agent/`).

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

- **T1.5 Parts A–B (2026-09-26)** — done in-session (no subagent). Part A: T0.1/T0.2 get no Reviewer
  step; T1.1–T1.3 docs fixed (citations, outputs, perception-style verification); `blackcolors` +
  `previews/` added to the plan layout. Part B: `tools/render_palette_previews.py` →
  `assets/palettes/previews/` (8 PNGs, pixel-checked against the JSON); `palettes/verify.json` (6 items);
  `tables/verify.json` (11 T1.2 + 2 T1.3 items); `SEMANTICS` ranges fixed (only `semantics` lines changed
  on regeneration). The review app reports 0 validation problems and every count matches.
  **New findings:** (1) `region_overrides.json` was missing region 9's `secret_timer` color-31 value
  `0x00f0` (`fmain2.c:383`). It is now emitted under `conditional_regions` (extractor + test updated).
  (2) `quest_data.json` `events` (39) also duplicates T1.4's `event_msg` and was added to Part C.

- **T1.3 redesign (2026-09-26)** — the user flagged T1.3 as confusing and its MAGIC data as broken.
  Investigation: `extract_quest_data.py` reads no source (hand-typed constants). `magic_effects` gets
  3 of 7 cases wrong (Vial = HEAL, Skull = kill spell, Stone mis-cited) and misses the case-5→7
  fall-through and the no-charge `return`s (`fmain.c:3301-3366`); `inventory[34]` has the wrong gold
  name. User decision: ship only source-extracted data. The plan is written in the T1.3 task file
  (no code changed). Open question: keep or drop `item_effects.json`. It also makes T1.5 Part C
  unnecessary. `reference/RESEARCH-items-world.md:43-49` citations are stale (fix is part of the plan).

- **Agent-first rule + tools cleanup (2026-09-26)** — user decision, applied globally (`AGENTS.md`,
  `_SHARED.md`, `plan.md`, `docs/tools-conventions.md`): **agents extract; scripts only decode**
  (binary or too-large input, and they must genuinely parse the original). Deleted: `extract_quest_data.py`,
  `extract_item_effects.py`, `fold_item_quest_tables.py` (+test), `verify_cycle_overflow.py` (unreferenced),
  `render_palette_previews.py` + `assets/palettes/previews/` (the app's swatch view covers them),
  `research_agent/` (+test, `.env.example`, `docs/agents/research.md`, its 4 deps), and
  `assets/tables/{quest_data,item_effects}.json` with their `verify.json` items. Kept: the cheat tools.
  T1.3 was rewritten as agent-authored. T1.5 Part C was cancelled. `reference/quest_db.json`'s wrong `magic_effects` and "20 Gold Pieces"
  (source: "100 Gold Pieces", `fmain.c:418`) were fixed 2026-09-26.

- **"Behavior is not an asset" (2026-09-26)** — user decision: the goal is a new game that looks and feels
  the same, so `assets/` holds sensory content only (`plan.md`, `_SHARED.md`). **T1.3 dropped** (item
  effects, menus and brother stats are already in `reference/logic/magic.md`, `menu-system.md`,
  `brother-succession.md`). **T1.2 dropped**, `assets/tables/` deleted; the split is recorded in the T1.2
  task file, and `tools/extract_tables.py` now writes temporary tables to `tools/results/tables/`.
  Reference fixes: `RESEARCH-items-world.md` MAGIC citations corrected; `logic/magic.md` corrected (an
  on-tile Blue Stone with no matching stone still heals and is consumed, `fmain.c:3331-3348`).
  `validate_citations.py` and `lint_logic.py` fixed to read `src/` (5588/5588 citations valid; lint clean).

- **T2.1 (2026-09-27)** — done in-session (no subagent). `tools/extract_sprites.py` was rewritten: it reads `cfiles`
  from `fmain2.c` (the hand-typed CFILES/palette are gone) and the palette from the accepted `pagecolors.json`.
  It writes indexed frames, a sheet, a 16-24 highlight mask and an atlas JSON per actor, with table-derived
  `animations`/`facings`/`bow_walk_offsets` (statelist/diroffs/bow_x/bow_y) and, for OBJECTS, `inventory_icons`
  (inv_list) and `world_object_ids` (enum obytes + itrans). Hand-authored, cited `bindings` (user chose full
  semantics) cover motion-state -> state_index, weapon overlays, per-race rules for the ENEMY sheets (parity,
  wraith glide, snake +0x24, Loraii, DKnight), brother falls, SETFIG NPC frames, carriers, dragon, raft and OBJECTS
  effects. The script keeps `bindings` when it reruns. Self-check: RGBA output is pixel-identical to
  `sprite_output/` (661 files); indexed round-trip, mask and rect tests pass (`test_extract_sprites.py`, 9 tests);
  all 185 JSON citation ranges resolve; the review app reports 0 problems and 0 count failures.
  **Findings:** (1) `cfiles[12]` duplicates Julian's blocks and is never loaded, so it is skipped (user decision).
  (2) OBJECTS `numblocks` 36 covers 18,432 of 18,560 bytes, so planes 1-4 of frame 115 are never loaded (the
  frame decodes as noise). (3) Out-of-range or unexpected frame indices (hero fall at tactic 15; noble/sorceress
  dying; guard/bartender dying art) are logged as `reference/PROBLEMS.md` P25. (4) The hero's early fall frames
  come from the necromancer ENEMY sheet (falls happen only at xtype 52, where file 9 is loaded).

- **T2.1 revision (2026-09-27)** — user asked for one sprite set per actor plus an animation player in the review app.
  Decisions (asked): repack per actor with resolved frame lists and per-frame origin; ship highlight and silhouette
  masks (silhouette = `make_mask`, `fsubs.asm:1619-1653`); weapons = what the game can equip (enemies: `weapon_probs`
  row, `fmain.c:2757-2758`). `extract_sprites.py` now applies the cited render rules in code (parity, wraith glide,
  snake +0x24, Loraii slots, DKnight index 1, fight transitions with the state 6/7 -> 8 remap, dying order, bow/wand/hand
  overlays with draw order, hero fall runs), replacing the hand-written `bindings`. Only `objects.json` keeps its
  hand-written `bindings`. `asset_common.write_bit_mask` was factored out of the highlight writer. Review app: new
  `sprite` view kind (`models.py`, schema regenerated) and `SpriteViewer.tsx`. Checks: all 545 actor frames are
  pixel-identical to their origin in `sprite_output/`; `test_extract_sprites.py` checks masks, rects, modes and overlay
  rules; full suite passes (158); validate and lint are clean; the review store reports 0 problems, 0 count failures and
  793/793 files covered. **Correction:** the hero-fall item in P25 was wrong (`tactic++` runs before the draw, so step 3
  is drawn from OBJECTS); P25 was rewritten, and princess/king indexing and salamander->snake mixing were added. The
  running review server needs a restart to accept the `sprite` view.

- **T2.1 review round 1 (REJECT, 2026-09-27) processed.** Traced each note in the source:
  - Heroes: the head shake is statelist 84/85 through `frustflag` (`fmain.c:1654-1658`), not OSCIL. The note is
    reworded, and Frustrated plays shake x5, then statelist 40. The arrow shown when dying with the bow is correct
    (hand-weapon rule, k = 0, wpn_no 0, `fmain.c:2422-2444`). Wand while dying is 103 + facing for every brother,
    so the JSON is identical and any difference comes from the facing picked.
  - Necromancer: wand only, so no melee (`fmain.c:2164-2166`). Dying ends by turning into the Woodcutter
    (`fmain.c:1747-1755`), so the Phil-fall DEAD frame is gone. Woodcutter: no fight (weapon 0 turns it
    CONFUSED, `fmain.c:2151-2152`), so no Loraii or fall frames remain.
  - Weapons: bow/wand holders never melee. Each mode lists its own weapons, including 0 = none (weapon 0/8 draws no
    overlay). The viewer now shows only those.
  - Salamander removed: race 5 is never spawned (P25). Guard merged into one set with front and back standing
    frames. Royals have no dying/dead (pax extents, review).
  - Bartender: all 8 frames, Dying = 6 and Dead = 7 by art (user choice A). The source's 2/3 is recorded as a bug
    in P25 (user, 2026-09-28). Landed swan moved from raft to bird; raft is 1 frame.
  - Objects: `objects/items/*.png`, one per item, with half-frames split (40 items); `objects.json` `items`.
  - Checks: sprite tests 10/10 (new review tests written first). Review store: 0 problems, 0 count failures,
    786/786 files covered.
  - Added `assets/sprites/raw/` (user request): 17 raw cfile sheets in original frame order plus `raw.json`, with
    the `raw_sheets` review item. The test checks every frame against `decode_frames`. Covered 804/804.
  - Added `objects/weapons/{dirk,mace,sword,bow,wand}/` and `objects/effects/{arrow,fireball,bubbles}/` (user
    request). Weapon sheets hold every overlay frame the actor sets draw (with `used_by`); effects come from
    `fmain.c:2319-2322` and `2492-2497`. Undrawn rows are blanked. There are 8 new review items, and the review app
    can now expand truncated file lists. 41 items, 923/923 covered.
  - Review round 2 (REJECT, 38/41 OK), fixed:
    - necromancer: Woodcutter frame dropped from Dying; it has its own set (22 frames).
    - object_items: ob 102 is renamed Turtle eggs (`fmain.c:3170-3171`, `fmain2.c:1284`).
    - weapon_bow: only bow art is kept (30, 80-87). The hand-weapon frames 0/10/12 stay in the hero JSON only.
    - Checks: 164 tests pass; 916/916 covered.
  - **Review round 3: ACCEPT** (2026-09-29T04:21:45Z, 41/41 OK, "Looks good"). The review app now carries
    unchanged OK items over between rounds and sends `/files/` with `Cache-Control: no-cache`.

## Next
Wave 0 complete. T2.1, T1.1 and T1.5 accepted (2026-09-29). Remaining
Wave 1: **T1.4** (narrative text, scope extended — see Plan review log entry). Wave 2 (T2.1–T2.8) unblocked. Side track: build the browser site in small bursts per
[`site/PLAN.md`](../../site/PLAN.md) (Phase 1 next).
