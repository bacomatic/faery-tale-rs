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
| [T1.4](T1.4-text.md) Narrative text | **DONE** ✅ | T0.1 | ACCEPTED 2026-10-02 (round 2, by the user in-session). 8 JSON files in `assets/text/` (39/27/23/61 + 8/20/29/37), 12 placard card previews, `text/verify.json` (8 items). |
| [T1.5](T1.5-retrofit-done.md) Retrofit done tasks | **DONE** ✅ | T1.1 | Accepted with T1.1 (2026-09-29). Parts A–B done; Part C cancelled. T1.2 part moot (dropped). Reviewed via the T1.1 page. |
| [T2.1](T2.1-sprites.md) Sprites | **DONE** ✅ | T0.1, T1.1 | ACCEPTED 2026-09-29 (round 3, 41/41 OK). Per-actor sets, objects, 40 item PNGs, weapon/effect sheets, raw sheets; `sprites/verify.json` (41 items). |
| [T2.2](T2.2-tiles.md) Tile atlas | **DONE** ✅ | T0.1, T1.1 | ACCEPTED 2026-09-30 (10/10 OK). 10 regions × (indexed/RGBA/highlight/shadow atlas + `tiles.json`); tiles are **16×32**. |
| [T2.2.1](T2.2.1-master-atlas.md) Master tile atlas | **DONE** ✅ | T2.2, T2.5 | ACCEPTED 2026-09-30 (2/2 OK). Added during T2.2 review. 955 unique (art, colour 31, shadow mask) tiles + 2 secret-timer variants (512×960); shadow-mask atlas alongside + per-region reference maps in `master.json`. |
| [T2.3](T2.3-masks.md) Shadow/collision masks | **DONE** ✅ | T0.1 | ACCEPTED 2026-09-30 (1/1 OK). 192 PNGs + sheet + `masks.json`. |
| [T2.4](T2.4-screens.md) IFF screens | **DONE** ✅ | T0.1 | ACCEPTED 2026-09-30. 9 PNGs + `screens.json`. |
| [T2.5](T2.5-world.md) World maps | **DONE** ✅ | T0.1, T2.2, T2.2.1 | ACCEPTED 2026-10-02 (round 2, 11/11 OK). Redesigned 2026-09-30: `assets/maps/` — overworld + 62 interiors + 5 dungeons + astral plane as index-layer PNGs with previews; `assets/world/` removed. |
| [T2.6](T2.6-music.md) Music + instruments | **DONE** ✅ | T0.1 | ACCEPTED 2026-10-02. 28 tracks, `format.json`, waveforms/envelopes, 35 WAV previews. |
| [T2.7](T2.7-sfx.md) SFX | **DONE** ✅ | T0.1 | ACCEPTED 2026-10-02. 6 byte-exact WAVs + `sfx.json` + 7 playable 44.1 kHz previews (effect 5's 1989 Hz header is below what browsers play). |
| [T2.8](T2.8-fonts.md) Fonts | **DONE** ✅ | T0.1 | ACCEPTED 2026-10-02. `amber_9` (97 glyphs) + `topaz_8` (225 glyphs). |
| [T3.1](T3.1-shaders.md) Reference shaders + light-level renders | **DONE** ✅ | T2.1, T2.2 | ACCEPTED 2026-10-02 (round 2). 3 GLSL ES 3.00 shaders (run live in the review app, pixel-identical to the Python renders), 277 preview PNGs + strips, `shaders/verify.json` (10 items). |
| [T3.2](T3.2-formats.md) Format spec | **IMPLEMENTED (awaiting review)** | Wave 1 + Wave 2 | 2026-10-02. `assets/FORMATS.md` + 10 `formats/*.md`; root `assets/verify.json` (11 items). |
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

- **T2.2–T2.8 implemented in parallel (2026-09-28)** — one git worktree per task under `.worktrees/T2.n`
  (gitignored), one commit per branch `t2/{tiles,masks,screens,world,music,sfx,fonts}`, all off `research`.
  Implementer subagents wrote the code/verify.json; they could not execute Python in the background, so the
  extractors were run, debugged and committed in-session. An octopus merge of all seven branches applies cleanly:
  205 tests pass, the review store reports 0 problems and every count matches. Review app: `FileViewer` now
  renders `.wav` files with `<audio>` (needed by T2.6/T2.7).
  - **T2.2**: tiles are **16×32**, not 16×16 as `plan.md` said (`next_image` copies 32 scanlines,
    `fsubs.asm:755-771`; `vsc equ 32`, `fsubs.asm:1701`; `img_y = map_y>>5`, `fmain.c:1981`); `experiment/shaders/`
    had decoded only the top half. Plan and task file corrected. Atlases are 256×512. RGBA renders index 31 opaque
    with the region colour-31 override. Regions 1/3 and 5/7 load identical image blocks (`fmain.c:617-623`).
  - **T2.3**: entries are indexed by terra byte 0 (`fmain.c:2577-2595`), applied by `_maskit` (`fsubs.asm:1047-1083`)
    as 32 words → 16×32; set bit = terrain in front. Entries 171–191 are all-zero (P26).
  - **T2.4**: the loader skips `CMAP` (`iffsubs.c:157-159`), so PNGs use the `LoadRGB4` palette each screen is shown
    with (`introcolors` / first `win_colors` frame / `textcolors`); CMAPs kept in `screens.json` (diffs in P26).
  - **T2.5**: sector grids ship as two shared pools (`sectors_outdoor.json` block 32, `sectors_indoor.json` block 96)
    rather than per region, since `file_index` gives one block per group. Previews are colour-coded (map / collision
    / mask-mode); region 5 matches `reference/region_5.png`. Mask-mode comment vs code mismatch → P26.
  - **T2.6**: `Seek(file,S_WAVBUF,0)` is `OFFSET_CURRENT`, so envelopes start at v6 byte **2048** (the first cut used
    1024 and rendered silence). Instrument word: high byte = waveform, low byte = envelope (`gdriver.asm:241, 377-380`).
    `ptable` has 78 entries (`gdriver.asm:204-222`). Added 7 four-voice `song_N.wav` mixes besides the 28 per-voice WAVs.
  - **T2.7**: 8-bit WAVs (signed→unsigned XOR 0x80); effect 5 nominal period 1800 (the launch sites), 3200 at the
    hit site is recorded in `sfx.json`. `audio.md:425` labels effect 2 "Killing blow?"; it is the arrow/witch hit.
  - **T2.8**: hunk-wrapped `DiskFontHeader`; Amber/9's default glyph overruns the strip by 8 bits (read linearly, as
    the blitter does; P26). `dfh_Name` is empty in both files.
  - **NTSC timing (user decision):** the game does not play correctly on PAL, so T2.6 previews use a 60 Hz
    vertical blank and T2.6/T2.7 use the 3,579,545 Hz Paula clock (the task files had said PAL; corrected in
    `plan.md`, `T2.7-sfx.md`, `_SHARED.md`). Cross-checked against the port's sequencer
    (`faery-tale-rs/src/game/audio.rs`, `songs.rs`): same v6 layout (envelopes at 2048), note gap, rest,
    instrument/tempo/loop handling and 78-entry ptable.
  - **T2.2 review note (2026-09-28):** the indexed atlases showed the app background through index 31. User
    decision: world tiles have no key colour; colour-31 keying applies only to things rendered on top of them.
    `write_indexed_png` gained `transparent=False`; the indexed atlases were regenerated without tRNS.
  - **T2.2.1 added (user request during T2.2 review):** `tools/extract_master_atlas.py` reads the T2.2 atlases and
    the T2.5 map/sector data and emits `assets/tiles/master/`: every tile some sector on a region's map contains
    (`fsubs.asm:565-604`), deduplicated by index content (tiles with index 31 kept per colour-31 value so the RGBA
    atlas is exact). 1976 used (region, tile) pairs → 954 master tiles (948 patterns + 6 colour-31 variants);
    per-region unused counts range from 8 (regions 8/9) to 107 (region 0). `master.json.region_maps` is the
    256-entry tile→master map per region. Review backend fix: task-ID sort crashed on `T2.2.1` vs `T2.2`.
  - **T2.2.1 refinement (user: crystal palace tiles were in the secret set).** Regions 8/9 share map data, so
    "used" is now decided per island (4-connected non-zero sectors) by the region the game loads on entry:
    `doorlist[].secs` (`fmain.c:1926`), coordinate-derived region for the stargate pair (`fmain.c:1950`, `2632-2637`
    → 8), necromancer xfer → 9 (`fmain.c:1787`). Region 9 shows 3 of 66 islands (146 tiles used); 16 islands have
    no entry. The crystal palace (sectors 121-124, `astral` group tiles 221/241-253) is entered with `secs=1` →
    region 8, so only cave tiles **114 and 115** are secret-timer tiles. Master: 945 tiles from 1874 pairs.
    Side finding: the stargate loads the astral plane as region **8** (coordinate formula), not 9.
  - **Secret passages (user: critical gameplay feature).** In region 9 `fade_page` sets colour 31 to `0x00f0`
    while `secret_timer` runs (Crystal Orb, magic case 8 `fmain.c:3307`; decrement `fmain.c:1381`; `fmain2.c:382-384`),
    otherwise `0x0445` — the same value as palette index 9, so the passages are invisible until revealed. First cut
    shipped index-31 masks and a `atlas_rgba_secret.png` per atlas; the user judged that wasteful for two tiles. Final:
    one set — the master atlas appends the two affected tiles (cave 114/115) once more as **variants** (945, 946),
    linked in `master.json.index_31.secret_variants`; `tiles.json.index_31` lists the index-31 tiles per region and,
    for region 9, the timer colour. Indexed atlases stay **verbatim** source indices (an interim 35-entry-palette
    variant was rejected); instead the review app's image viewer gained a **colour-31 selector** for indexed PNGs
    (`GET /files/…?color31=<rgb4>` rewrites PLTE entry 31; options from `GET /api/color31` = the four
    `region_overrides.json` values). Master: 947 tiles.
  - **T2.3 review question (2026-09-28):** "is this all the masks?" Yes — `shadow_mem` (192 × 64 B, `fmain.c:642`,
    `1222`) is the game's only stored mask set; sprite silhouettes are computed at run time (T2.1). Cross-checked with
    the T2.5 terra data: entries 0-170 are referenced, 160 applied with a non-zero occlusion mode, 11 have pixels but
    are never referenced (18, 44, 84, 85, 93, 94, 147, 148, 154, 159, 160), 171-191 are blank. Recorded in
    `masks.json.usage`; the arbitrary "entry 0 / entry 191" review items (from the task file's example) were dropped in
    favour of the contact sheet with these counts.
  - **Shadow masks alongside the tiles (user request, 2026-09-28).** Every region atlas and the master atlas now
    ship `atlas_shadowmask.png`: per tile cell, the `assets/masks/` entry named by the tile's terra byte 0 when its
    occlusion mode (terra byte 1 & 15) is non-zero, else empty (`fmain.c:2577-2595`); `tiles[].mask` / `mask_mode`
    in `tiles.json` and `master.json`. The master dedup key now includes (mask, mode): 11 tiles share art but differ
    in mask/mode across sources (e.g. master 182: none in regions 1-2, mask 3 mode 3 in region 3), so they are kept
    once per combination → 955 + 2 variants = **957** master tiles, a strict 1:1 lookup for art, highlight and shadow.
    `extract_tiles.py` reads the mask store via `extract_masks` and the terra entries from `assets/world/`.
  - **T2.5 redesign (user, 2026-09-30: "walls of JSON I won't review").** The per-region sector/terra JSON bundle
    (`assets/world/`) is gone. `tools/extract_maps.py` now ships `assets/maps/`: the **overworld** (regions 0-7
    stitched, 2048×1024 tiles) and every interior space cut out of the shared region-8/9 sheet by a sub-tile
    walkability flood fill from each entry (door landing `fmain.c:1919-1924`, stargate `1944-1948` + `2632-2637`,
    quicksand drop `1784-1789`; `px_to_im`/`prox` rules: 1 and ≥10 block, 12 Shard, 15 openable door) → **62
    interiors**, **5 dungeons** (tombs, dragon cave, maze caves, spider pit, troll cave — *not* one complex; a
    first tile-granular probe had leaked through partial walls), the **astral plane** (region 8, via the doom-tower
    stargate). Cells belonging to another space are blanked. Layers are index PNGs (`tiles.png` 8-bit original id,
    `master.png` 16-bit master index) + `map.json`, decoded by `tools/decode_map_layers.py`; previews rendered from
    the real tiles with entries ringed in magenta. Format in `assets/maps/README.md`. Collision moved into the
    master atlas: 21 master tiles had conflicting `(feature_type, subtile_mask)` across sources (same art, different
    tile id, e.g. region 8's black tile as wall/half-wall/floor), so the dedup key now includes them → 971 + 2
    variants = **973** master tiles from 1788 used pairs; "used" for regions 8/9 is now "appears in a shipped space".
    `extract_tiles.py`/`extract_masks.py`/`extract_master_atlas.py` read terra via `decode_map_data.load_regions`.
    Source bug → P27: cabin-yard gates #4/#5/#9 point into cabins 7/8/6's yards (three `yc2` values) — **corrected in the shipped maps** (user decision, `extract_maps.DOOR_FIXES`, originals kept as `fix.source_yc2`); P26: tile 7 (type 10) is a
    passage corner; 161/162 (type 13) are beds. The `--assets` legend step (swatch view + `legend.png`) is moot.
  - **T2.5 review round 1 (REJECT, 2026-10-01) — renames + fixes:** maps renamed per review (marheim castle,
    citadel of doom, forbidden keep [= narr.asm's name for the doorlist's "unreachable castle", reached by swan],
    witchwood cave, tambry tavern/tambry_N, marheim_N); `map.json` keeps `source_name` + narr.asm `place_names`;
    stargate entries labelled "portal to/from …"; previews outline every openable door tile (type 15) in cyan with
    its `open_list` name so SECRET/TUNNEL doors are visible; a stray mammoth-manor wall stub inside the dragon
    cave's box is now blanked (fragments not 8-connected to the space's own tiles are dropped — the only case);
    the "index layers" verify item was folded into the index item.
  - Doc drift noticed, not fixed: `text-display.md:43` (OpenFont is at `fmain.c:778`), `iff-loading.md` asm/struct
    line numbers off by 2, `audio.md:162-170` (84-entry ptable) and `:538` (seek "redundant").

- **T1.4 (2026-10-01)** — done in-session (no subagent). `tools/extract_text.py` parses `src/narr.asm` directly
  (`;` outside quotes starts a comment; `'…'`/`"…"` literals incl. the `',"'",'` idiom; `equ` symbols `XY`/`ETX`;
  `a/b` operands with integer division, so `21/2` → 10 → x = 20) and writes `assets/text/`: `event_msg` (39),
  `place_msg` (27), `inside_msg` (23), `speeches` (61), `question` (8, `qq` order), `placard_text` (20, `mst` order,
  decoded per `_ssp` `fsubs.asm:497-536` into `{x_half, x, y, text}` segments; a run with no `XY` marker has null
  position — those entries follow a `name()` call, `fmain2.c:1588`), `place_tbl` (29) and `inside_tbl` (37) as
  `{lo, hi, msg_index, comment, line}` rows. A `dc.b 0` with no pending string is an empty entry: `place_msg[0..1]`,
  `inside_msg[0..1]` and `speeches[52]` are `""`; the commented-out `narr.asm:197` line is absent; `q8`'s trailing
  `dc.w 0` pad (`narr.asm:82`) is skipped. No string contains `$`. Checks: `test_extract_text.py` 10/10, full suite
  232; review store 0 problems, 8/8 counts match. `.gitkeep` removed. Out of scope, noted: `_titletext`
  (`fsubs.asm`, also an `_ssp` stream) is not in `narr.asm` and was not extracted.
  - **Previews (user: "render the placard text, the wall of JSON is not interesting").** `tools/render_placards.py`
    → `assets/text/previews/placard_*.png`: the 12 cards as the call sites compose them (`fmain.c:2859-2879`,
    `fmain2.c:1586-1591`, `1607`, `fmain.c:1235`), `name()` spliced as "Julian", Amber/9 drawn like `Text()`
    (kern, blit at baseline − 7, advance space), the `_placard` meander border simulated from `fsubs.asm:387-475`
    (final state, pen 24). Finding: the copy-protection lead-in (msg12) is **not** in Amber/9 — `rp_map` keeps
    `tfont` (topaz-8) from `fmain.c:781` and nothing sets `afont` on it before `placard_text(19)` at line 1235; in
    Amber/9 its 37-char lines overrun the 320-px page, in topaz-8 they fit. The placard verify item is now the image
    set (12 cards) with the JSON kept as a count.
  - **Review round 1 (REJECT, 2026-10-02):** "provide the answers along with the questions". `question.json` is now
    a table `{index, label, line, question, answer}`; the answers are parsed from `char *answers[]`
    (`fmain2.c:1306-1307`), which `copy_protect_junk` indexes with the same `j` as `question(j)` (`fmain2.c:1316-1317`).
    They match the user's list (LIGHT, HEED, DEED, SIGHT, FLIGHT, CREED, BLIGHT, NIGHT).
  - **Round 2: ACCEPT** (2026-10-02, stated by the user in-session; `review_results.json` holds only the round-1
    REJECT — submit in the app if the recorded verdict should match).

- **T3.1 (2026-10-02)** — done in-session (no subagent). **Dropped `region_crossfade.glsl`** (user decision): no
  source for it — overworld region changes are seamless disk loads (`fmain.c:2964-2976`, `3548-3614`), doors cut
  to `fade_page(100,100,100)` (`fmain.c:1929`), cinematics use `fade_down`/`fade_normal`; removed from `plan.md`
  and the task file. Shipped: `assets/shaders/{daynight_live,daynight_bank,fade_to_black,daynight_dim,
  moonlight_blue,green_jewel}.glsl` as **GLSL ES 3.00** (`#version 300 es` first line) so they run verbatim in
  WebGL2; `shaders.json` (textures/uniforms per shader); `README.md` (effects matrix, driving values).
  `tools/render_light_levels.py` → `shaders/previews/`: every region atlas (0–7 at levels 0, 95, 105, 111, 120,
  136, 150, 165, 180 + Green Jewel at 0; 8/9 at 180 only — `day_fade` passes 100,100,100 indoors) and all 30 actor
  sheets (same spread), 382 renders + 38 jewel + 40 labelled strips + `previews.json`, via
  `experiment/shaders/fade_page.py` (LUT re-checked equal to `daynight_lut.json` at all 10 levels).
  **Review app:** new `shader` view kind (`models.py`, schema regenerated) + `ShaderViewer.tsx`: subject
  picker, uniform sliders, baked-level buttons, and a pixel diff of the GPU output against the Python PNG.
  Headless-Chrome check (SwiftShader): all 6 shaders compile; `daynight_live` (every level + jewel),
  `daynight_bank` (every layer) and `fade_to_black` (weights 100) are **pixel-identical** on all 40 subjects
  (764 comparisons, 0 differing). Review store: 0 problems, all counts match. The running review server was
  restarted for the new view kind. Note: the jewel renders are strongly red/magenta — that is what the source
  does (`r` weight 100 with `r1 = max(r1, g1)` while g/b stay at night floors, `fmain2.c:1655, 407`).
  - **Review round 1 (REJECT, 2026-10-02) processed:**
    - Renders trimmed to actors that appear outdoors (reviewer: Loraii/necromancer/woodcutter are astral-plane
      only, the dragon is in his cave, every NPC but beggar/ranger/spectre/ghost is indoors — none is ever
      palette-faded). `render_light_levels.INDOOR_ONLY_ACTORS`; 27 subjects (10 regions + 17 actors), 252 + 25
      jewel renders, 27 strips; stale PNGs removed.
    - **Decomposition shaders dropped** (`daynight_dim`, `moonlight_blue`, `green_jewel`: "the live shader is
      enough"); removed from `shaders.json`, both READMEs, `formats/shaders.md`, `plan.md`, the task file.
    - `fade_to_black` gained **modes** in `shaders.json` + the viewer: *fade_down / fade_normal* (one weight `i`,
      step 5 → `(i,i,i)`), *intro zoom* (`x` 0..160 step 4 → `y = x*5/8`, weights `(2y-40, 2y-70, 2y-100)`,
      `fmain.c:1199, 1209, 2917, 2930`) and *free*.
    - Checks on `:8766`: 0 validation problems, all counts match; headless Chrome: 3 shaders compile, every
      live/bank/fade comparison identical on all 27 subjects. The T3.1 `verify.json` keeps the user's expanded
      formatting (10 items now).
  - **Review round 2: ACCEPT** (2026-10-02T17:27Z).
- **T3.2 (2026-10-02)** — done in-session. `assets/FORMATS.md` (index + global conventions) and
  `assets/formats/{palettes,sprites,tiles,masks,maps,screens,text,audio,fonts,shaders}.md`, written from the
  shipped JSON (every field name checked against the real files); the palette-effects matrix and
  `highlight_mask` format live in `formats/palettes.md`; the audio synth model in `formats/audio.md`.
  Root `assets/verify.json` created (11 T3.2 items, view `markdown`). `tools/check_md_links.py` now takes an
  optional directory argument (`… assets` → all links valid; the default `reference/` run still reports its 10
  pre-existing issues, untouched). `.gitkeep` removed from `shaders/` and `formats/`. 232 tests pass.

## Next
**T3.1 accepted.** T3.2 (format spec) awaits the human Reviewer in the app. Then Wave 4 (T4.1 manifest, T4.2 bundle acceptance).
Side track: build the browser site in small bursts per [`site/PLAN.md`](../../site/PLAN.md) (Phase 1 next).
