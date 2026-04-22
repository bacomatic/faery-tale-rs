# Discovery: Encounter & Spawning System

**Status**: complete
**Investigated**: 2026-04-05
**Requested by**: orchestrator
**Prompt summary**: Trace the complete encounter/spawning system — encounter_chart, extent_list, find_place, set_encounter, danger levels, special extents, peace zones, indoor mechanics.

## encounter_chart — `fmain.c:52-63`

Struct definition at `fmain.c:44-51`:
```c
struct encounter {
    char hitpoints, agressive, arms, cleverness, treasure, file_id;
} encounter_chart[];
```

| Index | Monster      | hitpoints | aggressive | arms | cleverness | treasure | file_id | Comment |
|-------|-------------|-----------|-----------|------|-----------|----------|---------|---------|
| 0     | Ogre         | 18  | TRUE (1)  | 2    | 0          | 2        | 6       | — |
| 1     | Orcs         | 12  | TRUE (1)  | 4    | 1          | 1        | 6       | — |
| 2     | Wraith       | 16  | TRUE (1)  | 6    | 1          | 4        | 7       | — |
| 3     | Skeleton     | 8   | TRUE (1)  | 3    | 0          | 3        | 7       | — |
| 4     | Snake        | 16  | TRUE (1)  | 6    | 1          | 0        | 8       | swamp region |
| 5     | Salamander   | 9   | TRUE (1)  | 3    | 0          | 0        | 7       | lava region |
| 6     | Spider       | 10  | TRUE (1)  | 6    | 1          | 0        | 8       | spider pits |
| 7     | DKnight      | 40  | TRUE (1)  | 7    | 1          | 0        | 8       | elf glade |
| 8     | Loraii       | 12  | TRUE (1)  | 6    | 1          | 0        | 9       | astral plane |
| 9     | Necromancer  | 50  | TRUE (1)  | 5    | 0          | 0        | 9       | final arena |
| 10    | Woodcutter   | 4   | NULL (0)  | 0    | 0          | 0        | 9       | — |

**Field usage**:
- `hitpoints` → assigned to `an->vitality` — `fmain.c:2764`
- `aggressive` → never read anywhere in the codebase after definition. Appears to be unused metadata.
- `arms` → indexes `weapon_probs[arms*4+wt]` for weapon selection — `fmain.c:2757-2758`
- `cleverness` → 0 or 1; added to ATTACK1/ARCHER1 base goal — `fmain.c:2762-2763`
- `treasure` → indexes `treasure_probs[treasure*8+rand8()]` for loot drops — `fmain.c:3272-3273`
- `file_id` → sprite sheet file number on disk, checked against `actor_file` to avoid reloading — `fmain.c:2724-2725`

### weapon_probs[] — `fmain2.c:860-868`

```c
char weapon_probs[] = {
    0,0,0,0,    /* 0: no weapons */
    1,1,1,1,    /* 1: dirks only */
    1,2,1,2,    /* 2: dirks and maces */
    1,2,3,2,    /* 3: mostly maces */
    4,4,3,2,    /* 4: swords and bows */
    5,5,5,5,    /* 5: magic wand */
    8,8,8,8,    /* 6: touch attack */
    3,3,3,3,    /* 7: swords only */
};
```

Weapon code meanings from `fmain.c:72`: `0=none, 1=dagger, 2=mace, 3=sword, 4=bow, 5=wand`; 8=touch attack (undocumented in comment, present in table).

### treasure_probs[] — `fmain2.c:852-858`

```c
char treasure_probs[] = {
     0, 0, 0, 0, 0, 0, 0, 0,   /* 0: no treasure */
     9,11,13,31,31,17,17,32,    /* 1: stone,vial,totem,gold,keys */
    12,14,20,20,20,31,33,31,    /* 2: keys,skull,gold,nothing */
    10,10,16,16,11,17,18,19,    /* 3: magic and keys */
    15,21,0,0,0,0,0,0           /* 4: jade skull and white key */
};
```

## extent_list — `fmain.c:339-371`

Struct definition at `fmain.c:333-337`:
```c
struct extent {
    UWORD x1, y1, x2, y2;
    UBYTE etype, v1, v2, v3;
};
```

`EXT_COUNT` = 22 (`fmain.c:372`). Loop checks indices 0–21. Index 22 (whole world) is the sentinel fallback — `extn` naturally points to it when no other extent matches.

| Idx | x1    | y1    | x2    | y2    | etype | v1 | v2 | v3 | Comment |
|-----|-------|-------|-------|-------|-------|----|----|----|---------|
| 0   | 2118  | 27237 | 2618  | 27637 | 70    | 0  | 1  | 11 | bird (swan) |
| 1   | 0     | 0     | 0     | 0     | 70    | 0  | 1  | 5  | turtle (movable via save) |
| 2   | 6749  | 34951 | 7249  | 35351 | 70    | 0  | 1  | 10 | dragon |
| 3   | 4063  | 34819 | 4909  | 35125 | 53    | 4  | 1  | 6  | spider pit |
| 4   | 9563  | 33883 | 10144 | 34462 | 60    | 1  | 1  | 9  | necromancer |
| 5   | 22945 | 5597  | 23225 | 5747  | 61    | 3  | 2  | 4  | turtle eggs |
| 6   | 10820 | 35646 | 10877 | 35670 | 83    | 1  | 1  | 0  | princess rescue |
| 7   | 19596 | 17123 | 19974 | 17401 | 48    | 8  | 8  | 2  | graveyard |
| 8   | 19400 | 17034 | 20240 | 17484 | 80    | 4  | 20 | 0  | around city (peace) |
| 9   | 0x2400 (9216) | 0x8200 (33280) | 0x3100 (12544) | 0x8a00 (35328) | 52 | 3 | 1 | 8 | astral plane |
| 10  | 5272  | 33300 | 6112  | 34200 | 81    | 0  | 1  | 0  | king pax |
| 11  | 11712 | 37350 | 12416 | 38020 | 82    | 0  | 1  | 0  | sorceress pax |
| 12  | 2752  | 33300 | 8632  | 35400 | 80    | 0  | 1  | 0  | peace 1 — buildings |
| 13  | 10032 | 35550 | 12976 | 40270 | 80    | 0  | 1  | 0  | peace 2 — specials |
| 14  | 4712  | 38100 | 10032 | 40350 | 80    | 0  | 1  | 0  | peace 3 — cabins |
| 15  | 21405 | 25583 | 21827 | 26028 | 60    | 1  | 1  | 7  | hidden valley (DKnight) |
| 16  | 6156  | 12755 | 12316 | 15905 | 7     | 1  | 8  | 0  | swamp region |
| 17  | 5140  | 34860 | 6260  | 37260 | 8     | 1  | 8  | 0  | spider region |
| 18  | 660   | 33510 | 2060  | 34560 | 8     | 1  | 8  | 0  | spider region (west) |
| 19  | 18687 | 15338 | 19211 | 16136 | 80    | 0  | 1  | 0  | village (peace) |
| 20  | 16953 | 18719 | 20240 | 17484 | 3     | 1  | 3  | 0  | around village |
| 21  | 20593 | 18719 | 23113 | 22769 | 3     | 1  | 3  | 0  | around city |
| *22* | 0   | 0     | 0x7fff | 0x9fff | 3   | 1  | 8  | 0  | *whole world (sentinel)* |

### etype Categories — `fmain.c:326-330`, code analysis

| etype range | Category | Behavior |
|-------------|----------|----------|
| 0–49 | Regular encounter zone | xtype stored, used in danger calc and encounter overrides. Random encounters per timer. |
| 50–59 | Forced group encounter | Monsters spawn immediately on entry. v1=count, v3=encounter_type. |
| 52 | Astral plane (special) | Force encounter_type=8 (Loraii), synchronous load, no encounter_number override. |
| 60, 61 | Special figure | Unique NPC spawned at extent center if not already present. |
| 70 | Carrier | Bird/turtle/dragon — loads carrier via load_carrier(v3). |
| 80 | Peace zone | Blocks random encounters (xtype ≥ 50 fails `xtype < 50` check). |
| 81 | King peace | Peace + weapon draw blocked: event(15) "Even % would not be stupid enough to draw weapon in here." |
| 82 | Sorceress peace | Peace + weapon draw blocked: event(16) "A great calming influence comes over %..." |
| 83 | Princess rescue | Triggers rescue() if ob_list8[9].ob_stat set. |

### Extent field usage by category

**Regular (etype 0–49)**: v1, v2, v3 are not used by the regular danger/spawn path. The etype value itself becomes xtype, which drives the danger level formula.

**Forced group (etype 50–59, excluding 52)**:
- v1 = exact number of enemies to force-spawn — `fmain.c:2708`
- v2 = used in load_actors (encounter_number = v1 + rnd(v2)) but overwritten — `fmain.c:2723, 2708`
- v3 = encounter_chart index (monster type) — `fmain.c:2704`

**Astral (etype 52)**:
- v1, v2 = used in load_actors as above — `fmain.c:2723`
- v3 = encounter_chart index → 8 (Loraii), but code hardcodes encounter_type=8 — `fmain.c:2696`

**Special figure (etype 60, 61)**:
- v1 = enemy count for spawning — `fmain.c:2708`
- v3 = encounter_chart index / race — `fmain.c:2704, 2689`

**Carrier (etype 70)**:
- v3 = carrier file ID: 11=bird (swan), 5=turtle, 10=dragon — `fmain.c:2719`

**Peace (etype 80–83)**: v1–v3 generally unused by encounter logic.

### Savegame Extent Persistence

Only extents 0 and 1 (bird and turtle) are persisted in saves — `fmain2.c:1530`:
```c
saveload((void *)extent_list, 2 * sizeof(struct extent));
```

This allows the turtle's position to be saved (it starts at 0,0,0,0 and is repositioned via `move_extent()`). The bird extent is repositioned during `rescue()` — `fmain2.c:1596`:
```c
move_extent(0, 22205, 21231);
```

`move_extent()` at `fmain2.c:1560-1566` creates a 500×400 box centered on (x,y).

## find_place — `fmain.c:2647-2720`

```c
find_place(flag) short flag;
```

Called from main loop every frame as `find_place(2)` — `fmain.c:2049`.

### Algorithm

**Step 1: Place Name Detection** (`fmain.c:2649-2673`)

1. `j = hero_sector = hero_sector & 255` (mask to 8 bits) — `fmain.c:2651`
2. If `region_num > 7` (indoor): use `inside_tbl`/`inside_msg`, add 256 to hero_sector — `fmain.c:2655-2656`
3. Else (outdoor): use `place_tbl`/`place_msg` — `fmain.c:2657`
4. Linear scan of 3-byte entries {sector_low, sector_high, msg_index} — `fmain.c:2659-2662`
   - Tables defined in `narr.asm:86` (place_tbl) and `narr.asm:117` (inside_tbl)
5. If found message is #4 (mountains) — special region_num adjustments — `fmain.c:2664-2667`:
   - Indoor: no change
   - `region_num & 1` (odd): force i=0 (suppress message)
   - `region_num > 3`: force i=5
6. If region in flux or misaligned: force i=0 — `fmain.c:2668`
7. If place changed and flag set: display message — `fmain.c:2670-2672`

**Step 2: Extent Detection** (`fmain.c:2674-2720`)

1. Linear scan of extent_list[0..21] — `fmain.c:2676-2679`
2. Test: `hero_x > x1 && hero_x < x2 && hero_y > y1 && hero_y < y2` (exclusive bounds) — `fmain.c:2677-2678`
3. First match wins (lower index = higher priority)
4. If no match: extn → index 22 (whole world sentinel, etype=3)
5. If xtype changed (entered new extent), handle by category — see extent_list section above

**Key observations**:
- Extent scan is first-match, so overlapping extents are resolved by index order. Small/specific extents should appear before large/general ones.
- The graveyard (idx 7, etype 48) is checked before the surrounding "around city" peace zone (idx 8, etype 80), so graveyard encounters fire while the surrounding area is peaceful.
- The spider pit (idx 3, etype 53) is checked before the overlapping peace zones (idx 12-14), so forced spider spawns happen.
- The whole world sentinel has `etype=3`, meaning `xtype=3` everywhere that no other extent matches.

## Main Loop Encounter Flow — `fmain.c:2058-2091`

Two separate periodic checks drive encounters:

### Placement Check — every 16 frames (`fmain.c:2058-2078`)

Condition: `(daynight & 15) == 0 && encounter_number > 0 && !actors_loading` — `fmain.c:2058`

Process:
1. Set mixflag and wt — `fmain.c:2059-2060`:
   - `mixflag = rand()` (random 31-bit value)
   - If `xtype > 49`: `mixflag = 0` (no mixing in special zones)
   - If `(xtype & 3) == 0`: `mixflag = 0` (no mixing when xtype divisible by 4)
   - `wt = rand4()` (0–3)
2. Try up to 10 random locations via `set_loc()` — `fmain.c:2061`
3. Each location checked with `px_to_im(encounter_x, encounter_y) == 0` (walkable terrain) — `fmain.c:2063`
4. If valid location found:
   - Fill empty slots 3–6 via `set_encounter(anix, 63)`, incrementing anix — `fmain.c:2064-2067`
   - If slots full but encounter_number remains, recycle DEAD enemy slots (indices 3–6) — `fmain.c:2068-2074`
   - Wraith corpses (race==2) can be overwritten even if visible — `fmain.c:2071`
5. Break after first successful placement — `fmain.c:2076`

### Danger Check — every 32 frames (`fmain.c:2080-2091`)

Condition: `(daynight & 31) == 0 && !actors_on_screen && !actors_loading && !active_carrier && xtype < 50` — `fmain.c:2080-2081`

Process:
1. Calculate danger_level — `fmain.c:2082-2083`:
   - Indoor (region_num > 7): `danger_level = 5 + xtype`
   - Outdoor: `danger_level = 2 + xtype`
2. Roll: `rand64() <= danger_level` — `fmain.c:2085`
   - `rand64()` returns 0–63, so probability = `(danger_level + 1) / 64`
3. If roll succeeds — `fmain.c:2086-2091`:
   - `encounter_type = rand4()` → 0–3 (ogre, orc, wraith, skeleton)
   - **Swamp override** (xtype==7): wraith (2) → snake (4) — `fmain.c:2087-2088`
   - **Spider region override** (xtype==8): force spider (6), mixflag=0 — `fmain.c:2089`
   - **xtype==49 override**: force wraith (2), mixflag=0 — `fmain.c:2090`
   - Call `load_actors()` to initiate disk I/O — `fmain.c:2091`

### Danger Level Table (all regular extents)

| Zone | etype/xtype | Outdoor danger | Indoor danger | Outdoor spawn probability |
|------|------------|---------------|--------------|--------------------------|
| Whole world (sentinel) | 3 | 5 | 8 | 6/64 = 9.4% |
| Around village | 3 | 5 | 8 | 6/64 = 9.4% |
| Around city | 3 | 5 | 8 | 6/64 = 9.4% |
| Swamp region | 7 | 9 | 12 | 10/64 = 15.6% |
| Spider region | 8 | 10 | 13 | 11/64 = 17.2% |
| Graveyard | 48 | 50 | 53 | 51/64 = 79.7% |

### Monster Count

Via `load_actors()` — `fmain.c:2723`:
```c
encounter_number = extn->v1 + rnd(extn->v2);
```

For the whole world sentinel (v1=1, v2=8): `1 + rnd(8)` = **1 to 8** monsters.
For around village/city (v1=1, v2=3): `1 + rnd(3)` = **1 to 3** monsters.
For swamp/spider (v1=1, v2=8): `1 + rnd(8)` = **1 to 8** monsters.
For graveyard (v1=8, v2=8): `8 + rnd(8)` = **8 to 15** monsters (!!).

Note: Only up to 4 enemy actor slots exist (indices 3–6), so excess encounter_number is placed over time by the placement check recycling dead slots.

## set_encounter — `fmain.c:2736-2770`

```c
set_encounter(i, spread) USHORT i, spread;
```

Returns TRUE if placed, FALSE after MAX_TRY (15) failed placement attempts.

### Position Logic

1. **DKnight fixed position**: If `extn->v3 == 7`: hardcoded (21635, 25762) — `fmain.c:2741`
2. **Normal**: Up to 15 tries — `fmain.c:2742-2747`:
   - `xtest = encounter_x + bitrand(spread) - (spread/2)`
   - `ytest = encounter_y + bitrand(spread) - (spread/2)`
   - `bitrand(n)` = `rand() & n` — `fsubs.asm:308-310`
   - For spread=63: offset range [-31, +32] from encounter origin
   - Accept if `proxcheck(xtest,ytest,i) == 0` (no collision)
   - **Astral special**: also accept if `px_to_im(xtest,ytest) == 7` — `fmain.c:2746`
3. Placement fails if j reaches MAX_TRY: return FALSE — `fmain.c:2748`

**Note**: When `extn->v3 == 7`, the loop body is never entered, so `j` is uninitialized. The `if (j==MAX_TRY)` check reads an uninitialized variable. In practice the DKnight is always placed because the check likely fails (j won't equal 15 by coincidence), but this is technically a bug.

### Race Selection — `fmain.c:2753-2755`

- If `mixflag & 2` AND encounter_type ≠ 4 (snake):
  - `race = (encounter_type & 0xfffe) + rand2()`
  - This allows mixing paired types: 0↔1 (ogre↔orc), 2↔3 (wraith↔skeleton)
- Otherwise: `race = encounter_type`

### Weapon Selection — `fmain.c:2756-2758`

1. If `mixflag & 4`: `wt = rand4()` (random column 0–3 in weapon_probs row)
2. `w = encounter_chart[race].arms * 4 + wt`
3. `an->weapon = weapon_probs[w]`

### Goal Selection — `fmain.c:2762-2763`

- If `weapon & 4` (bow): `goal = ARCHER1 + cleverness` (3 or 4)
- Else: `goal = ATTACK1 + cleverness` (1 or 2)

Goal meanings: ATTACK1=1 (stupid melee), ATTACK2=2 (clever melee), ARCHER1=3 (stupid ranged), ARCHER2=4 (clever ranged) — `ftale.h:28-31`

### Full Properties Set on Spawned Enemy

| Property | Value | Source |
|----------|-------|--------|
| abs_x, abs_y | computed position | fmain.c:2749-2750 |
| type | ENEMY | fmain.c:2752 |
| race | encounter_type or mixed | fmain.c:2753-2755 |
| weapon | weapon_probs[arms*4+wt] | fmain.c:2757-2758 |
| state | STILL (13) | fmain.c:2759 |
| environ | 0 | fmain.c:2760 |
| facing | 0 | fmain.c:2760 |
| goal | ATTACK/ARCHER + cleverness | fmain.c:2762-2763 |
| vitality | encounter_chart[race].hitpoints | fmain.c:2764 |
| rel_x | abs_x - map_x - 8 | fmain.c:2765 |
| rel_y | abs_y - map_y - 26 | fmain.c:2766 |

## load_actors — `fmain.c:2722-2735`

```c
load_actors()
```

1. Computes `encounter_number = extn->v1 + rnd(extn->v2)` — `fmain.c:2723`
2. If `actor_file != encounter_chart[encounter_type].file_id` — `fmain.c:2724`:
   - Updates `actor_file` — `fmain.c:2725`
   - Resets `anix = 3` (clears enemy slots) — `fmain.c:2726`
   - Reads new shape file from disk asynchronously — `fmain.c:2727-2728`
   - Sets `actors_loading = TRUE` — `fmain.c:2729`
   - Clears `active_carrier = 0` — `fmain.c:2730`
3. If same file already loaded: encounter_number is set but no disk I/O occurs. Monsters can be placed immediately.

### Async Load Completion

Checked every frame in main loop — `fmain.c:2052-2057`:
```c
if (actors_loading == TRUE && CheckDiskIO(8))
{   prep(ENEMY); motor_off();
    actors_loading = FALSE;
    anix = 3;
}
```

## set_loc — `fmain2.c:1714-1720`

```c
set_loc()
{   register long d,j;
    j = rand8();            /* direction 0-7 */
    d = 150 + rand64();     /* distance 150-213 */
    encounter_x = newx(hero_x,j,d);
    encounter_y = newy(hero_y,j,d);
}
```

Sets encounter origin point to a random location 150–213 pixels away from the hero in a random direction. Called during the main loop placement check (`fmain.c:2062`) to find where to cluster a new group of spawned enemies.

`newx(x, dir, speed)` / `newy(y, dir, speed)` apply the direction vector tables from `fsubs.asm:1273+` to compute screen-space offsets.

## Special Extents

### Bird / Turtle / Dragon (etype 70, indices 0–2)

When hero enters an etype 70 extent — `fmain.c:2716-2719`:
- If `xtype < 70`: `active_carrier = 0` (deactivate any carrier)
- If `xtype == 70` and no matching carrier loaded: `load_carrier(extn->v3)`

`load_carrier(n)` — `fmain.c:2784-2804`:
- Sets anim_list[3] type to DRAGON (if n==10) or CARRIER
- Places carrier at extent corner + (250, 200)
- Sets vitality=50, state=STILL, race=n
- `actor_file = active_carrier = n`

Carrier v3 values: 11=bird (swan), 5=turtle, 10=dragon.
The turtle extent initially has bounds (0,0,0,0) so it can never be entered. Must be repositioned via `move_extent()` (saved/loaded in savegames).

### Spider Pit (etype 53, index 3)

Forced encounter: spawns 4 spiders (v1=4, v3=6) immediately on entry. Uses the `xtype >= 50 && flag == 1` path — `fmain.c:2700-2713`:
- encounter_x/y = hero position
- encounter_type = v3 (6 = Spider)
- mixflag = wt = 0 (no mixing)
- Spawns v1 (4) enemies with spread 63

### Necromancer (etype 60, index 4)

Special figure: spawns necromancer (v3=9) at extent center. Uses the etype 60/61 path — `fmain.c:2687-2693`:
- Only spawns if `anim_list[3].race != 9` or `anix < 4`
- encounter_x/y = center of extent
- Falls through to forced spawn path

### Turtle Eggs (etype 61, index 5)

Special figure variant: spawns 3 snakes (v1=3, v3=4) at extent center. Same logic as etype 60 but v2=2 affects load_actors count.

### Princess (etype 83, index 6)

When entered and `ob_list8[9].ob_stat` (princess is captured) — `fmain.c:2684-2685`:
- Calls `rescue()` — `fmain2.c:1584-1605`:
  - Displays placard text (princess-specific)
  - Increments `princess` counter
  - Teleports hero to (5511, 33780)
  - Moves bird extent to (22205, 21231) via `move_extent(0,...)`
  - Grants rewards: +100 wealth, +3 to stuff[16..21], jade ring (stuff[28]), etc.
  - Clears `ob_list8[9].ob_stat = 0`
- Resets `flag = 0`, jumps to `findagain` to re-scan

### Hidden Valley DKnight (etype 60, index 15)

Special figure: spawns DKnight (v3=7) at fixed position (21635, 25762) — not at extent center. The fixed position is hardcoded in `set_encounter()` at `fmain.c:2741`.

### Graveyard (etype 48, index 7)

Regular encounter zone with very high danger. etype=48 < 50 so it uses the normal danger system:
- `danger_level = 2 + 48 = 50` (outdoor)
- Spawn probability: 51/64 ≈ 79.7% per check
- v1=8, v2=8 → 8–15 monsters per batch
- `encounter_type = rand4()` (0–3, no special override for xtype==48)
- This is the highest-density encounter zone in the game

### Astral Plane (etype 52, index 9)

Forced Loraii encounter — `fmain.c:2695-2698`:
- `encounter_type = 8` (Loraii) — hardcoded, ignoring v3
- Calls `load_actors()` synchronously + `prep(ENEMY)` + `motor_off()`
- `actors_loading = FALSE` — wait for immediate load
- Does NOT set `encounter_number` from `v1` (unlike etype 50+ path) — uses the value from `load_actors()`: `v1 + rnd(v2)` = `3 + rnd(1)` = **3** Loraii
- Special placement in `set_encounter`: accepts terrain type 7 as valid (astral-specific) — `fmain.c:2746`

## Peace Zones

### How They Work

Peace zones have `etype >= 80`, which sets `xtype >= 80`. This blocks random encounters via the condition `xtype < 50` — `fmain.c:2081`.

Additionally, for etype 81 and 82, weapon drawing is blocked — `fmain.c:1412-1415`:
```c
if (xtype > 80)
{   if (inum != SHOOT1 && xtype == 81) event(15);
    if (inum != SHOOT1 && xtype == 82) event(16);
    inum = SHOOT1;  /* force unarmed state */
```

- `event(15)`: "Even % would not be stupid enough to draw weapon in here." (king's zone) — `narr.asm:25`
- `event(16)`: "A great calming influence comes over %, preventing him from drawing his weapon." (sorceress zone) — `narr.asm:26`
- `etype 80` (generic peace): blocks encounters but does NOT block weapon drawing (80 is not > 80)

Wait — re-reading `fmain.c:1412`: `if (xtype > 80)` — this means etype 81, 82, 83 block weapons. etype 80 does NOT block weapons (80 is not > 80). So the "around city" peace zone (etype 80) allows weapon drawing despite being peaceful.

### Peace Zone Summary

| Extent | etype | Blocks encounters | Blocks weapon draw | Special |
|--------|-------|------------------|--------------------|---------|
| Around city | 80 | Yes | No | — |
| King pax | 81 | Yes | Yes (event 15) | — |
| Sorceress pax | 82 | Yes | Yes (event 16) | — |
| Peace 1–3 | 80 | Yes | No | — |
| Village | 80 | Yes | No | — |
| Princess | 83 | Yes | Yes (> 80) | Triggers rescue() |

### Non-Combat in Peace Zone

Additionally, peace zones protect against combat escalation. At `fmain.c:1669`:
```c
if (k > 15 || xtype > 80) goto cpx;
```
This prevents combat processing when xtype > 80.

## xtype — `fmain.c:575`

```c
USHORT xtype;
```

Set exclusively by `find_place()` from `extn->etype` — `fmain.c:2683`:
```c
xtype = extn->etype;
```

It serves as the global "current zone type" variable. Used in:
- **Danger level calculation**: `danger_level = base + xtype` — `fmain.c:2082-2083`
- **Encounter type overrides**: xtype==7 (swamp), xtype==8 (spider), xtype==49 — `fmain.c:2087-2090`
- **Mixflag suppression**: xtype > 49 or (xtype & 3)==0 — `fmain.c:2059-2060`
- **Peace zone test**: xtype < 50 for random encounters — `fmain.c:2081`
- **Weapon draw block**: xtype > 80 — `fmain.c:1412`
- **Combat suppression**: xtype > 80 — `fmain.c:1669`
- **Carrier management**: xtype < 70 deactivates carrier, xtype==70 loads carrier — `fmain.c:2716-2719`
- **Aftermath message suppression**: xtype < 50 for kill/flee counts — `fmain2.c:263`

## Indoor Encounters — `fmain.c:2082`

Indoor encounters (region_num > 7) differ only in:
1. **Higher danger base**: `danger_level = 5 + xtype` vs outdoor `2 + xtype` — `fmain.c:2082-2083`
   - For the standard xtype=3 zone: indoor danger=8 (12.5% chance) vs outdoor=5 (9.4% chance)
2. **Place name lookup**: Uses `inside_tbl`/`inside_msg` instead of outdoor tables — `fmain.c:2655-2656`

There is no separate indoor encounter_chart or special indoor spawn logic. The same encounter types and mechanics apply. Indoor just has a +3 danger boost.

## Unresolved

1. **xtype == 49 dead code**: No extent in extent_list has etype=49. The check at `fmain.c:2090` (`if (xtype == 49) { encounter_type = 2; mixflag = 0; }`) appears to be dead code or refers to a removed/planned extent. Cannot determine intent from source alone.

2. **Uninitialized `j` in DKnight path**: When `extn->v3 == 7`, `set_encounter()` skips the for-loop, leaving `j` uninitialized. The subsequent `if (j==MAX_TRY) return FALSE` check reads garbage. Likely a bug that works in practice because uninitialized stack values rarely equal exactly 15.

3. **`aggressive` field unused**: The `agressive` (sic) field in encounter_chart is set but never read anywhere in the codebase. Its purpose — whether it was once used for AI behavior or is purely documentary — cannot be determined from source.

4. **etype 48 vs 49 intent**: The graveyard has etype=48, which makes it a regular encounter zone. Whether the much-higher-than-normal etype value (48 vs typical 3–8) is intentional game design or whether etype 48/49 were meant to have special handling is unclear.

5. **Turtle extent repositioning**: The turtle extent starts at (0,0,0,0) and is only saved/loaded. The code path that first repositions it (making the turtle accessible) is not obvious from the encounter system alone — it may be in quest logic or NPC interactions.

## Cross-Cutting Findings

- **Wraith corpse recycling**: Dead wraiths (race==2) can be overwritten by new spawns even while visible on screen — `fmain.c:2071`. This means wraith bodies disappear faster than other monster types.
- **Safe zone tracking**: The safe_x/safe_y respawn point is only updated when `!actors_on_screen && !actors_loading && !witchflag` — `fmain.c:2189-2194`. Being in constant combat prevents safe zone updates, so death respawns you at the last peaceful location.
- **Auto apple eating**: During safe zone updates, if hunger > 30 and player has apples (stuff[24]), one is consumed automatically — `fmain.c:2195-2196`.
- **Load game resets**: Loading a savegame clears `wt`, `encounter_number`, `encounter_type`, and `actors_loading` to 0 — `fmain2.c:1542-1547`.
- **Graveyard enclosed by peace**: The graveyard (idx 7) is physically inside the "around city" peace zone (idx 8). First-match priority means the graveyard functions as a deadly enclave within a peaceful region.
- **xtype debug display**: `fmain2.c:455` shows xtype in debug output: `text(" Extent = ",10); prdec(xtype,2);`

## Refinement Log

- 2026-04-05: Initial complete discovery pass covering all 11 requested questions.
