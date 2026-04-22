# Discovery: Dark Knight (DreamKnight)

**Status**: complete
**Investigated**: 2026-04-08
**Requested by**: orchestrator
**Prompt summary**: Trace ALL mechanics related to the dark knight — identity, spawning, AI behavior, door-blocking goals, speech text, combat, quest connections, and sprite data.

## A. What IS the Dark Knight?

The dark knight is called "DKnight" internally and "DreamKnight" or "Knight of Dreams" in narrative text. It is a **special fixed encounter** (not a random roaming NPC) placed at a hardcoded position in the "hidden valley."

### Encounter Chart Entry (race 7)

- `fmain.c:61`: `{ 40, TRUE,7,1,0, 8 }  /* 7 - DKnight - elf glade */`
  - **hitpoints**: 40 (highest among non-boss enemies; Necromancer has 50)
  - **aggressive**: TRUE
  - **arms**: 7 → weapon_probs index 28-31 = `3,3,3,3` = sword only (`fmain2.c:867`)
  - **cleverness**: 1 (clever — uses ATTACK2 goal mode)
  - **treasure**: 0 (Group 0 — no treasure drops)
  - **file_id**: 8 → `cfiles[8]` = `{ 1,32,64, 40, ENEMY, 1000 }` (`fmain2.c:653`)

### Spawning

The DKnight is spawned via the **extent system** as a special figure encounter (etype 60):

- `fmain.c:360`: `{ 21405,25583, 21827,26028, 60, 1, 1, 7 }  /* hidden valley */`
  - Zone bounds: x1=21405, y1=25583, x2=21827, y2=26028
  - etype=60 (special figure encounter)
  - v1=1 (encounter_number — exactly 1 creature)
  - v2=1 (unused for etype 60)
  - v3=7 (encounter_type → race 7 = DKnight)

**spawn trigger** (`fmain.c:2687-2691`): When the hero enters the hidden valley extent and etype changes to 60, the code checks `if (anim_list[3].race != extn->v3 || anix < 4)`. If the actor in slot 3 is not already a DKnight (race 7) or there aren't enough actors loaded, it spawns one. This means the DKnight **respawns** every time the player re-enters the zone.

**Hardcoded position** (`fmain.c:2741`): `if (extn->v3==7) { xtest = 21635; ytest = 25762; }` — unlike all other encounters which use random placement, the DKnight is always placed at exact coordinates (21635, 25762). The random placement loop is skipped entirely.

**Bug**: Because the for-loop is skipped, the variable `j` is never initialized. The subsequent `if (j==MAX_TRY) return FALSE;` at `fmain.c:2749` reads uninitialized `j`. This is technically undefined behavior but harmless in practice — an uninitialized short register is unlikely to equal exactly 15.

**Actor setup** (`fmain.c:2750-2764`):
- type = ENEMY
- race = 7 (encounter_type, mixflag is 0 so no race mixing)
- weapon = 3 (sword, from weapon_probs[28])
- state = STILL
- goal = ATTACK2 (ATTACK1 + cleverness 1 = 2)
- vitality = 40

## B. DOOR_SEEK / DOOR_LET Goal Modes

### Definitions

- `ftale.h:53`: `#define DOOR_SEEK 11  /* dknight blocking door */`
- `ftale.h:54`: `#define DOOR_LET  12  /* dknight letting pass */`

### Implementation Status: **NEVER USED**

Comprehensive search results:

1. **No C file references**: grep for `DOOR_SEEK` and `DOOR_LET` across all `.c` files returns zero matches. These symbols are never referenced in any C source file.
2. **No numeric assignment**: grep for `goal\s*=\s*1[12]` across all `.c` files returns zero matches. No code ever sets goal to 11 or 12.
3. **fmain.c goal definitions** (`fmain.c:121-131`): The goal mode `#define` block in fmain.c stops at index 10 (CONFUSED). DOOR_SEEK and DOOR_LET are only defined in `ftale.h`.
4. **do_tactic() has no case** (`fmain2.c:1664-1700`): The `do_tactic()` function handles tactics PURSUE, SHOOT, RANDOM, BUMBLE_SEEK, BACKUP, FOLLOW, EVADE, and EGG_SEEK. There is no case for DOOR_SEEK (11) or DOOR_LET (12). If called with these values, the function silently does nothing.

### What the DKnight Actually Does Instead

Rather than using the tactic system, the DKnight has **hardcoded behavior** directly in the AI loop (`fmain.c:2162-2169`):

```
thresh = 14 - mode;
if (an->race == 7) thresh = 16;              // fmain.c:2163
if ((an->weapon & 4)==0 && xd < thresh && yd < thresh)
{   set_course(i,hero_x,hero_y,0);
    if (an->state >= WALKING) an->state = FIGHTING;
}
else if (an->race == 7 && an->vitality)       // fmain.c:2168
{   an->state = STILL; an->facing = 5;  }    // fmain.c:2169
else do_tactic(i,tactic);
```

Behavior:
- **In melee range** (xd < 16 AND yd < 16): Enters FIGHTING state, attacks the player
- **Outside melee range**: Stands STILL, facing direction 5 (south) — does NOT pursue, does NOT use any tactic
- **The extra melee range**: Normal thresh = 14 - mode. For ATTACK2 (mode=2), that's 12. DKnight overrides to 16 — a 33% larger engagement radius.

This hardcoded "stand still facing south" behavior IS the door-blocking mechanic. The DKnight physically blocks passage by standing at a fixed position and refusing to move. DOOR_SEEK/DOOR_LET were presumably planned as a more sophisticated behavior (seek a specific door, let the player pass after defeat) but were replaced with this simpler hardcoded approach.

## C. speak(41) and speak(42) — Message Text

### speak(41) — Proximity Speech (`narr.asm:462-465`)

Comment: `; 41 - dreamknight message 1`

Text: `"Ho there, young traveler!" said the black figure. "None may enter the sacred shrine of the People who came Before!"`

Triggered at `fmain.c:2101`: `case 7: speak(41); break; /* dknight */`
— Fires when the hero is the nearest person and the DKnight (race 7) is the `nearest_person` actor, as part of the NPC proximity speech system (`fmain.c:2095-2103`).

### speak(42) — Death Speech (`narr.asm:466-469`)

Comment: `; 42 - dream knight message 2`

Text: `"Your prowess in battle is great." said the Knight of Dreams. "You have earned the right to enter and claim the prize."`

Triggered at `fmain.c:2775`: `if (an->race == 7) speak(42);`
— Fires inside `checkdead()` when the DKnight's vitality drops below 1. This is the only race-specific death speech in the game (necromancer death at race 9 triggers `speak(44)` via a different mechanism in `fmain2.c`).

## D. Dark Knight in Combat

### Stats Summary

| Stat | Value | Citation |
|------|-------|----------|
| Hitpoints | 40 | `fmain.c:61` |
| Weapon | Sword (3) | `fmain2.c:867` (weapon_probs[28-31]) |
| Goal | ATTACK2 (clever) | Derived: ATTACK1 + cleverness(1) = 2 |
| Melee threshold | 16 (overridden) | `fmain.c:2163` |
| Treasure group | 0 (no drops) | `fmain.c:61` (treasure=0) |
| Aggressive | TRUE | `fmain.c:61` |

### Special Behaviors

1. **Stands still facing south** (`fmain.c:2168-2169`): When alive and outside melee range, DKnight does NOT pursue. It stands still (state=STILL, facing=5/south). This is unique — all other enemies use the tactical system when outside melee range.

2. **Extended melee range** (`fmain.c:2163`): thresh overridden to 16 regardless of mode. Normal ATTACK2 enemies have thresh = 12.

3. **No fleeing** (`fmain.c:2138-2140`): The flee condition `else if (an->vitality < 2 || (xtype > 59 && an->race != extn->v3))` — for etype 60 zones (xtype > 59), actors whose race MATCHES extn->v3 do NOT flee. Since the DKnight's race (7) matches v3 (7), it never enters FLEE mode, even at vitality 1.

4. **No treasure drops**: treasure=0 → Group 0, which has no drops (`fmain2.c:854-856`, treasure_probs).

### Combat Flow

1. Player enters hidden valley extent → DKnight spawns at fixed position (21635, 25762)
2. DKnight stands still facing south, blocking passage
3. speak(41): "None may enter the sacred shrine..."
4. Player approaches within 16 pixels → DKnight enters FIGHTING state, attacks with sword
5. Player must defeat DKnight's 40 HP
6. On death → speak(42): "Your prowess in battle is great..."
7. brave++ (player's bravery stat increments)
8. DKnight enters DYING → DEAD states, no longer blocking

## E. Quest Progression

### Direct Quest Connections: None

- No quest flags are set when the DKnight dies. `checkdead()` at `fmain.c:2769-2781` only does: speak(42), brave++, prq(7).
- No `stuff[]` inventory items are granted or checked.
- grep for `race == 7` in `fmain2.c` returns zero matches — no quest logic references the DKnight.
- The DKnight is NOT tied to the golden statues, keys, or any other quest items.

### Indirect Quest Connection: The Sacred Shrine

The DKnight guards "the sacred shrine of the People who came Before" (per speak(41)). Once defeated, the player can physically pass through. However:

- **No programmatic gate**: There is no code that checks whether the DKnight is alive/dead to enable/disable passage. The "door" is simply the DKnight's physical body standing at (21635, 25762).
- **Respawning**: The DKnight respawns every time the player re-enters the hidden valley extent (`fmain.c:2687-2691`). The player must defeat it each time.
- **brave++**: The only lasting mechanical effect of killing the DKnight is incrementing `brave`. Bravery affects max vitality (`15+brave/4` at `fmain.c:2901`), combat strength (`brave/20+5` at `fmain.c:2249`), and enemy hit chance (`rand256()>brave` at `fmain.c:2260`).

### What's Beyond the DKnight: The Elf Glade (Sunstone)

The DKnight stands directly in front of doorlist entry #48 — the **elf glade** door (`fmain.c:288`):

```
{ 0x5470,0x6480, 0x2c80,0x8d80, HSTONE,1 }  /* elf glade */
```

- Outside coords: (0x5470, 0x6480) = (21616, 25728) — 19 pixels from DKnight position (21635, 25762)
- Inside coords: (0x2c80, 0x8d80) = (11392, 36224) — region 8 (building interiors, F9)

Inside the elf glade, at ob_list8 index 18 (`fmain2.c:1092`):

```
{ 11410, 36169, 27+128, 1 }   /* sunstone */
```

The **Sun Stone** (`stuff[7]`) is the "prize" the Knight of Dreams refers to in speak(42). It is a critical quest item:

- **Makes the Witch vulnerable**: Without the Sun Stone (`stuff[7]==0`), melee attacks (weapon < 4) against the Witch (race 0x89) are blocked with `speak(58)` — `fmain2.c:231-233`. With the Sun Stone, any weapon works.
- **USE interaction**: When `witchflag` is set and the player USEs the Sun Stone (hit==8), triggers `speak(60)`: "The Sunstone has made the witch vulnerable!" — `fmain.c:3462`.
- **Inventory entry**: `fmain.c:389`: `{ 27, 10,70, 0, 8,8, 1, "Sun Stone" }`

The elf glade also contains other items nearby (all ob_list8, region 8):
- Index 38: (11855, 36206) — Footstool
- Index 39: (11909, 36198) — Chest (container)
- Index 40-42: (11918-11938, 36246) — Bird totems (cabinet items)
- Index 49: (11936, 36207) — Scrap of paper

**Quest chain**: Defeat DKnight → enter elf glade door → pick up Sun Stone → USE Sun Stone near Witch → Witch becomes vulnerable → kill Witch → Witch drops Golden Lasso (`stuff[5]`).

## F. The `dKnight` Sprite

### Disk Layout

- `mtrack.c:73`: `{ "dh0:z/dKnight", 1000, 40 }` — char_map index 10
  - Stored at disk blocks 1000-1039 (40 blocks = 20,480 bytes)

### Loading Info

- `fmain2.c:653`: `{ 1,32,64, 40, ENEMY, 1000 }  /* dknight file (spiders) */`
  - cfiles index 8 (file_id 8 from encounter_chart)
  - width=1 (16 pixels — 1 word per row)
  - height=32 (32 pixels tall)
  - count=64 (64 frames of animation)
  - numblocks=40 (disk blocks to read)
  - seq_num=ENEMY
  - file_id=1000 (starting disk block)

### Sprite Dimensions

The DKnight sprite is **16×32 pixels** (width=1 word × height=32 lines), the same size as most character sprites (player characters, ogre, ghost, necromancer, etc.). It uses the standard ENEMY sequence slot.

### Note on Comment

The comment at `fmain2.c:653` says "dknight file (spiders)" — the "(spiders)" likely refers to the sprite sheet also being used for spiders, or it's a vestigial comment from when the spider entry shared this file_id. Spiders (encounter_type 6) use file_id=8 in the encounter_chart (`fmain.c:60`), which maps to the same cfiles entry. This means **spiders and the DKnight share the same sprite sheet on disk** (blocks 1000-1039).

## References Found

### fmain.c
- `fmain.c:61` — **definition** — encounter_chart[7]: `{ 40, TRUE,7,1,0, 8 }` DKnight stats
- `fmain.c:121-131` — **definition** — goal mode defines (stops at CONFUSED=10, no DOOR_SEEK/DOOR_LET)
- `fmain.c:360` — **definition** — extent_list hidden valley: `{ 21405,25583, 21827,26028, 60, 1, 1, 7 }`
- `fmain.c:579` — **definition** — `USHORT encounter_type;`
- `fmain.c:2087-2090` — **read** — encounter_type assignment logic (not for DKnight; DKnight uses etype 60 path)
- `fmain.c:2095-2103` — **read** — NPC proximity speech, race 7 → speak(41)
- `fmain.c:2101` — **read** — `case 7: speak(41); break; /* dknight */`
- `fmain.c:2138-2140` — **read** — flee condition: special encounters (etype>59) with matching race don't flee
- `fmain.c:2163` — **read** — melee threshold override: `if (an->race == 7) thresh = 16;`
- `fmain.c:2168-2169` — **read** — stand-still behavior: `if (an->race == 7 && an->vitality) { an->state = STILL; an->facing = 5; }`
- `fmain.c:2687-2691` — **call** — etype 60 spawn trigger: checks `anim_list[3].race != extn->v3`
- `fmain.c:2704` — **write** — `encounter_type = extn->v3;` (sets to 7 for hidden valley)
- `fmain.c:2741` — **read** — hardcoded DKnight position: `if (extn->v3==7) { xtest = 21635; ytest = 25762; }`
- `fmain.c:2749` — **read** — `if (j==MAX_TRY) return FALSE;` — uses uninitialized j for DKnight
- `fmain.c:2750-2764` — **write** — set_encounter assigns all actor fields for spawned DKnight
- `fmain.c:2775` — **read** — death speech: `if (an->race == 7) speak(42);`
- `fmain.c:2777` — **write** — `if (i) brave++;` — bravery increment on DKnight death

### fmain2.c
- `fmain2.c:653` — **definition** — cfiles[8]: `{ 1,32,64, 40, ENEMY, 1000 }` DKnight sprite loading info
- `fmain2.c:860-867` — **definition** — weapon_probs: index 28-31 (arms=7) = `3,3,3,3` (swords)
- `fmain2.c:1664-1700` — **definition** — do_tactic(): no case for DOOR_SEEK or DOOR_LET

### ftale.h
- `ftale.h:53` — **definition** — `#define DOOR_SEEK 11  /* dknight blocking door */`
- `ftale.h:54` — **definition** — `#define DOOR_LET  12  /* dknight letting pass */`

### narr.asm
- `narr.asm:462-465` — **definition** — speak(41): "Ho there, young traveler!...None may enter the sacred shrine..."
- `narr.asm:466-469` — **definition** — speak(42): "Your prowess in battle is great...earned the right to enter..."

### mtrack.c
- `mtrack.c:73` — **definition** — `{ "dh0:z/dKnight", 1000, 40 }` — disk layout for DKnight sprites

## Cross-Cutting Findings

- **Spiders share DKnight sprite sheet**: Both Spider (encounter_type 6, file_id=8) and DKnight (encounter_type 7, file_id=8) load from cfiles[8], which reads disk blocks 1000-1039. The comment "dknight file (spiders)" at `fmain2.c:653` confirms this. This is notable — 64 frames of animation are shared between two different enemy types.
- **Uninitialized variable bug**: `fmain.c:2741-2749` — when spawning DKnight (extn->v3==7), the for-loop placement is skipped, leaving `j` uninitialized. The `if (j==MAX_TRY)` check reads garbage. Harmless but technically undefined behavior.
- **DOOR_SEEK/DOOR_LET are vestigial**: These tactic defines in ftale.h were likely planned as a door-blocking AI system but were replaced by the simpler hardcoded "stand still facing south" logic at `fmain.c:2168-2169`. This is confirmed by the fact that fmain.c's own tactic defines (`fmain.c:121-131`) omit DOOR_SEEK/DOOR_LET entirely — they exist only in the shared header.
- **DKnight proximity speech uses race, not type**: The proximity speech system at `fmain.c:2095-2103` checks `anim_list[nearest_person].race` — for the DKnight this is the numeric race value 7, not an NPC type constant. This is the same approach used for the necromancer (race 9).

## Unresolved

- **What object/item lies beyond the DKnight in the hidden valley?** The speak(42) text says "claim the prize" but no code directly connects DKnight death to a specific reward. The hidden valley extent is small (422×445 world units). Determining what's there requires either map data analysis or gameplay observation — cannot be determined from source code alone.
- **Spider/DKnight sprite sharing mechanism**: How do 64 frames of animation serve two different enemy types? Are frames partitioned between spider and DKnight, or do they use entirely different animation indices into the same sheet? This requires analysis of the sprite sheet itself or the animation system.
- **Why "(spiders)" in the comment?** The comment `/* dknight file (spiders) */` could mean: (a) this file was originally spiders and was repurposed; (b) spiders and DKnight share the same visual frames; (c) it's a comment error. Cannot be determined from source code alone.

## Refinement Log
- 2026-04-08: Initial comprehensive discovery pass. Traced all race==7 references, all DOOR_SEEK/DOOR_LET references, narr.asm messages, encounter spawning, AI behavior, combat stats, quest connections, and sprite data.
