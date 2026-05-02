## Appendices

### A. Constants Reference

| Constant | Value | Description |
|----------|-------|-------------|
| MAXCOORD | 0x7FFF (32767) | Maximum X coordinate |
| MAXSHAPES | 25 | Maximum sprites per rendering frame (per page) |
| PAGE_DEPTH | 5 | Bitplanes per game page (32 colors) |
| TEXT_DEPTH | 4 | Bitplanes per status bar (16 colors) |
| SCREEN_WIDTH | 288 | Visible playfield width (pixels) |
| PAGE_HEIGHT | 143 | Scanline where text viewport begins |
| TEXT_HEIGHT | 57 | HUD/status bar height (pixels) |
| PHANTA_WIDTH | 320 | Full bitmap width including scroll margins |
| RAST_HEIGHT | 200 | Full raster height per page |
| TILE_WIDTH | 16 | Tile width in pixels |
| TILE_HEIGHT | 32 | Tile height in pixels |
| TILES_X | 19 | Visible tile columns |
| TILES_Y | 6 | Visible tile rows |
| ACTOR_SLOTS | 20 | Maximum actor array size (`anim_list[20]`) |
| MAX_MISSILES | 6 | Maximum simultaneous missiles |
| MAX_OBJECTS_PER_SECTOR | 250 | World objects per sector |
| SHAPE_BYTES | 22 | Size of `struct shape` in bytes |
| MAGICBASE | 9 | First magic item inventory index |
| KEYBASE | 16 | First key inventory index |
| STATBASE | 25 | Gold statues inventory index |
| GOLDBASE | 31 | Gold inventory index |
| ARROWBASE | 35 | Arrow inventory index |
| EXT_COUNT | 22 | Extent entries scanned |
| DOORCOUNT | 86 | Total door entries |
| VOICE_SZ | 3584 | Audio waveform + envelope buffer size |
| SAMPLE_SZ | 5632 | Sound effect sample buffer size (6 samples) |
| IMAGE_SZ | 81920 | Full tileset image buffer size (256 tiles × 5 planes) |
| SHADOW_SZ | 12288 | Shadow/terrain occlusion mask buffer size |
| SECTOR_SZ | 36864 | Sector (32 KB) + region map (4 KB) buffer size |
| SHAPE_MEM_SZ | 78000 | Sprite sheet data buffer size |
| TERRA_MEM_SZ | 1024 | Terrain property table buffer size (2 × 512) |
| WAV_MEM_SZ | 1024 | Waveform data buffer size (8 × 128) |
| VOL_MEM_SZ | 2560 | Volume envelope buffer size (10 × 256) |
| SCORE_MEM_SZ | 5900 | Music score data buffer size |
| BACKSAVE_LIMIT | 5920 | Background save buffer capacity per page |
| DAYNIGHT_MAX | 24000 | Day/night counter wrap point (0–23999) |
| DAYNIGHT_INIT | 8000 | Initial daynight value (morning) on revive |
