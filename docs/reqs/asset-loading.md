## 23. Asset Loading

### Requirements

| ID | Requirement |
|----|-------------|
| R-ASSET-001 | The `image` file (901120 bytes, 1760 sectors × 512 bytes) shall be the primary data source for tilesets, terrain, sprites, shadow masks, and other binary assets. |
| R-ASSET-002 | `file_index[10]` (one per region) shall map regions to their 4 image bank sector addresses, 2 terrain table sector addresses, sector map start, region map start, and setfig character set ID. Each entry uses the `struct need` format: `image[4]`, `terra1`, `terra2`, `sector`, `region`, `setchar`. |
| R-ASSET-003 | `cfiles[18]` shall map sprite sets to disk sector addresses and dimensions (width in 16px units, height in pixels). |
| R-ASSET-004 | IFF/ILBM files (`page0`, `p1a`–`p3b`, `hiscreen`, `winpic`) shall be loaded with chunk parsing for FORM, ILBM, BMHD, and BODY. The CMAP chunk shall be skipped — the game uses hardcoded programmatic palettes, not embedded palette data. ByteRun1 RLE decompression shall handle control byte N ≥ 0 (copy N+1 literal bytes), N < 0 and ≠ −128 (repeat next byte 1−N times), and −128 (no-op). |
| R-ASSET-005 | Font (Amber 9pt) shall be loaded from hunk-format file `fonts/Amber/9`. The ROM font Topaz 8 is used for status bar and menu text. |
| R-ASSET-006 | Audio data: `v6` file contains waveforms (1024 bytes, 8 × 128-byte waveforms) + volume envelopes (2560 bytes, 10 × 256-byte envelopes); `songs` file contains 28 music tracks (7 songs × 4 channels); sound effect samples loaded from `image` sectors 920–930 (5632 bytes, 6 samples). |
| R-ASSET-007 | Region loading shall load 4 image banks (each 40 sectors = 20480 bytes), 2 terrain tables (each 512 bytes), sector map (32768 bytes), and region map (4096 bytes = 8 sectors), updating the minimap and performing any format conversion needed for display. |
| R-ASSET-008 | Shadow mask data (12288 bytes, 24 sectors from sectors 896–919) shall be loaded into `shadow_mem` for terrain occlusion during sprite compositing. |
| R-ASSET-009 | `shape_mem` (78000 bytes) shall be used as a temporary decompression buffer during IFF BODY loading, since shape loading and IFF image loading never overlap. |

### User Stories

- As a player, the game loads all regions, images, music, and fonts without errors from the original data files.
- As a player, story page images and the victory image display correctly using ByteRun1 decompressed IFF data.

---


