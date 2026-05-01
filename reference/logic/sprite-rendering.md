# Sprite Rendering — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c, fmain2.c
> Cross-refs: [RESEARCH §5](../RESEARCH.md#5-rendering--display), [_discovery/sprite-compositing.md](../_discovery/sprite-compositing.md), [game-loop.md#render_sprites](game-loop.md#render_sprites), [inventory.md](inventory.md), [combat.md](combat.md)

## Overview

This spec covers per-sprite size derivation, weapon-overlay encoding, and the
inventory items-page render. Together with the canonical render driver
[`render_sprites`](game-loop.md#render_sprites) it specifies every site in the
game loop that decides *how big* a sprite is, *which frame* of the OBJECTS
sheet a weapon overlay maps to, and *where* on the screen to put each
inventory icon. The blitter primitives themselves (`save_blit`, `mask_blit`,
`shape_blit`, `clear_blit`, `make_mask`, `maskit`, `BltBitMap`) are platform
APIs — they are documented in
[`_discovery/sprite-compositing.md`](../_discovery/sprite-compositing.md) and
treated as opaque calls here.

The 1987 implementation hardcodes five size-related decisions that any port
must reproduce verbatim to render correctly:

1. **Per-frame half-height list** for OBJECTS — `fmain.c:2477-2480`. The
   OBJECTS sheet stores all frames as 16-scanline rows, but a fixed list of
   `inum` values render as 8-scanline strips packed two-per-row. The list is
   `{0x1b, 8..12, 25, 26, 0x11..0x17}` plus any frame with the high bit
   (`inum & 128`) set.
2. **Bit-7 half-height flag** — `fmain.c:2479` and `fmain.c:2524`. When bit 7
   of `inum` is set, the same frame index is interpreted as the bottom 8
   scanlines of the row above; the renderer also nudges Y by +8.
3. **Forced terrain-mask extent** — `fmain.c:2570`. For the small-object set
   (OBJECTS with `inum < 8`) and for every weapon-overlay pass, `blithigh`
   is forced to 32 before the terrain-mask stamp loop runs. The override
   exists for the (now commented-out) `ystop` formula at `fmain.c:2575`; the
   live loop iterates by tile and is independent of `blithigh`, so the
   override is observable only if a port reintroduces height-dependent
   masking.
4. **Weapon-class base offsets** — `fmain.c:2440-2444`. The hand-weapon
   overlay frame is `statelist[inum].wpn_no + k` where `k` is hardcoded per
   class: bow `0`, mace `32`, sword `48`, Dirk `64`. The bow takes a separate
   per-frame `bow_x[]/bow_y[]` pixel-perfect offset table and a different
   inum-derivation path; the wand (k=5) takes a third path that picks the
   frame from `an.facing + 103`.
5. **Inventory items-page literal width** — `fmain.c:3136`. The
   [items-page render](#render_inventory_items_page) blits each icon with
   `BltBitMap(..., 16, h, ...)` — 16 pixels wide, regardless of which item
   is being drawn — and treats the OBJECTS sheet as a 5-plane bitmap with
   32-byte plane stride (one 16-px-wide × 16-scanline frame per plane).

The spec organises these as nine helper functions consumed by
[`render_sprites`](game-loop.md#render_sprites) in `game-loop.md` plus the
items-page render that lives in `do_option`'s `CMODE_ITEMS` branch.

## Symbols

Locals introduced in this file:

- `xsize, ysize: u8` — sprite dimensions in pixels (`xsize` is always a
  multiple of 16).
- `xstart, ystart, xstop, ystop: i16` — sprite bounding box in screen pixels
  before clipping.
- `xstart1, ystart1, xstop1, ystop1: i16` — same after clipping to the 320×174
  playfield.
- `xoff, yoff: i16` — clip offsets from `(xstart, ystart)` to
  `(xstart1, ystart1)`.
- `xbw, xew: i16` — left/right word indices (16-px units) of the clipped
  rect.
- `blitwide, blithigh: u8` — clipped width in 16-px words and height in
  scanlines.
- `cwide: u8` — sprite source modulus in bytes (`xsize / 8`).
- `ground: i16` — Y of the actor's feet in screen pixels (`ystart + 32`).
- `passmode: i8` — `0` = body pass, `1` = weapon-overlay pass.
- `pass: i8` — running pass counter inside the actor loop (0 → 1 → 2).
- `crack: i8` — small-shape backsave-slot index (0..4).
- `backalloc: object` — bump pointer into the per-page backsave pool.
- `aoff, boff, bmod, shift, wmask: i16` — blitter pointer/mod/shift state set
  by the clip helper for `mask_blit` and `shape_blit`.

**Proposed `SYMBOLS.md` additions** (registered in this pass):

- `bow_x: list[i8]`, `bow_y: list[i8]` — `fmain2.c:877-881` — 32-entry
  per-bow-frame pixel-perfect overlay offset tables.
- `bowshotx: list[i8]`, `bowshoty: list[i8]` — `fmain2.c:884-885` — 8-entry
  per-facing arrow spawn-velocity tables (used by missile_step; declared here
  for completeness because `fmain.c:206` extern's them alongside `bow_x`).
- `gunshoty: list[i8]` — `fmain2.c:886` — 8-entry per-facing fireball Y
  spawn offset.
- `statelist: list` — `fmain.c:154` — 87-entry `state` table mapping animation
  frame `inum` to `{figure, wpn_no, wpn_x, wpn_y}`.
- `inv_list: list` — `fmain.c:380-424` — 36-entry `inv_item` table; per-slot
  rendering metadata (`image_number`, `xoff`, `yoff`, `ydelta`, `img_off`,
  `img_height`, `maxshown`, `name`).
- `terra_mem: list[u8]` — `fmain.c:658` — 1024-byte terrain table indexed by
  4-byte stride: `[0]` = mask tile id, `[1]` = high-nibble terrain code +
  low-nibble occlusion code.
- `minimap: list[i16]` — `fmain.c` — 114-entry per-screen-tile remap into
  `terra_mem`.
- `bmask_mem: object` — `fmain.c:877` — single-plane compositing-mask buffer
  (max 96 scanlines × 5 words).
- `shadow_mem: object` — `fmain.c` — 12 KB terrain occlusion-tile pool
  (loaded from disk block 896).
- `pagea: object`, `pageb: object` — `fmain2.c:779` — off-screen scratch
  BitMaps; `pagea` is reframed by `render_inventory_items_page` as a
  read-only window onto `seq_list[OBJECTS].location`.
- `CBK_SIZE = (96 << 6) + 5` — `fmain.c:680` — `clear_blit` size word: 96
  scanlines × 5 words = max compositing-mask extent.
- `BACKSAVE_LIMIT_BYTES = 5920` — `fmain.c:2548` — `74*80`; per-page
  cumulative backsave budget. When exceeded the actor loop stops drawing.
- `SMALL_SHAPE_THRESHOLD = 64` — `fmain.c:2542` — `savesize` below this
  qualifies for the bitplane-tail fast path.
- `SMALL_SHAPE_SLOT_MAX = 5` — `fmain.c:2542` — at most five small shapes
  per page can use the bitplane-tail.
- `PLANE_TAIL_OFFSET = 7680` — `fmain.c:2543` — `192 * 40`; byte offset into
  a bitplane past the visible 192 scanlines.
- `PLAYFIELD_X_MAX = 319` — `fmain.c:2513` — rightmost visible column.
- `PLAYFIELD_Y_MAX = 173` — `fmain.c:2513` — bottommost visible scanline.
- `GROUND_OFFSET = 32` — `fmain.c:2413` — actor `rel_y` to feet-Y delta.
- `WPN_K_BOW = 0` — `fmain.c:2443` — base-frame offset for bow overlays.
- `WPN_K_MACE = 32` — `fmain.c:2440` — base-frame offset for mace.
- `WPN_K_SWORD = 48` — `fmain.c:2441` — base-frame offset for sword.
- `WPN_K_DIRK = 64` — `fmain.c:2442` — base-frame offset for Dirk.
- `WPN_WAND_INUM_BASE = 103` — `fmain.c:2436` — wand frame = facing + 103.
- `WPN_FIERY_DEATH_INUM = 0x58` — `fmain.c:2454` — OBJECTS frame for the
  fiery-death overlay.
- `INUM_BIT7_HALF_HEIGHT = 0x80` — `fmain.c:2479,2524` — high bit of `inum`
  marks half-height + Y+8 nudge.
- `OBJECTS_FRAME_NOMASK_LO = 100` — `fmain.c:2568` — special OBJECTS frames
  100..101 skip terrain masking.
- `OBJECTS_FRAME_NOMASK_HI = 102` — `fmain.c:2568` — exclusive upper bound.
- `RACE_NOMASK_A = 0x85` — `fmain.c:2569` — race code that skips terrain
  masking.
- `RACE_NOMASK_B = 0x87` — `fmain.c:2569` — race code that skips terrain
  masking.
- `FIERY_DEATH_RECT_X1 = 22833` — `fmain.c:2565` — island-of-no-return
  west edge.
- `FIERY_DEATH_RECT_X2 = 26428` — `fmain.c:2565` — east edge.
- `FIERY_DEATH_RECT_Y1 = 26425` — `fmain.c:2566` — north edge.
- `FIERY_DEATH_RECT_Y2 = 26527` — `fmain.c:2566` — south edge.
- `XTYPE_FIERY_DEATH = 84` — `fmain.c:2564` — extent encounter type for the
  fiery-death overlay.
- `XTYPE_BOW_OVERRIDE = 80` — `fmain.c:2401` — extents above 80 force a
  weapon pass even on dead/dying actors.
- `MASK_FORCE_HEIGHT = 32` — `fmain.c:2570` — `blithigh` override for the
  terrain-mask stamp loop on small objects and weapon overlays.
- `OCC_NEVER = 0` — `fmain.c:2585` — terrain occlusion code: never apply.
- `OCC_NOT_LEFT = 1` — `fmain.c:2586` — apply except when at sprite's left
  edge.
- `OCC_TOP_HALF = 2` — `fmain.c:2587` — apply only when feet sit in upper
  half of tile.
- `OCC_BRIDGE = 3` — `fmain.c:2588` — apply unless sector 48 and not
  leader-1 (FALL state always uses this).
- `OCC_NOT_LEFT_TOP = 4` — `fmain.c:2590` — combined left + top half.
- `OCC_LEFT_TOP_AND = 5` — `fmain.c:2591` — apply unless left edge AND top
  half.
- `OCC_FULL_IF_ABOVE = 6` — `fmain.c:2592` — full tile (id 64) if not
  topmost row.
- `OCC_TOP_QUARTER = 7` — `fmain.c:2593` — apply only when feet sit in
  top quarter.
- `BRIDGE_SECTOR = 48` — `fmain.c:2588` — sector index of bridge tile.
- `OCC_GROUND_THRESH_HALF = 35` — `fmain.c:2587,2590` — feet-Y boundary for
  top-half occlusion gate.
- `OCC_GROUND_THRESH_QUARTER = 20` — `fmain.c:2593` — feet-Y boundary for
  top-quarter occlusion gate.
- `OCC_FULL_TILE_ID = 64` — `fmain.c:2592` — `terra_mem` tile id used for
  full coverage.
- `INV_ICON_WIDTH = 16` — `fmain.c:3136` — fixed inventory-icon blit width.
- `INV_ICON_X_OFFSET = 20` — `fmain.c:3131` — left-margin shift added to
  every `inv_list[].xoff` when blitting.
- `OBJ_PLANE_STRIDE = 32` — `fmain.c:3122-3125` — bytes per plane in the
  OBJECTS sheet (one 16-px-wide × 16-scanline frame).
- `OBJ_FRAME_STRIDE = 80` — `fmain.c:3133` — bytes per logical frame slot
  in the inventory icon offset (16 wide × 5 planes).
- `INV_PAGE_HEIGHT = 8000` — `fmain.c:3119` — `InitBitMap(&pagea, 5, 16,
  8000)` — 5-plane × 16-byte rows × 500 lines (overstated; reads only into
  the OBJECTS data window).
- `INV_PAGE_WIDTH_BYTES = 16` — `fmain.c:3119` — bytewidth of `pagea`.
- `INV_BLIT_MINTERM = 0xC0` — `fmain.c:3136` — `BltBitMap` minterm = `D = A`.
- `INV_BLIT_MASK = 0xff` — `fmain.c:3136` — all 5 planes participate.
- `GOLDBASE = 31` — already in SYMBOLS.md (alias of `GOLDBASE`); the
  inventory loop only renders rows `0..GOLDBASE-1` (gold piles use a
  different render path).
- `struct ShapeClip` — see §4 of SYMBOLS.md.

## resolve_pass_params

Source: `fmain.c:2400-2409`
Called by: `render_sprites`
Calls: none

```pseudo
def resolve_pass_params(an: Shape, pass_count: i8) -> i8:
    """Compute passmode (0=body, 1=weapon) for the current pass given the
    actor's facing and weapon class. Body and weapon are drawn in an order
    that depends on facing so the weapon appears in front when the actor
    faces toward the camera and behind when the actor faces away."""
    if (an.facing - 2) & 4:                    # fmain.c:2402 — DIR_E..DIR_W (3..7) take XOR-flip
        passmode = pass_count ^ 1               # fmain.c:2402 — 1 = invert pass order
    else:
        passmode = pass_count                   # fmain.c:2403
    if an.weapon == 4 and an.state < 24:        # fmain.c:2404 — 4 = WEAPON_BOW; 24 = STATE_SHOOT1
        if (an.facing & 4) == 0:                # fmain.c:2405 — 4 = bit 2 of facing (north half)
            passmode = pass_count ^ 1            # fmain.c:2405 — 1 = invert
        else:
            passmode = pass_count                # fmain.c:2406
    return passmode
```

## needs_weapon_pass

Source: `fmain.c:2400-2401, 2408`
Called by: `render_sprites`
Calls: none

```pseudo
def needs_weapon_pass(an: Shape) -> bool:
    """True when the actor must run a second sprite pass (passmode=1) to
    draw the wielded weapon as an OBJECTS overlay."""
    if an.weapon <= 0:                          # fmain.c:2400
        return False
    if an.weapon >= 8:                          # fmain.c:2400 — 8 = WEAPON_TOUCH (monster-only, no overlay)
        return False
    if an.state < 15:                           # fmain.c:2401 — 15 = STATE_DEAD; alive actors always paint weapon
        return True
    if an.state >= 24:                          # fmain.c:2401 — 24 = STATE_SHOOT1; shooting actors keep their bow
        return True
    if xtype > 80:                              # fmain.c:2401 — 80 = XTYPE_BOW_OVERRIDE; high-extent zones force overlay
        return True
    return False
```

## select_atype_inum

Source: `fmain.c:2420-2466`
Called by: `render_sprites`
Calls: `bow_x`, `bow_y`, `statelist`, `WEAPON_BOW`, `WEAPON_WAND`

```pseudo
def select_atype_inum(an: Shape, i: i16, passmode: i8, pass_count: i8) -> ShapeClip:
    """Pick (atype, inum) for the current pass and apply the per-state
    weapon-pixel offsets on top of an.rel_x / an.rel_y. Returns a partially
    populated ShapeClip with fields xstart, ystart, ground, atype, inum
    set; the clip helper later fills the rest. The offscreen flag is set
    when a pass-specific override (e.g. fiery-death corpse) wants the loop
    to skip drawing this iteration."""
    clip = ShapeClip()                          # caller-supplied scratch
    clip.xstart = an.rel_x
    clip.ystart = an.rel_y
    clip.ground = an.rel_y + 32                 # fmain.c:2413 — 32 = GROUND_OFFSET; feet-Y in screen pixels
    inum = an.index                             # fmain.c:2396

    if passmode != 0:                           # fmain.c:2420 — weapon-overlay pass
        k = an.weapon
        if k == 4 and inum < 32:                # fmain.c:2422 — 4 = WEAPON_BOW; 32 = bow-frame count
            clip.xstart = clip.xstart + bow_x[inum]   # fmain.c:2423
            clip.ystart = clip.ystart + bow_y[inum]   # fmain.c:2423
        else:
            clip.xstart = clip.xstart + statelist[inum].wpn_x   # fmain.c:2425
            clip.ystart = clip.ystart + statelist[inum].wpn_y   # fmain.c:2426

        if k == 4 and inum < 32:                # fmain.c:2429 — bow inum derivation
            inum = inum / 8                     # fmain.c:2430 — 8 = frames per direction quadrant
            if inum & 1:                        # fmain.c:2431
                inum = 30                       # fmain.c:2431 — 30 = OBJECTS frame: bow drawn east-west
            elif inum & 2:                      # fmain.c:2432
                inum = 0x53                     # fmain.c:2432 — 0x53 = OBJECTS frame: bow drawn north
            else:
                inum = 0x51                     # fmain.c:2433 — 0x51 = OBJECTS frame: bow drawn south
        elif k == 5:                            # fmain.c:2435 — 5 = WEAPON_WAND
            inum = an.facing + 103              # fmain.c:2436 — 103 = WPN_WAND_INUM_BASE
            if an.facing == 2:                  # fmain.c:2437 — DIR_NE
                clip.ystart = clip.ystart - 6   # fmain.c:2437 — 6 = NE wand vertical anchor adjust
        else:                                   # fmain.c:2439 — hand weapons (Dirk / mace / sword)
            if k == 2:                          # fmain.c:2440 — 2 = WEAPON_MACE
                k = 32                          # fmain.c:2440 — 32 = WPN_K_MACE
            elif k == 3:                        # fmain.c:2441 — 3 = WEAPON_SWORD
                k = 48                          # fmain.c:2441 — 48 = WPN_K_SWORD
            elif k == 1:                        # fmain.c:2442 — 1 = WEAPON_DIRK
                k = 64                          # fmain.c:2442 — 64 = WPN_K_DIRK
            elif k == 4:                        # fmain.c:2443 — 4 = WEAPON_BOW (state >= SHOOT1 fallthrough)
                k = 0                           # fmain.c:2443 — 0 = WPN_K_BOW
            inum = statelist[inum].wpn_no + k   # fmain.c:2444
        clip.atype = 1                          # fmain.c:2446 — 1 = OBJECTS
        clip.inum = inum
        clip.offscreen = False
        return clip

    atype = an.type                             # fmain.c:2449
    if (atype == 2 and an.race != 8) or atype == 0:   # fmain.c:2450 — 2 = ENEMY, 8 = RACE_LORAII, 0 = PHIL
        if fiery_death and an.environ > 0:      # fmain.c:2451
            if an.state == 15:                  # fmain.c:2452 — 15 = STATE_DEAD
                clip.offscreen = True           # fmain.c:2452 — corpse vanishes inside fiery-death zone
                return clip
            if an.state == 14:                  # fmain.c:2453 — 14 = STATE_DYING
                clip.atype = 1                  # fmain.c:2454 — 1 = OBJECTS
                clip.inum = 0x58                # fmain.c:2454 — 0x58 = WPN_FIERY_DEATH_INUM
                clip.offscreen = False
                return clip
        elif an.state == 22:                    # fmain.c:2456 — 22 = STATE_FALL
            if an.tactic < 16:                  # fmain.c:2457 — 16 = STATE_SINK; uses tactic field as fall-frame counter
                atype = 2                       # fmain.c:2457 — 2 = ENEMY (early fall frames stay on actor sheet)
            else:
                atype = 1                       # fmain.c:2457 — 1 = OBJECTS (late fall frames use puff sprites)
        else:
            inum = statelist[inum].figure       # fmain.c:2458
        if an.race == 4:                        # fmain.c:2459 — 4 = RACE_SNAKE
            if an.state < 14:                   # fmain.c:2459 — 14 = STATE_DYING
                inum = inum + 0x24              # fmain.c:2459 — 0x24 = snake-frame offset into ENEMY sheet
        elif i > 0:                             # fmain.c:2460 — non-hero actor
            if an.race & 1:                     # fmain.c:2460 — odd race uses odd frames (gender/variant flag)
                inum = inum | 1                 # fmain.c:2460
            else:
                inum = inum & 0xfffe            # fmain.c:2460 — 0xfffe = ~1
    if atype == 1 and an.race == 0:             # fmain.c:2462 — 1 = OBJECTS, race 0 = hidden object
        clip.offscreen = True                   # fmain.c:2462 — hidden objects render only via Look (look_command)
        return clip
    if atype == 5 and riding == 0 and actor_file == 11:  # fmain.c:2463 — 5 = CARRIER, file 11 = swan-when-not-mounted reskin
        atype = 3                               # fmain.c:2464 — 3 = RAFT
        inum = 1                                # fmain.c:2464 — 1 = idle raft frame
    clip.atype = atype
    clip.inum = inum
    clip.offscreen = False
    return clip
```

## compute_sprite_size

Source: `fmain.c:2472-2480`
Called by: `compute_shape_clip`
Calls: `seq_list`, `OBJECTS`

```pseudo
def compute_sprite_size(atype: i8, inum: i16) -> ShapeClip:
    """Pick (xsize, ysize) in pixels for the current sprite. Width comes
    straight from the sheet; height comes from the sheet *unless* the frame
    falls into the OBJECTS half-height set, in which case the renderer
    overrides it to 8 scanlines. The override list is the per-frame size
    table the OBJECTS sheet does not carry in metadata.

    The half-height set (`fmain.c:2478-2479`):
      - inum == 0x1b  (arrow shaft)
      - 8 <= inum <= 12  (small ground items)
      - inum == 25 or inum == 26  (bones, scrap)
      - 0x10 < inum < 0x18  (arrow flight frames 0x11..0x17)
      - (inum & 0x80) != 0  (explicit half-height flag)
    """
    clip = ShapeClip()
    clip.xsize = seq_list[atype].width * 16     # fmain.c:2472 — 16 = pixels per width-word
    clip.ysize = seq_list[atype].height         # fmain.c:2473
    if atype != OBJECTS:                        # fmain.c:2477
        return clip
    if inum == 0x1b:                            # fmain.c:2478 — 0x1b = arrow shaft
        clip.ysize = 8                          # fmain.c:2479 — 8 = half-row scanlines
    elif inum >= 8 and inum <= 12:              # fmain.c:2478 — 8..12 = small ground items
        clip.ysize = 8                          # fmain.c:2479
    elif inum == 25 or inum == 26:              # fmain.c:2478 — 25, 26 = bones / scrap
        clip.ysize = 8                          # fmain.c:2479
    elif inum > 0x10 and inum < 0x18:           # fmain.c:2479 — 0x11..0x17 = arrow flight frames
        clip.ysize = 8                          # fmain.c:2479
    elif (inum & 0x80) != 0:                    # fmain.c:2479 — 0x80 = INUM_BIT7_HALF_HEIGHT
        clip.ysize = 8                          # fmain.c:2479
    return clip
```

## compute_shape_clip

Source: `fmain.c:2468-2528`
Called by: `render_sprites`
Calls: `compute_sprite_size`

```pseudo
def compute_shape_clip(an: Shape, i: i16, passmode: i8, atype: i8, inum: i16, xstart_in: i16, ystart_in: i16, ground_in: i16) -> ShapeClip:
    """Apply map-scroll bias and per-state Y shifts, derive xsize/ysize
    (incl. the OBJECTS half-height set), clip to the 320×174 playfield,
    and apply the bit-7 half-height nudge. Returns a fully populated
    ShapeClip with .offscreen set when the sprite is fully outside the
    playfield or otherwise vetoed. The .blitwide / .blithigh fields are
    expressed in 16-px-wide source words and scanlines respectively."""
    clip = compute_sprite_size(atype, inum)
    clip.atype = atype
    clip.inum = inum

    xstart = xstart_in + (map_x & 15)           # fmain.c:2468 — 15 = 16-px sub-tile mask
    ystart = ystart_in + (map_y & 31)           # fmain.c:2469 — 31 = 32-px sub-tile mask
    ground = ground_in + (map_y & 31)           # fmain.c:2470

    if passmode == 0 and atype != OBJECTS:      # fmain.c:2489 — body pass on actor sheet
        if i == 0 and riding == 11:             # fmain.c:2490 — 11 = RIDING_SWAN
            ystop = ystart + clip.ysize - 1
            ystop = ystop - 16                  # fmain.c:2490 — 16 = swan-mount feet shift
        elif an.environ == 2:                   # fmain.c:2491 — 2 = ENVIRON_WADE
            ystop = ystart + clip.ysize - 1
            ystop = ystop - 10                  # fmain.c:2491 — 10 = wading hide-legs amount
        elif an.environ > 29:                   # fmain.c:2492 — 29 just above ENVIRON_DROWN-1
            if an.state == 15:                  # fmain.c:2493 — 15 = STATE_DEAD
                clip.offscreen = True
                return clip
            ystart = ystart + 27                # fmain.c:2494 — 27 = drowning bubble Y-anchor
            ystop = ystart + 7                  # fmain.c:2495 — 7 = bubble height-1
            clip.atype = OBJECTS                # fmain.c:2496
            clip.inum = 97 + ((cycle + i) & 1)  # fmain.c:2497 — 97 = first drowning bubble frame
            atype = OBJECTS
            inum = clip.inum
        elif an.environ > 2:                    # fmain.c:2499 — sinking ramp
            ystart = ystart + an.environ        # fmain.c:2500
            ystop = ystart + clip.ysize - 1
        else:
            ystop = ystart + clip.ysize - 1     # fmain.c:2487
    else:                                       # fmain.c:2502 — weapon overlay or OBJECTS body
        ystop = ystart + clip.ysize - 1
        if an.environ > 29:                     # fmain.c:2503 — drowning skips weapon
            clip.offscreen = True
            return clip
        if an.environ > 2:                      # fmain.c:2504 — sinking ramp
            ystart = ystart + an.environ        # fmain.c:2505
            ystop = ystop + an.environ          # fmain.c:2506
            if ystop > ground:                  # fmain.c:2507 — clip weapon to ground
                ystop = ground
            if ystart >= ground:                # fmain.c:2508
                clip.offscreen = True
                return clip
            ground = ground + an.environ        # fmain.c:2509

    xstop = xstart + clip.xsize - 1             # fmain.c:2486
    if xstart > 319 or ystart > 173 or xstop < 0 or ystop < 0:   # fmain.c:2513 — 319 = PLAYFIELD_X_MAX, 173 = PLAYFIELD_Y_MAX
        clip.offscreen = True
        return clip

    xstart1 = xstart                            # fmain.c:2517
    if xstart < 0:
        xstart1 = 0
    ystart1 = ystart                            # fmain.c:2518
    if ystart < 0:
        ystart1 = 0
    xstop1 = xstop                              # fmain.c:2519
    if xstop > 319:                             # fmain.c:2519 — 319 = PLAYFIELD_X_MAX
        xstop1 = 319                            # fmain.c:2519 — 319 = PLAYFIELD_X_MAX
    ystop1 = ystop                              # fmain.c:2520
    if ystop > 173:                             # fmain.c:2520 — 173 = PLAYFIELD_Y_MAX
        ystop1 = 173                            # fmain.c:2520 — 173 = PLAYFIELD_Y_MAX
    xoff = xstart1 - xstart                     # fmain.c:2521
    yoff = ystart1 - ystart                     # fmain.c:2522

    if atype == OBJECTS and (inum & 0x80) != 0:  # fmain.c:2524 — 0x80 = INUM_BIT7_HALF_HEIGHT
        clip.inum = inum & 0x7f                  # fmain.c:2524 — 0x7f = strip the flag
        yoff = yoff + 8                          # fmain.c:2524 — 8 = lower-half row offset

    xbw = xstart1 / 16                          # fmain.c:2527 — 16 = pixels per source word
    xew = xstop1 / 16                           # fmain.c:2528

    clip.xstart = xstart
    clip.ystart = ystart
    clip.xstop = xstop
    clip.ystop = ystop
    clip.xstart1 = xstart1
    clip.ystart1 = ystart1
    clip.xstop1 = xstop1
    clip.ystop1 = ystop1
    clip.xoff = xoff
    clip.yoff = yoff
    clip.xbw = xbw
    clip.xew = xew
    clip.ground = ground
    clip.blitwide = xew - xbw + 1               # fmain.c:2530
    clip.blithigh = ystop1 - ystart1 + 1        # fmain.c:2531
    if xoff & 15:                                # fmain.c:2533 — 15 = sub-word mask
        clip.blitwide = clip.blitwide + 1        # fmain.c:2533
    clip.cwide = clip.xsize / 8                  # fmain.c:2483 — 8 = bits per byte
    clip.offscreen = False
    return clip
```

## reserve_save_slot

Source: `fmain.c:2535-2558`
Called by: `render_sprites`
Calls: `fp_drawing`

```pseudo
def reserve_save_slot(clip: ShapeClip, crack_in: i8, backalloc_in: object) -> ShapeClip:
    """Compute the per-sprite blitter scratch fields and reserve a backsave
    slot. Small sprites (savesize < 64 bytes) get a fast-path slot at the
    tail of an unused bitplane (past the visible 192 scanlines); up to 5
    such slots are recycled per page. Larger sprites bump-allocate from
    fp_drawing.backsave. Returns the clip with shp_*, aoff, boff, bmod,
    shift, wmask filled in plus an updated crack and backalloc; if the
    cumulative budget is exhausted the .offscreen flag is set so the
    caller stops drawing for the rest of the frame."""
    planesize = seq_list[clip.atype].bytes      # fmain.c:2536
    clip.savesize = clip.blitwide * clip.blithigh * 2   # fmain.c:2537 — 2 = bytes per word
    clip.blitsize = (clip.blithigh * 64) + clip.blitwide  # fmain.c:2538 — 64 = blitter size shift (1 << 6)

    clip.shapedata_offset = planesize * 5 * clip.inum    # fmain.c:2540 — 5 = bitplanes
    clip.maskdata_offset = planesize * clip.inum         # fmain.c:2541

    crack_out = crack_in
    backalloc_out = backalloc_in
    if clip.savesize < 64 and crack_in < 5:     # fmain.c:2542 — 64 = SMALL_SHAPE_THRESHOLD; 5 = SMALL_SHAPE_SLOT_MAX
        clip.backsave_slot = crack_in           # fmain.c:2543 — bitplanes[crack] + 7680
        crack_out = crack_in + 1                # fmain.c:2543
    else:
        clip.backsave_addr = backalloc_in       # fmain.c:2545
        backalloc_out = backalloc_out + 5 * clip.savesize    # fmain.c:2546 — 5 = bitplanes
        fp_drawing.saveused = fp_drawing.saveused + 5 * clip.savesize   # fmain.c:2547
        if fp_drawing.saveused >= 5920:         # fmain.c:2548 — 5920 = BACKSAVE_LIMIT_BYTES
            clip.offscreen = True               # fmain.c:2548 — out of room; stop drawing this frame
            return clip

    clip.aoff = (clip.ystart1 & 31) * clip.blitwide * 2   # fmain.c:2552 — 31 = sub-tile, 2 = bytes per word
    clip.boff = (clip.xoff / 16) * 2 + clip.cwide * clip.yoff   # fmain.c:2553 — 16 = px per word, 2 = bytes per word
    clip.coff = clip.xbw * 2 + clip.ystart1 * 40   # fmain.c:2554 — 2 = bytes per word, 40 = screen modulus (320/8)
    clip.shift = clip.xstart & 15               # fmain.c:2555 — 15 = sub-word shift
    if clip.xoff & 15:                          # fmain.c:2556
        clip.coff = clip.coff - 2               # fmain.c:2556 — 2 = back up one word for sub-word A-channel underflow
        clip.wmask = 0                          # fmain.c:2556
    else:
        clip.wmask = -1                         # fmain.c:2556 — -1 = 0xFFFF (16-bit all-ones mask)
    clip.bmod = clip.cwide - (clip.blitwide * 2)  # fmain.c:2557 — 2 = bytes per word
    clip.cmod = 40 - (clip.blitwide * 2)        # fmain.c:2558 — 40 = screen modulus (320/8); 2 = bytes per word

    clip.crack_after = crack_out
    clip.backalloc_after = backalloc_out
    clip.offscreen = False
    return clip
```

## should_apply_terrain_mask

Source: `fmain.c:2563-2569`
Called by: `render_sprites`
Calls: none

```pseudo
def should_apply_terrain_mask(an: Shape, i: i16, atype: i8, inum: i16) -> bool:
    """Four early-exit gates that bypass terrain occlusion masking. When any
    of these fires the sprite is drawn unmasked (fmain.c:2599 `nomask:`)."""
    if atype == 5:                              # fmain.c:2564 — 5 = CARRIER (swan / turtle / dragon never occlude)
        return False
    if i == 0 and riding == 11:                 # fmain.c:2564 — 11 = RIDING_SWAN; hero on swan rides above terrain
        return False
    if i == 0 and xtype == 84:                  # fmain.c:2564 — 84 = XTYPE_FIERY_DEATH
        return False
    if i == 0 and hero_x > FIERY_DEATH_RECT_X1 and hero_x < FIERY_DEATH_RECT_X2 and hero_y > FIERY_DEATH_RECT_Y1 and hero_y < FIERY_DEATH_RECT_Y2:   # fmain.c:2565-2566 — fiery-death rectangle around island-of-no-return
        return False
    if atype == 1 and inum > 99 and inum < 102:   # fmain.c:2568 — 1 = OBJECTS, 100..101 = bubble / spell-effect frames
        return False
    if an.race == 0x85 or an.race == 0x87:      # fmain.c:2569 — RACE_NOMASK_A / RACE_NOMASK_B (transparent setfigs)
        return False
    return True
```

## compute_terrain_mask

Source: `fmain.c:2570-2598`
Called by: `render_sprites`
Calls: `terra_mem`, `minimap`, `maskit`

```pseudo
def compute_terrain_mask(an: Shape, i: i16, clip: ShapeClip) -> None:
    """Stamp terrain occlusion-mask tiles into bmask_mem for every map tile
    overlapping the sprite. The fmain.c:2570 line forces blithigh = 32
    when the sprite is a small object (OBJECTS frames 0..7) or any weapon
    overlay; this is a vestigial size override (the live loop iterates by
    16-px tile via xm/ym, not by blithigh) but any port must keep the
    write because the now-commented `ystop` formula at fmain.c:2575 and a
    future refactor may depend on it.

    Per-tile occlusion type comes from terra_mem[cm+1] & 15 and gates the
    maskit() call: cases 0..7 select different feet-Y / left-edge
    conditions; FALL state (an.state == 22) overrides the bridge case to
    always apply."""
    blithigh = clip.blithigh                    # fmain.c:2531
    if (clip.atype == 1 and clip.inum < 8) or clip.passmode != 0:   # fmain.c:2570 — 1 = OBJECTS; 8 = small-object frame range
        blithigh = 32                           # fmain.c:2570 — 32 = MASK_FORCE_HEIGHT (vestigial; see Notes)
    ym1 = clip.ystart1 / 32                     # fmain.c:2571 — 32 = scanlines per terrain tile
    ym2 = (clip.ystop1 / 32) - ym1              # fmain.c:2572 — 32 = scanlines per tile

    xm = 0
    while xm < clip.blitwide:
        ym = 0
        while ym <= ym2:
            ystop = clip.ground - ((ym + ym1) * 32)   # fmain.c:2576 — 32 = scanlines per tile (was: ystart1&31 + blithigh - ym<<5)
            cm = ((xm + clip.xbw) * 6) + ym + ym1     # fmain.c:2577 — 6 = minimap row stride (6 tiles/screen-row)
            cm = minimap[cm] * 4                # fmain.c:2578 — 4 = bytes per terra_mem entry
            k = terra_mem[cm + 1] & 15          # fmain.c:2579 — 15 = low-nibble occlusion code
            skip = False
            if an.state == 22:                  # fmain.c:2580 — 22 = STATE_FALL
                if cm <= (220 * 4):             # fmain.c:2581 — 220 = falling-tile threshold; 4 = stride
                    skip = True
                else:
                    k = 3                       # fmain.c:2582 — 3 = OCC_BRIDGE; falls always render bridge mask
            if not skip:
                if k == 0:                      # fmain.c:2585 — 0 = OCC_NEVER
                    skip = True
                elif k == 1:                    # fmain.c:2586 — 1 = OCC_NOT_LEFT
                    if xm == 0:
                        skip = True
                elif k == 2:                    # fmain.c:2587 — 2 = OCC_TOP_HALF
                    if ystop > 35:              # fmain.c:2587 — 35 = OCC_GROUND_THRESH_HALF
                        skip = True
                elif k == 3:                    # fmain.c:2588 — 3 = OCC_BRIDGE
                    if hero_sector == 48 and i != 1:   # fmain.c:2588 — 48 = BRIDGE_SECTOR
                        skip = True
                elif k == 4:                    # fmain.c:2590 — 4 = OCC_NOT_LEFT_TOP
                    if xm == 0 or ystop > 35:   # fmain.c:2590 — 35 = top-half threshold
                        skip = True
                elif k == 5:                    # fmain.c:2591 — 5 = OCC_LEFT_TOP_AND
                    if xm == 0 and ystop > 35:  # fmain.c:2591 — 35 = top-half threshold
                        skip = True
                elif k == 6:                    # fmain.c:2592 — 6 = OCC_FULL_IF_ABOVE
                    if ym != 0:
                        cm = 64 * 4             # fmain.c:2592 — 64 = OCC_FULL_TILE_ID; 4 = stride
                elif k == 7:                    # fmain.c:2593 — 7 = OCC_TOP_QUARTER
                    if ystop > 20:              # fmain.c:2593 — 20 = OCC_GROUND_THRESH_QUARTER
                        skip = True
            if not skip:
                maskit(xm, ym, clip.blitwide, terra_mem[cm])   # fmain.c:2595
            ym = ym + 1
        xm = xm + 1
```

## render_inventory_items_page

Source: `fmain.c:3120-3145`
Called by: `take_command`
Calls: `seq_list`, `inv_list`, `BltBitMap`, `InitBitMap`, `SetRGB4`, `SetRast`, `LoadRGB4`, `stillscreen`, `prq`, `OBJECTS`

```pseudo
def render_inventory_items_page(bm: object) -> None:
    """Items-page render (do_option / CMODE_ITEMS / hit==5). Reframes the
    OBJECTS sprite sheet as a 5-plane BitMap and blits each populated
    inv_list[] slot at its (xoff+20, yoff) using a literal 16-pixel width
    and the per-slot img_height. The icon is repeated stuff[j] times
    (capped at inv_list[j].maxshown), spaced by inv_list[j].ydelta. Only
    rows 0..GOLDBASE-1 render here; gold piles use a different render
    path."""
    SetRast(rp_map, 0)                          # fmain.c:3118 — 0 = clear color
    InitBitMap(pagea, 5, 16, 8000)              # fmain.c:3119 — 5 = bitplanes, 16 = bytewidth, 8000 = INV_PAGE_HEIGHT
    data = seq_list[OBJECTS].location           # fmain.c:3120
    pagea.Planes[0] = data                      # fmain.c:3122
    pagea.Planes[1] = data + 32                 # fmain.c:3123 — 32 = OBJ_PLANE_STRIDE; one 16-wide × 16-scanline plane
    pagea.Planes[2] = data + 64                 # fmain.c:3124 — 64 = 2 * stride
    pagea.Planes[3] = data + 96                 # fmain.c:3125 — 96 = 3 * stride
    pagea.Planes[4] = data + 128                # fmain.c:3126 — 128 = 4 * stride

    j = 0
    while j < 31:                               # fmain.c:3128 — 31 = GOLDBASE; gold piles render elsewhere
        num = stuff[j]
        if num > inv_list[j].maxshown:          # fmain.c:3130
            num = inv_list[j].maxshown          # fmain.c:3130
        x = inv_list[j].xoff + 20               # fmain.c:3131 — 20 = INV_ICON_X_OFFSET (left-margin shift)
        y = inv_list[j].yoff                    # fmain.c:3132
        n = inv_list[j].image_number * 80 + inv_list[j].img_off   # fmain.c:3133 — 80 = OBJ_FRAME_STRIDE (16 wide × 5 planes)
        h = inv_list[j].img_height              # fmain.c:3134
        i = 0
        while i < num:                          # fmain.c:3135
            BltBitMap(pagea, 0, n, bm, x, y, 16, h, 0xC0, 0xff, 0)   # fmain.c:3136 — 16 = INV_ICON_WIDTH, 0xC0 = D=A minterm, 0xff = all-planes mask
            y = y + inv_list[j].ydelta          # fmain.c:3137
            i = i + 1
        j = j + 1

    SetRGB4(vp_page, 31, 0, 0, 0)               # fmain.c:3140 — 31 = palette index, 0/0/0 = black
    stillscreen()                                # fmain.c:3141
    LoadRGB4(vp_page, pagecolors, 31)           # fmain.c:3142 — 31 = palette entry count
    viewstatus = 4                              # fmain.c:3143 — 4 = pickup placard mode
    prq(5)                                       # fmain.c:3144 — 5 = ITEMS submenu prompt id
```

## Notes

- **Vestigial `blithigh = 32` override.** The line at `fmain.c:2570` forces
  `blithigh = 32` when `(atype == OBJECTS && inum < 8)` or any weapon-pass
  iteration. The body of the per-tile loop at `fmain.c:2573-2598` does
  *not* read `blithigh` (the commented-out formula at `fmain.c:2575` did,
  via `ystop = (ystart1 & 31) + blithigh - (ym << 5)`); the live loop
  derives `ystop` from `ground - ((ym+ym1) << 5)` instead. The override
  is therefore observable only if a port reintroduces the height-driven
  formula. The five-site catalogue counts it as a fixed-size site
  because any port must decide whether to keep the write.

- **Half-height set is data-less.** The OBJECTS sheet stores all frames as
  16-scanline rows but `compute_sprite_size` uses a hardcoded inum-list
  to discover that some frames are actually 8 scanlines tall (with a
  second 8-scanline frame packed below them in the same row, addressable
  via the bit-7 flag). There is no per-frame size table in the sheet
  metadata or anywhere else — the list at `fmain.c:2478-2479` *is* the
  per-frame size table. A port that switches to a sprite format with
  per-frame dimensions can replace `compute_sprite_size` with a metadata
  lookup, but the *rendered output* must still match the inum-list set.

- **Bit-7 dual role.** The high bit of `inum` does two things: it forces
  `ysize = 8` at `fmain.c:2479` (inside `compute_sprite_size`) and it
  shifts the source-data Y-offset by +8 at `fmain.c:2524` (inside
  `compute_shape_clip`). The clip helper masks the bit out of the
  effective `inum` after applying the offset, so downstream blitter
  pointer math addresses the correct row. Both effects must be kept
  paired in any port.

- **Weapon-class k offsets and bow special-case.** `select_atype_inum`
  encodes `k = {0, 32, 48, 64}` for `{bow, mace, sword, Dirk}` plus a
  separate path for the wand (`k == 5`). The bow gets a per-frame
  `bow_x[]/bow_y[]` pixel-perfect overlay (32 entries, indexed by
  `inum`) and a different inum-derivation that picks one of three
  static OBJECTS frames (30, 0x51, 0x53) by direction. The wand uses
  `WPN_WAND_INUM_BASE = 103` plus `an.facing` and a single special-case
  Y-shift on `DIR_NE`. All other weapons share the body's `statelist[inum]`
  weapon offsets via `wpn_x`, `wpn_y`, `wpn_no`.

- **Inventory width hardcode.** The items-page render at
  [`render_inventory_items_page`](#render_inventory_items_page) blits every
  icon at literal 16 pixels wide regardless of `inv_list` row contents. The
  `inv_list` struct has no width field — width is implicit because
  the OBJECTS sheet rows are 16-px-wide and the renderer treats `pagea`
  as a 5-plane bitmap with 32-byte plane stride (one frame per plane).
  A port that wants variable-width inventory icons must add a width
  field to `inv_list` and read it here.

- **Two-pass coupling.** [`resolve_pass_params`](#resolve_pass_params) and
  [`needs_weapon_pass`](#needs_weapon_pass) together govern whether and in
  what order body and weapon are drawn. The XOR-with-pass logic at
  `fmain.c:2402` and `fmain.c:2405` ensures that the body always renders
  with `passmode == 0` and the weapon with `passmode == 1`, but the
  *iteration order* depends on facing: when facing toward the camera
  (DIR_S, DIR_SE, DIR_SW, DIR_E, DIR_W with bow shooting south) the
  weapon iterates first so the body draws on top; when facing away the
  body iterates first so the weapon draws on top. This produces correct
  occlusion without per-pixel depth.

- **Rendering vs. logic-tier scope.** The blitter primitives
  (`save_blit`, `mask_blit`, `shape_blit`, `clear_blit`, `make_mask`,
  `maskit`, `BltBitMap`) are platform APIs and stay outside this spec —
  see `_discovery/sprite-compositing.md`. A port that targets SDL or
  another graphics layer will replace those primitives wholesale; the
  helpers in this file are the *behavioral* contract that determines
  what gets drawn and where.
