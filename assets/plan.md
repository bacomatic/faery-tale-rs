# Asset Extraction & Format-Conversion Pipeline

## Context

The project currently consumes the original 1987 Amiga *Faery Tale Adventure* data directly:
graphics, world data, audio, and gameplay tables are read out of the original ADF disk image
(`game/image`), external binary files (`game/songs`, `game/v6`, `game/fonts/`), and — for the
hardcoded tables — out of the C source (`fmain.c`, `fmain2.c`) / Rust constants / `faery.toml`.

A **separate, future porting effort** will reimplement the game. That effort must be
**language- and implementation-neutral** and must run **without** the original game assets or
any source code. This effort produces exactly one thing: a **self-contained `assets/` bundle**
in modern, open formats, plus the **tooling** that generates it (kept for future refinement).

**Hard requirement: pixel-/byte-exact fidelity.** A 16×N sprite must convert to the identical
pixels; every table value and palette entry must match the original bit-for-bit. This is a
*format conversion only* — no gameplay, rendering, or engine work, and no creative changes.

### Decisions locked with the user
- **Exporter language:** Python, built by extending the existing `tools/` (kept separate from
  `faery-tale-rs`; retained for future use). The Rust decoders in the sibling `faery-tale-rs`
  checkout are **not** the producer but are used as a verification oracle (see Verification).
- **Graphics:** emit **both** indexed PNG (exact 0–31 index per pixel) **+** palette files
  (original 12-bit Amiga values) **and** baked RGBA PNG. Document which palette effects require
  the indexed path.
- **Sprites:** per-frame PNGs **+** a packed sheet **+** a JSON atlas, **one atlas per actor
  type** (no global pack).
- **Data/metadata format:** **JSON** for everything (palettes, sector/map, terra flags,
  animation/stat tables, sprite atlases, narrative text, audio metadata).
- **Audio:** **structured synth data** (note/event streams + 8 waveforms + 10 ADSR envelopes as
  JSON) **+** PCM SFX as **WAV**. No baked music render.
- **Deliverable:** a **committed `assets/` directory** in this repo (research branch).
- **Docs:** a **full per-resource format spec** (markdown) so the bundle is self-sufficient.

## Source data locations (read-only inputs)

| Input | Path | Contains |
|---|---|---|
| ADF disk image | `faery-tale-rs/game/image` (901,120 B = 1760×512) | tiles, sprite cfiles, sector/map/terra, shadow masks, 6 PCM SFX (blocks 920–930) |
| Music | `faery-tale-rs/game/songs` (5,984 B) | 28 packed track event streams |
| Instruments | `faery-tale-rs/game/v6` (4,628 B) | 8×128B waveforms + 10×256B envelopes |
| Fonts | `faery-tale-rs/game/fonts/` | Amiga `.font` glyph data |
| IFF screens | `faery-tale-rs/game/` (`p1a`, `p1b`, `p2a`, `hiscreen`, …) | intro/placard/hi-score images |
| Hardcoded tables | `faery-tale-research/src/fmain.c`, `fmain2.c`, `ftale.h` | statelist, encounter_chart, inv_list, weapon/treasure probs, diroffs, fallstates, setfig_table, file_index, trans_list, palettes |
| Narrative text | `faery-tale-rs/faery.toml` `[narr]` (cross-check `reference/.../narr-asm-*.md`) | event/place/inside messages, speeches |

The exporter must take a configurable `--game-dir` (default `../faery-tale-rs/game`) and
`--src-dir` (default `src/`) so it never hardcodes the sibling path. Create a `game/` symlink →
`../faery-tale-rs/game` for the existing `extract_sprites.py` default, OR pass `--image`.

## Output layout (committed `assets/`)

```
assets/
  manifest.json                 # master index: every file, type, checksum, source ref
  FORMATS.md                    # full per-resource format spec (links into formats/)
  formats/                      # one .md per resource type (field semantics, units, conventions)
  palettes/
    pagecolors.json  textcolors.json  introcolors.json  sun_colors.json
    region_overrides.json       # per-region color-31 variants (desert/dungeon)
    # each: {index, rgb4 (0x0RGB Amiga OCS 12-bit), rgba8} so both exact + convenient forms exist
  tiles/                        # background tile atlas, per region
    region_<NN>/atlas_indexed.png  atlas_rgba.png  atlas_highlightmask.png  tiles.json
  sprites/                      # one folder + atlas PER actor type (18 cfiles)
    <actor>/frame_000.png …     <actor>_sheet.png   <actor>_highlightmask.png   <actor>.json (atlas: frame→rect, w/h, transparency=index 31)
  masks/                        # shadow/collision masks (indexed PNG + JSON bit layout)
  world/                        # sector maps, region maps, terra/collision flags (JSON)
  audio/
    music/<track>.json          # event streams (note/rest/instrument/tempo/end)
    instruments/waveforms.json  envelopes.json
    sfx/sfx_<n>.wav
  fonts/<font>/glyphs/*.png  <font>.json   # glyph atlas + metrics (y_size,baseline,char_loc,width)
  screens/<name>.png            # IFF intro/placard/hi-score images → RGBA PNG
  shaders/                      # reference shaders for the shader-doable palette effects
    fade_to_black.glsl  daynight_dim.glsl  region_crossfade.glsl
    moonlight_blue.glsl  green_jewel.glsl
    daynight_live.glsl  daynight_bank.glsl   # full day/night incl. veg boost (RGBA + highlight_mask)
    README.md                   # maps each effect → indexed-path vs RGBA+shader; pseudocode
  tables/                       # all hardcoded gameplay tables → JSON
    statelist.json encounter_chart.json inv_list.json weapon_probs.json
    treasure_probs.json rand_treasure.json diroffs.json fallstates.json
    setfig_table.json file_index.json trans_list.json
  text/
    event_msg.json speeches.json place_msg.json inside_msg.json
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
   `offset(T,P,R) = (T/64)*20480 + P*4096 + (T%64)*64 + R*2` (matches `tile_atlas.rs`; tiles are
   16×16, verified in `experiment/shaders/`). Emit indexed PNG + RGBA PNG + a **1-bit highlight
   mask** PNG (`atlas_highlightmask.png`: 1 where index ∈ 16–24) + `tiles.json` per region. The mask
   drives the night vegetation boost on the RGBA day/night path (Graphics §5a). Region→image-group
   block numbers come from `file_index[]` / `faery.toml [[world.region]]`.
3. **Palettes** — **[new]** `tools/extract_palettes.py`. Extract `pagecolors`, `textcolors`,
   `introcolors`, `sun_colors`, `blackcolors` from `fmain.c`, plus per-region color-31 overrides
   (`fade_page` in `fmain2.c`: region 4 = 0x0980, region 9 = 0x0445, else 0x0bdf). Emit both
   `rgb4` (the Amiga OCS 12-bit `0x0RGB` value) and `rgba8` per entry. Reuse `extract_table.py`
   to pull the raw C arrays.
4. **Shadow/collision masks** — **[new]** `tools/extract_masks.py`. 256 entries × 64 B (32 rows ×
   2 B, 1 bit/pixel) from ADF. Emit indexed PNG + JSON bit layout.
5. **IFF screens** — **[new]** `tools/extract_screens.py`. Parse IFF/ILBM (BMHD/CMAP/BODY,
   ByteRun1) for intro/placard/hi-score images → RGBA PNG. (Mirror `iff_image.rs` logic.)
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
    `shaders/README.md` documents inputs/uniforms for each.

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

### Audio
7. **Music event streams** — **[new]** `tools/extract_music.py`. Parse `game/songs`: 28 tracks,
   each `i32 packlen` (BE) + `packlen×2` bytes of `(command,value)` events. Decode to JSON
   (note/rest/set-instrument/tempo/end + loop flag). Encode the Paula period table reference.
8. **Instruments** — same tool. Parse `game/v6`: bytes 0–1023 = 8×128 signed waveform samples;
   1024–3583 = 10×256 envelope tables → `waveforms.json`, `envelopes.json`.
9. **SFX** — **[new]** `tools/extract_sfx.py`. 6 samples from ADF blocks 920–930, 8-bit PCM →
   WAV (document original Paula playback rate ≈ 8000 Hz; do **not** resample — store native).

### Fonts
10. **Amiga fonts** — **[new]** `tools/extract_fonts.py`. Parse `.font` + DiskFont (ID 0x0F80):
    glyph bitmaps → per-glyph PNG + packed atlas, metrics JSON (`y_size`, `baseline`, `modulo`,
    `lo_char`/`hi_char`, `char_loc` offset+width). Mirror `font.rs`.

### Tables & text (extract C/Rust constants → JSON)
11. **Gameplay tables** — `tools/extract_table.py` **[reuse/extend]** (generic C-array
    extractor). Export each to `assets/tables/*.json`: `statelist` (87×4), `encounter_chart`
    (11×6), `inv_list` (36), `weapon_probs` (32), `treasure_probs` (40), `rand_treasure` (16),
    `diroffs` (16), `fallstates` (24), `setfig_table` (14×3), `file_index` (10×9), `trans_list`
    (9×4). Field names/semantics from the inventory + `reference/RESEARCH-data-structures.md`.
12. **Item/quest data** — `tools/extract_item_effects.py`, `tools/extract_quest_data.py`
    **[reuse]** → fold their JSON into `assets/tables/`.
13. **Narrative text** — **[new]** `tools/extract_text.py`. Pull `[narr]` strings from
    `faery.toml` → `event_msg.json`, `speeches.json`, `place_msg.json`, `inside_msg.json`.
    Preserve `%` (player name) and `$` (target) placeholders; document them in the spec.

### Orchestration & docs
14. **Manifest + driver** — **[new]** `tools/build_assets.py`. Runs every extractor in order,
    writes `assets/manifest.json` (path, type, byte size, SHA-256, source reference per entry).
15. **Format spec** — **[new]** author `assets/FORMATS.md` + `assets/formats/*.md`: one section
    per resource type covering field meanings, units, coordinate systems, the
    **transparency convention (sprite index 31)**, the **palette-effects matrix** (which effects
    are shader-doable on RGBA and how — the night vegetation boost on palette indices 16–24 needs
    the **1-bit `highlight_mask`** that ships with each tile atlas and sprite sheet, not the indexed
    path; cross-linked to `assets/shaders/` and `experiment/shaders/FINDINGS.md`), and the
    **audio synth model** (period table, VBL tempo, envelopes). Document the `highlight_mask` format
    (1 bit/pixel, set where source index ∈ 16–24, transparency follows index 31) under
    `formats/palettes.md`.

## Verification

- **Round-trip:** re-decode each emitted indexed PNG back to indices and assert equality with the
  raw bitplane decode; assert RGBA PNG == palette-applied indices.
- **Rust oracle (cross-check):** where a Rust decoder exists (`tile_atlas.rs`, `iff_image.rs`,
  `palette.rs`, `songs.rs`, `audio.rs`, `font.rs`), dump its in-memory output (small Rust test
  harness or existing tests) and **diff** against the Python export. This catches any drift
  between the chosen Python producer and the canonical tested decoders.
- **Existing artifacts:** diff new sprite PNGs against the current `sprite_output/` (730 files)
  to confirm no pixel regressions from the `extract_sprites.py` extension.
- **Day/night reference (kept):** `experiment/shaders/` holds the standalone, verified day/night
  decomposition (`fade_page.py` = verbatim port of `fade_page()`), the per-light-level RGB LUT
  (`daynight_lut.json`), baked frame bank + highlight masks, the two reference shaders, and
  `compare.py` (bit-exact proof). Ports use it to verify their RGBA + `highlight_mask` day/night path
  against the original code. Retain it in the repo alongside `tools/`; do not delete.
- **Checksums:** `build_assets.py` records SHA-256 in the manifest; re-running must be
  deterministic (stable byte output).
- **Tool tests:** add `pytest` cases under `tools/tests/` for each new extractor (offsets,
  counts, BE parsing, palette conversion `0xF→0xFF`).
- **End-to-end:** `tools/run.sh build_assets.py --game-dir ../faery-tale-rs/game` regenerates the
  full bundle clean; `manifest.json` validates; spot-open a tile atlas, a sprite sheet, a screen,
  and play one SFX WAV.

## Out of scope
- Any gameplay, rendering, engine, or porting code.
- Save-game data as a shipped asset (the **format** is documented via `decode_savegame.py` in the
  spec, but save files are user state, not game assets).
- Creative/visual changes — conversion must be exact.
