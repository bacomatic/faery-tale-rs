## 26. Asset Formats & Data Loading

### 26.1 Disk Image (`image` file)

Single 901120-byte file (1760 sectors × 512 bytes). Key sector allocations:

| Sectors | Content | Size |
|---------|---------|------|
| 32–95 | Sector maps (outdoor regions) | — |
| 96–159 | Sector maps (indoor regions) | — |
| 149–159 | Terrain property tables | — |
| 160–199 | Region maps (5 ranges of 8 sectors each) | — |
| 200–879 | Image banks (40 sectors each, 17 distinct banks) | 20480 bytes/bank |
| 880 | Copy protection check sector | 512 bytes |
| 896–919 | Shadow masks | 12288 bytes (24 sectors) |
| 920–930 | Sound effect samples (6 samples) | 5632 bytes (11 sectors) |
| 931–955 | Setfig sprites (5 sets × 5 sectors) | — |
| 960–1171 | Enemy + carrier sprites | — |
| 1312–1501 | Object, raft, turtle, and player sprites | — |

### 26.2 File-Based Assets

| File | Content | Format |
|------|---------|--------|
| `v6` | Waveforms (1024 bytes: 8 × 128-byte waveforms) + envelopes (2560 bytes: 10 × 256-byte envelopes) | Raw binary |
| `songs` | 28 music tracks (7 songs × 4 channels) | Custom packed tracker |
| `fonts/Amber/9` | Proportional bitmap font (Amber 9pt) — used for in-game scrolling messages and placard text | Amiga hunk format |
| `hiscreen` | HUD/status bar graphics | IFF/ILBM |
| `page0` | Title screen | IFF/ILBM |
| `p1a`–`p3b` | Story page images (3 pairs) | IFF/ILBM |
| `winpic` | Victory image | IFF/ILBM |

The system ROM font **Topaz 8** is used for status bar labels, menu text, and map-mode text.

### 26.3 IFF/ILBM Format

`unpackbrush()` loads IFF ILBM images with the following chunk handling:

| Chunk | Handling |
|-------|---------|
| FORM | Validate as IFF container |
| ILBM | Subtype marker (no-op) |
| BMHD | Read bitmap header (dimensions, compression mode) |
| CMAP | **Skipped** — game uses hardcoded programmatic palettes, not embedded palette data |
| GRAB | **Skipped** |
| CAMG, CRNG | **Skipped** |
| BODY | Decompress into target bitmap |

**ByteRun1 decompression**:
- Control byte 0–127: Copy next (N+1) bytes literally
- Control byte −1 to −127: Repeat next byte (1−N) times
- Control byte −128: No-op (not handled by the original assembly routine; the C fallback does handle it; in practice the game's compressor never emits −128)

The `compress` global selects between raw copy (0) and ByteRun1 decompression. Data is bulk-read into `shape_mem` as a temporary buffer, then decompressed scanline-by-scanline into destination bitplanes.

### 26.4 Sprite Format

5 bitplanes of image data per frame, loaded contiguously into `shape_mem` (78000 bytes). A 1-bit mask plane is generated at load time by ORing all 5 image planes and inverting. Dimensions from `cfiles[]` table (width in 16px units, height in pixels).

Each sprite set stores image data followed by mask data:
- Image data for frame `inum`: `seq_list[type].location + (planesize × 5 × inum)`
- Mask data for frame `inum`: `seq_list[type].maskloc + (planesize × inum)`

### 26.5 Tileset Format

5-bitplane Amiga planar bitmap. 4 banks per region, 64 tiles per bank = 256 tiles total. Each tile: 16×32 pixels = 64 bytes per bitplane. Each bank: 40 disk sectors = 20480 bytes (5 planes × 4096 bytes/plane). Total: 81920 bytes (`IMAGE_SZ`).

### 26.6 Memory Buffer Sizes

| Buffer | Size (bytes) | Purpose |
|--------|-------------|---------|
| `image_mem` | 81920 | Tile image data (256 tiles × 5 planes) |
| `sector_mem` | 36864 | Sector map (32 KB) + region map (4 KB) |
| `terra_mem` | 1024 | Terrain attribute tables (2 × 512 bytes) |
| `shape_mem` | 78000 | Sprite sheet data (all character sprites); also used as temp IFF decompression buffer |
| `shadow_mem` | 12288 | Terrain occlusion masks |
| `sample_mem` | 5632 | Audio sample data (6 samples) |
| `wavmem` | 1024 | Waveform data (8 × 128 bytes) |
| `scoremem` | 5900 | Music score data (7 songs × 4 tracks) |
| `volmem` | 2560 | Volume envelope data (10 × 256 bytes) |

### 26.7 Palette Loading

All game palettes are managed programmatically — CMAP chunks in IFF files are always skipped:
- Playfield: 32 colors loaded from `pagecolors[]`, modulated by `fade_page()` for day/night
- Text bar: 20 colors loaded from `textcolors[]`
- Intro: `introcolors[]` used during `screen_size()` zoom animation

---


