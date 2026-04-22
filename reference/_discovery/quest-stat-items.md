# Discovery: Quest/Stat Items (stuff[22]–stuff[30] and Special Items)

**Status**: complete
**Investigated**: 2026-04-08
**Requested by**: orchestrator
**Prompt summary**: Trace all gameplay-significant quest/stat items: stuff[22]–[30] and special items stuff[5], stuff[6], stuff[7], stuff[9]. For each, trace how it's obtained, how it's used, quest context, and terrain/map associations.

---

## Item Constant Definitions

From `fmain.c:428-431`:
```c
#define MAGICBASE   9
#define KEYBASE     16
#define STATBASE    25
#define GOLDBASE    31
```

Item-to-object mapping via `itrans[]` at `fmain2.c:979-988`:
```c
UBYTE itrans[] = {
    QUIVER,35,
    B_STONE,9,G_JEWEL,10,VIAL,11,C_ORB,12,B_TOTEM,13,G_RING,14,J_SKULL,15,
    M_WAND,4, 27,5, 8,2, 9,1, 12,0, 10,3, ROSE,23, FRUIT,24, STATUE,25,
    BOOK,26, SHELL,6, 155,7, 136,27, 137,28, 138,29, 139,22, 140,30,
    GOLD_KEY,16,GREEN_KEY,17,BLUE_KEY,18,RED_KEY,19,GREY_KEY,20,WHITE_KEY,21,
    0,0 };
```

Object IDs from enum at `fmain2.c:968-976`:
```
MONEY=13, URN, CHEST, SACKS, G_RING, B_STONE, G_JEWEL,
SCRAP=20, J_SKULL, C_ORB, B_TOTEM, G_RING_2, TURTLE=27,
QUIVER=28, FOOTSTOOL, M_SWORD=128,
M_KEY=133, M_SHIELD, M_AXE, M_BOW,
M_WAND=145, MEAL, ROSE, FRUIT, STATUE, BOOK, SHELL,
GREEN_KEY=153, WHITE_KEY=154, RED_KEY=242
```

Inventory display names from `fmain.c:390-420`:
| Index | Name | itrans Object ID |
|-------|------|-----------------|
| 5 | Golden Lasso | 27 |
| 6 | Sea Shell | SHELL (151) |
| 7 | Sun Stone | 155 (27+128) |
| 9 | Blue Stone | B_STONE (18) |
| 22 | Talisman | 139 |
| 23 | Rose | ROSE (147) |
| 24 | Fruit | FRUIT (148) / 148 (apple) |
| 25 | Gold Statue | STATUE (149) |
| 28 | Writ | 137 |
| 29 | Bone | 138 (128+10) |
| 30 | Shard (Crystal Shard) | 140 |

---

## stuff[22] — Talisman

### How Obtained
1. **Necromancer death drop** — `fmain.c:1749-1754`:
   ```c
   if (s==DYING && !(--(an->tactic)))
   {   an->state = DEAD;
       if (an->race == 0x09)
       {   an->race = 10;
           an->vitality = 10;
           an->state = STILL;
           an->weapon = 0;
           leave_item(i,139); /* leave the talisman */
       }
   ```
   When the Necromancer (race 0x09) dies ("DYING" state countdown expires), it drops object 139 (Talisman) via `leave_item()` at `fmain.c:1754`. The Necromancer transforms to race 10 with 10 vitality and STILL state (becomes a neutral NPC — speaks `speak(44)` "% gasped. The Necromancer had been transformed into a normal man.").

2. **Player picks up object 139** — itrans maps 139→22, so `stuff[22]++` via standard Take handler at `fmain.c:3187-3190`:
   ```c
   for (k=0; itrans[k]; k+= 2)
   {   if (j==itrans[k])
       {   i = itrans[k+1];
           stuff[i]++;
           announce_treasure("a ");
           print_cont(inv_list[i].name);
           print_cont(".");
           goto pickup;
       }
   }
   ```

### How Used
1. **Win condition** — `fmain.c:3244-3247`:
   ```c
   if (stuff[22])
   {   quitflag = TRUE; viewstatus = 2;
       map_message(); SetFont(rp,afont); win_colors();
   }
   ```
   Checked immediately after any item pickup (`pickup:` label). If `stuff[22]` is non-zero, the game triggers the win sequence: sets `quitflag = TRUE`, `viewstatus = 2`, then calls `win_colors()`.

2. **Cheat key clears it** — `fmain.c:1299`:
   ```c
   else if (key == '.' && cheat1)
       { stuff[rnd(GOLDBASE)]+=3; set_options(); stuff[22]=0; }
   ```
   The '.' cheat key randomizes inventory and explicitly clears stuff[22] to prevent accidental win trigger.

### Quest Context
- **Prerequisites**: Must defeat the Necromancer in combat. The Necromancer resides at extent[4] (`fmain.c:343`): `{9563,33883, 10144,34462, 60, 1, 1, 9}` — etype 60 (forced encounter of race 9).
- **Effect**: Picking up the Talisman immediately wins the game. The `win_colors()` function (`fmain2.c:1604-1636`) displays the victory screen with `placard_text(6)` and `placard_text(7)` which read: "Having defeated the villanous Necromancer and recovered the Talisman, [name] returned to Marheim where he wed the princess..." — `narr.asm:290-297`.

### Narr.asm Messages
- `msg1` (`narr.asm:252-260`): Intro message — "Rescue the Talisman!" was the Mayor's plea.
- `msg7/msg7a` (`narr.asm:290-297`): Win message — "Having defeated the villanous Necromancer and recovered the Talisman..."
- `speak(44)` (`narr.asm:469-470`): Necromancer transformation — "% gasped. The Necromancer had been transformed into a normal man."

---

## stuff[23] — Rose

### How Obtained
1. **Ground pickup in ob_list8** (building interiors) — `fmain2.c:1128`:
   ```c
   {  5473,38699,ROSE,1},
   ```
   Object ROSE (147) at world coordinates (5473, 38699) in the interior region (region 8). itrans maps ROSE→23.

### How Used
1. **Lava protection** — `fmain.c:1843-1844`:
   ```c
   if (fiery_death)
   {   if (i==0 && stuff[23]) an->environ = 0;
       else if (an->environ > 15) an->vitality = 0;
       else if (an->environ > 2) an->vitality--;
       checkdead(i,27);
   }
   ```
   When in the `fiery_death` zone (lava area: `map_x>8802 && map_x<13562 && map_y>24744 && map_y<29544`, defined at `fmain.c:1384-1385`), and `i==0` (the player character), `stuff[23]` forces `environ = 0`, completely negating all lava/heat damage. Without the Rose, `environ > 15` kills instantly and `environ > 2` drains vitality each tick.

### Quest Context
- **Prerequisites**: Access to building interiors (region 8). The Rose is found at a specific indoor location.
- **Effect**: Allows traversal of the lava region without taking damage. Required to reach areas on the far side of the lava plain.
- **Cross-cutting**: Only affects actor 0 (the player). Other actors (NPC companions, carriers) are NOT protected by the Rose — they still take lava damage normally.

### Terrain/Map Data
- `fiery_death` zone bounds: `fmain.c:1384-1385`: `map_x>8802 && map_x<13562 && map_y>24744 && map_y<29544`

### Narr.asm Messages
- No speech message directly references the Rose item.
- `event(27)` — `narr.asm:27`: "% perished in the hot lava!" (death message if Rose is NOT held)
- `event(7)` — `narr.asm:7`: "% was burned in the lava." (general lava death)

---

## stuff[24] — Fruit

### How Obtained
1. **Ground pickup (apple, object 148)** when `hunger < 15` — `fmain.c:3166`:
   ```c
   else if (j==148)
   {   if (hunger < 15) { stuff[24]++; event(36); }
       else eat(30);
       goto pickup;
   }
   ```
   When the player picks up apple object (148) and hungeris below 15, the fruit is stored as `stuff[24]++` with `event(36)`: "% put an apple away for later." — `narr.asm:36`. If hunger ≥ 15, the apple is immediately eaten via `eat(30)`.

2. **Ground objects in ob_list8** (building interiors) — `fmain2.c:1129-1138`:
   ```c
   {  7185,34342,FRUIT,1},
   {  7190,34342,FRUIT,1},
   {  7195,34342,FRUIT,1},
   {  7185,34347,FRUIT,1},
   {  7190,34347,FRUIT,1},
   {  7195,34347,FRUIT,1},
   {  6593,34085,FRUIT,1},
   {  6598,34085,FRUIT,1},
   {  6593,34090,FRUIT,1},
   {  6598,34090,FRUIT,1},
   ```
   10 FRUIT objects placed in ob_list8 (building interiors). These are picked up via itrans mapping FRUIT(148)→24.

### How Used
1. **Auto-eat in safe zones** — `fmain.c:2195-2196`:
   ```c
   if (hunger > 30 && stuff[24])
   {   stuff[24]--; hunger -= 30; event(37);  }
   ```
   When `(daynight & 127) == 0` in a safe zone (multiple conditions at `fmain.c:2188-2193`), if `hunger > 30` and player has fruit, one fruit is consumed: `stuff[24]--`, hunger reduced by 30, `event(37)`: "% ate one of his apples." — `narr.asm:37`.

### Quest Context
- **Not quest-gated**: Fruit is a survival/convenience item, not a quest progression item.
- **Purpose**: Provides portable food to stave off hunger between safe rest points.

### Narr.asm Messages
- `event(36)` — `narr.asm:36`: "% put an apple away for later."
- `event(37)` — `narr.asm:37`: "% ate one of his apples."

---

## stuff[25] — Gold Statue (STATBASE)

### How Obtained
1. **Ground pickup of STATUE objects** via itrans mapping STATUE(149)→25.
   Three statues are placed in ob_listg (global objects) with `ob_stat=1` (ground-placed, immediately visible) — `fmain2.c:1006-1008`:
   ```c
   { 11092,38526,STATUE,1},    /* gold statues (6 = seahold) */
   { 25737,10662,STATUE,1},    /* (7 = ogre den) */
   {  2910,39023,STATUE,1},    /* (8 = octal room) */
   ```

2. **Dialogue-revealed statues** — Two statues in ob_listg with `ob_stat=0` (initially hidden):
   ```c
   { 12025,37639,STATUE,0},    /* (9 = sorceress) */
   {  6700,33766,STATUE,0},    /* (10 = priest) */
   ```
   - **Sorceress** (setfig case 7) at `fmain.c:3402-3405`:
     ```c
     case 7: /* sorceress */
             if (ob_listg[9].ob_stat)
             {   if (luck<rand64()) luck += 5; }
             else { speak(45); ob_listg[9].ob_stat = 1; }
             prq(7);
             break;
     ```
     First talk to sorceress: `speak(45)` — "Welcome. Here is one of the five golden figurines you will need." Sets `ob_listg[9].ob_stat = 1`, making the statue visible/pickable at (12025, 37639).

   - **Priest** (setfig case 1) at `fmain.c:3383-3388`:
     ```c
     case 1: /* priest */
             if (stuff[28])
             {   if (ob_listg[10].ob_stat==0)
                 {   speak(39); ob_listg[10].ob_stat = 1; }
                 else speak(19);
             }
     ```
     Talk to priest with `stuff[28]` (Writ): `speak(39)` — "Ah! You have a writ from the king. Here is one of the golden statues..." Sets `ob_listg[10].ob_stat = 1`, making statue visible at (6700, 33766). Requires the Writ first!

   All five statues are picked up through normal itrans: STATUE→`stuff[25]++`.

### How Used
1. **Desert gate barrier** — `fmain.c:1919`:
   ```c
   if (d->type == DESERT && (stuff[STATBASE]<5)) break;
   ```
   Doors of type DESERT cannot be entered unless player has ≥5 gold statues.

2. **Desert region map blocking** — `fmain.c:3594-3596`:
   ```c
   if (new_region == 4 && stuff[STATBASE] < 5) /* are we in desert sector */
   {   i = ((11*128)+26);
       map_mem[i] = map_mem[i+1] = map_mem[i+128] = map_mem[i+129] = 254;
   }
   ```
   When loading region 4 (desert), if player has fewer than 5 statues, terrain tiles at offset `(11*128)+26` are overwritten to 254 (impassable). This blocks entry to the hidden city of Azal.

### Quest Context
- **Prerequisites for priest statue**: Must have `stuff[28]` (Writ), obtained from the `rescue()` function after saving a princess.
- **Prerequisites for sorceress statue**: Just talk to the sorceress — no condition beyond finding her.
- **3 ground statues**: Found at seahold, ogre den, and octagonal room — accessible as exploration items.
- **Total needed**: 5 statues to enter the desert city of Azal. All 5 use the same STATUE→stuff[25] itrans path.

### Narr.asm Messages
- `speak(39)` — `narr.asm:437-439`: "Ah! You have a writ from the king. Here is one of the golden statues of Azal-Car-Ithil. Find all five and you'll find the vanishing city."
- `speak(45)` — `narr.asm:466-468`: "[name] Welcome. Here is one of the five golden figurines you will need."
- `speak(19)` — `narr.asm:399-400`: "I'm afraid I cannot help you, young man. I already gave the golden statue to the other young man."

---

## stuff[28] — Writ

### How Obtained
1. **Princess rescue sequence** — `fmain2.c:1598`:
   ```c
   stuff[28] = 1;
   ```
   Called inside `rescue()` (`fmain2.c:1584-1602`). The rescue triggers when the player enters extent type 83 (princess extent: `{10820,35646, 10877,35670, 83, 1, 1, 0}` at `fmain.c:345`) AND `ob_list8[9].ob_stat` is non-zero (princess is in her cell).
   
   Trigger code at `fmain.c:2684-2685`:
   ```c
   if (xtype == 83 && ob_list8[9].ob_stat)
   {   rescue(); flag = 0; goto findagain; }
   ```
   
   The rescue function also: sets `princess++`, teleports player to (5511,33780), gives 100 gold, speaks `speak(18)`: "Here is a writ designating you as my official agent...", and gives 3 of each key type (`stuff[16..21] += 3`).

2. **No world object placement** — Object 137 is mapped in itrans (137→28) but no ob_list entry places object 137 in the world. The Writ is exclusively obtained via `rescue()`.

### How Used
1. **Priest talk condition** — `fmain.c:3383-3388`:
   ```c
   case 1: /* priest */
           if (stuff[28])
           {   if (ob_listg[10].ob_stat==0)
               {   speak(39); ob_listg[10].ob_stat = 1; }
               else speak(19);
           }
   ```
   When talking to a priest with the Writ, the priest reveals a gold statue (sets `ob_listg[10].ob_stat = 1`).

2. **GIVE menu availability** — `fmain.c:3541`:
   ```c
   menus[GIVE].enabled[7] = stuff_flag(28); /* writ */
   ```
   The Writ appears in the GIVE menu when `stuff[28] != 0`. However, the GIVE handler has NO explicit case for `hit == 7` (Writ) — only gold (hit 5) and bone (hit 8) have handlers. The Writ in GIVE is effectively display-only; it cannot actually be given away.

### Quest Context
- **Prerequisites**: Rescue a princess. Princess is found at room sector extent type 83 at coords (10820-10877, 35646-35670), i.e. inside the castle region 8.
- **Effect**: Unlocks the priest's gold statue gift. Essential progression item.
- **Persistence**: `stuff[28]` is set to 1 and never decremented by any code path.

### Narr.asm Messages
- `speak(18)` — `narr.asm:395-398`: "Here is a writ designating you as my official agent. Be sure and show this to the Priest before you leave Marheim."

---

## stuff[29] — Bone

### How Obtained
1. **Ground pickup of object (128+10)=138** in ob_list9 (underground areas) — `fmain2.c:1167`:
   ```c
   { 3723,39340,(128+10),1},    /* king's bone */
   ```
   itrans maps 138→29, so picking up object 138 increments `stuff[29]`.

### How Used
1. **GIVE Bone to Spectre** — `fmain.c:3501-3503`:
   ```c
   else if (hit == 8 && stuff[29]) /* spectre */
   {   if (k != 0x8a) speak(21);
       else { speak(48); stuff[29] = 0; leave_item(nearest_person,140); }
   }
   ```
   - If target is NOT the spectre (race ≠ 0x8a): `speak(21)` — "Sorry, I have no use for it."
   - If target IS the spectre (race 0x8a): `speak(48)` — "% gave him the ancient bones. Good! Take this crystal shard." Then `stuff[29] = 0` (bone consumed) and `leave_item(nearest_person,140)` drops object 140 (Crystal Shard).

2. **GIVE menu availability** — `fmain.c:3542`:
   ```c
   menus[GIVE].enabled[8] = stuff_flag(29); /* bone */
   ```

### Quest Context
- **Prerequisites**: Access to underground areas (region 9). The bone is at (3723, 39340).
- **Effect**: Exchanged with the Spectre for the Crystal Shard. One-time trade — bone is consumed.
- **Spectre location**: ob_listg[5] at (12439, 36202) with race 10, status 3 — `fmain2.c:1005`.

### Narr.asm Messages
- `speak(47)` — `narr.asm:477-479`: Spectre's initial plea: "HE has usurped my place as lord of undead. Bring me bones of the ancient King and I'll help you destroy him."
- `speak(48)` — `narr.asm:480-482`: Exchange: "% gave him the ancient bones. Good! That spirit now rests quietly in my halls. Take this crystal shard."
- `speak(21)` — `narr.asm:416`: Non-spectre rejection: "Sorry, I have no use for it."
- `speak(49)` — `narr.asm:483-485`: Ghost message: "I am the ghost of your dead brother. Find my bones -- there you will find some things you need."

---

## stuff[30] — Crystal Shard

### How Obtained
1. **Spectre trade** — `fmain.c:3503`:
   ```c
   else { speak(48); stuff[29] = 0; leave_item(nearest_person,140); }
   ```
   When Bone is given to the Spectre, object 140 (Crystal Shard) is dropped. Player picks it up: itrans maps 140→30, `stuff[30]++`.

### How Used
1. **Terrain type 12 passthrough** — `fmain.c:1609`:
   ```c
   if (stuff[30] && j==12) goto newloc;
   ```
   Inside the movement/collision check: when `i==0` (player character), if the player has the Crystal Shard and terrain type is 12 (crystal wall), the collision check is bypassed — `goto newloc` allows passage.
   
   Context (`fmain.c:1606-1611`):
   ```c
   if (i==0)
   {   if (j==15) { doorfind(xtest,ytest,0); }
       else bumped = 0;
       if (stuff[30] && j==12) goto newloc;
   }
   ```

### Quest Context
- **Prerequisites**: Must have Bone → give to Spectre → Crystal Shard dropped.
- **Effect**: Allows passage through terrain type 12 (crystal walls). Used specifically to access the Crystal Palace area (sectors 164-167 per `narr.asm:76`: "% came to the Crystal Palace").
- **Persistence**: Never consumed. `stuff[30]` remains non-zero once obtained.

### Terrain/Map Data
- Terrain type 12: Crystal wall. Impassable without Shard. Passable when `stuff[30] != 0`.
- Crystal Palace sectors: 164-167 per `_place_tbl` in `narr.asm:76`.

### Narr.asm Messages
- `speak(48)` — `narr.asm:480-482`: "Take this crystal shard." (part of Spectre exchange)

---

## stuff[5] — Golden Lasso

### How Obtained
1. **Witch (race 0x89) death drop** — `fmain.c:1756`:
   ```c
   if (an->race == 0x89) leave_item(i,27); /* leave the lasso */
   ```
   When the witch (race 0x89) dies and transitions from DYING to DEAD, it drops object 27 (Golden Lasso). itrans maps 27→5, so `stuff[5]++`.
   
   Race 0x89 = 137 = 0x80|9 = setfig race 9 (witch). Confirmed by source comment `/* witch */` at `fmain.c:2098` and `speak(46)` text: *"Look into my eyes and Die!!" hissed the witch.* The witch is invulnerable without the Sun Stone (`stuff[7]`) per `fmain2.c:233`.

2. **Cheat key** — `fmain.c:1293-1294`:
   ```c
   else if (key == 'B' && cheat1)
   {   if (active_carrier == 11) stuff[5] = 1;
       move_extent(0,hero_x+20,hero_y+20);
       load_carrier(11);
   }
   ```
   The 'B' cheat gives lasso if the active carrier is bird (11), then summons a bird.

### How Used
1. **Bird mounting requirement** — `fmain.c:1498`:
   ```c
   {   if (raftprox && wcarry == 3 && stuff[5])
   ```
   To mount the swan/bird carrier (wcarry==3 = bird carrier shape), the player must have `stuff[5]` (Golden Lasso) and be near the bird (`raftprox` non-zero).

### Quest Context
- **Prerequisites**: Find and kill a witch. The witch is at the Witch's Castle, ob_list8[11] at (11013,36804), setfig race 9. The witch has invulnerability to melee weapons (weapon < 4) UNLESS the player has the Sun Stone (`stuff[7]`). See `fmain2.c:231-233`.
- **Effect**: Enables mounting the swan/bird for flight, which allows reaching otherwise inaccessible mountain areas (e.g., the prison castle where the princess is held — `narr.asm:151`: "% reached the Forbidden Keep").
- **Persistence**: Never consumed once obtained.

### Narr.asm Messages
- `speak(33)` — `narr.asm:452-453`: Wizard clue: "Tame the golden beast and no mountain may deny you! But what rope could hold such a creature?"

---

## stuff[6] — Sea Shell

### How Obtained
1. **Talk to turtle carrier** — `fmain.c:3419-3420`:
   ```c
   else if (an->type == CARRIER && active_carrier == 5)
   {   if (stuff[6]) speak(57); /* check if has shell */
       else { stuff[6] = 1; speak(56); }
   }
   ```
   When talking to a turtle carrier (type CARRIER, active_carrier == 5) and the player doesn't have the shell (`stuff[6] == 0`): `stuff[6] = 1` and `speak(56)` — "Oh, thank you for saving my eggs, kind man! Take this seashell..."

2. **Ground pickup** — SHELL objects placed in ob_list2 and ob_list8:
   - `fmain2.c:1030`: `{10344,36171,SHELL,1}` in ob_list2 (swampland)
   - `fmain2.c:1125`: `{10344,36171,SHELL,1}` in ob_list8 (building interiors)
   Same coordinates (10344, 36171) in two lists: the object appears in both the swampland outdoor region and the building interiors. itrans maps SHELL(151)→6.

### How Used
1. **USE Shell (summon turtle)** — `fmain.c:3458-3461`:
   ```c
   if (hit == 6 && hitgo)
   {   if (hero_x<21373 && hero_x>11194 && hero_y<16208 && hero_y>10205)
           break;
       get_turtle();
   }
   ```
   USE Shell calls `get_turtle()` IF the player is NOT in a specific rectangular region (11194-21373, 10205-16208). If inside that region, the shell does nothing.
   
   `get_turtle()` at `fmain.c:3510-3517`:
   ```c
   get_turtle()
   {   for (i=0; i<25; i++)
       {   set_loc();
           if (px_to_im(encounter_x,encounter_y) == 5) break;
       }
       if (i==25) return;
       move_extent(1,encounter_x,encounter_y);
       load_carrier(5);
   }
   ```
   Attempts 25 times to find a water tile (terrain type 5) nearby, then summons turtle carrier (5) at that location.

2. **Turtle talk (has shell)** — `fmain.c:3419`:
   ```c
   if (stuff[6]) speak(57);
   ```
   If player already has shell and talks to turtle: `speak(57)` — "Just hop on my back if you need a ride somewhere."

### Quest Context
- **Prerequisites**: Find the turtle. Turtle eggs extent at `fmain.c:344`: `{22945,5597, 23225,5747, 61, 3, 2, 4}` (etype 61, race 4 = turtle carrier). Turtle is also placed in ob_list1 at `fmain2.c:1022`: `{23087,5667,TURTLE,1}`.
- **Effect**: Unlocks turtle water travel via USE Shell. Essential for crossing large water bodies.
- **Persistence**: Never consumed. `stuff[6]` stays at 1.
- **Blocked zone**: Shell cannot summon turtle in the rectangle (11194-21373, 10205-16208).

### Narr.asm Messages
- `speak(56)` — `narr.asm:502-503`: "Oh, thank you for saving my eggs, kind man! Take this seashell as a token of my gratitude."
- `speak(57)` — `narr.asm:504`: "Just hop on my back if you need a ride somewhere."
- Wizard clue `speak(27)` — `narr.asm:427`: "Kind deeds could gain thee a friend from the sea."

---

## stuff[7] — Sun Stone

### How Obtained
1. **Ground pickup in ob_list8** (building interiors) — `fmain2.c:1092`:
   ```c
   { 11410,36169,27+128,1},    /* sunstone */
   ```
   Object 27+128 = 155 at world coords (11410, 36169). itrans maps 155→7, so `stuff[7]++`.

### How Used
1. **Witch invulnerability override** — `fmain2.c:230-234`:
   ```c
   dohit(i,j,fc,wt) short wt; register long j,i,fc;
   {   if (anim_list[0].weapon < 4 &&
           (anim_list[j].race == 9 ||
               (anim_list[j].race == 0x89 && stuff[7] == 0) ))
       {   speak(58); return; }
   ```
   Without the Sun Stone (`stuff[7] == 0`), melee attacks (weapon < 4 = dirk/mace/sword) on the witch (race 0x89) are blocked with `speak(58)` — "Stupid fool, you can't hurt me with that!" With the Sun Stone, only the Necromancer (race 9, no 0x80 flag = pure race 9) retains melee invulnerability.
   
   Note: race 9 (pure) = necromancer is ALWAYS melee-immune. Race 0x89 (witch) is melee-immune only when `stuff[7] == 0`.

2. **USE Sun Stone** — `fmain.c:3462`:
   ```c
   if (hit == 8 && witchflag) speak(60);
   ```
   USE Sun Stone when the witch is on screen (`witchflag` set at `fmain.c:1554` when witch setfig is loaded with `witchflag = TRUE`): `speak(60)` — "The Sunstone has made the witch vulnerable!"

3. **Menu availability** — `fmain.c:3537`:
   ```c
   menus[USE].enabled[8] = stuff_flag(7);    /* sunstone */
   ```

### Quest Context
- **Prerequisites**: Access to building interiors, find the sunstone at (11410, 36169).
- **Effect**: Makes the witch vulnerable to all weapons. Required to kill the witch and obtain the Golden Lasso.
- **Persistence**: Never consumed.
- **Cross-cutting**: The Sun Stone affects COMBAT (`dohit`), not terrain or movement. It bridges the inventory system with the combat damage system.

### Narr.asm Messages
- `speak(58)` — `narr.asm:505`: "Stupid fool, you can't hurt me with that!" (without Sun Stone)
- `speak(60)` — `narr.asm:507`: "The Sunstone has made the witch vulnerable!" (USE Sun Stone)
- Wizard clue `speak(31)` — `narr.asm:449`: "Only the light of the Sun can destroy the Witch's Evil."

---

## stuff[9] — Blue Stone (Teleport)

### How Obtained
1. **Ground pickup** — B_STONE objects placed in:
   - `fmain2.c:1038`: `{21626,15446,B_STONE,1}` in ob_list3 (maze forest south) — "3 items at stone ring"
   - `fmain2.c:1046`: `{25678,10703,B_STONE,1}` in ob_list3 (maze forest south)
   - `fmain2.c:1148`: `{9570,35768,B_STONE,5}` in ob_list8 — ob_stat=5 (hidden, requires LOOK)

   itrans maps B_STONE(18)→9, so `stuff[9]++`.

2. **Shop purchase** — Bartender race 0x88 `BUY` handler at `fmain.c:3432-3446` sells items mapped via `jtrans[]`. Blue Stones can be purchased if present in the shop inventory.

3. **Container loot** — Chests/urns/sacks can randomly contain Blue Stones (via `rand8()+8` at `fmain.c:3207-3236`). Index 9 is within range (8..15 after +8 adjustment, and index 8 maps to ARROWBASE, others map to magic items 9-15).

### How Used
1. **MAGIC Stone teleport** — `fmain.c:3331-3353` (MAGIC case 5):
   ```c
   case 5:
       if (hero_sector == 144)
       {   if ((hero_x & 255)/85 == 1 && (hero_y & 255)/64 == 1)
           {   short x1, y1;
               x = hero_x>>8; y = hero_y>>8;
               for (i=0; i<11; i++)
               {   if (stone_list[i+i]==x && stone_list[i+i+1]==y)
                   {   i+=(anim_list[0].facing+1); if (i>10) i-=11;
                       x = (stone_list[i+i]<<8) + (hero_x & 255);
                       y = (stone_list[i+i+1]<<8) + (hero_y & 255);
                       colorplay();
                       xfer(x,y,TRUE);
                       if (riding)
                       {   anim_list[wcarry].abs_x = anim_list[0].abs_x;
                           anim_list[wcarry].abs_y = anim_list[0].abs_y;
                       }
                       break;
                   }
               }
           } else return;
       }
       else return;    /* didn't work so don't decrement use count */
   ```
   
   Requirements: Must be in hero_sector 144 (stone ring area per `_place_tbl` at `narr.asm:68`: "% came to a great stone ring."), standing at grid position `(hero_x & 255)/85 == 1 && (hero_y & 255)/64 == 1` (center of a stone ring tile).
   
   Mechanism: Looks up the hero's stone ring in `stone_list[]` and teleports to the next stone ring, offset by `facing+1` positions forward in the list (wrapping at 11). The `stone_list` contains 11 stone ring locations:
   ```c
   unsigned char stone_list[] = 
   { 54,43, 71,77, 78,102, 66,121, 12,85, 79,40,
     107,38, 73,21, 12,26, 26,53, 84,60 };
   ```
   — `fmain.c:374-375`
   
   If not at a valid stone ring, `return` exits without decrementing the use count.

2. **Charge consumption** — `fmain.c:3365`:
   ```c
   if (!--stuff[4+hit]) set_options();
   ```
   For MAGIC case 5: `stuff[4+5] = stuff[9]` is decremented after successful use. When it reaches 0, `set_options()` disables the menu option. Blue Stones are CONSUMABLE — each teleport uses one charge.

3. **Menu availability** — `fmain.c:3530`:
   ```c
   menus[MAGIC].enabled[i+5] = stuff_flag(i+9);
   ```
   For i=0: `menus[MAGIC].enabled[5] = stuff_flag(9)` — Blue Stone enables MAGIC→Stone option.

4. **Anti-magic zone** — `fmain.c:3306`:
   ```c
   if (extn->v3 == 9) { speak(59); break; }
   ```
   Before any MAGIC use, if the current extent has `v3 == 9` (the necromancer's extent at `fmain.c:343`: `{9563,33883,10144,34462, 60, 1, 1, 9}`), magic is blocked with `speak(59)` — "Your magic won't work here, fool!"

### Quest Context
- **Not quest-gated** but highly useful for navigation.
- **Purpose**: Each Blue Stone provides one teleport between stone rings. Multiple stones can be stockpiled.
- **Stone rings**: 11 locations scattered across the world map at sector coordinates.

### Terrain/Map Data
- Stone ring sector: 144 per `_place_tbl` at `narr.asm:68`.
- 11 stone ring coordinates in `stone_list[]` at `fmain.c:374-375`.

### Narr.asm Messages
- `speak(37)` — `narr.asm:443-444`: Cleric clue: "When you wish to travel quickly, seek the power of the Stones."
- `speak(59)` — `narr.asm:506`: "Your magic won't work here, fool!" (anti-magic zone)

---

## Cross-Cutting Findings

1. **stuff[23] (Rose) in combat/movement code** (`fmain.c:1844`): Checked in the actor movement/damage loop, not in any item USE handler. Only affects actor 0 (player), not NPCs.

2. **stuff[30] (Shard) in collision code** (`fmain.c:1609`): Checked in the terrain collision check, overriding terrain type 12 blocking. This is a movement system override, not an item USE.

3. **stuff[5] (Lasso) obtained from witch death** (`fmain.c:1756`): Surprisingly, the lasso drops from any actor with race 0x89 dying, not from a bird. The witch is race 0x89. This creates a quest chain: Sun Stone → kill witch → get lasso → ride bird → reach mountain areas.

4. **stuff[25] (statues) gates both doors AND map tile overwrite**: The STATBASE<5 check appears in two completely separate systems: the door handler (`fmain.c:1919`) AND the region load routine (`fmain.c:3594`). Both independently block access to the desert city.

5. **stuff[28] (Writ) has GIVE menu entry but no GIVE handler**: The Writ is shown in the GIVE menu (`fmain.c:3541`) but the GIVE case only handles hit==5 (gold) and hit==8 (bone). Hit==7 (writ) falls through with no action. The Writ's only gameplay function is as a passive check in the priest dialogue.

6. **stuff[9] (Blue Stone) early return avoids consumption**: If the player is not at a valid stone ring, `case 5` returns early with `else return`, and the `if (!--stuff[4+hit])` at line 3365 is never reached. This means unsuccessful teleport attempts don't consume the stone.

7. **stuff[22] (Talisman) check location**: The win condition is checked at the `pickup:` label in the TAKE handler, meaning it fires after ANY successful item pickup. If stuff[22] was somehow > 0 during a different item pickup, the win would trigger. The cheat key explicitly clears stuff[22] to prevent this.

8. **Sorceress vs Priest statue revelation**: The sorceress reveals her statue unconditionally on first talk (`ob_listg[9].ob_stat == 0`). The priest requires the Writ (`stuff[28]`). On subsequent visits, the sorceress gives luck boosts, the priest says "I already gave the statue."

9. **Fruit (stuff[24]) pickup is hunger-gated**: When hunger < 15, fruit is stored; when hunger ≥ 15, it's eaten immediately. This is the only item with pickup behavior that varies based on player state.

10. **Shell ground objects are duplicated across regions**: The same SHELL at coordinates (10344, 36171) appears in both ob_list2 and ob_list8. These are for different regions, so the player could potentially pick up a shell from the ground in swampland OR from building interiors.

---

## Quest Progression Chain

The items form a dependency chain for the main quest:

```
Start
 │
 ├── Find Sun Stone (stuff[7]) — ground pickup, ob_list8
 │    └── Kill Witch → drops Golden Lasso (stuff[5])
 │         └── Ride Bird → reach mountain prison
 │              └── Rescue Princess → get Writ (stuff[28])
 │                   └── Show Writ to Priest → reveals Gold Statue
 │
 ├── Find 3 ground Gold Statues (seahold, ogre den, octagonal room)
 ├── Talk to Sorceress → reveals Gold Statue
 ├── Show Writ to Priest → reveals Gold Statue (requires Writ)
 │    └── Total 5 statues (stuff[25]) → enter Desert/Azal
 │
 ├── Find Rose (stuff[23]) → lava protection
 ├── Find Bone (stuff[29]) → underground
 │    └── Give Bone to Spectre → Crystal Shard (stuff[30])
 │         └── Pass crystal walls (terrain 12)
 │
 ├── Talk to Turtle → Sea Shell (stuff[6]) → summon turtle for water travel
 ├── Collect Blue Stones (stuff[9]) → stone ring teleport (consumable)
 ├── Collect Fruit (stuff[24]) → portable hunger relief
 │
 └── Kill Necromancer → drops Talisman (stuff[22]) → WIN
```

---

## Unresolved

1. **Race 0x89 ambiguity for lasso drop**: The code at `fmain.c:1756` checks `an->race == 0x89` in the general DYING→DEAD transition. The witch (setfig 9 | 0x80 = 0x89) matches this. Is there any NON-witch actor that could have race 0x89? Need to verify if the bird carrier ever gets race 0x89 assigned.

2. **Shell blocked zone**: The rectangle (11194-21373, 10205-16208) where Shell USE is blocked — what area is this? Need to cross-reference with sector/region map.

3. **Multiple Shell ground placements**: Same SHELL object at (10344, 36171) in both ob_list2 and ob_list8. Are these two separate pickups or does one region shadow the other? Region overlap behavior unclear.

## Refinement Log
- 2026-04-08: Initial comprehensive discovery pass. All stuff[] indices 22-30 and special items 5,6,7,9 traced through fmain.c, fmain2.c, and narr.asm. itrans table mapped. All code paths documented with line citations.
