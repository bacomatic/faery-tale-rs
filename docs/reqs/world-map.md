## 2. World & Map

### Requirements

| ID | Requirement |
|----|-------------|
| R-WORLD-001 | The game world shall use pixel coordinates with X range 0–32767 (`MAXCOORD` = 0x7FFF) and Y range 0–40959 (0x9FFF), with unsigned 16-bit wrapping at boundaries. |
| R-WORLD-002 | The world shall be divided into 10 regions: 8 outdoor (2×4 grid, indices 0–7), 1 building interior (index 8), 1 dungeon (index 9). |
| R-WORLD-003 | Region number shall be computed from tile-level sector coordinates using the formula: `region = (xs >> 6) & 1 + ((ys >> 5) & 3) * 2`, where xs and ys are derived from pixel coordinates via `tile_x = map_x >> 4`, `tile_y = map_y >> 5`, `sector_x = tile_x >> 4`, `sector_y = tile_y >> 3`. |
| R-WORLD-004 | Each region shall load its own tileset (4 image banks of 64 tiles each = 256 tiles), two terrain property tables (1024 bytes total), sector map (256 sectors × 128 bytes = 32768 bytes), and region map (4096 bytes). Asset configuration is defined by `file_index[10]`, which specifies 4 image bank references, 2 terrain table IDs, sector map start, region map start, and setfig character set ID. |
| R-WORLD-005 | Terrain properties shall be encoded as 4-byte entries: byte 0 = mask shape index (for terrain occlusion), byte 1 **upper nibble (bits 4–7) = terrain type (0–15, drives movement speed, water/ice/lava/pit physics, and collision)**, byte 1 **lower nibble (bits 0–3) = mask application rule (0–7, controls sprite occlusion behavior)**, byte 2 = sub-tile collision mask (8 sub-regions), byte 3 = big_color. |
| R-WORLD-006 | Crossing a region boundary shall trigger automatic region data reload. Region transitions occur via outdoor boundary crossing (`gen_mini`), door transitions, or respawn. Non-blocking incremental loading (`load_next`) shall be used during normal gameplay; blocking loading (`load_all`) for immediate transitions. |
| R-WORLD-007 | A minimap cache (19×6 = 114 entries) shall be maintained, mapping viewport tile positions to terrain tile IDs for fast terrain mask lookups during sprite compositing. The `genmini` function resolves world coordinates through the two-level map hierarchy to fill this buffer. |
| R-WORLD-008 | The map shall use a two-level hierarchy: a region map (128×32 grid of sector indices, 4 KB) and sector data (256 sectors × 128 bytes each, where each sector is a 16×8 grid of tile indices). |
| R-WORLD-009 | Each region shall support up to 250 world objects per sector, each described by a 6-byte `struct object` (x, y, object type ID, status byte). Two object arrays (`ob_listg[]`, `ob_list8[]`) shall track active objects. |
| R-WORLD-010 | Desert access restriction: if `region == 4` and the player has fewer than 5 gold statues (`stuff[STATBASE] < 5`), desert map squares shall be blocked. |
| R-WORLD-011 | Tiles shall be 16×32 pixels. Each region loads 4 image banks of 64 tiles each = 256 distinct tile IDs. Each bank occupies `IMAGE_SZ = 81920` bytes (256 tiles × 5 bitplanes × 64 bytes per plane). |
| R-WORLD-012 | The visible playfield shall be a 19×6 tile grid (304×192 pixels), matching the 320×200 raster with a 16-pixel horizontal scroll margin. |
| R-WORLD-013 | A 19×6 = 114-entry `minimap` cache shall map viewport tile positions to terrain tile IDs for fast terrain-mask lookups during sprite compositing. `genmini()` rebuilds this buffer on full redraws by resolving world coordinates through the two-level (region-map → sector) hierarchy. |

### User Stories

- As a player, I can walk seamlessly from one region to another without noticing data loads.
- As a player, I see different terrain tilesets when entering distinct regions (snow, desert, swamp, etc.).
- As a player, I cannot enter the desert until I have collected enough gold statues.

---


