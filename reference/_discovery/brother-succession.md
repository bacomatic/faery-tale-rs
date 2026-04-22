# Discovery: Brother Succession (Death & Respawn)

**Status**: complete
**Investigated**: 2026-04-05
**Requested by**: orchestrator
**Prompt summary**: Trace the complete death and brother succession system — checkdead, fairy rescue, revive, blist, mod1save, game over, ghost brothers.

## checkdead

**Function**: `fmain.c:2769-2781`

```c
checkdead(i,dtype) register long i, dtype;
{   register struct shape *an;
    an = &(anim_list[i]);
    if (an->vitality < 1 && an->state != DYING && an->state != DEAD)
    {   an->vitality = 0; an->tactic = 7;
        an->goal = DEATH; an->state = DYING;
        if (an->race == 7) speak(42);
        else if (an->type == SETFIG && an->race != 0x89) kind -= 3;
        if (i) brave++; else { event(dtype); luck -= 5; setmood(TRUE); }
        if (kind < 0) kind = 0;
        prq(7);
    }
    if (i == 0) prq(4);
}
```

### Trigger conditions

`checkdead(i, dtype)` is called for actor `i` with death-type `dtype`. It triggers if `an->vitality < 1` and the actor is not already DYING or DEAD.

### Callers (all in fmain.c)

| Call site | dtype | Cause | Code |
|-----------|-------|-------|------|
| `fmain2.c:249` | 5 | Combat hit (melee/missile) | `checkdead(j,5)` inside `dohit()` |
| `fmain.c:1847` | 27 | Lava/fire damage | `checkdead(i,27)` — `fiery_death` zone |
| `fmain.c:1850` | 6 | Drowning (environ==30, not raft/bridge) | `checkdead(i,6)` |
| `fmain.c:3359` | 0 | Magic spell kills enemies | `checkdead(i,0)` |

### Death types (event messages from narr.asm:11-37)

| dtype | event_msg index | Message text |
|-------|----------------|--------------|
| 0 | 0 (or none for enemies) | (none for hero at this call site) |
| 5 | 5 | "% was hit and killed!" |
| 6 | 6 | "% was drowned in the water!" |
| 7 | 7 | "% was burned in the lava." |
| 27 | 27 | "% perished in the hot lava!" |

### Hero-specific effects (i == 0)

When the hero dies (`i == 0`):
- `event(dtype)` — displays the death message (narr.asm `_event_msg` indexed by `dtype`)
- `luck -= 5` — luck decreases by 5 (`fmain.c:2777`)
- `setmood(TRUE)` — updates the status display
- `prq(4)` — always called for hero to refresh vitality display

### Enemy/NPC effects (i != 0)

- If enemy race == 7 (dark knight variant): `speak(42)` (`fmain.c:2775`)
- If SETFIG type and race != 0x89 (witch): `kind -= 3` — killing non-witch NPCs reduces kindness (`fmain.c:2776`)
- `brave++` — killing any non-hero increases bravery (`fmain.c:2777`)

### Death states

- `DYING` = 14 (`fmain.c:93`) — dying animation, `tactic` counts down from 7
- `DEAD` = 15 (`fmain.c:94`) — fully dead
- `SINK` = 16 (`fmain.c:95`) — sinking/drowning animation
- `FALL` = 22 (`fmain.c:100`) — pit fall animation

Transition from DYING→DEAD occurs when `tactic` counts to 0 (`fmain.c:1747-1748`):
```c
if (s==DYING && !(--(an->tactic)))
{   an->state = DEAD;
```

### Additional death paths (not via checkdead)

- **Hunger starvation**: `fmain.c:2206-2208` — when `hunger > 100 || fatigue > 160`, `anim_list[0].vitality -= 2` per 128-tick cycle. Eventually vitality reaches 0 and the main loop detects DEAD state.
- **Lava (environ > 15)**: `fmain.c:1845` — `an->vitality = 0` directly, then `checkdead(i,27)`
- **Lava (environ > 2)**: `fmain.c:1846` — `an->vitality--`, then `checkdead(i,27)`
- **Drowning (environ == 30)**: `fmain.c:1850` — `an->vitality--`, then `checkdead(i,6)`
- **Pit fall (terrain 9, xtype 52)**: `fmain.c:1767-1770` — sets state to FALL, does NOT call checkdead. Pit fall is non-lethal; fairy always rescues via `revive(FALSE)`.

## Fairy Rescue

**Code**: `fmain.c:1388-1403`

When the hero's state is DEAD or FALL, the fairy rescue system activates via the `goodfairy` counter (`unsigned char`, declared `fmain.c:592`).

### Mechanism

```c
if (inum == DEAD || inum==FALL)
{   if (goodfairy == 1) { revive(FALSE); inum = STILL; }
    else if (--goodfairy < 20) ; /* do ressurection effect/glow */
    else if (luck<1 && goodfairy<200) { revive(TRUE); inum = STILL; }
    else if (anim_list[0].state == FALL && goodfairy<200)
    {   revive(FALSE); inum = STILL; }
    else if (goodfairy < 120)
    {   /* fairy sprite animation */
        an = anim_list + 3;
        anix = 4;
        an->abs_x = hero_x + goodfairy*2 - 20;
        an->abs_y = hero_y;
        an->type = OBJECTS;
        an->index = 100 + (cycle & 1);
        an->state = STILL;
        an->weapon = an->environ = 0;
        an->race = 0xff;
        actors_on_screen = TRUE;
        battleflag = FALSE;
    }
}
```

### Timeline (frame-by-frame)

`goodfairy` starts at 0 (reset in `revive()` at `fmain.c:2834`). When hero enters DEAD/FALL:

1. **Frame 1**: `goodfairy` is 0. Not == 1. `--goodfairy` wraps unsigned 0→255. 255 ≥ 20, so no glow. All other checks fail since 255 ≥ 200.
2. **Frames 2–56** (goodfairy 255→200): Counts down. All checks fail (goodfairy ≥ 200). Nothing visible.
3. **Frame 57** (goodfairy goes from 200→199 via decrement): First frame where `goodfairy < 200` is true.
   - If `luck < 1`: **`revive(TRUE)`** → **brother dies, next brother takes over**
   - If state == FALL: **`revive(FALSE)`** → **fall recovery, same brother**
   - Otherwise: continues counting down
4. **Frames 57–136** (goodfairy 199→120): Each frame, luck/fall checks repeat. If luck drops below 1 at any point, brother dies.
5. **Frames 137–199** (goodfairy 119→20): `goodfairy < 120` is true → **fairy sprite visible**, flying toward hero. Position: `hero_x + goodfairy*2 - 20`. AI is suspended during this time (`fmain.c:2112`).
6. **Frames 200–218** (goodfairy 19→1): `--goodfairy < 20` → glow effect only.
7. **Frame 219** (goodfairy == 1): **`revive(FALSE)`** → **fairy rescues hero, same brother continues**.

### Key insight: luck determines rescue vs. succession

- `checkdead()` decrements `luck -= 5` on hero death (`fmain.c:2777`)
- If `luck >= 1` after this decrement → fairy rescues → same brother continues at last safe point
- If `luck < 1` after decrement → brother dies → next brother takes over
- FALL state always gets `revive(FALSE)` regardless of luck → pit falls are non-lethal

## revive

**Function**: `fmain.c:2814-2911`

`revive(new)` where `new` = TRUE means new brother (death succession), FALSE means same brother resurrection (fairy rescue or fall recovery).

### Common setup (both paths)

```c
an = &(anim_list[1]);
an->type = RAFT;
an->abs_x = 13668; an->abs_y = 14470;    /* raft position */

an = &(anim_list[2]);
an->type = SETFIG;
an->abs_x = 13668; an->abs_y = 15000;    /* setfig position */

an = &(anim_list[0]);
an->type = PHIL;
an->goal = USER;

handler_data.laydown = handler_data.pickup = 0;
battleflag = goodfairy = mdex = 0;
```

### New brother path (new == TRUE)

#### Step 1: Place dead brother ghost

```c
if (brother > 0 && brother < 3)
{   ob_listg[brother].xc = hero_x;      /* ghost bones at death location */
    ob_listg[brother].yc = hero_y;
    ob_listg[brother].ob_stat = 1;       /* make bones visible on ground */
    ob_listg[brother+2].ob_stat = 3;     /* activate ghost setfig */
}
```
- `fmain.c:2837-2841`
- `ob_listg[1]` / `ob_listg[2]` = dead brother bones (object ID 28 = bones)
- `ob_listg[3]` / `ob_listg[4]` = ghost setfigs (object ID 11 = ghost)
- Ghost setfig `ob_stat = 3` means it appears as a setfig NPC in the world
- Only done for brothers 1 (Julian) and 2 (Phillip) — Kevin (brother 3) has no successor to find his bones

#### Step 2: Load new brother stats

```c
ob_list8[9].ob_stat = 3;                /* reset princess to setfig state */
br = blist + brother;                    /* index into blist (0-based) */
brave = br->brave; luck = br->luck; kind = br->kind;
wealth = br->wealth; stuff = br->stuff;
brother++;                               /* advance to next brother (1→2→3) */
```
- `fmain.c:2843-2847`
- Stats are loaded from `blist[]` array (see Brother Stats section below)
- `stuff` pointer switches to the new brother's inventory array
- `brother` is incremented AFTER loading stats but BEFORE placard display

#### Step 3: Clear inventory

```c
for (i=0; i<GOLDBASE; i++) stuff[i] = 0;   /* has no stuff */
stuff[0] = an->weapon = 1;                  /* okay, a dirk, then */
```
- `fmain.c:2849-2850`
- `GOLDBASE = 31` (`fmain.c:429`) — clears first 31 inventory slots (items, magic, keys, stats)
- Only a dirk (weapon index 1) is given to the new brother
- Gold slots (31-34) are NOT cleared (they're part of the inventory array but above GOLDBASE)

#### Step 4: Reset position and timers

```c
secret_timer = light_timer = freeze_timer = 0;
safe_x = 19036; safe_y = 15755; region_num = safe_r = 3;
```
- `fmain.c:2852-2853`
- New brothers spawn at coordinates (19036, 15755) in region 3 (Tambry manor area)
- This is hardcoded — all new brothers start at the same location

#### Step 5: Display placard text

The placard sequence depends on which brother is starting:

| `brother` value (after ++) | First placard | Second placard | Meaning |
|---------------------------|---------------|----------------|---------|
| 1 (Julian) | `placard_text(0)` | — | "Rescue the Talisman!" intro story |
| 2 (Phillip) | `placard_text(1)` | `placard_text(2)` | Julian's luck ran out / Phillip sets out |
| 3 (Kevin) | `placard_text(3)` | `placard_text(4)` | Phillip failed / Kevin takes up quest |
| >3 (game over) | `placard_text(5)` | — | "And so ends our sad tale" / "Stay at Home!" |

Placard text content (from `narr.asm:252-308`):
- **placard_text(0)** (msg1): `"Rescue the Talisman!" was the Mayor's plea...And so Julian set out on his quest to recover it.`
- **placard_text(1)** (msg2): `Unfortunately for Julian, his luck had run out. Many months passed and Julian did not return...`
- **placard_text(2)** (msg3): `So Phillip set out, determined to find his brother and complete the quest.`
- **placard_text(3)** (msg4): `But sadly, Phillip's cleverness could not save him from the same fate as his older brother.`
- **placard_text(4)** (msg5): `So Kevin took up the quest, risking all...Young and inexperienced, his chances did not look good.`
- **placard_text(5)** (msg6): `And so ends our sad tale. The Lesson of the Story: Stay at Home!`

The first placard displays for 120 ticks (≈2.4 seconds at 50Hz). For brothers 2 and 3, a second placard follows after 80 more ticks.

#### Step 6: Load shape data and start

```c
actor_file = 6; set_file = 13; shape_read();
```
- `fmain.c:2882`
- `shape_read()` (`fmain2.c:673-680`) calls `read_shapes(brother-1)` to load the correct character sprite file

#### Step 7: Print journey message

```c
if (brother < 4)
{   message_off();
    hero_place = 2;
    event(9);                    /* "% started the journey in his home village of Tambry" */
    if (brother == 1) print_cont(".");
    else if (brother == 2) event(10);   /* "as had his brother before him." */
    else if (brother == 3) event(11);   /* "as had his brothers before him." */
}
```
- `fmain.c:2885-2892`
- Event 9 = `"% started the journey in his home village of Tambry"` (`narr.asm:19`)
- Event 10 = `"as had his brother before him."` (`narr.asm:20`)
- Event 11 = `"as had his brothers before him."` (`narr.asm:21`)

### Common finalization (both paths)

```c
hero_x = an->abs_x = safe_x;
hero_y = an->abs_y = safe_y;
map_x = hero_x - 144;
map_y = hero_y - 90;
new_region = safe_r; load_all();
an->vitality = (15+brave/4);
an->environ = 0;
an->state = STILL;
an->race = -1;
daynight = 8000; lightlevel = 300;
hunger = fatigue = 0;
anix = 3;
```
- `fmain.c:2896-2907`
- Hero placed at `safe_x, safe_y` (for new brothers: 19036, 15755; for fairy rescue: last safe point)
- **Vitality**: set to `15 + brave/4` — depends on bravery stat
- **Time**: daynight reset to 8000, lightlevel to 300
- **Hunger/fatigue**: reset to 0
- `fiery_death = xtype = 0` (`fmain.c:2911`)

### Fairy rescue path (new == FALSE)

When `new` is FALSE (fairy rescue or fall recovery):
- `fade_down()` called (`fmain.c:2894`)
- Skips all placard text, ghost placement, stat/inventory reset
- Hero respawns at current `safe_x, safe_y` with current stats
- Vitality restored to `15 + brave/4`
- Hunger and fatigue reset to 0

## Brother Stats (blist)

**Definition**: `fmain.c:2806-2812`

```c
struct bro {
    char    brave,luck,kind,wealth;
    UBYTE   *stuff;
} blist[] = 
{   { 35,20,15,20,julstuff },    /* julian's attributes */
    { 20,35,15,15,philstuff },   /* phillip's attributes */
    { 15,20,35,10,kevstuff } };  /* kevin's attributes */
```

| Brother | brave | luck | kind | wealth | stuff array | Vitality (15+brave/4) |
|---------|-------|------|------|--------|-------------|----------------------|
| Julian (blist[0]) | 35 | 20 | 15 | 20 | julstuff | 23 |
| Phillip (blist[1]) | 20 | 35 | 15 | 15 | philstuff | 20 |
| Kevin (blist[2]) | 15 | 20 | 35 | 10 | kevstuff | 18 |

**Inventory arrays**: `fmain.c:432`
```c
UBYTE *stuff, julstuff[ARROWBASE], philstuff[ARROWBASE], kevstuff[ARROWBASE];
```
- `ARROWBASE = 35` (`fmain.c:430`)
- Each brother has an independent 35-byte inventory array
- `stuff` is a pointer that switches to the active brother's array

### Stat design notes

- **Julian**: Highest bravery (strongest fighter, highest vitality), balanced luck
- **Phillip**: Highest luck (most likely to be fairy-rescued), balanced bravery
- **Kevin**: Highest kindness, lowest wealth and bravery (weakest fighter, lowest vitality)
- Total of each stat across all brothers: brave=70, luck=75, kind=65, wealth=45

## Inventory Transfer (mod1save)

**Function**: `fmain.c:3621-3630`

```c
mod1save()
{   /* save stuff */
    saveload(julstuff,35);
    saveload(philstuff,35);
    saveload(kevstuff,35);
    /* set stuff pointer */
    stuff = blist[brother-1].stuff;
    /* save missile list - mdex already saved */
    saveload((void *)missile_list,6 * sizeof (struct missile));
}
```

This function is called as part of the save/load system (`fmain2.c:1516`). It serializes all three brothers' inventory arrays (35 bytes each) and the missile list.

### Key detail: stuff pointer restoration

After loading, `stuff = blist[brother-1].stuff` reassigns the active `stuff` pointer to the current brother's inventory array. Since `brother` is 1-indexed during play, `blist[brother-1]` correctly indexes back to the 0-based array.

### Picking up dead brother's bones

When a living brother picks up a dead brother's bones (object ID 28), their inventories merge:

```c
/* fmain.c:3174-3177 */
ob_listg[3].ob_stat = ob_listg[4].ob_stat = 0;    /* remove both ghosts */
for (k=0; k<GOLDBASE; k++)
{   if (x==1) stuff[k] += julstuff[k];
    else stuff[k] += philstuff[k];
}
```
- `fmain.c:3173-3177`
- `x` is the object index (1 = Julian's bones, 2 = Phillip's bones)
- All 31 item slots (0 to GOLDBASE-1) from the dead brother's array are added to the current brother's inventory
- Both ghost setfigs (ob_listg[3] and ob_listg[4]) are disabled (ob_stat = 0) regardless of which bones are picked up

### What does NOT transfer between brothers

- Stats (brave, luck, kind, wealth) are loaded fresh from blist[]
- Quest progress (princess counter, quest flags in ob_listg/ob_list8) persists globally — NOT per-brother
- Hunger and fatigue reset to 0
- Position resets to Tambry (19036, 15755)

## Game Over

**Code**: `fmain.c:2872`

```c
if (brother > 3) { quitflag = TRUE; Delay(500); }
```

When `brother` exceeds 3 (all three brothers dead):
1. `placard_text(5)` displays: `"And so ends our sad tale. The Lesson of the Story: Stay at Home!"` (`narr.asm:299-302`)
2. `quitflag = TRUE` — sets the game termination flag
3. `Delay(500)` — 10-second pause (500 ticks at 50Hz)
4. `viewstatus = 2` (`fmain.c:2910`) — if `brother > 3`, viewstatus set to 2 (vs. 3 for normal play)

The game then exits the main loop because `quitflag` is true.

## Ghost Brothers

### ob_listg layout for brothers

**Definition**: `fmain2.c:1001-1006`

```c
ob_listg[] = {   /* global objects */
    {     0,    0,0,0},            /* [0] special item (for 'give') */
    {     0,    0,28,0},           /* [1] dead brother 1 - to be filled in later */
    {     0,    0,28,0},           /* [2] dead brother 2 - to be filled in later */
    { 19316,15747,11,0},           /* [3] ghost brother 1 - to be filled in later */
    { 18196,15735,11,0},           /* [4] ghost brother 2 - to be filled in later */
    ...
```

| Index | Object ID | Initial coords | Purpose |
|-------|-----------|----------------|---------|
| ob_listg[1] | 28 (bones) | (0,0) / filled at death | Julian's bones |
| ob_listg[2] | 28 (bones) | (0,0) / filled at death | Phillip's bones |
| ob_listg[3] | 11 (ghost) | (19316,15747) | Julian's ghost setfig |
| ob_listg[4] | 11 (ghost) | (18196,15735) | Phillip's ghost setfig |

### How ghosts are placed

When a brother dies in `revive()` (`fmain.c:2837-2841`):
- `ob_listg[brother].xc = hero_x` — bones placed at death location
- `ob_listg[brother].yc = hero_y`
- `ob_listg[brother].ob_stat = 1` — bones become visible on ground
- `ob_listg[brother+2].ob_stat = 3` — ghost setfig becomes active NPC

Ghost setfig type 11 corresponds to `setfig_table[11]` at `fmain.c:39`: `{ 16,7,0 }` — cfile 16, image base 7, can_talk = 0.

### Collecting bones

When a brother picks up bones (`fmain.c:3173-3177`):
- `ob_listg[3].ob_stat = ob_listg[4].ob_stat = 0` — BOTH ghost setfigs are removed (regardless of which bones are collected)
- Dead brother's inventory items (0 to GOLDBASE-1) are merged into current brother's inventory

### Ghost initial positions

The ghost setfig initial coordinates at `fmain2.c:1005-1006`:
- Ghost 1 (Julian): (19316, 15747) — near Tambry manor area
- Ghost 2 (Phillip): (18196, 15735) — near Tambry manor area

These are default positions; when a brother actually dies, the ghost activates at these positions while the bones are placed at the actual death location.

## brother Variable

**Declaration**: `fmain.c:567` — `short brother;`

### Values

| Value | State |
|-------|-------|
| 0 | Initial (before first revive call) |
| 1 | Julian active (set after first `revive(TRUE)`) |
| 2 | Phillip active |
| 3 | Kevin active |
| >3 | Game over |

### Lifecycle

1. `brother` starts at 0 (C static initialization)
2. `revive(TRUE)` at game start (`fmain.c:1245`): reads `blist[0]` (Julian), then `brother++` → 1
3. If Julian dies: `revive(TRUE)` reads `blist[1]` (Phillip), then `brother++` → 2
4. If Phillip dies: `revive(TRUE)` reads `blist[2]` (Kevin), then `brother++` → 3
5. If Kevin dies: `revive(TRUE)` with `brother` already 3, reads beyond blist bounds (but game over check at `brother > 3` fires)

Wait — step 5 needs re-examination. When Kevin dies:
- `br = blist + brother` where `brother == 3` → reads `blist[3]` which is OUT OF BOUNDS (blist has only 3 entries: 0,1,2)
- `brother++` → 4
- `brother > 3` at `fmain.c:2872` → `quitflag = TRUE`

This means the code briefly reads garbage from `blist[3]` for stats — but since `quitflag` is immediately set, the game ends before these values matter.

### Save/load

`brother` is part of the 80-byte block saved with `saveload((void *)&map_x,80)` at `fmain2.c:1507`. The block starts at `map_x` (`fmain.c:557`) and `brother` is at `fmain.c:567` — within the first 80 bytes of that contiguous variable block.

### Usage for sprite selection

`read_shapes(brother-1)` (`fmain2.c:676`) loads the character sprite file for the current brother (0=Julian, 1=Phillip, 2=Kevin).

## princess Counter

**Declaration**: `fmain.c:568` — `short princess;`

The `princess` variable tracks how many princesses have been rescued. It is a global variable, NOT per-brother:

- It persists across brother succession (saved in the 80-byte block alongside `brother`)
- Starts at 0
- Incremented in `rescue()` (`fmain2.c:1594`): `princess++`
- Used to select which princess text to display during rescue: `i = princess*3` → `placard_text(8+i)` (`fmain2.c:1587-1588`)

### Princess names by index

| princess value | Placard offset | Princess name |
|----------------|---------------|---------------|
| 0 | 8,9,10 (msg8/8a/8b) | Katra |
| 1 | 11,12,13 (msg9/9a/9b) | Karla (Katra's sister) |
| 2 | 14,15,16 (msg10/10a/10b) | Kandy (Katra's and Karla's sister) |

The princess counter does NOT reset between brothers. If Julian rescues Katra (princess=0→1), and Phillip later rescues a princess, she will be Karla (princess=1→2).

## Unresolved

- **blist[3] out-of-bounds read**: When Kevin dies, `br = blist + 3` reads past the array. The values read are whatever follows `blist` in memory. This is likely benign since `quitflag` fires immediately, but the exact memory layout after blist is unknown without deeper analysis.
- **Ghost interaction**: Ghost setfigs have `can_talk = 0` (setfig_table[11]), so they presumably cannot be spoken to. Whether they have any other interaction beyond the bones pickup is not traced.
- **ob_list8[9] reset**: `revive()` sets `ob_list8[9].ob_stat = 3` (`fmain.c:2843`). This is the princess object. Setting it to 3 re-enables the princess as a setfig when a new brother starts. The prior `rescue()` sets it to 0 after rescue. This means each brother can potentially rescue a princess. The interaction between this and the `princess` counter is clear — the counter tracks which princess text to show, while `ob_stat` controls physical presence.
- **fallstates content**: `fallstates[]` (`fmain2.c:871-874`) contains animation frame indices per brother for the FALL state: `{ 0,0,0,0,0,0, 0x20,0x22,0x3a,0x6f,0x70,0x71, 0x24,0x27,0x3c,0x6f,0x70,0x71, 0x37,0x38,0x3d,0x6f,0x70,0x71 }`. The first 6 entries (all zeros) correspond to brother 0 (pre-game) — purpose unknown. Entries for brothers 1-3 contain distinct animation sequences.

## Cross-Cutting Findings

- **ob_list8[9] reset in revive**: `fmain.c:2843` — the princess object is re-enabled to `ob_stat = 3` during every brother succession. This interacts with the princess rescue system in `fmain2.c:1586`.
- **Combat discovery overlap**: The `dohit()` → `checkdead()` path is documented in `reference/_discovery/combat.md`. The goodfairy mechanism is also partially documented there.
- **AI suspension during fairy**: `fmain.c:2112` — `if (goodfairy && goodfairy < 120) break;` completely halts enemy AI during fairy animation. Documented in `reference/_discovery/ai-system.md:217`.
- **Inventory merge on bone pickup**: `fmain.c:3173-3177` — this is part of the object interaction system, interacting with the inventory system.
- **shape_read uses brother**: `fmain2.c:676` — `read_shapes(brother-1)` — the graphics loading system depends on the brother variable.

## Refinement Log

- 2026-04-05: Initial comprehensive discovery pass covering checkdead, fairy rescue mechanism, revive (both paths), blist stats, mod1save, game over, ghost brothers, brother variable lifecycle, and princess counter.
