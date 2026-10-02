# Fonts

Directory: `fonts/`. The two bitmap fonts the game draws text with, decoded from their Amiga
`DiskFontHeader` files into 1-bit glyph PNGs plus metrics.

| Font | Dir | Glyphs | Size | Used for |
|---|---|---|---|---|
| Amber/9 | `amber_9/` | 97 (codes 32–127 + default) | proportional, 9 px tall, baseline 7 | in-game scrolling messages, placard/story text (`LoadSeg("fonts/Amber/9")`, `src/fmain.c:774-775`) |
| topaz 8 | `topaz_8/` | 225 (codes 32–255 + default) | fixed 8×8, baseline 6 | status bar labels/menus, map-mode text. The game opens this from ROM (`OpenFont(&topaz_ta)`, `src/fmain.c:650, 778`); the shipped disk file is a stand-in with the same glyphs |

```
fonts/<font>/
  <font>.json          metrics + layout
  <font>_atlas.png     the tf_CharData strip: 1-bit, (modulo*8) × y_size px, glyph pixels at bit_offset
  glyphs/NNN.png       one 1-bit PNG per glyph, named by decimal code (065.png = 'A'); default.png = the font's default glyph
```

PNGs are 1-bit greyscale, set bit = ink (white/opaque), clear = transparent (`tRNS` 0). A glyph PNG
is `width` × `y_size` px; `width` 0 glyphs (Amber/9 space) have `file: null`.

## `<font>.json`

| Field | Meaning |
|---|---|
| `name`, `source_file` | id and the original file |
| `y_size`, `baseline`, `x_size` | `TextFont` height, baseline row (text is placed with its baseline at pen y, so the glyph top is at `y - baseline`), nominal width |
| `bold_smear`, `style`, `style_names`, `flags`, `flags_names` | `TextFont` style/flag fields (`FPF_PROPORTIONAL`, `FPF_DESIGNED`, …) |
| `lo_char`, `hi_char`, `glyph_count` | code range; the extra glyph is the default for out-of-range codes |
| `modulo` | `tf_Modulo`: bytes per row of the strip (`atlas` width = `modulo*8`) |
| `proportional`, `has_char_space`, `has_char_kern` | whether `tf_CharSpace` / `tf_CharKern` exist (Amber/9 yes, topaz no) |
| `atlas`, `atlas_size` | the strip PNG |
| `glyphs[]` | `{code, char, file, bit_offset, width, kern?, space?, overflows_strip_by?}` — `bit_offset`/`width` are the `tf_CharLoc` entry (bit column in the strip, bits wide); `kern` = `tf_CharKern` (pen advance before the glyph), `space` = `tf_CharSpace` (pen advance after); `overflows_strip_by` marks a glyph whose `bit_offset + width` runs past the strip row — the game's blitter reads on into the next row, and the PNG does the same (Amber/9's default glyph) |
| `game_use` | `{loaded_by, loaded_at[], role, used_at[]}` |
| `layout` | where everything sits in the hunk file (`disk_font_header_at`, `text_font_at`, `tf_char_data_at`, `tf_char_loc_at`, `tf_char_space_at`, `tf_char_kern_at`, `reloc32_offsets`, …) and `note` with the decoding rules |
| `dfh_file_id`, `dfh_revision`, `dfh_name` | `DiskFontHeader` fields (`dfh_name` is empty in both files) |

## Drawing text like `Text()`

For a proportional font (Amber/9), for each character: `pen_x += kern`; blit the glyph with its
top at `pen_y - baseline`; `pen_x += space`. For topaz-8 every glyph advances `x_size` = 8. The
game's wrap and `%` substitution rules are in [text.md](text.md); the placard previews in
`text/previews/` were rendered this way.
