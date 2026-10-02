# Faery Tale Adventure — asset bundle format spec

This bundle is the sensory content of *The Faery Tale Adventure* (MicroIllusions, 1987, Amiga),
extracted losslessly from the original disk image and sources: pixels, colours, samples, glyphs
and text. Behaviour is **not** in the bundle — game rules and their constants are specified in
`reference/logic/` of the research repo. Every JSON file carries `source`/`citations` fields
(`src/<file>:<line>[-<line>]`) pointing at the original code that defines the data.

| Resource | Directory | Spec |
|---|---|---|
| Palettes, colour conversion, highlight mask, palette-effects matrix | `palettes/` | [formats/palettes.md](formats/palettes.md) |
| Sprites: actor sets, objects/items, weapon & effect overlays, raw sheets | `sprites/` | [formats/sprites.md](formats/sprites.md) |
| Background tiles: per-region atlases, master atlas, collision | `tiles/` | [formats/tiles.md](formats/tiles.md) |
| Shadow (occlusion) masks | `masks/` | [formats/masks.md](formats/masks.md) |
| Maps: overworld, interiors, dungeons, astral plane | `maps/` | [formats/maps.md](formats/maps.md) |
| IFF screens (title, story pages, victory, hi-score) | `screens/` | [formats/screens.md](formats/screens.md) |
| Narrative text | `text/` | [formats/text.md](formats/text.md) |
| Music, instruments, sound effects — and the synth model | `audio/` | [formats/audio.md](formats/audio.md) |
| Fonts | `fonts/` | [formats/fonts.md](formats/fonts.md) |
| Reference shaders + light-level renders | `shaders/` | [formats/shaders.md](formats/shaders.md) |

## Conventions that apply everywhere

**Colour.** The Amiga OCS palette is 12-bit; a colour is written `rgb4` = `"0x0RGB"` (one hex
nibble per channel). `rgba8` is the 8-bit expansion by nibble replication: `n → n*17`
(`0xF → 255`, `0x1 → 17`), alpha 255. The conversion is exact and reversible (`channel // 17`),
which is what lets shaders recover the original nibble from an RGBA texture
([palettes.md](formats/palettes.md)).

**Indexed PNGs are verbatim.** Every indexed PNG (`atlas_indexed.png`, `*_sheet.png`, `frame_*.png`,
`items/*.png`) stores the original 5-bit palette index per pixel (PNG colour type 3, 32-entry
PLTE from `palettes/pagecolors.json`). They round-trip to the source bitplanes losslessly.

**Transparency: index 31.** On everything drawn *on top of the background* (sprites, items,
overlays) palette index 31 is the key colour and is written transparent (PNG `tRNS`). Background
tiles are the background: their index 31 is opaque and takes the region's colour-31 value
(`palettes/region_overrides.json`).

**1-bit masks.** `*_highlightmask.png`, `*_silhouettemask.png`, `atlas_shadowmask.png`,
`masks/mask_*.png` and font glyphs are 1-bit greyscale PNGs: set bit = white/opaque, clear bit =
transparent (`tRNS` 0). The meaning of a set bit is given in each spec.

**Sheets and atlases** lay frames or tiles in a grid of fixed cells, `columns` per row, cell
`n` at column `n % columns`, row `n // columns`; JSON `rect`/`x,y,w,h` give pixel rectangles
`[x, y, w, h]` with the origin at the top-left, y down.

**Units.** Pixels are the game's low-resolution pixels (320×200 playfield). World/map
coordinates are in pixels unless the field says `_tile` or `_sector`. A tile is **16×32** px.
Timing is **NTSC**: the game tick is the 60 Hz vertical blank, the Paula clock is 3,579,545 Hz.

**Previews are not data.** Every `previews/` directory (`maps/*/preview.png`, `audio/*/previews/`,
`text/previews/`, `shaders/previews/`) holds non-authoritative renders for humans; the JSON/PNG/WAV
next to them is the deliverable. They ship because they are useful, and they are listed in the
manifest like any other file.

**`verify.json`.** Each resource directory has a `verify.json` with the human-review items used to
accept the bundle (schema: `tools/review/PLAN.md` in the repo). It is informational for consumers.
