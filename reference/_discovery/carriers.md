# Discovery: Carrier Transport — Turtle, Bird, Dragon, Swan

**Status**: complete
**Investigated**: 2026-04-06
**Requested by**: orchestrator
**Prompt summary**: Trace the carrier (mount/transport) system — load_carrier, get_turtle, riding flag, mount/dismount, extent entries, FLYING state, terrain, swan/lasso, dragon, door interaction.

## Key Variables

- `riding` — `fmain.c:563` — short; values: 0 (not riding), 1 (on raft), 5 (on turtle), 11 (on swan/bird)
- `flying` — `fmain.c:563` — declared alongside `riding` but **never written or tested anywhere in codebase** — appears unused
- `wcarry` — `fmain.c:563` — short; set to 3 if `active_carrier` is nonzero, else 1 (`fmain.c:1456`)
- `active_carrier` — `fmain.c:574` — short; stores the carrier file ID (5=turtle, 10=dragon, 11=bird/swan), or 0 if none
- `actor_file` — `fmain.c:573` — which actor sprite file is currently loaded; set equal to carrier ID when carrier is loaded
- `fiery_death` — `fmain.c:87` — boolean; true when hero is in lava region (`fmain.c:1384-1385`)

## Type System

From `ftale.h:88`:
```c
enum sequences {PHIL, OBJECTS, ENEMY, RAFT, SETFIG, CARRIER, DRAGON};
```
- PHIL=0, OBJECTS=1, ENEMY=2, RAFT=3, SETFIG=4, CARRIER=5, DRAGON=6

## Extent List Entries (Carrier Extents)

From `fmain.c:340-342`, the first three extent_list entries are carriers (etype=70):

| Index | Extent Coords | etype | v1 | v2 | v3 | Comment |
|-------|--------------|-------|----|----|----|----|
| 0 | (2118,27237)-(2618,27637) | 70 | 0 | 1 | 11 | Bird/Swan extent |
| 1 | (0,0)-(0,0) | 70 | 0 | 1 | 5 | Turtle extent (initially degenerate) |
| 2 | (6749,34951)-(7249,35351) | 70 | 0 | 1 | 10 | Dragon extent |

The `v3` field identifies the carrier file ID: 5=turtle, 10=dragon, 11=bird/swan.

**Turtle extent starts degenerate** (0,0)-(0,0) — it must be repositioned via `move_extent(1,x,y)` before the turtle is active. This happens in `get_turtle()` and the cheat key 'B'.

## load_carrier(n) — Complete Trace

**Location**: `fmain.c:2784-2801`

```c
load_carrier(n) short n;
{   register struct shape *an;
    register long i;
    an = &(anim_list[3]);                          // carrier always uses anim slot 3
    if (n == 10) an->type = DRAGON; else an->type = CARRIER;  // dragon gets DRAGON type
    if (n==10) i = 2; else if (n==5) i = 1; else i = 0;      // extent index: 0=bird, 1=turtle, 2=dragon
    if (actor_file!=n)
    {   nextshape = seq_list[ENEMY].location;
        read_shapes(n); prep(an->type);             // load sprite data from disk
        motor_off();
    }
    an->abs_x = extent_list[i].x1 + 250;           // position at center of extent
    an->abs_y = extent_list[i].y1 + 200;
    an->index = an->weapon = an->environ = 0;
    an->state = STILL;
    an->vitality = 50;
    anix = 4;                                       // expand active actor count to include slot 3
    an->race = actor_file = active_carrier = n;     // set all tracking variables
}
```

**Key behaviors**:
- Carrier always occupies `anim_list[3]` (slot 3).
- If file ID `n` differs from current `actor_file`, shapes are loaded from disk into the ENEMY memory area (reusing ENEMY shape space).
- Position is set to extent center (x1+250, y1+200) — the extent is 500×400 units, so this is the midpoint.
- `anix` is set to 4, meaning the game loop now processes slots 0-3 (hero, raft, setfig, carrier).
- Dragon (n=10) gets type DRAGON; turtle (n=5) and bird (n=11) get type CARRIER.

### Carrier Sprite Files (cfiles table)

From `fmain2.c:649-656`:

| Index | Width | Height | Count | Blocks | Seq Slot | File ID | Identity |
|-------|-------|--------|-------|--------|----------|---------|----------|
| 5     | 2     | 32     | 16    | 20     | CARRIER  | 1351    | Turtle |
| 10    | 3     | 40     | 5     | 12     | DRAGON   | 1160    | Dragon |
| 11    | 4     | 64     | 8     | 40     | CARRIER  | 1120    | Bird/Swan |

## Extent Trigger Logic

**Location**: `fmain.c:2675-2719`

When the hero enters an extent with `etype == 70`:
```
if (xtype < 70) active_carrier = 0;           // leaving carrier extent clears carrier
else if (xtype == 70 &&
    (!active_carrier || (!riding && actor_file != extn->v3)) )
        load_carrier(extn->v3);
```

**Conditions for loading**: either no carrier is active, OR the player is not riding and the loaded actor file doesn't match. This means carriers are spawned automatically when entering their extent zone.

**Leaving a carrier extent** (`xtype < 70`): `active_carrier = 0` — the carrier is deactivated. However, `anix` is not reduced, so the carrier slot may persist until something overwrites it.

## get_turtle() — Turtle Summoning

**Location**: `fmain.c:3510-3517`

```c
get_turtle()
{   for (i=0; i<25; i++)
    {   set_loc();
        if (px_to_im(encounter_x,encounter_y) == 5) break;
    }
    if (i==25) return;                           // failed to find water
    move_extent(1,encounter_x,encounter_y);      // reposition turtle extent
    load_carrier(5);                             // load turtle
}
```

**Trigger**: Using the Sea Shell item (USE menu, hit==6) — `fmain.c:3459-3461`:
```c
if (hit == 6 && hitgo)
{   if (hero_x<21373 && hero_x>11194 && hero_y<16208 && hero_y>10205)
        break;
    get_turtle();
}
```

The sea shell is blocked in a specific coordinate rectangle (roughly the swamp/inland area: x 11194-21373, y 10205-16208).

**`set_loc()`** (`fmain2.c:1714-1719`): picks a random direction (0-7) and distance (150-213 = 150+rand64()), calculates encounter_x/y from hero position.

**Mechanic**: Tries up to 25 random locations near the hero. If any lands on terrain type 5 (water), the turtle extent is moved there and the turtle loaded. If no water found in 25 tries, nothing happens.

### Turtle Extent Repositioning

`move_extent(e,x,y)` — `fmain2.c:1560-1565`:
```c
move_extent(e,x,y) short e,x,y;
{   register struct extent *ex;
    ex = extent_list + e;
    ex->x1 = x - 250;
    ex->y1 = y - 200;
    ex->x2 = x + 250;
    ex->y2 = y + 200;
}
```
Creates a 500×400 extent centered on (x,y).

### How the Turtle Gets Its Sea Shell Gift

**Location**: `fmain.c:3418-3421` — in the TALK action handler:
```c
else if (an->type == CARRIER && active_carrier == 5)
{   if (stuff[6]) speak(57);    /* already has shell - "Just hop on my back..." */
    else { stuff[6] = 1; speak(56); }   /* gives shell - "Oh, thank you..." */
}
```

Talking to the turtle gives the Sea Shell (stuff[6]) and triggers speak(56): "Oh, thank you for saving my eggs, kind man!". If already owned, speak(57): "Just hop on my back if you need a ride somewhere."

## Carrier Types — Detailed Behavior

### Swan/Bird (actor_file == 11, riding == 11)

**Mount**: `fmain.c:1497-1507` — in the CARRIER type motion handler:
```c
if (actor_file == 11)
{   if (raftprox && wcarry == 3 && stuff[5])
    {   d = anim_list[0].facing;
        xtest = an->abs_x = anim_list[0].abs_x;
        ytest = an->abs_y = anim_list[0].abs_y;
        riding = 11;
    }
    ...
}
```

**Mount conditions**: raftprox (hero within 16px of carrier), wcarry==3 (carrier is active), AND `stuff[5]` (Golden Lasso) is owned.

**Flight physics** — `environ == -2` enables inertial movement (`fmain.c:1464`, `fmain.c:1581-1596`):
- When `riding==11`: `anim_list[0].environ = -2` every frame
- The `k == -2` branch in WALKING state uses velocity/acceleration model:
  - `nvx = vel_x + newx(20,d,2)-20` — adds directional acceleration
  - `nvy = vel_y + newy(20,d,2)-20`
  - Velocity capped: max abs value of 32 horizontal (40-8), 40 vertical (40)
  - Position: `abs_x + vel_x/4`, `abs_y + vel_y/4`
  - `set_course(0,-nvx,-nvy,6)` — auto-faces opposite of velocity (facing into the wind)
  - No terrain collision check (`proxcheck` not called when flying)

**Dismount**: `fmain.c:1417-1427` — triggered by fire button while riding==11:
```c
else if (riding==11)
{   if (fiery_death) event(32);           // "Ground is too hot for swan to land."
    else if (dif_x>-15 && dif_x<15 && dif_y>-15 && dif_y<15)
    {   ytest = anim_list[0].abs_y - 14;
        if (proxcheck(anim_list[0].abs_x,ytest,0)==0 &&
            proxcheck(anim_list[0].abs_x,ytest+10,0)==0)
        {   riding = 0;
            anim_list[0].abs_y = ytest;
        }
    }
    else event(33);                       // "Flying too fast to dismount."
}
```

**Dismount conditions**:
1. NOT in fiery_death (lava) zone — otherwise event(32): "Ground is too hot for swan to land."
2. Velocity must be low: both vel_x and vel_y between -15 and 15
3. Ground below must be passable: two proxcheck calls at y-14 and y-4
4. If velocity too high: event(33): "Flying too fast to dismount."
5. On successful dismount, hero Y is shifted up by 14 pixels.

**Swan sprite animation**: `dex = d` (facing direction, 0-7) — only 8 frames, one per direction. No walk cycle animation.

**Swan AI when not riding**: stays in place (`xtest = an->abs_x; ytest = an->abs_y`).

**Swan rendering when not riding and on ground**: `fmain.c:2463-2464`:
```c
if (atype == CARRIER && riding == 0 && actor_file == 11)
{   atype = RAFT; inum = 1; }
```
When the swan is grounded and not being ridden, it renders using the RAFT sprite sheet (image index 1) instead of CARRIER.

### Turtle (actor_file == 5, riding == 5)

**Mount**: `fmain.c:1512-1516` — in the CARRIER type motion handler:
```c
else  // not actor_file == 11
{   if (raftprox && wcarry == 3)
    {   d = anim_list[0].facing;
        xtest = anim_list[0].abs_x;
        ytest = anim_list[0].abs_y;
        riding = 5;
        dex = d+d;
        if (anim_list[0].state == WALKING) dex += (cycle&1);
    }
```

**Mount conditions**: hero within proximity AND carrier is active. No item requirement (unlike swan which needs lasso).

**Movement when mounted** — `fmain.c:1599`:
```c
if (i==0 && riding == 5) e = 3;
```
Walking speed is 3 (vs normal 2 for enemies, vs variable for other cases).

**Turtle autonomous movement** (when not ridden) — `fmain.c:1520-1540`:
```c
else
{   xtest = newx(an->abs_x,d,3);
    ytest = newy(an->abs_y,d,3);
    if (px_to_im(xtest,ytest) != 5)         // if not water, try turning
    {   d = (d+1)&7;
        ... (tries d+1, d-2, d-3) ...
    }
    riding = FALSE;
    dex = d+d+(cycle&1);
}
j = px_to_im(xtest,ytest);
if (j == 5) { an->abs_x = xtest; an->abs_y = ytest; }
```

The turtle moves at speed 3, constrained to terrain type 5 (water). It tries its current direction; if blocked, tries turning right, left, and further left. It only actually moves if the destination is water.

**Turtle sprite animation**: `dex = d+d + (cycle&1)` — 16 frames total, 2 per direction (walk cycle).

**Dismount**: The turtle dismount happens implicitly — when the hero walks away from the turtle (raftprox becomes false), `riding` is set to FALSE in the CARRIER handler's else branch (`fmain.c:1538`).

### Dragon (actor_file == 10)

**Type**: DRAGON (not CARRIER) — `fmain.c:2788`:
```c
if (n == 10) an->type = DRAGON; else an->type = CARRIER;
```

**The dragon is NOT rideable.** It is a hostile NPC that shoots fire.

**Dragon behavior** — `fmain.c:1482-1493`:
```c
if (an->type == DRAGON)
{   dex = 0;
    if (s == DYING) dex = 3;
    else if (s == DEAD) dex = 4;
    else if (rand4()==0)
    {   ms->speed = 5;
        mdex++; dex = rand2() + 1;
        effect(5,1800 + rand256());
        an->facing = 5;              // always faces south
        ms->missile_type = 2;        // fire missile
        goto dragshoot;
    }
    dex = 0;
}
```

Dragon fires a missile (type 2 = fire) with 25% chance per frame (rand4()==0). Dragon always faces direction 5 (south). Fire missile speed is 5. Sound effect 5 plays at pitch 1800+rand256().

**Dragon AI**: `fmain.c:2114-2117`:
```c
if (an->type == CARRIER)
{   if ((daynight & 15) == 0)
        set_course(i,hero_x,hero_y,5);
    continue;
}
```
Note: this AI block checks for CARRIER type, not DRAGON. Since dragon has type DRAGON, this AI block is **skipped for the dragon**. The dragon does not chase the hero — it stays in place and shoots.

**Dragon rendering**: `fmain.c:1857`:
```c
else if (an->type==RAFT || an->type==CARRIER || an->type==DRAGON)
{   an->rel_x = wrap(an->abs_x - map_x - 16);
    an->rel_y = wrap(an->abs_y - map_y - 16);
}
```
Dragon uses the same screen offset as other carriers (16px offset vs hero's 8px/26px).

### Raft (type RAFT, riding == 1)

**Not a carrier** in the load_carrier sense. The raft is a separate entity, always present as `anim_list[1]` (slot 1), type RAFT.

**Raft behavior** — `fmain.c:1562-1572`:
```c
else if (an->type == RAFT)
{   riding = FALSE;
    if (wcarry != 1 || raftprox != 2) goto statc;
    xtest = anim_list[0].abs_x;
    ytest = anim_list[0].abs_y;
    j = px_to_im(xtest,ytest);
    if (j < 3 || j >5) goto statc;
    an->abs_x = xtest;
    an->abs_y = ytest;
    riding = 1;
    goto statc;
}
```

**Raft mount conditions**: `wcarry == 1` (no active carrier) AND `raftprox == 2` (within 9px), AND hero is on terrain types 3, 4, or 5 (brush, shore, or water). Raft follows hero position exactly.

## Mount/Dismount Summary

| Carrier | riding value | Mount Condition | Dismount Condition |
|---------|-------------|-----------------|-------------------|
| Raft | 1 | wcarry==1, raftprox==2, terrain 3-5 | Automatic when conditions fail |
| Turtle | 5 | raftprox (16px), wcarry==3 | Automatic when proximity lost |
| Swan | 11 | raftprox, wcarry==3, stuff[5] (Lasso) | Fire button + slow speed + clear ground |
| Dragon | N/A | Never mounted | N/A |

## Movement While Riding

### Speed

- **Raft** (riding==1): Normal hero speed (e=2), raft just follows
- **Turtle** (riding==5): Speed 3 (`fmain.c:1599`: `if (i==0 && riding == 5) e = 3;`)
- **Swan** (riding==11): Inertial flight; max velocity ~32 horizontal, ~40 vertical; position updates by vel/4 per frame

### Terrain

- **Raft**: Hero must be on terrain types 3-5 (brush/shore/water)
- **Turtle**: Turtle confined to water (type 5) autonomously; when ridden, hero walks at speed 3 on any terrain (the turtle follows)
- **Swan**: No terrain collision check at all — `environ == -2` triggers the inertial code path which skips proxcheck

### Animation Suppression

`fmain.c:1632`:
```c
if (!(riding && i==0) && an->race != 2) dex += ((cycle+i)&7);
```
When riding (any value) and actor slot is 0 (hero), walk cycle animation is suppressed.

## Carrier Interactions

### Doors — Riding Blocks All Doors

`fmain.c:1900`:
```c
if (riding) goto nodoor3;
```
Any nonzero `riding` value completely bypasses the door search. This applies to raft, turtle, and swan.

### Combat While Riding

**Collision skip** — `fmain.c:2348-2350`:
```c
if (anim_list[k1].state==DEAD || (k2==0 && riding) || k1==1) y1 -= 32;
if (anim_list[k2].state==DEAD || (k1==0 && riding) || k2==1) y2 -= 32;
```
Sort-order adjustment: when riding, the hero (slot 0) gets a Y adjustment of -32, making them draw behind other actors.

**Combat exclusion for carriers** — `fmain.c:2254`, `fmain.c:2286`:
```c
anim_list[i].type == CARRIER) continue;    // melee combat skips CARRIERs
anim_list[j].type == CARRIER) continue;    // missile combat skips CARRIERs
```
Carriers cannot be hit by melee or ranged attacks.

**Swan riding blocks nearest_person** — `fmain.c:2338`:
```c
if (i && an->type!=OBJECTS && an->state!=DEAD && riding != 11)
```
When riding swan (riding==11), nearest_person distance calculation is skipped — you can't talk/interact with NPCs.

### Freeze Spell Blocked While Riding

`fmain.c:3308`:
```c
case 10: if (riding > 1) return; freeze_timer += 100; break;
```
The freeze spell (MAGIC case 10) is blocked when riding > 1 (turtle or swan). Raft (riding==1) allows freeze.

### Stone Circle Teleport While Riding

`fmain.c:3338-3341`:
```c
if (riding)
{   anim_list[wcarry].abs_x = anim_list[0].abs_x;
    anim_list[wcarry].abs_y = anim_list[0].abs_y;
}
```
If riding when using stone circle teleport, the carrier is teleported along with the hero.

### World Wrap While Riding

`fmain.c:1833-1837`:
```c
if (riding > 1)
{   anim_list[3].abs_x = an->abs_x;
    anim_list[3].abs_y = an->abs_y;
}
```
When the hero wraps around the world map edges (region_num < 8), the carrier in slot 3 is teleported to match.

### Rendering

- **Swan riding** (riding==11): hero's ystop reduced by 16 — hero drawn higher up (`fmain.c:2490`)
- **Carrier/Swan rendering without mask**: `fmain.c:2564` — carriers and riding==11 skip terrain masking (drawn above terrain)
- **Swan on ground** (not riding, actor_file==11): rendered as RAFT sprite (`fmain.c:2463-2464`)
- **Carrier relative position** (`fmain.c:1853-1856`): swan riding uses (abs_x - map_x - 32, abs_y - map_y - 40); other carriers use (-16, -16)

### Random Encounters Suppressed

`fmain.c:2081`:
```c
!actors_loading && !active_carrier && xtype < 50)
```
Random encounters only spawn when `active_carrier == 0`. Having any carrier active prevents random encounters.

### Carrier AI (Non-Riding)

`fmain.c:2114-2117` — in the AI tick loop:
```c
if (an->type == CARRIER)
{   if ((daynight & 15) == 0)
        set_course(i,hero_x,hero_y,5);
    continue;
}
```
Every 16 game ticks, the carrier's course is set toward the hero at mode 5 (slow approach). This applies to both turtle and swan when not being ridden.

### Cheat Key 'B' — Summon Bird

`fmain.c:1293-1296`:
```c
else if (key == 'B' && cheat1)
{   if (active_carrier == 11) stuff[5] = 1;   // give lasso if bird already active
    move_extent(0,hero_x+20,hero_y+20);       // move bird extent near hero
    load_carrier(11);                           // load bird
}
```

## FLYING State (21) — Not Related to Carriers

`FLYING` is defined as motion state 21 (`fmain.c:99`). Despite the name, it is **never assigned to any actor in the carrier system**. The `flying` variable (`fmain.c:563`) is declared but never written or read in the codebase. The swan flight system uses `environ == -2` instead.

The FLYING state constant appears only in:
1. Its definition (`fmain.c:99`)
2. The declaration of the `flying` variable (`fmain.c:563`)
3. A comment about mask application modes (`fmain.c:690`)

**None of these are actual usage.** The carrier flight system does not use the FLYING motion state.

## Cross-Cutting Findings

- **stuff[5] (Golden Lasso)** — Required to mount the swan (`fmain.c:1498`). The cheat key 'B' also auto-grants the lasso if the bird is already active (`fmain.c:1294`).
- **stuff[6] (Sea Shell)** — Given by talking to turtle (`fmain.c:3418-3421`). Required to summon turtle via USE menu (`fmain.c:3459-3461`: hit==6 is Sea Shell).
- **Turtle egg extent** — extent index 5 at `fmain.c:345`: `{ 22945, 5597, 23225, 5747, 61, 3, 2, 4 }` (etype 61, v3=4). This is a special encounter (the turtle eggs area), not directly part of the carrier system.
- **Sea Shell blocked in rectangle** — `fmain.c:3459`: `hero_x<21373 && hero_x>11194 && hero_y<16208 && hero_y>10205` — turtle summoning is blocked in this region (approximate swamp/inland area).
- **revive() deactivates carriers** — `fmain.c:2907`: `anix = 3` reduces actor count, removing the carrier from processing. `fiery_death = xtype = 0` also clears the extent type.
- **Carrier uses ENEMY shape memory** — `fmain.c:2790-2791`: `nextshape = seq_list[ENEMY].location; read_shapes(n);` — carrier sprites are loaded into the ENEMY shape slot, meaning enemies and carriers share memory and cannot coexist.
- **Dragon fire missiles** can hit the hero (type 2 missile, checked in `fmain.c:2286-2299`). Damage: `rand8()+4` (4-11). The dragon has 50 vitality (`fmain.c:2798`) and can be killed.

## Unresolved

None — all questions answered with citations.

## Refinement Log
- 2026-04-06: Initial comprehensive discovery pass. All 10 questions answered.
