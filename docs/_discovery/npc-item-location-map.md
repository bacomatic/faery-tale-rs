# Discovery: NPC & Quest Item Location Map

**Status**: complete
**Investigated**: 2025-01-15
**Requested by**: orchestrator
**Prompt summary**: Map all NPC and quest item locations by extracting all object list arrays, setfig_table identity mappings, extent_list spawns, carrier definitions, and door connections; cross-reference against world_db.json for place names and sector data.

## Source Data Overview

All object lists in `fmain2.c:1001-1168`. The `setfig_table` at `fmain.c:24-39` maps ob_id to NPC type. The `extent_list` at `fmain.c:338-370` defines special encounter zones. The `doorlist` at `fmain.c:239-326` defines 86 building entrances. The `encounter_chart` at `fmain.c:52-64` defines spawned enemies including quest bosses.

## NPC Identity Table (setfig_table)

From `fmain.c:24-39`:

| ob_id | NPC Type | cfile | image_base | can_talk | race code |
|-------|----------|-------|------------|----------|-----------|
| 0 | Wizard | 13 | 0 | yes | 0x80 |
| 1 | Priest | 13 | 4 | yes | 0x81 |
| 2 | Guard | 14 | 0 | no | 0x82 |
| 3 | Guard (back) | 14 | 1 | no | 0x83 |
| 4 | Princess | 14 | 2 | no | 0x84 |
| 5 | King | 14 | 4 | yes | 0x85 |
| 6 | Noble | 14 | 6 | no | 0x86 |
| 7 | Sorceress | 14 | 7 | no | 0x87 |
| 8 | Bartender | 15 | 0 | no | 0x88 |
| 9 | Witch | 16 | 0 | no | 0x89 |
| 10 | Spectre | 16 | 6 | no | 0x8A |
| 11 | Ghost | 16 | 7 | no | 0x8B |
| 12 | Ranger | 17 | 0 | yes | 0x8C |
| 13 | Beggar | 17 | 4 | yes | 0x8D |

## Goal/Speech Assignment Mechanism

When a setfig NPC is spawned by `set_objects()`, its `an->goal` is set to `i` -- the object's index within its region list (`fmain2.c:1275`). This goal value determines which speech text the NPC delivers:

- **Wizards**: `speak(27 + an->goal)` when `kind >= 10`; `speak(35)` otherwise -- `fmain.c:3380-3381`
- **Rangers**: `speak(53 + an->goal)` in most regions; `speak(22)` if `region_num == 2` -- `fmain.c:3412-3413`
- **Beggars**: Talk = `speak(23)` always; Give = `speak(24 + an->goal)` -- `fmain.c:3415,3498`
- **Priest**: `speak(36 + daynight%3)` normally; `speak(39)` when presenting writ -- `fmain.c:3382-3393`
- **Sorceress**: `speak(45)` on first visit (sets ob_listg[9].ob_stat=1) -- `fmain.c:3400-3404`
- **Witch**: `speak(46)` -- `fmain.c:3409`
- **Spectre**: `speak(47)` -- `fmain.c:3410`
- **King**: `speak(17)` if princess rescued (ob_list8[9].ob_stat set) -- `fmain.c:3398`
- **Princess**: `speak(16)` if rescued -- `fmain.c:3397`
- **Bartender**: `speak(13)` if not fatigued, `speak(12)` if night, `speak(14)` otherwise -- `fmain.c:3406-3408`
- **Ghost**: `speak(49)` -- `fmain.c:3411`

---

## COMPLETE NPC LOCATION MAP

### Wizards (ob_id=0, 8 total)

| Source | Coords | Region | Location | Goal | Speech |
|--------|--------|--------|----------|------|--------|
| ob_list2[0] | (13668,15000) | 2 - Swamp | Swamp land, outdoor | 0 | speak(27) |
| ob_list2[1] | (10627,13154) | 2 - Swamp | Swamp land, outdoor | 1 | speak(28) |
| ob_list3[2] | (20033,14401) | 3 - South Forest | Near Tambry, outdoor | 2 | speak(29) |
| ob_list5[3] | (22956,19955) | 5 - Farm/City | Near Marheim, outdoor | 3 | speak(30) |
| ob_list5[4] | (28342,22613) | 5 - Farm/City | Mountains near Marheim, outdoor | 4 | speak(31) |
| ob_list8[5] | (8878,38995) | 8 - Indoors | Cabin interior (sec 125), accessible from multiple cabins | 5 | speak(32) |
| ob_list8[6] | (7776,34084) | 8 - Indoors | City building #19 in Marheim (sec 95), door[61] outside r5 (22368,21568) | 6 | speak(33) |
| ob_list9[6] | (3460,37260) | 9 - Underground | Tombs of Hemsath tunnels (sec 22), door[20] outside r4 (13424,19296) | 6 | speak(33) |

### Rangers (ob_id=12, 4 total)

| Source | Coords | Region | Location | Goal | Speech |
|--------|--------|--------|----------|------|--------|
| ob_list0[0] | (3340,6735) | 0 - Snow | Mountains of Frost, western area | 0 | speak(53) |
| ob_list0[1] | (9678,7035) | 0 - Snow | Snow land, eastern area | 1 | speak(54) |
| ob_list0[2] | (4981,6306) | 0 - Snow | Snow land, northern area | 2 | speak(55) |
| ob_list2[2] | (4981,10056) | 2 - Swamp | Swamp land | 2 | speak(22) [region override] |

### Beggars (ob_id=13, 5 total -- 1 dummy)

| Source | Coords | Region | Location | Goal | Give Speech |
|--------|--------|--------|----------|------|-------------|
| ob_list3[1] | (18310,15969) | 3 - South Forest | Near village of Tambry | 1 | speak(25) |
| ob_list3[3] | (24794,13102) | 3 - South Forest | Great Bog (swamp) area | 3 | speak(27) |
| ob_list4[2] | (6817,19693) | 4 - Desert | Hidden city of Azal | 2 | speak(26) |
| ob_list5[0] | (22184,21156) | 5 - Farm/City | Near city of Marheim | 0 | speak(24) |
| ob_list6[0] | (24794,13102) | 6 - Lava | **DUMMY** -- same coords as ob_list3[3], comment says "DUMMY OBJECT" | -- | -- |

### Priest (ob_id=1, 1 total)

| Source | Coords | Region | Location |
|--------|--------|--------|----------|
| ob_list8[0] | (6700,33756) | 8 - Indoors | Castle of King Mar, chapel (sec 90). Door[50] main castle outside r5 (22000,21216) |

**Quest role**: Heals hero (`vitality = 15 + brave/4`) and calls `prq(4)`. When `stuff[28]` (writ from princess rescue) is set and `ob_listg[10].ob_stat == 0`, gives unique speech `speak(39)` and reveals gold statue `ob_listg[10].ob_stat = 1` -- `fmain.c:3382-3393`.

### King (ob_id=5, 1 total)

| Source | Coords | Region | Location |
|--------|--------|--------|----------|
| ob_list8[1] | (5491,33780) | 8 - Indoors | Castle of King Mar, throne room (sec 84). Door[50] outside r5 (22000,21216) |

**Quest role**: Speech changes after princess rescue -- `speak(17)` when `ob_list8[9].ob_stat` set -- `fmain.c:3398`.

### Princess (ob_id=4, 1 total)

| Source | Coords | Region | Location |
|--------|--------|--------|----------|
| ob_list8[9] | (10853,35656) | 8 - Indoors | Unreachable castle interior (sec 129). Door[67] outside r7 (22944,26464) southern mountains |

**Quest role**: Triggers `rescue()` when hero enters extent[6] (10820,35646)-(10877,35670) and `ob_list8[9].ob_stat` is nonzero -- `fmain.c:2684-2685`. Her `ob_stat` is set to 3 on brother death/succession (`fmain.c:2843`) and cleared to 0 by `rescue()` (`fmain2.c:1601`). Speech `speak(16)` after rescue -- `fmain.c:3397`.

### Sorceress (ob_id=7, 1 total)

| Source | Coords | Region | Location |
|--------|--------|--------|----------|
| ob_list8[10] | (12037,37614) | 8 - Indoors | Crystal Palace interior (sec 122). Doors[21-22] outside r0 (15840-15872,7104) snow land |

**Quest role**: First visit gives `speak(45)` and sets `ob_listg[9].ob_stat = 1` (reveals sorceress gold statue). Subsequent visits give luck bonus (`if luck < rand64() then luck += 5`). Calls `prq(7)` -- `fmain.c:3400-3404`. Peace zone extent[11] (11712,37350)-(12416,38020) surrounds her -- `fmain.c:355`.

### Witch (ob_id=9, 1 total)

| Source | Coords | Region | Location |
|--------|--------|--------|----------|
| ob_list8[11] | (11013,36804) | 8 - Indoors | Witch's Castle interior (sec 110). Door[79] outside r1 (26624,7008) maze forest north |

**Quest role**: `speak(46)` when talked to -- `fmain.c:3409`. When `witchflag` is TRUE (witch NPC is on-screen, `fmain2.c:1258`) and Sun Stone is used (`hit == 8`), triggers `speak(60)` -- `fmain.c:3462`. Drops lasso (ob_id=27) on death via `leave_item(i, 27)` -- `fmain.c:1756`.

### Spectre (ob_id=10, 1 total -- GLOBAL object)

| Source | Coords | Region | Location |
|--------|--------|--------|----------|
| ob_listg[5] | (12439,36202) | 8 - Indoors | Crypt interior (sec 114). Door[42] "crypt" outside r5 (19856,17280) |

**Visibility**: Dynamically toggled by day/night cycle -- `ob_stat = 3` (visible) when `lightlevel < 40`, `ob_stat = 2` (hidden) otherwise -- `fmain.c:2027-2028`. Only visible at night.

**Quest role**: `speak(47)` when talked to -- `fmain.c:3410`. When hero uses Give with bone item (`hit == 8 && stuff[29]`), spectre (race 0x8A) gives `speak(48)`, clears `stuff[29]`, and drops crystal shard via `leave_item(nearest_person, 140)` -- `fmain.c:3501-3503`.

### Bartenders (ob_id=8, 4 total)

| Source | Coords | Region | Location | Accessible via |
|--------|--------|--------|----------|----------------|
| ob_list8[12] | (9631,38953) | 8 | Shared inn interior (sec 104) | Road's End Inn (door[19] r2), Friendly Inn (door[24] r3), Forest Inn (door[26] r3) |
| ob_list8[13] | (10191,38953) | 8 | Shared inn interior (sec 104) | Same three inns |
| ob_list8[14] | (10649,38953) | 8 | Shared inn interior (sec 104) | Same three inns |
| ob_list8[15] | (2966,33964) | 8 | Tambry tavern (sec 65) | Village #1 (door[32] r3 (18896,15808), village of Tambry) |

**Function**: Buy menu for food/rest -- `fmain.c:3424-3430`.

### Guards (ob_id=2 forward, ob_id=3 back, 4 total)

| Source | Coords | Region | Location |
|--------|--------|--------|----------|
| ob_list8[3] | (5514,33668) | 8 | Castle of King Mar, throne room (sec 84) |
| ob_list8[4] | (5574,33668) | 8 | Castle of King Mar, throne room (sec 84) |
| ob_list8[7] | (5514,33881) | 8 | Castle of King Mar, corridor (sec 88) |
| ob_list8[8] | (5574,33881) | 8 | Castle of King Mar, corridor (sec 88) |

### Noble (ob_id=6, 1 total)

| Source | Coords | Region | Location |
|--------|--------|--------|----------|
| ob_list8[2] | (5592,33764) | 8 | Castle of King Mar, throne room (sec 84) |

**Note**: After princess rescue, `ob_list8[2].ob_id` is changed to 4 (princess), transforming the noble into the rescued princess -- `fmain2.c:1597`.

### Ghost Brothers (ob_id=11, 2 total -- GLOBAL, initially disabled)

| Source | Coords | Region | Location |
|--------|--------|--------|----------|
| ob_listg[3] | (19316,15747) | 3 - South Forest | Near village of Tambry, outdoor |
| ob_listg[4] | (18196,15735) | 3 - South Forest | Near village of Tambry, outdoor |

**Activation**: `ob_stat` set to 3 (visible) during brother succession when a brother dies -- `fmain.c:2841`. Set to 0 when bones are picked up -- `fmain.c:3174`.

---

## EXTENT-SPAWNED ENCOUNTERS (Quest Bosses)

These NPCs are not in object lists; they spawn dynamically from extent triggers.

### Necromancer (encounter_chart[9])

| Field | Value | Source |
|-------|-------|--------|
| Extent | [4]: (9563,33883)-(10144,34462) | fmain.c:344 |
| Region | 8 - Indoors (arena area) | -- |
| Type | etype=60, v1=1, v2=1, v3=9 | fmain.c:344 |
| HP | 50 | fmain.c:63 |
| Arms | 5 | fmain.c:63 |
| Cleverness | 0 | fmain.c:63 |
| Actor file | 9 | fmain.c:63 |

On approach: `speak(43)` -- `fmain.c:2100`. On entering extent with v3==9: magic is blocked (`speak(59)`) -- `fmain.c:3304`. Fixed spawn position at center of extent. Drops talisman (ob_id=139) on death -- `fmain.c:1754`.

### Dark Knight (encounter_chart[7])

| Field | Value | Source |
|-------|-------|--------|
| Extent | [15]: (21405,25583)-(21827,26028) | fmain.c:360 |
| Region | 7 - Southern Mountains | -- |
| Type | etype=60, v1=1, v2=1, v3=7 | fmain.c:360 |
| Fixed spawn | (21635, 25762) | fmain.c:2741 |
| HP | 40 | fmain.c:60 |
| Arms | 7 | fmain.c:60 |
| Cleverness | 1 | fmain.c:60 |
| Actor file | 8 | fmain.c:60 |

On approach: `speak(41)` -- `fmain.c:2101`. On death: `speak(42)` -- `fmain.c:2775`. Special AI: stands still facing hero (`an->state = STILL; an->facing = 5`) unless in melee range -- `fmain.c:2168-2169`. Extended melee threshold of 16 pixels (vs normal 14) -- `fmain.c:2163`.

### Turtle Egg Snakes (encounter_chart[4])

| Field | Value | Source |
|-------|-------|--------|
| Extent | [5]: (22945,5597)-(23225,5747) | fmain.c:345 |
| Region | 1 - Maze Forest North | -- |
| Type | etype=61, v1=3, v2=2, v3=4 | fmain.c:345 |
| Spawns | 3-4 snakes (v1 + rnd(v2)) | -- |

Snakes use EGG_SEEK tactic when `turtle_eggs` is set -- `fmain.c:2150`. After combat near turtle eggs, `get_turtle()` is called -- `fmain2.c:274`, `fmain.c:3510-3518`. This positions the turtle extent and loads the turtle carrier.

---

## CARRIER/MOUNT LOCATIONS

From `fmain.c:2784-2801` (`load_carrier`):

| Carrier | Actor file | Extent | Initial Position | Trigger |
|---------|-----------|--------|-----------------|---------|
| Bird/Swan | 11 | [0]: (2118,27237)-(2618,27637) r6 | Spawn at (2368,27437) | `move_extent(0,...)` repositions; 'B' cheat or rescue rewards |
| Turtle | 5 | [1]: dynamically positioned | Positioned by `get_turtle()` | Defeating snakes near turtle eggs (extent[5]) |
| Dragon | 10 | [2]: (6749,34951)-(7249,35351) r8 | Spawn at (6999,35151) | Entering extent area in indoor map |

**Swan after rescue**: `rescue()` calls `move_extent(0, 22205, 21231)` -- repositions bird extent to (21955,21031)-(22455,21431) in region 5 (farm/city area) -- `fmain2.c:1596`.

---

## GOLD STATUES (5 total, in ob_listg[6-10])

| Index | Coords | Region | Location | Initial stat | How revealed |
|-------|--------|--------|----------|-------------|--------------|
| ob_listg[6] | (11092,38526) | 8 | Castle interior (sec 115) -- "seahold" | 1 (visible) | Always visible |
| ob_listg[7] | (25737,10662) | 3 | Outdoor, south forest -- "ogre den" | 1 (visible) | Always visible |
| ob_listg[8] | (2910,39023) | 8 | Octagonal room (sec 36) inside Azal buildings | 1 (visible) | Always visible |
| ob_listg[9] | (12025,37639) | 8 | Crystal Palace interior (sec ~122) -- "sorceress" | 0 (hidden) | Talk to Sorceress sets stat=1 (`fmain.c:3403`) |
| ob_listg[10] | (6700,33766) | 8 | Castle of King Mar chapel area -- "priest" | 0 (hidden) | Present writ to Priest sets stat=1 (`fmain.c:3385`) |

---

## KEY QUEST ITEMS

| Item | ob_id | Source | Coords | Region | Location |
|------|-------|--------|--------|--------|----------|
| Sunstone | 155 (27+128) | ob_list8[18] | (11410,36169) | 8 | Elf Glade sanctuary (sec 127), door[48] outside r7 (21616,25728) |
| King's Bone | 138 (128+10) | ob_list9[8] | (3723,39340) | 9 | Tombs underground, door[20] outside r4 (13424,19296) Tombs of Hemsath |
| Sea Shell (indoor) | 151 | ob_list8[48] | (10344,36171) | 8 | Swamp shack interior, door[13] outside r2 (9344,13216) |
| Sea Shell (region 2) | 151 | ob_list2[4] | (10344,36171) | 2* | Same coords as ob_list8[48] -- duplicate/cross-region anomaly |
| Rose | 147 | ob_list8[51] | (5473,38699) | 8 | Building interior, near oasis buildings in Azal area |
| Spectre Note | 20 (SCRAP) | ob_list8[49] | (11936,36207) | 8 | Near elf glade/witch area interior |
| Outdoor Scrap | 20 (SCRAP) | ob_list3[11] | (17177,10599) | 3 | South forest, outdoor |
| Red Key | 242 | ob_list8[44] | (11652,38481) | 8 | Castle interior (sec ~115) |
| Red Key (hidden) | 242 | ob_list8[77] | (7313,38992) | 8 | Cabin interior (sec 125), revealed by Look |
| Turtle Eggs | 102 | ob_list1[0] | (23087,5667) | 1 | Maze forest north (Mountains of Frost) |

---

## DOOR-TO-BUILDING MAP (Quest-Relevant Buildings)

| Building | Door(s) | Outside Coords | Outside Region | Inside Coords |
|----------|---------|---------------|----------------|---------------|
| Main Castle (King Mar) | [50] | (22000,21216) | 5 - Farm/City | (5856,33760) |
| Crystal Palace | [21],[22] | (15840-15872,7104) | 0 - Snow | (12000-12032,37824) |
| Witch's Castle | [79] | (26624,7008) | 1 - Forest North | (10992,36960) |
| Crypt | [42] | (19856,17280) | 5 - Farm/City | (12416,36224) |
| Unreachable Castle | [67] | (22944,26464) | 7 - South Mountains | (10912,35680) |
| Elf Glade (Sanctuary) | [48] | (21616,25728) | 7 - South Mountains | (11392,36224) |
| Tombs of Hemsath | [20] | (13424,19296) | 4 - Desert | (1136,36576) r9 |
| Citadel of Doom | [16] | (11264,29024) | 6 - Lava | (10992,37728) |
| Mammoth Manor | [73] | (24816,12992) | 3 - South Forest | (9712,35776) |
| Road's End Inn | [19] | (12672,14528) | 2 - Swamp | (10112,39104) |
| Friendly Inn | [24] | (17024,15296) | 3 - South Forest | (10624,39104) |
| Forest Inn | [26] | (18304,12224) | 3 - South Forest | (9600,39104) |
| Village/Tambry Tavern | [32],[33] | (18896,15808-15872) | 3 - South Forest | (3024,33984-34048) |
| Dragon Cave | [4] | (5008,7008) | 0 - Snow | (6528,35936) r9 |

---

## PEACE/SAFE ZONES (No combat)

| Extent | Coords | Type | Region | Description | Source |
|--------|--------|------|--------|-------------|--------|
| [8] | (19400,17034)-(20240,17484) | 80 | 5 | Around city of Marheim | fmain.c:348 |
| [10] | (5272,33300)-(6112,34200) | 81 | 8 | King's domain (castle area) | fmain.c:354 |
| [11] | (11712,37350)-(12416,38020) | 82 | 8 | Sorceress domain | fmain.c:355 |
| [12] | (2752,33300)-(8632,35400) | 80 | 8 | Peace 1 -- buildings area | fmain.c:356 |
| [13] | (10032,35550)-(12976,40270) | 80 | 8 | Peace 2 -- special areas | fmain.c:357 |
| [14] | (4712,38100)-(10032,40350) | 80 | 8 | Peace 3 -- cabin areas | fmain.c:359 |
| [19] | (18687,15338)-(19211,16136) | 80 | 3 | Village of Tambry | fmain.c:365 |

---

## CROSS-REFERENCING: Source vs world_db.json

The `world_db.json` was verified against source code and found **consistent** in all cases:

1. **Object coordinates**: All 129 objects in world_db match source arrays exactly.
2. **Door coordinates**: All 86 doors match `doorlist[]` exactly (hex values converted to decimal).
3. **Extent coordinates**: All 23 extents match `extent_list[]` exactly.
4. **NPC identities**: All ob_id values correctly mapped to setfig names.

**Anomalies found in source (also reflected in world_db)**:
- `ob_list2[4]` shell at (10344,36171) has y-coordinate in region 8 space (y >= 32768), despite being in region 2's list -- same coords as `ob_list8[48]`. Appears to be a data oddity/duplicate.
- `ob_list6[0]` and `ob_list7[0]` are dummy/placeholder entries per source comments.
- `ob_list6[0]` (lava plain "beggar") shares exact coordinates (24794,13102) with `ob_list3[3]` (south forest beggar) -- source comment says "DUMMY OBJECT".

## Cross-Cutting Findings

- **Noble-to-Princess transformation**: After `rescue()`, `ob_list8[2].ob_id` changes from 6 (noble) to 4 (princess) -- `fmain2.c:1597`. The noble in the castle throne room becomes the rescued princess.
- **Spectre night-only visibility**: Global object `ob_listg[5]` toggles `ob_stat` between 3 and 2 every tick based on `lightlevel` -- `fmain.c:2027-2028`. This is in the main game loop, not the object system.
- **Wizard speech is positional**: The wizard's speech index depends on its position in its region's object list (goal = list index), NOT on a separate hint table -- `fmain2.c:1275`, `fmain.c:3381`.
- **Magic blocked in necromancer arena**: When `extn->v3 == 9`, magic use gives `speak(59)` and aborts -- `fmain.c:3304`. The necromancer fight is purely physical.
- **Bird extent relocates after rescue**: `rescue()` calls `move_extent(0, 22205, 21231)` moving the bird/swan carrier extent to region 5 -- `fmain2.c:1596`. This provides the swan mount as a reward.
- **Princess rescue state tracked via ob_list8[9].ob_stat**: This single field controls: rescue trigger activation (`fmain.c:2684`), king's dialogue change (`fmain.c:3398`), princess dialogue change (`fmain.c:3397`), and approach speech (`fmain.c:2099`). Set to 3 on brother succession (`fmain.c:2843`), cleared to 0 by `rescue()` (`fmain2.c:1601`).
- **Indoor wizard (ob_list8[5]) inhabits shared cabin space**: Since all 10 cabins share the same indoor region 8 tile space at sector 125, this wizard may be visible when entering certain cabins whose inside coordinates are near (8878,38995).

## Unresolved

None -- all object lists, NPC identities, extent spawns, doors, and cross-references fully traced from source.

## Refinement Log
- 2025-01-15: Initial comprehensive discovery -- complete NPC/item location map with building mappings, speech assignments, quest roles, and world_db cross-reference.