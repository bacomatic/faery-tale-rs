## RESEARCH.md

Human-readable reference notes extracted from `PLAN.md` so the plan stays task-focused.

Stable agent lookup keys live in `research_index.toml`.

## Maintenance workflow

- Add or update the human-readable note here first.
- Add or update a matching `[[entry]]` in `research_index.toml` with a stable `id`.
- Keep the `title` exactly aligned with this document section heading.
- Point `anchor` at the markdown heading slug in this file.
- Bump `last_updated` in `research_index.toml` when entries change.

---

## Game World & Map System: Data format

All game world data lives in `game/image`, an Amiga 880KB floppy disk image accessed as a flat file. `load_track_range(block, count, buf)` reads `count × 512` bytes from offset `block × 512`.

**Memory regions loaded from `game/image`:**

| Buffer | ADF source | Size | Description |
|---|---|---|---|
| `sector_mem` | `nd->sector`, 64 blocks | 32 768 B | 128 sectors × 256 bytes; tile indices |
| `map_mem` | `nd->region`, 8 blocks | 4 096 B | Region map; maps `(secx, secy)` → sector number |
| `terra_mem[0..512]` | `TERRA_BLOCK + nd->terra1`, 1 block | 512 B | Terrain type table, layer 1 |
| `terra_mem[512..1024]` | `TERRA_BLOCK + nd->terra2`, 1 block | 512 B | Terrain type table, layer 2 |
| `image_mem` | 4 tilesets × (5 planes × 8 blocks) | 81 920 B | Tile graphics; 4 groups × 64 tiles × 5 bitplanes × 64 B |

**Tile format:**
- 16 × 32 pixels, 5 bitplanes (→ 32 colours from a 32-entry palette)
- 64 bytes per plane per tile (2 bytes/row × 32 rows)
- `image_mem` layout: plane stride = `IPLAN_SZ` = 16 384 B; tileset stride within each plane = `QPLAN_SZ` = 4 096 B
- Tile `n` in group `g`, plane `p`: byte offset = `p × IPLAN_SZ + g × QPLAN_SZ + n × 64`

**Viewport and coordinate system:**
- World: up to `MAXCOORD = 32 768` units each axis (pixel coordinates)
- Image units: 16 px wide × 32 px tall per tile
- Sectors: 16 × 8 image units = 256 × 256 pixels (128 sectors per axis via `map_mem` grid)
- Screen playfield: 288 × 140 lores px, x-offset 16 px from left edge
- Viewport tile grid: `minimap[114]` = 19 columns × 6 rows (each short = tile index)
- `map_x = hero_x − 144`, `map_y = hero_y − 90` (hero centred)
- `img_x = map_x >> 4`, `img_y = map_y >> 5`

**Region table (`file_index[10]`):** Ten outdoor/indoor regions (F1–F10); each `struct need` holds: `image[4]` (4 tileset ADF block numbers), `terra1`, `terra2`, `sector`, `region`, `setchar`. Region 0–7 = outdoor, 8–9 = indoor/dungeon. `region_num` selects the active region.

**`genmini(img_x, img_y)`** (asm in `fsubs.asm`): Fills `minimap[114]` by walking the 19×6 tile grid, looking up each tile's sector via `map_mem`, then its byte value from `sector_mem`. Used as the tile index for rendering.

**`map_draw()`** (asm in `fsubs.asm`): Blits all 114 minimap tiles to the 5-plane screen bitplanes, column by column (19 strips × 6 tiles each). Tile `image_mem` offset = `plane_base + char_idx × 64`.

## Game World & Map System: Constants, addresses, and implementation notes

### Key constants and block addresses (region F4, region_num=3 is starting region)

```
file_index[3] = {320, 360, 400, 440, 2, 3, 32, 168, 21}
  image[0]=320, image[1]=360, image[2]=400, image[3]=440
  terra1=2, terra2=3, sector=32, region=168, setchar=21
TERRA_BLOCK = 149
```

So starting map data (region 3):
- sector_mem: ADF offset 32×512 = 16 384
- map_mem: ADF offset 168×512 = 86 016
- terra_mem[0]: ADF offset (149+2)×512 = 77 312
- terra_mem[512]: ADF offset (149+3)×512 = 77 824
- image[0] plane 0: ADF offset 320×512 = 163 840

### Notes

- The original renders via 5-plane Amiga bitplanes; we use RGBA32 SDL2 textures. The decode step (planar → packed pixel) is the same as used for IFF images in `bitmap.rs`.
- `minimap` in the original is a `short[114]` (tile index as signed 16-bit); in our port use `u8[114]` since tile indices fit in one byte (0–255 from `sector_mem`).
- Scroll: original uses Amiga copper-list `RxOffset`/`RyOffset` for sub-tile pixel scrolling; we achieve the same by shifting blit destination rects.
- `xreg` and `yreg` track which 64×32 sector block is currently loaded into `sector_mem`. Initially 0. Updated by `load_new_region()` as hero moves.

---

## Key Bindings: Original game key map

From `fmain.c` `letter_list[]` and the main game loop:

| Key (original) | Menu   | Action             |
|-----------------|--------|--------------------|
| Arrow keys      | —      | Movement (8 dirs)  |
| Numpad `1`–`9` | —      | Movement (8 dirs + center) |
| `0`             | —      | Fight / Attack     |
| `I`             | ITEMS  | List inventory     |
| `T`             | ITEMS  | Take / Pick up     |
| `?`             | ITEMS  | Look / Examine     |
| `U`             | ITEMS  | Use item           |
| `G`             | ITEMS  | Give item          |
| `Y`             | TALK   | Yell               |
| `S`             | TALK   | Say / Speak        |
| `A`             | TALK   | Ask                |
| `Space`         | GAME   | Pause toggle       |
| `M`             | GAME   | Map view           |
| `F`             | GAME   | Find (compass)     |
| `Q`             | GAME   | Quit               |
| `L`             | GAME   | Load game          |
| `O`             | BUY    | Food               |
| `R`             | BUY    | Arrow              |
| `8`             | BUY    | Vial               |
| `C`             | BUY    | Mace               |
| `W`             | BUY    | Sword              |
| `B`             | BUY    | Bow                |
| `E`             | BUY    | Totem              |
| `V`             | SAVEX  | Save game          |
| `X`             | SAVEX  | Exit / Load        |
| `F1`–`F7`      | MAGIC  | Cast spell 1–7     |
| `1`–`7`        | USE    | Use item in slot   |
| `K`             | USE    | Use special (key?) |
| `1`–`6` (KEYS) | KEYS   | Select key color   |

## Key Bindings: Design and compatibility notes

- The original game's `letter_list[]` is a flat array scanned linearly on each keypress — we replace this with a `HashMap` reverse index for O(1) lookup.
- Direction keys need special handling: the original tracks key-down/key-up separately (`keydir` set on press, cleared on release), so we need to track held-key state.
- The KEYS menu (`SelectKey1`..`SelectKey6`) is only active when `cmode == KEYS` in the original; our implementation can context-gate these actions.
- Buy menu keys are only relevant when a shop interface is open — scene-level filtering handles this.
- Numpad movement (`1`–`9`) should be first-class defaults, not secondary aliases.
- Controller mapping should remain logical-action based so keyboard/controller rebinding share one action graph.
- Preserve original one-fire-button gameplay semantics as baseline; extra controller buttons are optional shortcuts to existing actions.
- Cheat keys from the original (`B`, `.`, `R`, `=`, arrows-teleport) are intentionally excluded from the rebindable system and handled separately as debug/cheat commands.
