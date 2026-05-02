## 2. World Structure

### 2.1 Coordinate System

- Full world coordinate space: X range 0–32767 (`MAXCOORD` = 0x7FFF), Y range 0–40959 (0x9FFF)
- Coordinates wrap at world boundaries (the world is a torus)
- Tile width: 16 pixels, tile height: 32 pixels
- Sub-tile viewport offsets: `RxOffset = map_x & 15` (0–15), `RyOffset = map_y & 31` (0–31)

Coordinate hierarchy:

| Level | Variable(s) | Conversion | Range |
|-------|------------|------------|-------|
| Pixel | `map_x`, `map_y` | — | 0–32767 (X), 0–40959 (Y) |
| Tile | `img_x`, `img_y` | `map_x >> 4`, `map_y >> 5` | — |
| Sector | — | `(tile_x >> 4) - xreg`, `(tile_y >> 3) - yreg` | 0–63 / 0–31 |
| Region | `region_num` | `(sector_x >> 6) & 1 + ((sector_y >> 5) & 3) * 2` | 0–9 |

### 2.2 Region System

The world is divided into **10 regions**:
- Regions 0–7: Outdoor overworld arranged in a 2×4 grid
- Region 8: Building interiors
- Region 9: Dungeon interiors

Each region has its own asset configuration defined in the `file_index[10]` table using `struct need`:

| Field | Type | Purpose |
|-------|------|---------|
| `image[4]` | u16[4] | 4 image bank file indices |
| `terra1` | u16 | Terrain data file 1 |
| `terra2` | u16 | Terrain data file 2 |
| `sector` | u16 | Sector data file |
| `region` | u16 | Region data file |
| `setchar` | u16 | Set-character file needed |

Region index mapping:

| Index | Name | Description |
|-------|------|-------------|
| 0 | F1 — Snow | Northern ice region |
| 1 | F2 — Witch wood | Dark forest |
| 2 | F3 — Swamp | Marshlands |
| 3 | F4 — Plains | Central grasslands (contains village of Tambry) |
| 4 | F5 — Desert | Burning Waste and hidden city |
| 5 | F6 — Bay/city | Coastal area with Marheim |
| 6 | F7 — Volcanic | Lava fields |
| 7 | F8 — Forest | Southern woodlands |
| 8 | F9 — Buildings | All indoor building interiors |
| 9 | F10 — Dungeons | All cave and dungeon interiors |

### 2.3 Two-Level Map Hierarchy

1. **Region map** (`map_mem`, 4 KB): 128×32 grid of sector indices. Each outdoor region occupies a 64-wide × variable-high band.
2. **Sector data** (`sector_mem`, 32 KB): 256 sectors × 128 bytes each. Each sector is a **16×8 grid of tile indices**.

The `genmini` function resolves world pixel coordinates through this hierarchy: pixel → tile → sector (via region map) → tile index (via sector data), filling the 19×6 `minimap[]` buffer that `map_draw` renders directly.

### 2.4 World Objects

Each world object uses `struct object` (6 bytes):

| Field | Type | Size | Purpose |
|-------|------|------|---------|
| `xc` | u16 | 2 | World X coordinate |
| `yc` | u16 | 2 | World Y coordinate |
| `ob_id` | u8 | 1 | Object type ID |
| `ob_stat` | u8 | 1 | Status (0=inactive, 1+=active) |

Up to 250 objects per sector. Two arrays: `ob_listg[]` (global outdoor objects) and `ob_list8[]` (indoor objects).

### 2.5 Region Loading

Region transitions are triggered by outdoor boundary crossing (`gen_mini`), door transitions, or respawn.

- `MAP_FLUX` / `MAP_STABLE`: `new_region < NO_REGION(10)` means transition in progress.
- `load_all()`: Blocking loop — `while (MAP_FLUX) load_new_region()`.
- `load_next()`: Non-blocking incremental loader called during Phase 13 of the main loop.
- `load_new_region()`: Loads sector data, region map, terrain blocks, and 5 image planes incrementally. Desert gate: if `new_region == 4` and `stuff[STATBASE] < 5`, desert map squares are blocked.

---


