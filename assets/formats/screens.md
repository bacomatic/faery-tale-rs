# IFF/ILBM screens

Directory: `screens/`. The 9 ILBM brushes the game loads with `unpackbrush()`
(`src/iffsubs.c:139-189`), decoded to RGBA PNGs plus `screens.json`.

| File | Size | Shown with palette | Role |
|---|---|---|---|
| `page0.png` | 320×200 | `introcolors` | title page; the playfield zooms open over it |
| `p1a.png` `p2a.png` `p3a.png` | 112×147 | `introcolors` | story pages 1–3, left leaf (drawn at pixel x 32, y 24) |
| `p1b.png` `p2b.png` `p3b.png` | 128×127, 136×76, 136×78 | `introcolors` | story pages 1–3, right leaf (x 168/160/160, y 29/29/33) |
| `winpic.png` | 320×200 | `win_colors` first frame | victory picture after the win placard |
| `hiscreen.png` | 640×57 (hi-res) | `textcolors` | the status-bar / HUD frame below the playfield |

**The PNG colours are the palette the game shows the image with, not the file's CMAP.** The
loader skips `CMAP`, `GRAB`, `CAMG` and `CRNG` (`src/iffsubs.c:157-159`), so the file palettes
are never used; `screens.json` keeps them (`cmap_rgb4`) and lists where they differ from what the
game displays (`palette.cmap_differs_at`). Masking is never read: brushes are drawn opaque, so all
PNG alpha is 255.

`winpic` is a special case: it is only ever seen under the `win_colors()` animation
(`src/fmain2.c:1605-1636`), which sweeps `palettes/sun_colors.json` through entries 2–27 over 55
frames (one per 9 ticks, the first held 60) and then fades to black. The PNG shows the first frame
(`i = 25`); to reproduce the animation use the indexed data in `cmap_rgb4`/`palette.rgb4` as a
32-entry LUT: `fader[0] = fader[31] = 0`, `fader[1] = fader[28] = 0xfff`,
`fader[j] = sun_colors[i + j]` for `j = 2..27` (0 when `i + j <= 0`), `fader[29]/[30]` = `0x800`/`0x400`
while `i > -14`, else `0x100*((i+30)/2)` / `0x100*((i+30)/4)`.

## `screens.json`

| Field | Meaning |
|---|---|
| `loader` | `{function, source, notes[]}` — how `unpackbrush` reads BMHD/BODY (ByteRun1, row bytes `((w+15)/8) & ~1`, `x` is a byte offset so `pixel_x = 8*x`) |
| `screens[]` | per image: `name`, `file`, `chunks[]` (IFF chunks present), `bmhd` (`width height x y n_planes masking compression pad1 transparent_color x_aspect y_aspect page_width page_height`), `cmap_rgb4[]` (the file's own CMAP as `0x0RGB`), `palette` (`{name, file, source, rgb4[], loaded_by, cmap_differs_at: [{index, cmap, game}]}` — the palette used for the PNG), `shown` (`{call, dest, byte_x, pixel_x, y, role, via?}` — where and how the game blits it) |
