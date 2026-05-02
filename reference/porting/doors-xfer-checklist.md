# Doors & Region Transfer Checklist

Source scope:
- `fmain.c:210-325` (`door` struct, door type constants, `doorlist[86]`, `DOORCOUNT`)
- `fmain.c:1049-1128` (key enum, `door_open` struct, `open_list[17]`, `doorfind`)
- `fmain.c:1897-1960` (outdoor→indoor and indoor→outdoor transition code)
- `fmain.c:2625-2645` (`xfer`)
- `fmain2.c:277-293` (`proxcheck`)

Purpose:
- Ensure ports implement both traversal systems (doorlist + open_list) with correct semantics.

## A. Data Model

- [ ] Represent `struct door` with full 6-field payload: `xc1,yc1,xc2,yc2,type,secs` (`fmain.c:233-239`).
- [ ] Preserve `doorlist[86]` sorted by `xc1` ascending — the outdoor→indoor binary-search depends on order (`fmain.c:1920-1922`).
- [ ] Represent `struct door_open` with all 6 fields: `door_id,map_id,new1,new2,above,keytype` (`fmain.c:1053-1058`).
- [ ] Preserve `open_list[17]` exactly — index order and `map_id` values are matched against runtime `current_loads.image[]` (`fmain.c:1097-1118`).
- [ ] Implement the CAVE/VLOG collision: both share value 18 (`fmain.c:228-229`); any code testing `type == CAVE` must match VLOG entries too.

## B. Outdoor→Indoor Transition (`fmain.c:1897-1935`)

- [ ] Walk-into-door detection: `proxcheck` returns 15 (door terrain type) at `i==0` (player only) — trigger door search (`fmain.c:1607`).
- [ ] Binary search `doorlist` on `xc1` proximity: scan outward ±3 entries until `|d->xc1 - xtest| < 48` — `fmain.c:1906-1914`.
- [ ] Region assignment from `secs` field: `secs==1` → `new_region = 8` (buildings); `secs==2` → `new_region = 9` (dungeons) — `fmain.c:1926`.
- [ ] Inbound offset by door type: CAVE/VLOG → `(xc2+24, yc2+16)`; STAIR → `(xc2+8, yc2+16)`; horizontal (odd type) → `(xc2+8, yc2+16)`; vertical (even type) → `(xc2+16, yc2+8)` — `fmain.c:1930-1934`.
- [ ] Call `xfer(xc2_offset, yc2_offset, FALSE)` — FALSE = don't recalculate region from position (`fmain.c:1927`).
- [ ] Call `find_place(2)` after transfer to update extent and place name (`fmain.c:1928`).

## C. Indoor→Outdoor Transition (`fmain.c:1940-1960`)

- [ ] Indoor exit detection: `proxcheck` returns 15 at `i==0` triggers matching against the same doorlist entry used to enter.
- [ ] Outbound offset by door type: CAVE/VLOG → `(xc1-4, yc1+16)`; STAIR → `(xc1+8, yc1-8)`; horizontal → `(xc1+8, yc1-8)`; vertical → `(xc1-8, yc1+8)` — `fmain.c:1946-1950`.
- [ ] Call `xfer(xc1_offset, yc1_offset, TRUE)` — TRUE = recalculate region from destination position (`fmain.c:1948`).
- [ ] Call `find_place(FALSE/0)` after outdoor transfer — flag 0 suppresses place-name message (`fmain.c:1951`).
- [ ] Restore `region_num` to the world region (0–7) derived from destination coordinates.

## D. `xfer()` Semantics (`fmain.c:2625-2645`)

- [ ] Set `hero_x = xtest`, `hero_y = ytest` — unconditional coordinate update (`fmain.c:2628`).
- [ ] If `flag == TRUE`: recalculate `region_num` from the new `hero_x/hero_y` using the region grid — `fmain.c:2630-2636`.
- [ ] Reload map data for the new region: call load sequence for sector tiles and image blocks — `fmain.c:2637-2640`.
- [ ] Check `i==0 && j==15` after load: call `doorfind(xtest,ytest,0)` to auto-open NOKEY doors at entry — `fmain.c:2641-2643`.
- [ ] Set `viewstatus = 99` to force full display redraw — `fmain.c:2642`.

## E. `doorfind()` — Map Tile Unlocking (`fmain.c:1081-1128`)

- [ ] Probe terrain type 15 at `(x,y)`, `(x+4,y)`, `(x-8,y)` — use first hit (`fmain.c:1083-1085`).
- [ ] Find top-left corner: scan left by 16 px (up to twice) then scan down by 32 px — `fmain.c:1087-1089`.
- [ ] Convert to image coords: `imx = x >> 4`, `imy = y >> 5` — `fmain.c:1090-1091`.
- [ ] Look up `sec_id = *mapxy(imx, imy)` and `reg_id = current_loads.image[sec_id >> 6]` — `fmain.c:1093-1095`.
- [ ] Scan all 17 `open_list` entries: match `map_id == reg_id && door_id == sec_id` AND `keytype == 0 || keytype == supplied_keytype` — `fmain.c:1097-1118`.
- [ ] On match: apply `new1` at `(x,y)` and `new2` per `above` field (1=above, 3=left/back, 4=cabinet special, else=right/side) — `fmain.c:1100-1114`.
- [ ] Set `viewstatus = 99` after any tile modification — `fmain.c:1115`.
- [ ] On no match: if `!bumped && !keytype`, print "It's locked." and set `bumped = 1` — `fmain.c:1121-1123`.

## F. Key System (`fmain.c:1049`, `fmain.c:3472-3488`)

- [ ] Implement 6 key types (GOLD=1..WHITE=6) mapping to `stuff[KEYBASE+n]` (indices 16–21) — `fmain.c:427`.
- [ ] Key use: test all 9 directions at distance 16 px from hero; if `doorfind()` succeeds, decrement `stuff[KEYBASE+hit]` — `fmain.c:3477-3487`.
- [ ] Key count decrements on first successful `doorfind` call, not on every attempt — `fmain.c:3486`.
- [ ] Opened doors are NOT persistent: tile changes live only in `sector_mem`; reloading the region resets the door — `fmain.c:1100-1114`.

## G. Stargate Special Case (`doorlist[14-15]`)

- [ ] Stargate entry (idx 14, STAIR, secs=1): outdoor → region 8 at `(xc2+8, yc2+16)` — `doorlist` entry at `fmain.c:256`.
- [ ] Stargate return (idx 15, STAIR, secs=2): indoor → region 9 at `(xc2+8, yc2+16)` — bidirectional portal pairs must be implemented as separate doorlist entries (`fmain.c:257`).

## H. Known Quirks To Preserve (or Deliberately Normalize)

- [ ] Doorlist entries 0–3 are identical (same desert fort coords); binary search can match any of them — `fmain.c:240-243`.
- [ ] CAVE and VLOG share value 18; both use identical offset formulas — `fmain.c:228-229`.
- [ ] `doorfind` with `keytype=0` opens NOKEY doors (no key consumed); also called from `xfer` on entry — `fmain.c:2641-2643`.
- [ ] `bumped` flag suppresses repeated "It's locked." messages; reset in KEYS menu handler before probing — `fmain.c:3473`.

## I. Minimum Parity Test Matrix

- [ ] Entering a building teleports hero to matching `xc2/yc2` offset and sets `region_num` to 8 or 9 — `fmain.c:1926-1934`.
- [ ] Exiting returns hero to `xc1/yc1` offset with `region_num` recalculated from world coordinates — `fmain.c:1946-1951`.
- [ ] Using a Gold Key on a HGATE tile consumes one key and replaces the tile with open graphics — `open_list[8]`.
- [ ] Entering a doorway with a NOKEY door auto-opens it via `doorfind(xtest,ytest,0)` in `xfer` — `fmain.c:2641-2643`.
- [ ] Opened door tiles reset after sector reload (no persistence) — `fmain.c:1100-1114`.
- [ ] `find_place(2)` fires after outdoor→indoor; `find_place(0)` fires after indoor→outdoor — `fmain.c:1928,1951`.
