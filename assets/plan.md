# Asset Extraction & Format-Conversion Pipeline

## Context

The project currently consumes the original 1987 Amiga *Faery Tale Adventure* data directly:
graphics, world data, audio, and gameplay tables are read out of the original ADF disk image
(`src/assets/image`), external binary files (`src/assets/songs`, `src/assets/v6`,
`src/assets/fonts/`), and — for the hardcoded tables — out of the C source (`fmain.c`,
`fmain2.c`) and `src/narr.asm`.

A **separate, future porting effort** will reimplement the game. That effort must be
**language- and implementation-neutral** and must run **without** the original game assets or
any source code.

**This is a one-time extraction. The deliverable is exactly one thing: the committed,
self-contained `assets/` bundle** (modern open formats + docs). The extraction is done when the
assets exist, are verified correct, and are documented — not when a pipeline is built.

**Tools are byproducts, not the product.** Any script written under `tools/` during extraction is
kept only as a **historical record** of how an asset was produced (provenance). Tools are **not**
maintained, are **not** required to re-run, and their repeatability is **not** an acceptance
criterion. Do not build orchestration, drivers, or regeneration harnesses for their own sake.

**Agents extract; scripts only decode.** The implementer agent reads the source and writes the
data directly, citing every value. A script is warranted only when the input is **binary**
(bitplanes, samples, map/sector files) or **too large to transcribe reliably** (e.g. an 87-row C
array), and then it must genuinely parse the original input — never embed hand-typed data in a
script.

**Scope: sensory content, reproduced faithfully.** The goal is a new game that looks, sounds and
feels the same. The bundle holds what can't be expressed as behavior: graphics, palettes, fonts,
audio, narrative text, world maps. Convert these losslessly (a sprite keeps its exact pixels, a
palette its exact colors), which is the simplest way to make it look the same. This is a *format
conversion only* — no gameplay, rendering, or engine work, and no creative changes.

**Behavior is not an asset.** What the game *does* (item effects, menus, combat, AI) and the
constants it uses are specified at behavioral fidelity in `reference/logic/` (+ `RESEARCH-*.md`).
Do not mirror the original's internal tables or data layouts into `assets/` for their own sake.

### Decisions locked with the user
- **Extraction method:** agent-authored data by default; Python scripts under `tools/` only where
  the rule above allows. Scripts are kept for provenance only — they are byproducts, not a
  maintained deliverable.
- **Verification is human-in-the-loop, by perception only.** There is no automated acceptance
  harness. The human reviewer verifies the **final** assets visually, by listening, or by
  glancing at tables/JSON — never by re-extracting, hand-decoding, or writing verification code.
  Each extraction step ships **manual verification instructions** appropriate to the asset (see
  below), sized to minutes per task. Automated checks (round-trips, hand-decode diffs) may be
  used *by the implementer while extracting*, but they are optional aids, not the gate. Anything
  perception misses will surface during port implementation and can be revisited then.
- **Previews for non-perceivable assets.** Where the data can't be judged by reading JSON (music
  event streams, world/terra grids), the implementer also emits **preview artifacts** (rendered
  audio previews, region-map PNG renders) under `assets/<subdir>/previews/`. Previews are
  non-authoritative but **ship with the bundle** — they are useful to porting efforts as well.
- **Graphics:** emit **both** indexed PNG (exact 0–31 index per pixel) **+** palette files
  (original 12-bit Amiga values) **and** baked RGBA PNG. Document which palette effects require
  the indexed path.
- **Sprites:** per-frame PNGs **+** a packed sheet **+** a JSON atlas, **one atlas per actor
  type** (no global pack).
- **Data/metadata format:** **JSON** for everything (palettes, sector/map, terra flags,
  animation/stat tables, sprite atlases, narrative text, audio metadata).
- **Audio:** **structured synth data** (note/event streams + 8 waveforms + 10 ADSR envelopes as
  JSON) **+** PCM SFX as **WAV**. No baked music render.
- **Deliverable:** a **committed `assets/` directory** in this repo (research branch) — the assets
  themselves are the product.
- **Docs:** a **full per-resource format spec** (markdown) so the bundle is self-sufficient, plus
  **human verification instructions as data** (`verify.json`) per resource subdir, presented by the
  review app (`tools/review/`, see [`../tools/review/PLAN.md`](../tools/review/PLAN.md)).

### Manual verification instructions (required per step)
Because verification is human-in-the-loop **by perception**, every extraction step must add its
items to the `verify.json` in its `assets/` subdirectory (items tagged with the task ID; schema in
`tools/review/PLAN.md`). The review app renders them as the task's itemized review page, telling
the reviewer exactly what to look at or listen to — sized to **minutes per task**, never asking
the reviewer to decode, re-extract, or script anything. The app renders JSON data natively
(swatches, tables, text) and shows actual vs expected counts. The instructions must be
**appropriate to the asset type**:
- **Images** (sprites, tiles, screens, masks, font glyphs): pair **each filename** with a
  short **description derived from the source or reference** of what the reviewer should see
  (e.g. `sprites/julian/frame_000.png` — “hero facing south, green tunic; from `pagecolors[]`
  + cfile 0”). The reviewer opens the file and confirms it matches the description.
- **Tables / narrative text (JSON):** state the expected counts/dimensions and a few spot values
  with their **source citation** (file:line) so the reviewer can eyeball them at a glance.
- **Audio:** state the expected track/sample counts and give “play `sfx_0.wav` — should be
  <one-line description>” cues; for music, point at the preview renders (“`previews/track_03.wav`
  — Tambry village theme”).
- **Non-perceivable data (music JSON, world grids):** point the reviewer at the **preview
  artifacts** rather than the raw JSON.
- **Palettes:** list a few indices with their `rgb4`→`rgba8` values and source citation.

## Source data locations (read-only inputs)

| Input | Path | Contains |
|---|---|---|
| ADF disk image | `src/assets/image` (901,120 B = 1760×512) | tiles, sprite cfiles, sector/map/terra, shadow masks, 6 PCM SFX (blocks 920–930) |
| Music | `src/assets/songs` (5,984 B) | 28 packed track event streams |
| Instruments | `src/assets/v6` (4,628 B) | 8×128B waveforms + 10×256B envelopes |
| Fonts | `src/assets/fonts/` | Two Amiga fonts, both used by the game: `Amber/9` (loaded via `LoadSeg("fonts/Amber/9")`, `fmain.c:774`) and `Topaz/8` (the game opens ROM topaz-8 via `OpenFont`, `fmain.c:650,778`; the disk copy stands in for the ROM font). `amber.font` itself is never opened and lists sizes with no disk data — ignore it. |
| IFF screens | `src/assets/` — exactly 9 ILBM files: `p1a` `p1b` `p2a` `p2b` `p3a` `p3b` `page0` `winpic` `hiscreen` | intro/placard/win/hi-score images (`hiscreen` is 640×57 hires-width) |
| Hardcoded tables | `src/fmain.c`, `fmain2.c`, `ftale.h` | statelist, encounter_chart, inv_list, weapon/treasure probs, diroffs, fallstates, setfig_table, file_index, trans_list, palettes |
| Narrative text | `src/narr.asm` | event/place/inside messages, speeches |

The exporter must take a configurable `--game-dir` (default `src/assets`) and
`--src-dir` (default `src/`). All original data lives inside this repo under `src/`; no
external/sibling checkout is used.

## Output layout (committed `assets/`)

```
assets/
  manifest.json                 # index of the shipped bundle: every file, type, checksum, source ref
  FORMATS.md                    # full per-resource format spec (links into formats/)
  <subdir>/verify.json          # per-resource review items for the review app (ships; listed in manifest)
  formats/                      # one .md per resource type (field semantics, units, conventions)
  palettes/
    pagecolors.json  textcolors.json  introcolors.json  sun_colors.json  blackcolors.json
    region_overrides.json       # per-region color-31 variants (desert/dungeon)
    # each: {index, rgb4 (0x0RGB Amiga OCS 12-bit), rgba8} so both exact + convenient forms exist
  tiles/                        # background tile atlas, per region
    region_<NN>/atlas_indexed.png  atlas_rgba.png  atlas_highlightmask.png  tiles.json
  sprites/                      # one folder + atlas PER actor type (18 cfiles)
    <actor>/frame_000.png …     <actor>_sheet.png   <actor>_highlightmask.png   <actor>.json (atlas: frame→rect, w/h, transparency=index 31)
  masks/                        # shadow/collision masks (indexed PNG + JSON bit layout)
  world/                        # sector maps, region maps, terra/collision flags (JSON)
    previews/                   # non-authoritative region-map PNG renders (perception review + port aid)
  audio/
    music/<track>.json          # event streams (note/rest/instrument/tempo/end)
    music/previews/             # non-authoritative rendered audio previews (perception review + port aid)
    instruments/waveforms.json  envelopes.json
    sfx/sfx_<n>.wav  sfx/sfx.json  # per-effect trigger period expressions (see work item 9)
  fonts/<font>_<size>/glyphs/*.png  <font>_<size>.json   # per size file (amber_9, topaz_8): glyph atlas + metrics (y_size,baseline,char_loc,width)
  screens/<name>.png            # IFF intro/placard/hi-score images → RGBA PNG
  shaders/                      # reference shaders (reference-only; validated by the port) + light-level reference renders (previews/)
    fade_to_black.glsl  daynight_dim.glsl  region_crossfade.glsl
    moonlight_blue.glsl  green_jewel.glsl
    daynight_live.glsl  daynight_bank.glsl   # full day/night incl. veg boost (RGBA + highlight_mask)
    README.md                   # maps each effect → indexed-path vs RGBA+shader; pseudocode
  text/
    event_msg.json speeches.json place_msg.json inside_msg.json
    question.json placard_text.json place_tbl.json inside_tbl.json
```

## Work items

Grouped by resource. **[reuse]** = existing tool largely suffices; **[extend]** = adapt an
existing tool; **[new]** = new extractor module.

### Graphics
1. **Sprites** — `tools/extract_sprites.py` **[extend]**. Already decodes all 18 cfiles to PNG
   (5 bitplanes, transparency = index 31, `pagecolors[]`). Add: per-actor JSON atlas (frame→rect,
   dimensions, transparency index, frame counts from `CFILES`), an **indexed-PNG** output mode
   (currently RGBA), and emit into `assets/sprites/<actor>/`. Keep the existing labeled/2x debug
   variants out of the shipped bundle. Also emit a **1-bit highlight mask** per actor sheet
   (`<actor>_highlightmask.png`: 1 where the source index ∈ 16–24, else 0; transparency follows index 31)
   — **every** actor sheet uses some of indices 16–24 (the hero `julian`/`phillip`/`kevin` use 24,
   NPCs/enemies use much of the range), so the mask is required for the RGBA day/night path to
   reproduce the night vegetation boost pixel-exactly on sprites (see Graphics §5a and
   `experiment/shaders/`).
2. **Background tile atlas** — **[new]** `tools/extract_tiles.py`. Decode `image_mem` per region:
   256 tiles, 5 bitplanes, group-major then plane-major. Use the offset formula
   `offset(T,P,R) = (T/64)*20480 + P*4096 + (T%64)*64 + R*2` (tiles are
   16×16, verified in `experiment/shaders/`). Emit indexed PNG + RGBA PNG + a **1-bit highlight
   mask** PNG (`atlas_highlightmask.png`: 1 where index ∈ 16–24) + `tiles.json` per region. The mask
   drives the night vegetation boost on the RGBA day/night path (Graphics §5a). Region→image-group
   block numbers come from `file_index[]`.
3. **Palettes** — **[new]** `tools/extract_palettes.py`. Extract `pagecolors`, `textcolors`,
   `introcolors`, `sun_colors`, `blackcolors` from `fmain.c`, plus per-region color-31 overrides
   (`fade_page` in `fmain2.c`: region 4 = 0x0980, region 9 = 0x0445, else 0x0bdf). Emit both
   `rgb4` (the Amiga OCS 12-bit `0x0RGB` value) and `rgba8` per entry. Reuse `extract_table.py`
   to pull the raw C arrays.
4. **Shadow/collision masks** — **[new]** `tools/extract_masks.py`. **192** entries × 64 B (32 rows ×
   2 B, 1 bit/pixel) from ADF blocks 896–919 (three 4,096 B files `mask`/`mask2`/`mask3`,
   `mtrack.c` diskmap[21–23]; total 12,288 B = `SHADOW_SZ` at `fmain.c:642`, loaded by
   `load_track_range(896,24,shadow_mem,0)` at `fmain.c:1222`). Emit indexed PNG + JSON bit layout.
5. **IFF screens** — **[new]** `tools/extract_screens.py`. Parse IFF/ILBM (BMHD/CMAP/BODY,
   ByteRun1) for all 9 screen images in `src/assets/`: `p1a` `p1b` `p2a` `p2b` `p3a` `p3b`
   `page0` `winpic` `hiscreen` → RGBA PNG. Dimensions vary per file (`page0`/`winpic` 320×200;
   `hiscreen` 640×57 hires-width; placards smaller) — take them from each `BMHD`, do not assume.
5a. **Reference shaders** — **[new]** hand-authored, not generated. Provide GLSL (with
    pseudocode comments — GLSL is generic enough to translate to any pipeline) for each palette
    effect so the porting team can drive the RGBA assets directly:
    - `fade_to_black.glsl` — uniform multiply (also covers fade-from-black / scene transitions).
    - `daynight_dim.glsl` — `lightlevel`-driven uniform brightness scale.
    - `region_crossfade.glsl` — lerp between two RGBA region renders over 8 frames.
    - `moonlight_blue.glsl` — per-pixel blue injection from green (`b += g2*g`), with night
      channel floors (r≥10%, g≥25%, b≥60%).
    - `green_jewel.glsl` — per-pixel `r = max(r, g)` boost.
    - `daynight_live.glsl` / `daynight_bank.glsl` — the **full** day/night cycle incl. the
      vegetation night boost (see correction below). Port the verified reference from
      `experiment/shaders/` (`daynight_live.glsl` = live from full-bright RGBA + `highlight_mask`;
      `daynight_bank.glsl` = sample/cross-fade a prebaked per-light-level RGBA bank).
    `shaders/README.md` documents inputs/uniforms for each and states that the GLSL is
    **reference-only** — it is validated by the porting effort, not by this pipeline. The
    perception-reviewable output of this step is a set of **light-level reference renders**
    under `assets/shaders/previews/`: every region atlas + several actor sheets rendered at a
    spread of light levels plus moonlight/green-jewel, generated with the verified Python
    `fade_page()` port (bit-exact ground truth; doubles as golden images for the port).

    **Correction (was: "vegetation night boost on indices 16–24 is NOT shader-doable on RGBA").**
    The experiment in `experiment/shaders/` reproduces the entire `fade_page()` day/night cycle —
    including the indices-16–24 vegetation boost — **bit-exactly on prebaked RGBA**, two ways, both
    proven by `experiment/shaders/compare.py` (zero diff at every light level). The boost is a
    deterministic integer function of light level and palette index; baking discards only the
    palette **index**, and that is restored with **one extra bit per pixel** — the `highlight_mask`
    (index ∈ 16–24) emitted for both tiles (§2) and every sprite sheet (§1). The accurate
    statement: the vegetation night boost **is** shader-doable on RGBA **given the 1-bit highlight mask**
    (or by baking one RGBA frame per light level); only an *index-blind* RGBA dim — no mask — cannot
    reproduce it. The indexed atlas + palette LUT remains a valid alternative but is **no longer
    required**. Cross-reference `formats/palettes.md` and `experiment/shaders/FINDINGS.md`.

### World data
6. **Sector / region maps / terra** — `tools/decode_map_data.py` **[extend]** (already 1,467
   lines of map/sector/terra logic). Emit per-region JSON: sector tile-index grid, region map,
   and terra/collision flags (high nibble = feature type, low nibble = mask-application mode).
   Also emit **preview renders** (per-region map PNGs, e.g. tiles composited or color-coded
   terra) under `assets/world/previews/` so the reviewer can recognize the world by sight.

### Audio
7. **Music event streams** — **[new]** `tools/extract_music.py`. Parse `src/assets/songs`: 28 tracks,
   each `i32 packlen` (BE) + `packlen×2` bytes of `(command,value)` events. Decode to JSON
   (note/rest/set-instrument/tempo/end + loop flag). Encode the Paula period table reference.
   Also emit **preview renders** (a rough audio render per track from the waveforms/envelopes)
   under `assets/audio/music/previews/` so the reviewer can recognize each theme by ear. The
   previews are non-authoritative; the JSON is the deliverable (the locked "no baked music
   render" decision refers to the shipped music format, not these review aids).
8. **Instruments** — same tool. Parse `src/assets/v6`: bytes 0–1023 = 8×128 signed waveform samples;
   1024–3583 = 10×256 envelope tables → `waveforms.json`, `envelopes.json`.
9. **SFX** — **[new]** `tools/extract_sfx.py`. 6 samples from ADF blocks 920–930 (5,632 B buffer);
   each sample is framed by a **4-byte big-endian length prefix** (`read_sample()`,
   `fmain.c:1033-1041`) — walk the prefixes, do not split by fixed size. 8-bit PCM → WAV, byte-exact
   (no resample/normalize). **There is no fixed native rate:** the game plays each effect at a
   **randomized Paula period per trigger** (`effect(num,speed)` → `playsample(...,speed)`,
   `fmain.c:3617-3619`; call sites e.g. `effect(2,500+rand64())` `fmain2.c:238-241`,
   `fmain.c:1488/1680/1690/2262`). WAV header rate = the effect's **base-period rate**
   (PAL Paula: 3,546,895 / base period), documented as nominal container metadata. Also emit
   `sfx.json`: per effect — buffer offset, byte length, and every trigger's period expression
   (base + random range + RNG fn) with call-site citations.

### Fonts
10. **Amiga fonts** — **[new]** `tools/extract_fonts.py`. Parse `.font` + DiskFont (ID 0x0F80):
    glyph bitmaps → per-glyph PNG + packed atlas, metrics JSON (`y_size`, `baseline`, `modulo`,
    `lo_char`/`hi_char`, `char_loc` offset+width). Exactly two size files, both used by the game:
    **`Amber/9`** (`LoadSeg("fonts/Amber/9")`, `fmain.c:774`) and **`Topaz/8`** (the game opens
    ROM topaz-8 via `OpenFont(&topaz_ta)`, `fmain.c:650,778` — the disk copy stands in for the
    ROM font a port won't have). The `.font` headers are not used by the game; skip any sizes
    they declare beyond these two files.

### Tables & text (extract C-source constants → JSON)
11. **Gameplay tables** — **dropped (not an asset).** Game-design constants (`encounter_chart`,
    `weapon_probs`, `treasure_probs`, `rand_treasure`, `trans_list`) live in `reference/`. Art
    bindings (`statelist`, `diroffs`, `fallstates`, `setfig_table`, `inv_list`) become named
    metadata in the sprite atlases (item 2 / T2.1). `file_index` only organizes tiles and world
    data per region (T2.2/T2.5). `tools/extract_tables.py` can write temporary copies to
    `tools/results/tables/` for those extractors. See T1.2.
12. **Item/quest data** — **dropped (not an asset).** Item effects, menu shape and brother stats are
    behavior, specified in `reference/logic/` (`magic.md`, `menu-system.md`, `brother-succession.md`).
13. **Narrative text** — **[new]** `tools/extract_text.py`. Pull the `dc.b` message strings from
    `src/narr.asm` (`_event_msg`, `_speeches`, `_place_msg`, `_inside_msg`) →
    `event_msg.json`, `speeches.json`, `place_msg.json`, `inside_msg.json`.
    Preserve `%` (player name) and `$` (target) placeholders; document them in the spec.
    Also extract (required for a self-sufficient bundle): the 8 `_question` riddles →
    `question.json`; the 20 `_placard_text` positioned scroll entries (XY/ETX control bytes
    decoded to `{x_half, y, text}` segments) → `placard_text.json`; and the
    `_place_tbl` (29×3) / `_inside_tbl` (37×3) place-code→message lookup tables →
    `place_tbl.json`, `inside_tbl.json`.

### Bundle index & docs
14. **Bundle index** — **[new]** author `assets/manifest.json` describing the **committed** bundle:
    per file — path, type, byte size, SHA-256, source reference. This is a *description of the
    shipped artifact* for integrity/provenance, **not** a driver that regenerates it. A small
    helper script may generate it, but the manifest ships as data alongside the assets.
15. **Format spec** — **[new]** author `assets/FORMATS.md` + `assets/formats/*.md`: one section
    per resource type covering field meanings, units, coordinate systems, the
    **transparency convention (sprite index 31)**, the **palette-effects matrix** (which effects
    are shader-doable on RGBA and how — the night vegetation boost on palette indices 16–24 needs
    the **1-bit `highlight_mask`** that ships with each tile atlas and sprite sheet, not the indexed
    path; cross-linked to `assets/shaders/` and `experiment/shaders/FINDINGS.md`), and the
    **audio synth model** (period table, VBL tempo, envelopes). Document the `highlight_mask` format
    (1 bit/pixel, set where source index ∈ 16–24, transparency follows index 31) under
    `formats/palettes.md`.
15b. **Human verification instructions** — **[new]** as part of each extraction step, add the
    task's items to that resource's `assets/<subdir>/verify.json` per the "Manual verification
    instructions" rule above. These ship with the bundle and are listed in the manifest. There is
    no top-level checklist file; the review app's task list covers the whole bundle.
15c. **Review app** — **[new]** `tools/review/` (FastAPI + React/Vite/TS) presents each task's
    items, records OK/Problem per item and ACCEPT/REJECT per task, and appends every round to
    `assets/tasks/review_results.json`. Plan: [`../tools/review/PLAN.md`](../tools/review/PLAN.md).

## Verification (human-in-the-loop)

The assets are accepted by **human perception**, once: look at the images, listen to the audio
(and the music/world **previews**), glance at the tables. There is no automated regeneration or
acceptance harness, no determinism requirement, and the reviewer never re-extracts or decodes
anything. Each extraction step ships the `verify.json` items described in "Manual verification
instructions"; the human reviewer works through them in the review app to accept the bundle. Anything
perception misses will surface during port implementation and can be revisited then.

The following are **aids the implementer may run while extracting** (to gain confidence before
handing assets to the reviewer) — not gates:
- **Round-trip:** re-decode an emitted indexed PNG back to indices and compare with the raw
  bitplane decode; check RGBA PNG == palette-applied indices.
- **Manual cross-check:** re-derive a decoder's output by hand from the original data — offsets,
  counts, bit layouts read directly from `src/assets` and `src/` — and compare against the export.
- **Existing artifacts:** diff new sprite PNGs against the current `sprite_output/` (730 files) to
  confirm no pixel regressions from the `extract_sprites.py` extension.
- **Day/night reference (kept):** `experiment/shaders/` holds the standalone verified day/night
  decomposition (`fade_page.py` = verbatim port of `fade_page()`), the per-light-level RGB LUT
  (`daynight_lut.json`), baked frame bank + highlight masks, the two reference shaders, and
  `compare.py` (bit-exact proof). Retain it in the repo alongside `tools/`; do not delete.
- **Checksums:** `manifest.json` records SHA-256 per shipped file for integrity/provenance.

## Out of scope
- Any gameplay, rendering, engine, or porting code.
- Save-game data as a shipped asset (the **format** is documented via `decode_savegame.py` in the
  spec, but save files are user state, not game assets).
- Creative/visual changes — conversion must be exact.
