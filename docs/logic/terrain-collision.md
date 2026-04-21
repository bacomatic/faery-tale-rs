# Terrain & Collision — Logic Spec

> Fidelity: behavioral  |  Source files: fsubs.asm
> Cross-refs: [RESEARCH §6](../RESEARCH.md#6-terrain--collision), [movement.md#proxcheck](movement.md#proxcheck), [doors.md#doorfind](doors.md#doorfind), [astral-plane.md#find_place](astral-plane.md#find_place), [carrier-transport.md](carrier-transport.md), [_discovery/terrain-collision.md](../_discovery/terrain-collision.md)

## Overview

This spec documents the three low-level terrain primitives implemented in
`fsubs.asm`. They are called from almost every other logic doc:

- [`px_to_im`](#px_to_im) — the core terrain query. Converts an absolute
  world pixel `(x, y)` into a terrain code `0..15` by: selecting one of 8
  sub-tile collision bits from the low bits of `x`/`y`; translating
  `(x, y)` to image-tile and sector coordinates; looking up a sector id
  from `map_mem`; indexing `sector_mem` for the image-tile id; and finally
  reading the terrain high-nibble and sub-tile mask from `terra_mem`. Open
  space returns `TERRAIN_OPEN = 0`. Used by walk/still/missile/AI code paths,
  by encounter placement, and by carrier spawn checks.
- [`prox`](#prox) — two-probe collision test around the sprite's foot. Calls
  `px_to_im` at `(x+4, y+2)` and `(x-4, y+2)` and returns the first blocking
  terrain code encountered, or 0 if both probes pass. The two probes use
  asymmetric blocking thresholds (right probe blocks on code ≥ 10; left
  probe blocks on ≥ 8). The C wrapper [`proxcheck`](movement.md#proxcheck)
  layers actor-vs-actor bounding-box tests on top of this.
- [`mapxy`](#mapxy) — returns a pointer into `sector_mem` for a given
  image-tile coordinate. Used by [`doorfind`](doors.md#doorfind) to rewrite
  live door tiles when a key unlocks a door, and by any code path that needs
  to mutate the tile id under a given world position.

These three functions share the image→sector→tile decode pipeline. `px_to_im`
carries it through to the final terrain lookup; `mapxy` stops one step
earlier and returns the byte address of the tile id. Both honor the
column-wrap fixup that folds negative `secx` values past the region seam
into the left edge.

## Symbols

All numeric literals in the pseudo blocks below carry inline
`# fsubs.asm:LINE — meaning` annotations rather than being promoted to named
constants. The only behavioral magic number is the sub-tile bit seed `0x80`
and the image-tile footprint shifts (`>>4` for X, `>>5` for Y), both of
which are meaningful only in combination.

Proposed SYMBOLS.md additions (not yet registered; see final report):

- `sector_mem: bytes` — `fmain.c:643, 921` — 36864-byte allocation. First
  32768 bytes are 256 sector tiles at 128 bytes each (16×8 image-tile ids);
  remaining 4096 bytes are the region map grid.
- `map_mem: bytes` — `fmain.c:921` — alias for `sector_mem + SECTOR_OFF`.
  128×32 byte grid of sector ids covering the world.
- `terra_mem: bytes` — `fmain.c:928` — 1024-byte chip-RAM buffer. Two
  512-byte halves, each 128 entries × 4 bytes: `{maptag, terrain_rule,
  tile_mask, big_color}`.
- `SECTOR_OFF = 32768` — `fmain.c` — byte offset within `sector_mem` where
  the region map grid begins.

Globals already in [SYMBOLS.md](SYMBOLS.md) used here: `xreg`, `yreg`,
`TERRAIN_BLOCKED`.

## px_to_im

Source: `fsubs.asm:542-620`
Called by: `prox`, `walk_step`, `still_step`, `missile_step`, `update_environ`, `find_place`, `doorfind`, `set_encounter`, `actor_tick`
Calls: `xreg`, `yreg`, `map_mem`, `sector_mem`, `terra_mem`

```pseudo
def px_to_im(x: u16, y: u16) -> u8:
    """Terrain code 0..15 at world pixel (x, y); 0 means open."""
    # Stage 1 — sub-tile bit selection. The tile_mask byte in terra_mem has
    # 8 bits, one per 8x16 sub-region of the 16x32 image tile. Pick the
    # bit this pixel falls inside. fsubs.asm:548-559.
    tbit = 0x80                                         # fsubs.asm:549 — start at bit 7
    if (x & 8) != 0:                                    # fsubs.asm:550 — 8 = bit-3 of x
        tbit = tbit >> 4                                # fsubs.asm:552 — to bit 3
    if (y & 8) != 0:                                    # fsubs.asm:554 — 8 = bit-3 of y
        tbit = tbit >> 1                                # fsubs.asm:556 — halve
    if (y & 16) != 0:                                   # fsubs.asm:558 — 16 = bit-4 of y
        tbit = tbit >> 2                                # fsubs.asm:560 — quarter
    # Stage 2 — pixel -> image-tile -> sector coordinates. fsubs.asm:561-589.
    imx = x >> 4                                        # fsubs.asm:561 — 16 px per image tile X
    imy = y >> 5                                        # fsubs.asm:562 — 32 px per image tile Y
    secx = (imx >> 4) - xreg                            # fsubs.asm:564-566 — 16 img tiles per sector X
    # Column-wrap fixup: bit 6 means secx went negative or past 64.
    if (secx & 64) != 0:                                # fsubs.asm:568 — 64 = bit-6 test
        if (secx & 32) == 0:                            # fsubs.asm:570 — 32 = bit-5 test
            secx = 63                                   # fsubs.asm:571 — wrap to right edge (mask 63)
        else:
            secx = 0                                    # fsubs.asm:572 — wrap to left edge
    secy = (imy >> 3) - yreg                            # fsubs.asm:574-576 — 8 img tiles per sector Y
    if secy < 0:                                        # fsubs.asm:577 — row clamp low
        secy = 0
    if secy >= 32:                                      # fsubs.asm:578 — 32 = region-grid row count
        secy = 31                                       # fsubs.asm:579 — clamp to last row
    # Stage 3 — sector id lookup, then image-tile id within sector.
    map_index = (secy << 7) + secx + xreg               # fsubs.asm:581-584 — 128 sectors per row, add xreg origin
    sec_num = map_mem[map_index]                        # fsubs.asm:593 — byte = sector id 0..255
    local_imx = imx & 15                                # fsubs.asm:595 — 16 tiles per sector column
    local_imy = imy & 7                                 # fsubs.asm:596 — 8 tiles per sector row
    offset = (sec_num << 7) + (local_imy << 4) + local_imx   # fsubs.asm:598-602 — 128 bytes per sector
    image_id = sector_mem[offset]                       # fsubs.asm:604 — byte = landscape image-tile id
    # Stage 4 — terrain attribute lookup. 4 bytes per image tile:
    #   [+0] maptag, [+1] terrain-rule (hi nibble = type), [+2] tile_mask, [+3] big_color.
    terra_index = image_id << 2                         # fsubs.asm:607-608 — 4 bytes per entry
    mask_byte = terra_mem[terra_index + 2]              # fsubs.asm:610 — tile_mask byte
    if (mask_byte & tbit) == 0:                         # fsubs.asm:611-612 — this sub-tile is open
        return 0
    return terra_mem[terra_index + 1] >> 4              # fsubs.asm:614-615 — hi nibble = terrain type 0..15
```

The sub-tile mask is the reason terrain has 8 spatial sub-regions per image
tile rather than one. A half-blocking cliff or door frame is stored as an
image tile whose `tile_mask` has only the blocking sub-tiles set; the other
sub-tiles return 0 from `px_to_im` and are walk-through. The mask-rule low
nibble at `terra_mem[image_id*4+1] & 15` is used during rendering (occlusion
masking) and is not observed by `px_to_im` itself.

## prox

Source: `fsubs.asm:1590-1614`
Called by: `proxcheck`
Calls: `px_to_im`, `TERRAIN_BLOCKED`

```pseudo
def prox(x: u16, y: u16) -> u8:
    """Two-probe foot-level terrain block test. Returns the blocker code or 0."""
    # Right probe at (x+4, y+2). Blocks on TERRAIN_BLOCKED or on any
    # "non-applicable" code 10..15 (doors, crystal, unused). fsubs.asm:1592-1599.
    t = px_to_im(x + 4, y + 2)                          # fsubs.asm:1592-1595 — 4,2 = foot right offsets
    if t == TERRAIN_BLOCKED:                            # fsubs.asm:1596-1597 — hard wall
        return t
    if t >= 10:                                         # fsubs.asm:1598-1599 — 10 = threshold for codes 10..15
        return t
    # Left probe at (x-4, y+2). Same wall test, plus a stricter >= 8 gate
    # that catches lava (8) and pit (9) at the far-foot sample. Types 2..7
    # (water / slip / ice) never block here — they are walk-through with
    # environ side effects. fsubs.asm:1601-1609.
    t = px_to_im(x - 4, y + 2)                          # fsubs.asm:1601-1604 — -4,2 = foot left offsets
    if t == TERRAIN_BLOCKED:                            # fsubs.asm:1606-1607 — hard wall
        return t
    if t >= 8:                                          # fsubs.asm:1608-1609 — 8 = includes lava (8) and pit (9)
        return t
    return 0                                            # fsubs.asm:1611 — both probes clear
```

`prox` never returns codes 2..7 (water, ice, slippery): those are
walk-through terrains whose effects are applied separately by
`update_environ`. The asymmetry between the two probes (right blocks on
≥ 10, left blocks on ≥ 8) means lava-edge and pit-edge tiles are caught
only when the hero's left foot overlaps them, which produces the expected
sprite-centred "corner of the sprite bumps into danger first" behavior.
The per-actor lava/pit exception ("hero walks into lava; NPCs don't") is
applied by [`proxcheck`](movement.md#proxcheck) one layer up, not here.

## mapxy

Source: `fsubs.asm:1085-1130`
Called by: `doorfind`
Calls: `xreg`, `yreg`, `map_mem`, `sector_mem`

```pseudo
def mapxy(imx: u16, imy: u16) -> u32:
    """Return byte address within sector_mem of the tile id at image coord (imx, imy)."""
    # Same sector-decode as px_to_im Stage 2, but the input is already in
    # image-tile units (not pixels). fsubs.asm:1093-1096.
    secx = (imx >> 4) - xreg                            # fsubs.asm:1093-1095 — 16 img tiles per sector X
    # Column wrap fixup — identical to px_to_im. fsubs.asm:1097-1102.
    if (secx & 64) != 0:                                # fsubs.asm:1097 — 64 = bit-6 test
        if (secx & 32) == 0:                            # fsubs.asm:1099 — 32 = bit-5 test
            secx = 63                                   # fsubs.asm:1100 — wrap right (mask 63)
        else:
            secx = 0                                    # fsubs.asm:1101 — wrap left
    secy = (imy >> 3) - yreg                            # fsubs.asm:1104-1106 — 8 img tiles per sector Y
    # Row wrap fixup. mapxy wraps the row symmetrically instead of clamping;
    # this matches the way doorfind sweeps neighboring tiles near the seam.
    # fsubs.asm:1107-1112.
    if (secy & 32) != 0:                                # fsubs.asm:1107 — 32 = bit-5 test
        if (secy & 16) == 0:                            # fsubs.asm:1109 — 16 = bit-4 test
            secy = 31                                   # fsubs.asm:1110 — wrap bottom (mask 31)
        else:
            secy = 0                                    # fsubs.asm:1111 — wrap top
    map_index = (secy << 7) + secx + xreg               # fsubs.asm:1114-1117 — 128 sectors per row
    sec_num = map_mem[map_index]                        # fsubs.asm:1123 — sector id 0..255
    local_imx = imx & 15                                # fsubs.asm:1125 — 16 tiles per sector column
    local_imy = imy & 7                                 # fsubs.asm:1126 — 8 tiles per sector row
    offset = (sec_num << 7) + (local_imy << 4) + local_imx   # fsubs.asm:1128-1130 — 128 bytes per sector
    return offset                                       # fsubs.asm:1131-1132 — caller uses as &sector_mem[offset]
```

`mapxy` differs from `px_to_im` Stage 2 in two ways: (1) its input is in
image-tile units and so it skips the `>>4` / `>>5` pixel conversion, and
(2) it wraps `secy` symmetrically (both top and bottom edges) where
`px_to_im` clamps. Callers that mutate tiles — specifically
[`doorfind`](doors.md#doorfind) when a door is unlocked — use the returned
pointer to overwrite 1–4 consecutive bytes in `sector_mem` with the "open
door" tile ids. Because `map_mem` and `sector_mem` are shared chip-RAM, the
next call to `px_to_im` at that coordinate will see the new tile ids and the
renderer will pick up the new graphic on the next frame.
