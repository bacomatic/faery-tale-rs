# Extent System Checklist

Source scope:
- `fmain.c:2647-2720` (`find_place`)
- `fmain.c:333-372` (`struct extent`, `extent_list`, `EXT_COUNT`)
- `fmain2.c:1519, 1560-1566` (extent persistence and relocation)

Purpose:
- Ensure ports implement extent resolution and extent-driven behaviors with parity.

## A. Data Model

- [ ] Represent extents as ordered rows with full payload: `x1,y1,x2,y2,etype,v1,v2,v3` (`fmain.c:333-371`).
- [ ] Preserve table order exactly; overlap resolution is first-match in scan order (`fmain.c:2675-2680`).
- [ ] Include the sentinel row (whole world) so unresolved positions still map to a valid extent (`fmain.c:370`).
- [ ] Keep both `current_extent` (full row) and `xtype` (cached `etype`) as separate runtime state (`fmain.c:2675-2683`).

## B. Resolver Semantics

- [ ] Apply strict interior bounds (`>` / `<`), not inclusive bounds (`>=` / `<=`) (`fmain.c:2677-2678`).
- [ ] Re-resolve extent every `find_place` call, even when `xtype` does not change (`fmain.c:2675-2683`).
- [ ] Keep place-name lookup and extent lookup as two phases in one function call (`fmain.c:2649-2680`).
- [ ] Maintain indoor/outdoor table split (`region_num > 7`) and sector masking behavior (`hero_sector & 255`) (`fmain.c:2651-2658`).

## C. Transition Dispatcher (`xtype` Change Only)

- [ ] Trigger side effects only when `xtype != extn->etype` (`fmain.c:2682`).
- [ ] Princess extent (83): call `rescue()`, clear display flag, and re-run resolution (`goto findagain`) (`fmain.c:2684-2685`).
- [ ] Special figure extents (60/61): spawn only if slot-3 race mismatch or `anix < 4` (`fmain.c:2687-2693`).
- [ ] Astral extent (52): force Loraii preload via synchronous `load_actors()+prep(ENEMY)` (`fmain.c:2695-2698`).
- [ ] Generic forced extents (50-59 except 52): execute only under `flag == 1` (`fmain.c:2700`).
- [ ] `force:` block ordering: set `encounter_type`, zero `mixflag/wt`, `load_actors`, `prep`, then overwrite `encounter_number = v1` and place immediately (`fmain.c:2702-2713`).

## D. `find_place(flag)` Mode Contract

- [ ] Implement two independent flag effects: message display (`if(flag)`) and generic forced-extent gate (`flag == 1`) (`fmain.c:2672-2673`, `fmain.c:2700`).
- [ ] Match callsite behavior: `find_place(2)` main loop and outdoor->indoor door; `find_place(0)` indoor->outdoor door; `find_place(1)` whirlpool/sink transfer (`fmain.c:2050`, `fmain.c:1928`, `fmain.c:1951`, `fmain.c:1789`).

## E. Downstream Consumers (Must Read Full Extent Row)

- [ ] AI flee override uses `extn->v3` in special encounters (`fmain.c:2138-2140`).
- [ ] Magic-use lockout checks `extn->v3 == 9` (`fmain.c:3304-3305`).
- [ ] Carrier loader uses `extn->v3` to select bird/turtle/dragon (`fmain.c:2717-2719`).
- [ ] Forced encounter race uses `encounter_type = extn->v3` (`fmain.c:2704`).

## F. Carrier/Extent Coupling

- [ ] Leaving carrier extents (`xtype < 70`) clears `active_carrier` (`fmain.c:2716`).
- [ ] Entering carrier extent (`xtype == 70`) conditionally reloads based on active/riding/file match (`fmain.c:2717-2719`).
- [ ] Preserve mutable extents for bird/turtle movement and save/load persistence (`fmain2.c:1519`, `fmain2.c:1560-1566`, `fmain2.c:1596`, `fmain.c:1295`, `fmain.c:3516`).

## G. Known Quirks To Preserve (or Deliberately Normalize)

- [ ] `xtype == 49` encounter override exists but no shipped extent row uses 49 (`fmain.c:2090`, `fmain.c:339-371`).
- [ ] DKnight placement path reads uninitialized `j` after skipping loop (`fmain.c:2741-2749`).
- [ ] Placement loop can consume `encounter_number` even when `set_encounter` fails (`fmain.c:2065-2067`).

## H. Minimum Parity Test Matrix

- [ ] Crossing into a 60-zone re-spawns special figure on re-entry (DKnight/Necromancer) (`fmain.c:2687-2693`).
- [ ] Astral entry preloads Loraii with 3-body batch (`v1=3,v2=1`) and no generic 50-59 gate dependency (`fmain.c:2695-2698`, `fmain.c:2723`).
- [ ] Generic 50-59 behavior differs by caller mode (`flag=1` triggers, `flag=2` does not) (`fmain.c:2700`).
- [ ] AI flee in special zones respects `race != extn->v3` regardless of HP (`fmain.c:2138-2140`).
- [ ] Magic blocked only while current extent has `v3==9` (`fmain.c:3304-3305`).
