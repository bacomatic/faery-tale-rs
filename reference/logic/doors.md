# Doors — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c
> Cross-refs: [RESEARCH §6](../RESEARCH.md#12-door-system), [game-loop.md#check_door](game-loop.md#check_door), [menu-system.md#option_handler](menu-system.md#option_handler), [movement.md#walk_step](movement.md#walk_step), [_discovery/doors.md](../_discovery/doors.md)

## Overview

This spec covers the three low-level primitives that implement door and region transitions. The high-level per-frame straddle check that drives indoor↔outdoor swaps lives in [`check_door`](game-loop.md#check_door); the inline collision path that opens a freestanding door the hero walks into lives in [`walk_step`](movement.md#walk_step) at `fmain.c:1607`. This file documents what those sites call into:

- [`xfer`](#xfer) — the generic teleport primitive. Updates `hero_*`, `map_*`, optionally re-derives `new_region` from world coordinates, re-kicks disk loads via `load_all`, regenerates the minimap, forces a redraw, swaps music, and nudges the player out of any wall they landed in. Called by `check_door` (both directions), the quicksand→dungeon drop at `fmain.c:1788`, the princess rescue, brother-succession revive, save-load post-fixup, and the desert-gate zone handler.
- [`doorfind`](#doorfind) — given pixel `(x, y)` near a tile with terrain code `TERRAIN_DOOR=15`, walk to the door's upper-left tile, look up its `(door_id, map_id)` pair in `TABLE:open_list`, and — if the key type matches — rewrite 1–4 live `sector_mem` tiles to the "open" graphic and print "It opened." Otherwise prints "It's locked." on the first bump. Called inline from `walk_step` with `keytype=0` (bump-to-open on unlocked wood doors) and from [`use_key_on_door`](#use_key_on_door) with `keytype=1..6` (deliberate key use).
- [`use_key_on_door`](#use_key_on_door) — the `CMODE_KEYS` case body inside `do_option` (`fmain.c:3472-3488`). Sweeps the 9 directions around the hero at 16 px range, invokes `doorfind` with the selected key code, and on success decrements the key count in `stuff[KEYBASE+k]`.

`doorlist[]` (the 86-entry outdoor↔indoor coordinate table), its type constants (`HWOOD`, `VWOOD`, `HSTONE`, `CAVE`/`VLOG`, `DESERT`, `STAIR`, …), and the outdoor-binary-search / indoor-linear-scan traversal algorithm are normatively specified by [`check_door`](game-loop.md#check_door) and are not repeated here. `TABLE:doorlist`, `TABLE:open_list`, and the full type-to-value map are enumerated in `_discovery/doors.md`.

## Symbols

All numeric literals in the pseudo blocks below carry inline `# fmain.c:LINE — meaning` annotations rather than being promoted to named constants in SYMBOLS.md. Proposed SYMBOLS.md additions (see final report) are listed below; each is referenced via the `Calls:` line of every function that uses it until they are registered.

Proposed additions (not yet registered):

- `KEYBASE = 16` — `fmain.c:427` — `stuff[]` slot of the first (Gold) key. Keys live at `stuff[KEYBASE+0..5]`.
- `NOKEY = 0` — `fmain.c:1047` — `open_list.keytype` sentinel for doors that open on bump.
- `GOLD = 1`, `GREEN = 2`, `KBLUE = 3`, `RED = 4`, `GREY = 5`, `WHITE = 6` — `fmain.c:1047` — `enum ky` key codes; `doorfind(…, keytype)` matches `open_list[j].keytype` against these.
- `TABLE:doorlist` — `fmain.c:240-325` — 86-entry `struct door[]` for outdoor↔indoor transitions; sorted by `xc1`. Fields: `xc1, yc1, xc2, yc2, type, secs`.
- `TABLE:open_list` — `fmain.c:1059-1078` — 17-entry `struct door_open[]` for tile-level unlock: `{door_id, map_id, new1, new2, above, keytype}`.
- `hero_x: u16`, `hero_y: u16` — `fmain.c` — player world coordinates (aliases for `anim_list[0].abs_x/abs_y`).
- `map_x: i32`, `map_y: i32` — `fmain.c` — scroll origin of the playfield camera in world coordinates.
- `current_loads` — `fmain.c` — `struct file_index` mirror of the currently loaded region; `current_loads.image[i]` gives the image-block id used for tile ids with `sec_id>>6 == i`.
- `bumped: i8` — `fmain.c:1046` — one-frame latch that suppresses repeated "It's locked." messages.
- `encounter_number: i32` — `fmain.c` — pending-encounter counter; cleared to zero by `xfer`.

Globals already in [SYMBOLS.md](SYMBOLS.md) used here: `anim_list`, `stuff`, `viewstatus`, `keydir`, `region_num` (read via `check_door`), `VIEWSTATUS_CORRUPT`, `TERRAIN_DOOR`.

## xfer

Source: `fmain.c:2625-2645`
Called by: `check_door`, `walk_step` (quicksand drop, `fmain.c:1784-1790`), `rescue`, `revive`, `deserialize_save_record`, desert-gate zone handler
Calls: `load_all`, `gen_mini`, `setmood`, `proxcheck`, `anim_list`, `hero_x`, `hero_y`, `map_x`, `map_y`, `encounter_number`, `keydir`, `viewstatus`, `VIEWSTATUS_CORRUPT`

```pseudo
def xfer(xtest: u16, ytest: u16, flag: bool) -> None:
    """Teleport hero to (xtest, ytest), optionally re-deriving new_region from world coords."""
    # fmain.c:2626-2627 — keep camera origin locked to the same pixel offset from the hero.
    map_x = map_x + (xtest - hero_x)
    map_y = map_y + (ytest - hero_y)
    # fmain.c:2628-2629 — hero world position is mirrored in anim_list[0] and the global aliases.
    hero_x = xtest
    anim_list[0].abs_x = xtest
    hero_y = ytest
    anim_list[0].abs_y = ytest
    # fmain.c:2631 — cancel any queued wilderness encounter at the old location.
    encounter_number = 0
    if flag:
        # fmain.c:2633-2637 — recompute new_region from the new camera origin. 151/64 centre the
        # camera on the hero; >>8 converts world pixels to 256-px super-tiles; >>6 then >>5 fold
        # into the 2x8 region grid (x low bit, y three bits).
        xtest = (map_x + 151) >> 8                  # fmain.c:2633 — centring offset X
        ytest = (map_y + 64) >> 8                   # fmain.c:2634 — centring offset Y
        xtest = (xtest >> 6) & 1                    # fmain.c:2635 — region column bit
        ytest = (ytest >> 5) & 7                    # fmain.c:2636 — region row (0..7)
        new_region = xtest + (ytest + ytest)        # fmain.c:2637 — linearise into 0..9
    # fmain.c:2639 — drop any latched keypad direction so the hero stands still on arrival.
    keydir = 0
    # fmain.c:2640 — trigger async disk load for whichever region/image set is now active.
    load_all()
    # fmain.c:2641 — rebuild minimap and, critically, refresh xreg/yreg used by px_to_im.
    gen_mini()
    # fmain.c:2642 — mark display corrupt so the next frame does a full redraw.
    viewstatus = VIEWSTATUS_CORRUPT
    # fmain.c:2643 — swap background music / wave table for the new area.
    setmood(True)
    # fmain.c:2644 — nudge the hero south one pixel at a time until they are no longer inside a
    # blocking tile. Prevents getting stuck when a door endpoint lands in a wall after reload.
    while proxcheck(hero_x, hero_y, 0) != 0:
        hero_y = hero_y + 1
```

## doorfind

Source: `fmain.c:1081-1128`
Called by: `walk_step` (inline bump-open, `fmain.c:1607`), `use_key_on_door`
Calls: `px_to_im`, `mapxy`, `print`, `open_list`, `current_loads`, `bumped`, `keydir`, `viewstatus`, `VIEWSTATUS_CORRUPT`

```pseudo
def doorfind(x: u16, y: u16, keytype: u8) -> bool:
    """Unlock and visually open a door tile at (x, y) if it matches open_list and the key fits."""
    # fmain.c:1085-1088 — probe the target pixel, then 4 px right, then 4 px left of the original
    # (x is mutated by +4 first, so the third probe is x-8 relative to that). px_to_im returns
    # the high nibble of terra_mem[tile*4+1] — the terrain class.
    if px_to_im(x, y) != 15:                        # fmain.c:1085 — TERRAIN_DOOR
        x = x + 4                                   # fmain.c:1086 — probe east neighbour
        if px_to_im(x, y) != 15:                    # fmain.c:1086 — TERRAIN_DOOR
            x = x - 8                               # fmain.c:1087 — probe original -4
            if px_to_im(x, y) != 15:                # fmain.c:1087 — TERRAIN_DOOR
                return False                        # fmain.c:1088 — nothing door-like here
    # fmain.c:1090-1091 — multi-tile doors are anchored at the upper-left. Walk left up to twice
    # in 16-px tile steps; walk down once in 32-px steps.
    if px_to_im(x - 16, y) == 15:                   # fmain.c:1090 — 16 px = one image column
        x = x - 16                                  # fmain.c:1090 — 16 px = one image column
    if px_to_im(x - 16, y) == 15:                   # fmain.c:1091 — 16 px = one image column
        x = x - 16                                  # fmain.c:1091 — 16 px = one image column
    if px_to_im(x, y + 32) == 15:                   # fmain.c:1092 — 32 px = one image row
        y = y + 32                                  # fmain.c:1092 — 32 px = one image row
    # fmain.c:1093-1094 — convert pixel to image-grid indices. x>>4 == pixel/16, y>>5 == pixel/32.
    x = x >> 4                                      # fmain.c:1093 — 4 = log2(16 px per column)
    y = y >> 5                                      # fmain.c:1094 — 5 = log2(32 px per row)
    # fmain.c:1096 — read the sector-local tile id from the live tile map.
    sec_id = mapxy(x, y)[0]
    # fmain.c:1097 — resolve which image block owns this tile. Each tile id's high 2 bits index
    # into current_loads.image[] to yield the map_id stamped into open_list.
    reg_id = current_loads.image[sec_id >> 6]       # fmain.c:1097 — 6 = shift; 64 tiles/block
    # fmain.c:1100 — linear search of the 17-entry open_list.
    j = 0
    while j < 17:                                   # fmain.c:1100 — 17 = open_list length
        entry = open_list[j]
        if entry.map_id != reg_id:                  # fmain.c:1102
            j = j + 1
            continue
        if entry.door_id != sec_id:                 # fmain.c:1102
            j = j + 1
            continue
        # fmain.c:1103 — NOKEY (0) opens on any attempt; otherwise key must exactly match.
        if entry.keytype != 0 and entry.keytype != keytype:
            j = j + 1
            continue
        # fmain.c:1104 — primary tile rewrite (the "open" version of this door graphic).
        mapxy(x, y)[0] = entry.new1
        k = entry.new2                              # fmain.c:1105
        if k != 0:
            placement = entry.above                 # fmain.c:1106 — placement code
            if placement == 1:                      # fmain.c:1107 — 1 = tile above
                mapxy(x, y - 1)[0] = k
            elif placement == 3:                    # fmain.c:1108 — 3 = tile to the left
                mapxy(x - 1, y)[0] = k
            elif placement == 4:                    # fmain.c:1109 — 4 = cabinet 2x2 layout
                # fmain.c:1110-1111 — four-tile cabinet with hardcoded ids.
                mapxy(x, y - 1)[0] = 87             # fmain.c:1110 — 87 = cabinet top-left tile
                mapxy(x + 1, y)[0] = 86             # fmain.c:1111 — 86 = cabinet bottom-right
                mapxy(x + 1, y - 1)[0] = 88         # fmain.c:1111 — 88 = cabinet top-right
            else:
                # fmain.c:1114 — second tile to the right; if placement is neither 0 nor 2, the
                # placement value itself is a third tile id stamped one column further right.
                mapxy(x + 1, y)[0] = k
                if placement != 2:                  # fmain.c:1114 — 2 = no third tile
                    mapxy(x + 2, y)[0] = placement  # fmain.c:1114 — above field reused as tile id
        # fmain.c:1115-1117 — force redraw, drop latched direction, tell the player.
        viewstatus = VIEWSTATUS_CORRUPT             # fmain.c:1115
        keydir = 0                                  # fmain.c:1116
        print("It opened.")                         # fmain.c:1117
        return True
    # fmain.c:1122 — first bump with no key in hand prints the lock message once; the bumped
    # latch suppresses the repeat until the player steps off the door (walk_step clears it at
    # fmain.c:1608) or opens the keys menu (fmain.c:3474 resets bumped to 0).
    if bumped == 0 and keytype == 0:
        print("It's locked.")
    bumped = 1                                      # fmain.c:1123 — suppress next frame
    return False
```

## use_key_on_door

Source: `fmain.c:3472-3488`
Called by: `option_handler` (CMODE_KEYS branch)
Calls: `doorfind`, `newx`, `newy`, `print`, `print_cont`, `extract`, `gomenu`, `stuff`, `inv_list`, `bumped`, `hero_x`, `hero_y`

```pseudo
def use_key_on_door(hit: i32) -> None:
    """CMODE_KEYS case: try the selected key in all 9 directions around the hero."""
    # fmain.c:3473 — hit comes in as the menu-slot index. Keys occupy slots 5..10 in the KEYS
    # submenu, so subtract 5 to get a 0..5 key index that lines up with KEYBASE offsets and the
    # enum ky values (index+1 == GOLD..WHITE).
    hit = hit - 5                                   # fmain.c:3473 — 5 = KEYS submenu slot bias
    # fmain.c:3474 — clear the "It's locked." suppression latch so the first mismatched attempt
    # is allowed to speak. doorfind will re-arm it on any failed match.
    bumped = 0
    # fmain.c:3475 — skip the sweep entirely if the player has zero of this key.
    if stuff[hit + 16] == 0:                        # fmain.c:3475 — 16 = KEYBASE
        gomenu(0)                                   # fmain.c:3487 — 0 = CMODE_ITEMS
        return
    # fmain.c:3477 — iterate the 9 canonical directions (0..7 compass + 8 = no-move sentinel).
    # For each, project a trial pixel 16 px out from the hero and attempt to unlock there.
    opened = False
    i = 0
    while i < 9:                                    # fmain.c:3477 — 9 = 8 compass dirs + self
        x = newx(hero_x, i, 16)                     # fmain.c:3478 — 16 px reach
        y = newy(hero_y, i, 16)                     # fmain.c:3479 — 16 px reach
        # fmain.c:3480 — keytype is hit+1 to convert 0..5 index into GOLD..WHITE (1..6).
        if doorfind(x, y, hit + 1):                 # fmain.c:3480 — +1 maps to 1=GOLD..6=WHITE
            stuff[hit + 16] = stuff[hit + 16] - 1   # fmain.c:3480 — 16 = KEYBASE; key consumed
            opened = True
            break
        i = i + 1
    if not opened:
        # fmain.c:3483-3485 — "% tried a <keyname> but it didn't fit." The % is the hero name
        # substitution token expanded by extract/print.
        extract("% tried a ")                       # fmain.c:3483
        print_cont(inv_list[hit + 16].name)         # fmain.c:3484 — 16 = KEYBASE offset
        print_cont(" but it didn't")                # fmain.c:3484
        print("fit.")                               # fmain.c:3484
    gomenu(0)                                       # fmain.c:3487 — 0 = CMODE_ITEMS
```

## Notes

- **No `enter_door` / `exit_door` functions exist in the source.** The outdoor→indoor and indoor→outdoor halves are two branches of a single `if (region_num < 8) … else …` inside the main loop. Both halves are traced in [`check_door`](game-loop.md#check_door). The orientation-precision rules (`hero_y & 0x10` for horizontal doors, `hero_x & 15` for vertical), the `CAVE`/horizontal/vertical destination offsets, the `DESERT`→5-statue gate, the `riding` short-circuit, and the asymmetric fade (enter fades, exit is instant) are all specified there.
- **`TERRAIN_DOOR` inline handler.** The one-line collision trigger at `fmain.c:1607` — `if (i==0 && j==15) doorfind(xtest,ytest,0);` — is part of [`walk_step`](movement.md#walk_step). It fires `doorfind` with `keytype=0` whenever the hero's proposed next tile has terrain class 15, giving the "bump to open unlocked wood doors" behaviour. `bumped` is cleared on the next step where `j != 15`.
- **Quicksand→dungeon drop** at `fmain.c:1784-1790` uses `xfer(0x1080, 34950, False)` with an explicit `new_region = 9` and is not door-mediated. It lives in `walk_step`'s `STATE_SINK` path.
- **`STAIR` type (value 15) is horizontal.** `STAIR & 1 == 1`, so the stargate pair at `doorlist[14..15]` (`fmain.c:254-255`) uses the horizontal `(xc2+16, yc2)` / `(xc1+16, yc1+34)` offsets, not a dedicated stair codepath.
- **Opened-door state is not saved.** `doorfind` writes directly into `sector_mem` via `mapxy`; these edits live for the lifetime of the currently loaded sector. Any `xfer` that triggers a region reload discards them (see `load_all` inside `xfer`). No entry in the save record (see [save-load.md](save-load.md)) tracks per-door open state.
