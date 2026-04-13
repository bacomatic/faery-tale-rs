# Discovery: Carrier/Transport System

**Status**: complete
**Investigated**: 2025-01-27
**Requested by**: orchestrator
**Prompt summary**: Full analysis of the carrier/mount system - the rideable swan, turtle, dragon, and raft - including activation, quest gating, movement mechanics, extent repositioning, and the transport network.

## Architecture Overview

The carrier system uses four transport types defined by the sequences enum (fmain.c:88):

    enum sequences {PHIL, OBJECTS, ENEMY, RAFT, SETFIG, CARRIER, DRAGON};

Three carrier entities load into anim_list[3] (the carrier slot). The raft uses anim_list[1] (the permanent raft slot). The active_carrier variable (fmain.c:574) tracks which carrier file is currently loaded.

### Carrier Actor File IDs
| actor_file | Type      | In-Game Entity | extent_list index |
|------------|-----------|----------------|-------------------|
| 11         | CARRIER   | Swan           | 0 (bird extent)   |
| 5          | CARRIER   | Turtle         | 1 (turtle extent) |
| 10         | DRAGON    | Dragon         | 2 (dragon extent) |
| N/A        | RAFT      | Raft           | N/A (always present) |

### Key Variables
- riding (fmain.c:563) - 0=not riding, 1=raft, 5=turtle, 11=swan
- active_carrier (fmain.c:574) - currently loaded carrier file ID (0, 5, 10, or 11)
- wcarry (fmain.c:563) - index into anim_list for closest carrier: 3 if active_carrier set, 1 (raft) otherwise
- raftprox (fmain.c:564) - proximity to current carrier: 0=far, 1=near (<16px), 2=very near (<9px)
- actor_file (fmain.c:573) - which shape file is loaded for enemies/carriers

## References Found

### extent_list[] Carrier Activation Zones (fmain.c:338-370)
- fmain.c:339 - definition - bird (swan) extent: x1=2118 y1=27237 x2=2618 y2=27637, etype=70 v3=11
- fmain.c:340 - definition - turtle extent starts INACTIVE at (0,0)-(0,0), etype=70 v3=5
- fmain.c:341 - definition - dragon extent: x1=6749 y1=34951 x2=7249 y2=35351, etype=70 v3=10
- fmain.c:345 - definition - turtle eggs: x1=22945 y1=5597 x2=23225 y2=5747, etype=61 v1=3 v2=2 v3=4 (spawns snakes)
- fmain.c:346 - definition - princess extent: x1=10820 y1=35646 x2=10877 y2=35670, etype=83 (triggers rescue)

### active_carrier variable
- fmain.c:574 - definition - short active_carrier; /* is turtle of bird active */
- fmain.c:1294 - write - cheat key B: if (active_carrier == 11) stuff[5] = 1;
- fmain.c:1456 - read - if (active_carrier) wcarry = 3; else wcarry = 1;
- fmain.c:2716 - write - if (xtype < 70) active_carrier = 0;
- fmain.c:2730 - write - load_actors clears it: active_carrier = 0;
- fmain.c:2801 - write - load_carrier sets it: an->race = actor_file = active_carrier = n;
- fmain.c:3418 - read - talk to turtle: an->type == CARRIER && active_carrier == 5

### riding variable
- fmain.c:563 - definition - short riding, flying, wcarry;
- fmain.c:1417 - read - swan dismount: else if (riding==11)
- fmain.c:1423 - write - dismount: riding = 0;
- fmain.c:1464 - read - set environ: if (riding==11) anim_list[0].environ = -2;
- fmain.c:1502 - write - mount swan: riding = 11;
- fmain.c:1516 - write - mount turtle: riding = 5;
- fmain.c:1538 - write - turtle not mounted: riding = FALSE;
- fmain.c:1563 - write - raft not mounted: riding = FALSE;
- fmain.c:1572 - write - raft mounted: riding = 1;
- fmain.c:1582 - read - flying speed: if (riding == 11) e = 40; else e = 42;
- fmain.c:1599 - read - turtle walk speed: if (i==0 && riding == 5) e = 3;
- fmain.c:1833 - read - world wrap: if (riding > 1)
- fmain.c:1853 - read - swan render offset: if (an->type == CARRIER && riding == 11)
- fmain.c:1900 - read - door block: if (riding) goto nodoor3;
- fmain.c:2338 - read - nearest person: riding != 11 (disables NPC proximity when flying)
- fmain.c:2463 - read - swan ground render: if (atype == CARRIER && riding == 0 && actor_file == 11)
- fmain.c:2564 - read - no mask: riding==11
- fmain.c:3308 - read - freeze spell: if (riding > 1) return;
- fmain.c:3338 - read - blue stone teleport: if (riding)

### stuff[5] (Golden Lasso)
- fmain.c:387 - definition - inv_list[5]: Golden Lasso (image=27)
- fmain.c:1294 - write - cheat gives lasso: if (active_carrier == 11) stuff[5] = 1;
- fmain.c:1498 - read - swan mount check: if (raftprox && wcarry == 3 && stuff[5])

### stuff[6] (Sea Shell)
- fmain.c:388 - definition - inv_list[6]: Sea Shell (image=23)
- fmain.c:3419 - read - talk to turtle: if (stuff[6]) speak(57);
- fmain.c:3420 - write - receive shell: else { stuff[6] = 1; speak(56); }
- fmain.c:3457 - read - USE menu: if (hit == 6 && hitgo)

### move_extent() calls
- fmain2.c:1560-1567 - definition - move_extent(e,x,y): centers 500x400 extent at (x,y)
- fmain.c:1295 - call - cheat bird: move_extent(0,hero_x+20,hero_y+20);
- fmain.c:1545 - call - carrier following: move_extent(e,xtest,ytest);
- fmain2.c:1596 - call - rescue repositions bird: move_extent(0,22205,21231);
- fmain.c:3516 - call - get_turtle: move_extent(1,encounter_x,encounter_y);

### turtle_eggs variable
- fmain2.c:251 - definition - char turtle_eggs;
- fmain2.c:274 - read - aftermath auto-summon: if (turtle_eggs) get_turtle();
- fmain2.c:1224 - write - reset on region load: turtle_eggs = witchflag = FALSE;
- fmain2.c:1284 - write - object detection: if (list->ob_id == TURTLE) turtle_eggs = anix2;
- fmain.c:2150 - read - snake AI: if (an->race==4 && turtle_eggs) tactic = EGG_SEEK;
- fmain2.c:1697-1699 - read - egg seek target: set_course(i,23087,5667,0);

## Code Path: Carrier Activation

### Entry: Hero moves to new region (fmain.c:2675-2719)
1. fmain.c:2675-2678 - Scan extent_list for hero position match
2. fmain.c:2682 - Check if xtype changed (entered new extent)
3. fmain.c:2684-2685 - If xtype==83 and princess present -> rescue()
4. fmain.c:2686-2693 - If xtype==60 or 61 -> force encounter (special NPCs / turtle eggs snakes)
5. fmain.c:2716 - If xtype < 70 -> active_carrier = 0 (deactivate)
6. fmain.c:2717-2719 - If xtype == 70 -> load_carrier(extn->v3)

### load_carrier(n) (fmain.c:2784-2802)
1. fmain.c:2788 - Set type: DRAGON if n==10, else CARRIER
2. fmain.c:2789 - Map n to extent index: 10->2, 5->1, else->0
3. fmain.c:2790-2794 - Load shape data if different file
4. fmain.c:2795-2796 - Position at extent corner + (250,200) offset
5. fmain.c:2801 - Set race, actor_file, active_carrier = n

## Code Path: Swan Mounting and Flying

### Mounting (fmain.c:1495-1509)
1. fmain.c:1495 - Check an->type == CARRIER
2. fmain.c:1497 - Check actor_file == 11 (swan)
3. fmain.c:1498 - Check raftprox && wcarry==3 && stuff[5] (near + active + Golden Lasso)
4. fmain.c:1500-1501 - Snap swan to hero position
5. fmain.c:1502 - riding = 11

### Flying (fmain.c:1580-1594)
1. fmain.c:1464 - When riding==11, hero environ set to -2
2. fmain.c:1581 - Walking state with k==-2 triggers flying code path
3. fmain.c:1582 - Max velocity e=40 for swan (e=42 for other environ=-2 case)
4. fmain.c:1583-1588 - Accelerate with velocity capping
5. fmain.c:1589-1590 - Compute new position from velocity/4
6. fmain.c:1591-1594 - Set facing from velocity, goto newloc SKIPPING proxcheck (bypasses ALL terrain collision)

### Dismounting (fmain.c:1417-1428)
1. fmain.c:1418 - Blocked in fiery_death zone -> event(32) = narr.asm:50: "Ground is too hot for swan to land."
2. fmain.c:1419 - Must be slow (vel < 15 both axes)
3. fmain.c:1421-1422 - Must pass proxcheck at landing spot
4. fmain.c:1423-1424 - riding=0, adjust y position upward by 14
5. fmain.c:1427 - If too fast -> event(33) = narr.asm:51: "Flying too fast to dismount."

### fiery_death zone (fmain.c:1384-1385)
fiery_death = (map_x>8802 && map_x<13562 && map_y>24744 && map_y<29544);

## Code Path: Turtle Summoning

### Via Sea Shell USE (fmain.c:3457-3461)
1. fmain.c:3457 - USE menu hit==6 (Sea Shell slot)
2. fmain.c:3458 - Check NOT in swamp region: hero_x<21373 && hero_x>11194 && hero_y<16208 && hero_y>10205 -> break (blocked)
3. fmain.c:3460 - Call get_turtle()

### get_turtle() (fmain.c:3510-3518)
1. fmain.c:3511 - Try up to 25 random positions
2. fmain.c:3512 - set_loc() generates random nearby position (150-214px away, random direction)
3. fmain.c:3513 - Check px_to_im == 5 (water terrain)
4. fmain.c:3515 - If no water found in 25 tries, return (fail silently)
5. fmain.c:3516 - move_extent(1,...) reposition turtle extent to water location
6. fmain.c:3517 - load_carrier(5) spawn turtle

### Via Turtle Eggs aftermath (fmain2.c:253-274)
1. fmain2.c:1284 - During object loading, if TURTLE object visible, set turtle_eggs = anix2
2. fmain2.c:273-274 - aftermath() after combat: if (turtle_eggs) get_turtle();

### Turtle Mounting (fmain.c:1511-1516)
2. fmain.c:1516 - riding = 5

### Turtle Autonomous Swimming (fmain.c:1520-1542)
1. fmain.c:1521-1535 - When not riding, turtle tries current direction then 3 alternates
2. fmain.c:1523-1531 - Each direction checked: px_to_im(xtest,ytest) != 5 means not water
3. fmain.c:1541-1542 - Only moves if destination IS water (px_to_im == 5)

### Talking to Turtle gives Sea Shell (fmain.c:3418-3421)
1. fmain.c:3418 - Condition: an->type == CARRIER && active_carrier == 5
2. fmain.c:3419 - If already have shell (stuff[6]): speak(57)
3. fmain.c:3420 - If no shell: stuff[6] = 1; speak(56) (give shell)

## Code Path: Princess Rescue -> Swan Repositioning

1. fmain.c:2684-2685 - Hero enters princess extent (xtype==83) + ob_list8[9].ob_stat nonzero -> rescue()
2. fmain2.c:1594 - princess++ (increment rescue counter)
3. fmain2.c:1595 - xfer(5511,33780,0) teleport to throne room
4. fmain2.c:1596 - move_extent(0,22205,21231) - REPOSITION BIRD EXTENT to farmlands area
5. fmain2.c:1597 - ob_list8[2].ob_id = 4 (change noble to princess in throne room)
6. fmain2.c:1598 - stuff[28] = 1 (give Writ)
7. fmain2.c:1600 - wealth += 100
8. fmain2.c:1601 - ob_list8[9].ob_stat = 0 (disable princess encounter)
9. fmain2.c:1602 - Give 3 of each key type (stuff[16..21] += 3)

## Code Path: Dragon (NOT Rideable)

### Load (fmain.c:2788)
- if (n == 10) an->type = DRAGON; else an->type = CARRIER;

### Behavior (fmain.c:1481-1493)
- Dragon is HOSTILE - it fires missiles at the hero
- 25% chance per tick (rand4()==0) to shoot: missile_type=2, speed=5
- No mounting code exists - riding is NEVER set to 10 anywhere
- Dragon can be killed (DYING/DEAD states handled)

### Dragon Cave Door (fmain.c:244)
- Outdoor coords: (0x1390,0x1b60) = (5008,7008)
- Indoor coords: (0x1980,0x8c60) = (6528,35936)
- Type: CAVE

## Code Path: Raft

### Always Present (fmain.c:2820)
- anim_list[1].type = RAFT set during revive()

### Mounting (fmain.c:1562-1573)
1. fmain.c:1563 - riding = FALSE (reset each frame)
2. fmain.c:1564 - Check wcarry == 1 (no active carrier) AND raftprox == 2 (very close)
3. fmain.c:1568 - Check terrain: j = px_to_im(xtest,ytest)
4. fmain.c:1569 - Must be terrain 3-5: if (j < 3 || j >5) goto statc
5. fmain.c:1572 - riding = 1

## Transport Constraints Summary

### Swan (riding==11)
- REQUIRES: Golden Lasso (stuff[5] != 0)
- MOVEMENT: Momentum-based flying, bypasses ALL terrain collision (no proxcheck)
- SPEED: Max velocity 40 (effective ~10 px/frame)
- CANNOT DISMOUNT: In fiery_death zone (lava plains)
- CANNOT DISMOUNT: While moving fast (velocity >= 15)
- CANNOT DISMOUNT: Onto impassable terrain
- CANNOT ENTER DOORS: While riding

### Turtle (riding==5)
- REQUIRES: Sea Shell (stuff[6]) to summon via USE menu; NO item needed to mount once present
- MOVEMENT: Normal walking at speed 3 (normal walk is 2)
- CONSTRAINT: Turtle itself only moves on water terrain (px_to_im==5)
- SEA SHELL BLOCKED: In swamp region (hero_x 11194-21373, hero_y 10205-16208)
- CANNOT ENTER DOORS: While riding

### Raft (riding==1)
- REQUIRES: Very close proximity (<9 pixels) on water terrain (px_to_im 3-5)
- NO ITEM REQUIREMENT
- CANNOT ENTER DOORS: While riding

### Dragon (NOT rideable)
- HOSTILE boss enemy that occupies the carrier slot and fires at hero

## Carrier Slot Architecture

The carrier and enemy systems SHARE the same actor shape memory:
- fmain.c:2730 - load_actors() sets active_carrier = 0, clearing any carrier
- fmain.c:2791 - load_carrier() uses seq_list[ENEMY].location for shape data
- fmain.c:2080-2081 - Random encounters suppressed when active_carrier != 0

This means:
1. Having a carrier active prevents random enemy spawns
2. Enemy encounters unload the carrier
3. Only one carrier type can be loaded at a time

## Swan AI When Not Riding (fmain.c:2114-2117)
The swan periodically (every 16 daynight ticks) turns toward the hero via set_course, making it approachable.

## Swan Ground Rendering (fmain.c:2463-2464)
When not riding, swan renders as RAFT sprite image 1 (small stationary sprite on ground).

## Swan and Carrier: No Terrain Masking (fmain.c:2564-2567)
CARRIER type and rider (i==0 when riding==11) skip terrain masking entirely, drawing on top of everything.

## World Wrapping While Riding (fmain.c:1826-1837)
When riding > 1 (turtle or swan), world-edge wrapping syncs carrier position to hero.

## Cross-Cutting Findings

- fmain.c:1609 - stuff[30] (Crystal Shard) in collision: allows passing terrain type 12 (mountains)
- fmain.c:2080-2081 - active_carrier suppresses random encounters
- fmain.c:1900 - riding blocks ALL door entry
- fmain.c:2254 - CARRIER type excluded from melee hit detection
- fmain.c:2338 - riding==11 disables NPC proximity/interaction while flying
- fmain.c:3308 - Freeze spell blocked when riding > 1
- fmain2.c:273-274 - aftermath() auto-calls get_turtle() when turtle_eggs visible (combat-carrier cross-cutting)
- fmain.c:3458 - Sea Shell usage blocked in swamp by hardcoded coordinate bounds
- fmain.c:1632 - Animation frame cycling skipped for rider
- fmain.c:2490 - Rider drawn 16px higher when on swan

## Unresolved

- Golden Lasso normal acquisition path: stuff[5] is set by cheat key B (fmain.c:1294) but the normal game acquisition is not in carrier code. The itrans table (fmain2.c:982) maps object byte 27 to stuff[5]. A ground object with ob_id=27 would grant it, but no such object found in the ob_list arrays. May be inside a container or require runtime data analysis.

- Terrain attribute value meanings: px_to_im returns terrain attributes from terra_mem[] loaded at runtime. Value 5 is used for water (turtle movement check). The full mapping requires analyzing the terrain data files, not just source.

- Dragon cave geography: The dragon extent (6749-7249, 34951-35351) is in building interior coordinate space. The dragon cave door exterior is at (5008,7008). The coordinate mapping between outdoor and indoor spaces needs separate investigation.

- turtleprox variable: Declared at fmain.c:564 and reset at fmain.c:1455 but never set to TRUE. Vestigial/unused.

## Refinement Log
- 2025-01-27: Initial comprehensive discovery pass covering all four transport types, activation logic, quest gating, movement mechanics, extent repositioning, and cross-cutting interactions.
