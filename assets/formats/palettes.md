# Palettes, colour conversion, highlight mask, palette effects

Directory: `palettes/`. Sources: `src/fmain2.c:367-371` (`pagecolors`), `src/fmain.c:476-488`
(`textcolors`, `blackcolors`, `introcolors`), `src/fmain2.c:1569-1576` (`sun_colors`),
`src/fmain2.c:381-386` (colour-31 overrides).

## Palette files

| File | Entries | What it is |
|---|---|---|
| `pagecolors.json` | 32 | The playfield palette: every sprite and tile index refers to it. Faded every 4 ticks by `fade_page()` outdoors. |
| `textcolors.json` | 20 | The hi-res status/text viewport palette (`vp_text`); not affected by day/night. |
| `introcolors.json` | 32 | Palette the intro pages (`screens/page0`, `p*`) are shown with. |
| `blackcolors.json` | 32 | All `0x0000`; loaded to black a viewport before a transition. |
| `sun_colors.json` | 53 | The sunrise ramp swept over `screens/winpic` by `win_colors()` (see [screens.md](screens.md)). |
| `region_overrides.json` | — | Per-region value of playfield colour 31 (below). |

Palette files are JSON arrays of `{index, rgb4, rgba8}`:

```json
{"index": 2, "rgb4": "0x0e96", "rgba8": [238, 153, 102, 255]}
```

`rgb4` is the Amiga 12-bit colour `0x0RGB`; `rgba8` is `[r, g, b, 255]` with each channel
`nibble * 17`. Alpha is always 255 here — transparency is a property of the pixel *index* (31),
not of the colour.

### Colour 31 per region — `region_overrides.json`

`fade_page()` rewrites `pagecolors[31]` from the current region before every fade:

| Field | Value |
|---|---|
| `color_index` | `31` |
| `default` | `0x0bdf` light blue — every region not listed |
| `regions["4"]` | `0x0980` amber (desert) |
| `regions["9"]` | `0x0445` grey-blue (dungeons and caves) |
| `conditional_regions["9"]` | `0x00f0` bright green while `secret_timer != 0` (Crystal Orb): reveals the hidden passages drawn in colour 31 |

Each entry is `{index, rgb4, rgba8}` (+ `condition` text for the conditional one). The shipped tile
atlases already apply the per-region value to their `atlas_rgba.png`; the master atlas keeps the
two secret-passage tiles as extra variants ([tiles.md](tiles.md)).

## Highlight mask — `*_highlightmask.png`, `atlas_highlightmask.png`

A 1-bit greyscale PNG the same size as its indexed sheet/atlas. A pixel bit is **set (white)
where the source palette index is in 16..24 inclusive**; everything else, including the
transparent index 31, is clear (`tRNS` 0, so clear pixels are transparent). The mask is shipped
next to every actor sheet, object/weapon/effect sheet, region tile atlas and the master atlas.

Why it exists: `fade_page()` gives palette entries 16–24 an extra blue nudge at dusk and night
(`src/fmain2.c:412-413`). After baking a palette image to RGBA the index is gone, and this is the
only term of the day/night cycle that depends on it. The mask restores exactly that one bit per
pixel. Indices 16–24 are the foliage greens (16–19), water/shadow blues (20–22), cyan (23) and a
dark red (24) of `pagecolors`.

## Palette-effects matrix

Which palette manipulations of the original can be reproduced on prebaked RGBA, and what each needs.
The reference implementation of each is in [`shaders/`](../shaders/README.md); the proof that the
RGBA path is bit-exact is `experiment/shaders/FINDINGS.md` in the repo.

| Effect (source) | Driven by | Indexed path | RGBA path |
|---|---|---|---|
| Brightness scale with night floors r≥10 %, g≥25 %, b≥60 % (`fmain2.c:388-394, 408-410`) | `lightlevel` | recompute 32 colours | per pixel from recovered nibbles — exact |
| Moonlight blue `b1 = (b*bn + g2*g1)/100`, `g2 = (100-g)/3` (`fmain2.c:395, 410`) | `lightlevel` | same | per pixel — exact |
| **Vegetation night boost** on indices 16–24, +2/+1 blue (`fmain2.c:412-413`) | `lightlevel`, **palette index** | same | per pixel **only with the highlight mask** (or by baking one RGBA image per light level — `shaders/previews/`); an index-blind RGBA dim gets ~26 k tile pixels wrong by up to 34/255 |
| Green Jewel: red weight +200, `r1 = max(r1, g1)` (`fmain2.c:1655, 407`) | `light_timer` | same | per pixel — exact |
| Fade to/from black, 21 steps of 5 % (`fmain2.c:623-629`); intro per-channel fade (`fmain.c:2930`) | weights | same | per pixel — exact |
| Colour 31 per region / Crystal Orb reveal (`fmain2.c:381-386`) | region, `secret_timer` | swap entry 31 | baked into each region's atlas; secret tiles as master-atlas variants |
| `colorplay` teleport strobe: random entries 1..31 for 32 frames (`fmain2.c:425-432`) | random | 32 random palettes | needs the index — use the indexed PNGs |
| `win_colors` sunrise over `winpic` (`fmain2.c:1605-1636`) | `sun_colors` | drive the indexed image | needs the index — indexed path |

All fades are integer: each channel becomes a 4-bit value after truncating division, so the
original moves in visible steps. A float multiply looks smoother than the game did.
