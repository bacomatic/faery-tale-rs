# Discovery: Combat System — Damage Formula, Swing Mechanics, and Post-Combat

**Status**: complete
**Investigated**: 2026-04-05
**Requested by**: orchestrator
**Prompt summary**: Trace the complete combat system including damage formula, swing mechanics, aftermath/loot, death/succession, target selection, and weapon/monster tables.

## 1. Damage Formula — `dohit()`

### Function Signature — `fmain2.c:230`
```c
dohit(i,j,fc,wt) short wt; register long j,i,fc;
```
- `i` = attacker index (-1 = arrow, -2 = fireball, 0 = player, 3+ = monster)
- `j` = defender index
- `fc` = facing direction (0-7)
- `wt` = damage amount (weapon code, possibly modified)

### Immunity Checks — `fmain2.c:231-235`

**Condition 1** — Necromancer / Witch immunity:
```c
if (anim_list[0].weapon < 4 &&
    (anim_list[j].race == 9 ||
        (anim_list[j].race == 0x89 && stuff[7] == 0) ))
{   speak(58); return; }
```
- Race 9 = Necromancer (ENEMY type) — immune to weapons < 4 (dirk, mace, sword). Only bow (4) or wand (5) can damage.
- Race 0x89 = Witch (SETFIG type, setfig index 9) — immune to weapons < 4 **unless** the player has the Sunstone (`stuff[7] != 0`). With Sunstone, any weapon works.
- speak(58) = `"Stupid fool, you can't hurt me with that!"` — `narr.asm:513`

**Condition 2** — Absolute immunity:
```c
if (anim_list[j].race == 0x8a || anim_list[j].race == 0x8b) return;
```
- Race 0x8a = Spectre (setfig index 10 from `setfig_table`, `fmain.c:35`)
- Race 0x8b = Ghost (setfig index 11 from `setfig_table`, `fmain.c:36`)
- Completely immune to all damage. No message, silently returns.

### Damage Application — `fmain2.c:236-237`
```c
anim_list[j].vitality -= wt;
if (anim_list[j].vitality < 0) anim_list[j].vitality = 0;
```
**Damage = wt (the weapon/damage parameter passed to dohit).** Vitality floor is 0.

### Sound Effects — `fmain2.c:238-241`
```c
if (i==-1) effect(2,500+rand64());       // arrow hit
else if (i==-2) effect(5,3200+bitrand(511)); // fireball hit
else if (j==0) effect(0,800+bitrand(511));   // player takes melee hit
else effect(3,400+rand256());                // monster takes melee hit
```
`effect(num,speed)` at `fmain.c:3616` plays `sample[num]` at the given pitch. The random offset varies the pitch slightly per hit.

| Sound | `effect()` call | Trigger |
|-------|----------------|---------|
| sample[0] | `effect(0,800+bitrand(511))` | Player hit by melee |
| sample[1] | `effect(1,150+rand256())` | Near miss (see §2) |
| sample[2] | `effect(2,500+rand64())` | Arrow hit |
| sample[3] | `effect(3,400+rand256())` | Monster hit by melee |
| sample[4] | `effect(4,400+rand256())` | Bow shot release |
| sample[5] | `effect(5,3200+bitrand(511))` | Fireball hit |
| sample[5] | `effect(5,1800+rand256())` | Wand/dragon fireball launch |

### Knockback — `fmain2.c:243-245`
```c
if (anim_list[j].type != DRAGON && anim_list[j].type != SETFIG
    && move_figure(j,fc,2) && (i >= 0))
        move_figure(i,fc,2);
```
- Target pushed 2 pixels in attacker's facing direction via `move_figure()`.
- **DRAGON** (type 6) and **SETFIG** (type 4) are immune to knockback.
- If knockback succeeds AND attacker is melee (i >= 0), attacker also moves 2 pixels forward (follow-through).
- `move_figure()` at `fmain2.c:325` checks `proxcheck()` and fails if destination is blocked. If target can't be pushed, attacker doesn't slide forward either.

### Death Check — `fmain2.c:246`
```c
checkdead(j,5);
```
Always calls `checkdead()` on the defender with dtype=5 ("% was hit and killed!" — `narr.asm:16`).

## 2. Hit Detection — Melee Swing Loop

### Entry Point — `fmain.c:2237-2264`

The hit detection runs once per frame for every actor in a fighting state (state 0-8). It's inside the main rendering loop, after scrolling, before missile processing.

```c
for (i=0; i<anix; i++)
{   short xs,ys,wt,fc,bv,xd,yd;
    if (i > 0 && freeze_timer) break;      // frozen enemies can't swing
    if (i==1 || anim_list[i].state >= WALKING) continue;  // skip raft (index 1), non-fighters
    wt = anim_list[i].weapon;
    fc = anim_list[i].facing;
    if (!(wt & 4))                          // bit 2 = ranged weapon (bow=4, wand=5)
    {   if (wt >= 8) wt = 5;               // touch attack capped to 5
        wt += bitrand(2);                   // +0, +1, or +2 random bonus
```

### Strike Point Calculation — `fmain.c:2247-2248`
```c
xs = newx(anim_list[i].abs_x,fc,wt+wt) + rand8()-3;
ys = newy(anim_list[i].abs_y,fc,wt+wt) + rand8()-3;
```
- Strike point is `wt*2` pixels in front of the attacker in their facing direction.
- Random jitter: rand8()-3 = offset of -3 to +4 pixels on each axis.
- Longer weapons extend the strike point further.

### Hit Window (Bravery as Reach) — `fmain.c:2249-2250`
```c
if (i==0) bv = (brave/20)+5; else bv = 2 + rand4();
if (bv > 14) bv = 15;
```
- **Player reach**: `(brave / 20) + 5`, max 15 pixels.
  - Julian (brave=35): bv = 6
  - Phillip (brave=20): bv = 6
  - Kevin (brave=15): bv = 5
  - As brave grows from kills, reach increases.
- **Monster reach**: `2 + rand4()` = 2-5 pixels, re-rolled each frame.

### Target Matching — `fmain.c:2252-2263`
```c
for (j=0; j<anix; j++)
{   if (j==1 || j==i || anim_list[j].state == DEAD ||
        anim_list[i].type == CARRIER) continue;
    xd = anim_list[j].abs_x-xs;
    yd = anim_list[j].abs_y-ys;
    if (xd<0) xd = -xd;
    if (yd<0) yd = -yd;
    if (xd > yd) yd = xd;              // Chebyshev distance
    if ((i==0 || rand256()>brave) && yd < bv && !freeze_timer)
    { dohit(i,j,fc,wt); break; }
    else if ((yd<bv+2) && wt !=5) effect(1,150+rand256()); // near miss
}
```

**Distance metric**: Chebyshev distance (max of |dx|, |dy|) from strike point to target.

**Hit condition** (all must be true):
1. Distance < bv (reach)
2. `freeze_timer` == 0
3. For **player attacks** (i==0): automatic hit (always passes)
4. For **monster attacks** (i>0): `rand256() > brave` — bravery acts as dodge chance
   - Probability of monster hit landing: `(256 - brave) / 256`
   - Julian (brave=35): 86% hit rate for monsters
   - Brave=100 after many kills: 61% hit rate

**Near miss** (yd < bv+2 and weapon != wand): plays miss sound effect — `effect(1,150+rand256())`.

### Damage Dealt (Melee)

| Weapon | Code | Base | +Random | Damage Range |
|--------|------|------|---------|--------------|
| None | 0 | 0 | bitrand(2) | 0-2 |
| Dirk | 1 | 1 | bitrand(2) | 1-3 |
| Mace | 2 | 2 | bitrand(2) | 2-4 |
| Sword | 3 | 3 | bitrand(2) | 3-5 |
| Touch | 8→5 | 5 | bitrand(2) | 5-7 |

Bow and Wand (bit 2 set) don't use melee — they fire missiles.

## 3. Missile (Ranged) Combat — `fmain.c:2266-2299`

### Missile Hit Detection — `fmain.c:2276-2296`
```c
for (j=0; j<anix; j++)
{   if (j==0) bv = brave; else bv = 20;
    ...
    if ((i != 0 || bitrand(512)>bv) && yd < mt)
    {   if (ms->missile_type == 2) dohit(-2,j,fc,rand8()+4);
        else dohit(-1,j,fc,rand8()+4);
```
- `mt` (hit radius): 6 for arrows, 9 for fireballs — `fmain.c:2280-2281`
- **Missile damage**: `rand8() + 4` = **4-11** for both arrows and fireballs
- Arrow hits call `dohit(-1,...)`, fireballs call `dohit(-2,...)`
- Dodge check: For player target (j==0), `bv = brave`; for monsters, `bv = 20`
- Missile slot 0 has a dodge check `bitrand(512) > bv`; other slots always hit

### Witch Attack (Special Missile) — `fmain.c:2375`
```c
dohit(-1,0,anim_list[2].facing,rand2()+1);
```
- Witch (anim_list[2]) deals `rand2()+1` = **1-2 damage** per attack
- Treated as arrow-type hit (i=-1)
- Only fires when `witchflag` is set and player is within range (`calc_dist(2,0) < 100`)

### Dragon Attack (Special Missile) — `fmain.c:1489-1497`
Dragon (type DRAGON) fires fireballs via missile system:
- `ms->missile_type = 2` (fireball)
- `ms->speed = 5`
- Fires at `rand4()==0` rate (25% chance per frame)
- Damage handled by the missile hit loop: `rand8()+4` = 4-11

## 4. Swing State Machine — `trans_list[]`

### State Definitions — `fmain.c:139-149`

FIGHTING = 0. States 0-8 are the swing animation substates:

```c
struct transition trans_list[9] = {
    {  1, 8, 0, 1 },   // 0 - arm down, weapon low
    {  2, 0, 1, 0 },   // 1 - arm down, weapon diagonal down
    {  3, 1, 2, 8 },   // 2 - arm swing1, weapon horizontal
    {  4, 2, 3, 7 },   // 3 - arm swing2, weapon raised
    {  5, 3, 4, 6 },   // 4 - arm swing2, weapon diag up
    {  6, 4, 5, 5 },   // 5 - arm swing2, weapon high
    {  8, 5, 6, 4 },   // 6 - arm high, weapon up
    {  8, 6, 7, 3 },   // 7 - arm high, weapon horizontal
    {  0, 6, 8, 2 }};  // 8 - arm middle, weapon raise fwd
```

Each state has 4 possible next states, selected by `rand4()` — `fmain.c:1712`:
```c
s = an->state = trans_list[s].newstate[rand4()];
```

### Transition Processing — `fmain.c:1711-1716`
```c
else if (s < 9)
{   inum = diroffs[d+8];
    s = an->state = trans_list[s].newstate[rand4()];
    if (i>2 && s==6 || s==7) s = 8;    // monsters skip overhead states
    dex = s + inum;
    frustflag = 0;
    goto cpx;
}
```
- `diroffs[d+8]` selects the correct directional sprite offset for fighting sprites.
- `diroffs` at `fmain.c:1010`: `{16,16,24,24,0,0,8,8,56,56,68,68,32,32,44,44}`
  - d+8 indices: {56,56,68,68,32,32,44,44} for the 8 directions
- Monsters (i>2) that reach states 6 or 7 (overhead swings) are forced to state 8.
- `dex = s + inum` selects the exact sprite frame (base directional offset + swing substate).

### Player Fight Entry — `fmain.c:1431-1436`
```c
if (anim_list[0].weapon == 4) inum = SHOOT1;      // bow → shoot
else if (anim_list[0].weapon == 5)
{   if (inum < SHOOT1) inum = SHOOT1;   }          // wand → shoot
else inum = FIGHTING;                              // melee → fight
```
Button press (fire button or keyboard) with melee weapon sets `state = FIGHTING` (0), which enters the swing state machine.

### Enemy Fight Entry — `fmain.c:2166`
```c
if (an->state >= WALKING) an->state = FIGHTING;
```
Enemies transition to FIGHTING when within melee threshold:
```c
thresh = 14 - mode;
if (an->race == 7) thresh = 16;  // DKnight has extra range
if ((an->weapon & 4)==0 && xd < thresh && yd < thresh)
```

## 5. Aftermath — Post-Kill Rewards

### Function — `fmain2.c:253-275`
```c
aftermath()
{   register long dead, flee, i, j;
    dead = flee = 0;
    for (i=3; i<anix; i++)
    {   if (anim_list[i].type != ENEMY) ;
        else if (anim_list[i].state == DEAD) dead++;
        else if (anim_list[i].goal == FLEE) flee++;
    }
```

Called when `battleflag` goes from TRUE to FALSE (`fmain.c:2192`):
```c
if (battleflag==0 && battle2)
{   prq(7); prq(4); aftermath(); }
```

### Messages — `fmain2.c:259-270`
- If player is dead (vitality < 1): no message.
- If player vitality < 5 and enemies died: `"Bravely done!"`
- If xtype < 50 (not in special zones): reports dead/fled counts.

### Stat Rewards from Kills — `fmain.c:2777` (via `checkdead`)
```c
if (i) brave++; else { event(dtype); luck -= 5; setmood(TRUE); }
```
- **Each enemy kill**: `brave++` (bravery increases by 1).
- **Killing a SETFIG** (NPC): `kind -= 3` — `fmain.c:2776`
- No explicit experience system — bravery IS the experience reward.

### Body Search / Treasure — `fmain.c:3254-3281`

Triggered by the "Get" action when near a dead body:
```c
else if (anim_list[nearest].vitality == 0 || freeze_timer)
{   extract("% searched the body and found");
```

**Weapon drop**: `fmain.c:3256-3270`
```c
i = anim_list[nearest].weapon; if (i > 5) i = 0;
if (i)
{   print_cont("a "); print_cont(inv_list[i-1].name);
    stuff[i-1]++;
    if (i > anim_list[0].weapon) anim_list[0].weapon = i;
```
- Monster's weapon code (capped to 0-5; touch attacks give nothing).
- If the found weapon is better than current, auto-equips.
- Bow drops also give `rand8()+2` = 2-9 arrows — `fmain.c:3265-3268`.
- After taking weapon, `anim_list[nearest].weapon = -1` (marks as looted).

**Treasure drop**: `fmain.c:3271-3280`
```c
j = anim_list[nearest].race;
if (j & 0x80) j = 0;               // setfigs have no treasure
else
{   j = (encounter_chart[j].treasure * 8) + rand8();
    j = treasure_probs[j];
}
```
- Setfigs (race & 0x80) yield no treasure.
- Otherwise indexes `treasure_probs[]` by `encounter_chart[race].treasure * 8 + rand8()`.
- If `j >= GOLDBASE` (31): `wealth += inv_list[j].maxshown` (gold).
- Else: `stuff[j]++` (inventory item).

### Turtle Egg Check — `fmain2.c:274`
```c
if (turtle_eggs) get_turtle();
```
After battle, if turtle eggs are present, triggers turtle quest event.

## 6. Death System — `checkdead()`

### Function — `fmain.c:2769-2784`
```c
checkdead(i,dtype) register long i, dtype;
{   register struct shape *an;
    an = &(anim_list[i]);
    if (an->vitality < 1 && an->state != DYING && an->state != DEAD)
    {   an->vitality = 0; an->tactic = 7;
        an->goal = DEATH; an->state = DYING;
        if (an->race == 7) speak(42);       // DKnight death speech
        else if (an->type == SETFIG && an->race != 0x89) kind -= 3;
        if (i) brave++; else { event(dtype); luck -= 5; setmood(TRUE); }
        if (kind < 0) kind = 0;
        prq(7);
    }
    if (i == 0) prq(4);
}
```

### Death Effects:
| Target | Effect |
|--------|--------|
| Any actor | vitality=0, tactic=7, goal=DEATH, state=DYING |
| DKnight (race 7) | speak(42): "Your prowess in battle is great..." (`narr.asm:467-469`) |
| SETFIG (not witch 0x89) | `kind -= 3` (kindness penalty for killing NPCs) |
| Monster (i>0) | `brave++` |
| Player (i==0) | `event(dtype)` → death message; `luck -= 5`; `setmood(TRUE)` |

### Death Event Messages (dtype parameter):
| dtype | Source | Message (from `narr.asm:11+`) |
|-------|--------|------|
| 5 | `dohit()`/melee & missile | "% was hit and killed!" |
| 6 | drowning | "% was drowned in the water!" |
| 7 | lava | "% was burned in the lava." |
| 8 | witch | "% was turned to stone by the witch." |

### Dying Animation — `fmain.c:1718-1728`
```c
if (s == DYING)
{   if (an->tactic > 4) { /* falling frame 1 */ }
    else if (an->tactic > 0) { /* falling frame 2 */ }
    else { an->state = DEAD; dex = 82; }
}
```
`tactic` counts down from 7 to 0 (set at death). Over 7 frames, the actor plays the dying animation, then transitions to DEAD (state 15), sprite index 82.

### Special Death Drops — `fmain.c:1751-1756`
```c
if (an->race == 0x09) // Necromancer
{   an->race = 10; an->vitality = 10;
    an->state = STILL; an->weapon = 0;
    leave_item(i,139);     // leave the talisman
}
if (an->race == 0x89) leave_item(i,27); // witch → leave the lasso
```
- Necromancer transforms into Woodcutter (race 10) upon "death" and drops the Talisman (object 139).
- Witch drops the Golden Lasso (object 27).

### Good Fairy Resurrection — `fmain.c:1388-1407`

When the player is DEAD or FALL, `goodfairy` (unsigned char, starts at 0) counts down each frame:

1. **Frame 1**: `--goodfairy` wraps 0→255
2. **Frames 255→200**: Check `luck < 1` — if true, `revive(TRUE)` = **brother switch** (the character has permanently died)
3. **Frames 199→120**: idle
4. **Frames 119→20**: fairy sprite appears on screen as rescue animation — `fmain.c:1394-1403`
5. **Frames 19→2**: "resurrection effect/glow" (visual only)
6. **Frame 1**: `revive(FALSE)` = **fairy rescue** (same character, reset to safe point)

```c
if (goodfairy == 1) { revive(FALSE); inum = STILL; }
else if (--goodfairy < 20) ; /* resurrection effect/glow */
else if (luck<1 && goodfairy<200) { revive(TRUE); inum = STILL; }
else if (anim_list[0].state == FALL && goodfairy<200)
{   revive(FALSE); inum = STILL; }
```

**Key**: `luck < 1` triggers permanent death → brother succession. `luck >= 1` always leads to fairy rescue. Since each death costs 5 luck, the number of fairy rescues before brother switch ≈ starting_luck / 5.

### Brother Succession — `revive()` at `fmain.c:2812-2900`

`revive(TRUE)` = new brother:
```c
blist[] = {
    { 35,20,15,20,julstuff },   // Julian: brave=35, luck=20, kind=15, wealth=20
    { 20,35,15,15,philstuff },  // Phillip: brave=20, luck=35, kind=15, wealth=15
    { 15,20,35,10,kevstuff } }; // Kevin: brave=15, luck=20, kind=35, wealth=10
```
- `fmain.c:2803-2805` — starting stats per brother
- `brother` increments: 1=Julian, 2=Phillip, 3=Kevin, 4+=game over
- Inventory wiped: `for (i=0; i<GOLDBASE; i++) stuff[i] = 0` — `fmain.c:2849`
- Starting weapon: Dirk (weapon 1) — `fmain.c:2850`
- Vitality set: `15 + brave/4` — `fmain.c:2897`
- Dead brother's body placed in world — `fmain.c:2840-2843`
- Ghost of dead brother placed — `fmain.c:2844`

`revive(FALSE)` = fairy rescue (same brother):
- No stat changes
- Returns to last safe position (`safe_x`, `safe_y`)
- Vitality restored to `15 + brave/4`
- Hunger and fatigue reset to 0

## 7. Target Selection — `nearest_fig()`

### Function — `fmain2.c:302-311`
```c
nearest_fig(constraint,dist) char constraint; short dist;
{   register long d,i;
    nearest = 0;
    for (i=1; i<anix2; i++)
    {   if (anim_list[i].type == OBJECTS &&
            (constraint || anim_list[i].index == 0x1d)) continue;
        d = calc_dist(i,0);
        if (d<dist) { nearest = i; dist = d; }
    }
    return dist;
}
```
- Scans all active actors (1 through anix2-1), skips OBJECTS type (unless unconstrained and index != 0x1d).
- Returns closest actor's distance; sets global `nearest` to its index.
- Used for the "Get" and "Talk" commands to find the nearest interactable figure.

## 8. Distance Calculation — `calc_dist()`

### Function — `fmain2.c:315-321`
```c
calc_dist(a,b) register long a,b;
{   register long x,y; short dist;
    x = anim_list[a].abs_x - anim_list[b].abs_x; if (x<0) x = -x;
    y = anim_list[a].abs_y - anim_list[b].abs_y; if (y<0) y = -y;
    if (x>(y+y)) return x;
    if (y>(x+x)) return y;
    return (x+y)*5/7;
}
```
This is an **octagonal distance approximation**:
- If x > 2y: return x (nearly horizontal)
- If y > 2x: return y (nearly vertical)
- Otherwise: return (x+y)*5/7 ≈ 0.714*(x+y) (diagonal approximation)

This approximates Euclidean distance while avoiding floating-point math.

## 9. Weapon Types

### Weapon Code Table — `fmain.c:77`
```c
/* weapon: 0=none, 1=dagger, 2=mace, 3=sword, 4=bow, 5=wand */
```

| Code | Name | Type | Melee Damage | Missile Damage | Strike Range (`wt*2`) |
|------|------|------|-------------|----------------|---------------------|
| 0 | None | Melee | 0-2 | — | 0-4 px |
| 1 | Dirk | Melee | 1-3 | — | 2-6 px |
| 2 | Mace | Melee | 2-4 | — | 4-8 px |
| 3 | Sword | Melee | 3-5 | — | 6-10 px |
| 4 | Bow | Ranged | — | 4-11 | mt=6 |
| 5 | Wand | Ranged | — | 4-11 | mt=9 |
| 8 | Touch | Melee | 5-7 | — | 10-14 px |

- Melee damage: `wt + bitrand(2)` where wt is clamped to 5 for touch attacks — `fmain.c:2244-2245`
- Missile damage: `rand8() + 4` — `fmain.c:2292-2293`
- Touch attack (8) is monster-only, clamped to 5 for damage calc but uses original 8 for range calculation before the clamp line.

**Wait — re-reading the code carefully**: `wt` is set to 5 for touch attacks BEFORE the range/damage calc. So touch attack range = `(5 + bitrand(2)) * 2` = 10-14 px, and damage = 5 + bitrand(2) = 5-7.

### Inventory Names — `fmain.c:380-414`
| Index | Name | Notes |
|-------|------|-------|
| 0 | Dirk | stuff[0] |
| 1 | Mace | stuff[1] |
| 2 | Sword | stuff[2] |
| 3 | Bow | stuff[3] |
| 4 | Magic Wand | stuff[4] |

## 10. Monster Combat Stats — `encounter_chart[]`

### Table — `fmain.c:51-62`
```c
struct encounter {
    char hitpoints, agressive, arms, cleverness, treasure, file_id;
} encounter_chart[] = {
```

| # | Monster | HP | Arms | Clever | Treasure | Actual Weapons |
|---|---------|-----|------|--------|----------|---------------|
| 0 | Ogre | 18 | 2 | 0 | 2 | Dirk or Mace |
| 1 | Orcs | 12 | 4 | 1 | 1 | Bow, Bow, Sword, or Mace |
| 2 | Wraith | 16 | 6 | 1 | 4 | Touch only |
| 3 | Skeleton | 8 | 3 | 0 | 3 | Dirk, Mace, Sword, or Mace |
| 4 | Snake | 16 | 6 | 1 | 0 | Touch only |
| 5 | Salamander | 9 | 3 | 0 | 0 | Dirk, Mace, Sword, or Mace |
| 6 | Spider | 10 | 6 | 1 | 0 | Touch only |
| 7 | DKnight | 40 | 7 | 1 | 0 | Sword only |
| 8 | Loraii | 12 | 6 | 1 | 0 | Touch only |
| 9 | Necromancer | 50 | 5 | 0 | 0 | Wand only |
| 10 | Woodcutter | 4 | 0 | 0 | 0 | Unarmed |

**Cleverness** determines the AI goal mode — `fmain.c:2760`:
- If ranged weapon: `ARCHER1 + cleverness` (3 or 4)
- If melee: `ATTACK1 + cleverness` (1 or 2)
- Cleverness 0 = stupid (direct attack, no evasion)
- Cleverness 1 = clever (uses terrain, retreats when hurt)

**Arms → weapon_probs mapping**: `weapon_probs[arms*4 + wt]` where `wt` = rand4() if `mixflag & 4`, else 0 — `fmain.c:2756-2758`.

## 11. Treasure/Loot Tables

### `treasure_probs[]` — `fmain2.c:852-858`
```c
char treasure_probs[] = {
     0, 0, 0, 0, 0, 0, 0, 0,   // group 0: no treasure
     9,11,13,31,31,17,17,32,   // group 1: stone,vial,totem,gold,keys
    12,14,20,20,20,31,33,31,   // group 2: keys,skull,gold,nothing
    10,10,16,16,11,17,18,19,   // group 3: magic and keys
    15,21,0,0,0,0,0,0          // group 4: jade skull and white key
};
```

Indexed by `encounter_chart[race].treasure * 8 + rand8()`. The value is an `inv_list` index.

#### Decoded Treasure Groups:

**Group 0 (treasure=0)**: No drops. Used by: Snake, Salamander, Spider, DKnight, Loraii, Necromancer, Woodcutter.

**Group 1 (treasure=1)**: Used by Orcs.
| rand8() | Index | Item |
|---------|-------|------|
| 0 | 9 | Blue Stone |
| 1 | 11 | Glass Vial |
| 2 | 13 | Bird Totem |
| 3 | 31 | 2 Gold Pieces |
| 4 | 31 | 2 Gold Pieces |
| 5 | 17 | Green Key |
| 6 | 17 | Green Key |
| 7 | 32 | 5 Gold Pieces |

**Group 2 (treasure=2)**: Used by Ogres.
| rand8() | Index | Item |
|---------|-------|------|
| 0 | 12 | Crystal Orb |
| 1 | 14 | Gold Ring |
| 2 | 20 | Grey Key |
| 3 | 20 | Grey Key |
| 4 | 20 | Grey Key |
| 5 | 31 | 2 Gold Pieces |
| 6 | 33 | 10 Gold Pieces |
| 7 | 31 | 2 Gold Pieces |

**Group 3 (treasure=3)**: Used by Skeletons.
| rand8() | Index | Item |
|---------|-------|------|
| 0 | 10 | Green Jewel |
| 1 | 10 | Green Jewel |
| 2 | 16 | Gold Key |
| 3 | 16 | Gold Key |
| 4 | 11 | Glass Vial |
| 5 | 17 | Green Key |
| 6 | 18 | Blue Key |
| 7 | 19 | Red Key |

**Group 4 (treasure=4)**: Used by Wraiths.
| rand8() | Index | Item |
|---------|-------|------|
| 0 | 15 | Jade Skull |
| 1 | 21 | White Key |
| 2-7 | 0 | Nothing (Dirk slot = 0 = no treasure since it's checked) |

Wait — index 0 maps to inv_list[0] = "Dirk". But looking at the body search code:
```c
if (j) { ... stuff[j]++; }
else if (!i) print_cont("nothing");
```
`j == 0` is treated as "nothing" in the treasure branch. So group 4 slots 2-7 = no treasure.

### `weapon_probs[]` — `fmain2.c:860-868`
```c
char weapon_probs[] = {
    0,0,0,0,        // arms 0: no weapons
    1,1,1,1,        // arms 1: dirks only
    1,2,1,2,        // arms 2: dirks and maces
    1,2,3,2,        // arms 3: mostly maces
    4,4,3,2,        // arms 4: swords and bows
    5,5,5,5,        // arms 5: magic wand
    8,8,8,8,        // arms 6: touch attack
    3,3,3,3,        // arms 7: swords only
};
```

### Monster → Damage Summary

| Monster | Weapon(s) | Melee Damage | Notes |
|---------|-----------|--------------|-------|
| Ogre | Dirk/Mace | 1-4 | Random between dirk and mace |
| Orcs | Bow/Sword/Mace | 3-5 (melee), 4-11 (arrow) | 50% bow, 25% sword, 25% mace |
| Wraith | Touch | 5-7 | |
| Skeleton | Dirk/Mace/Sword | 1-5 | Wide range |
| Snake | Touch | 5-7 | |
| Salamander | Dirk/Mace/Sword | 1-5 | |
| Spider | Touch | 5-7 | |
| DKnight | Sword | 3-5 | 40 HP, tough fight |
| Loraii | Touch | 5-7 | |
| Necromancer | Wand | 4-11 (fireball) | Ranged only |
| Woodcutter | None | 0-2 | Non-aggressive |
| Witch | Special | 1-2 | `fmain.c:2375` |

## 12. Bravery and Luck in Combat

### Bravery
- **Starting values**: Julian=35, Phillip=20, Kevin=15 — `fmain.c:2803-2805`
- **Increases**: +1 per enemy killed — `fmain.c:2777`
- **Combat effects**:
  1. **Melee hit reach**: `(brave/20) + 5`, max 15 — `fmain.c:2249`
  2. **Monster melee dodge**: `rand256() > brave` must pass for monster to hit — `fmain.c:2260`
  3. **Missile dodge** (slot 0 only): `bitrand(512) > brave` — `fmain.c:2289`
  4. **Starting vitality**: `15 + brave/4` on revive — `fmain.c:2897`

### Luck
- **Starting values**: Julian=20, Phillip=35, Kevin=20 — `fmain.c:2803-2805`
- **Decreases**: -5 per player death — `fmain.c:2777`; -2 per ledge fall — `fmain.c:1770`
- **Combat effect**: When `luck < 1`, player death triggers **brother succession** instead of fairy rescue — `fmain.c:1391`
- **Deaths before brother switch**: ~luck/5 (Julian: ~4, Phillip: ~7, Kevin: ~4)

## 13. `fallstates[]` — `fmain2.c:871-874`
```c
UBYTE fallstates[] = {
    0,0,0,0,0,0,
    0x20,0x22,0x3a,0x6f,0x70,0x71,
    0x24,0x27,0x3c,0x6f,0x70,0x71,
    0x37,0x38,0x3d,0x6f,0x70,0x71 };
```
Indexed by `(tactic/5) + (brother*6)`. Each brother has 6 fall animation frames (tactic runs 0→30). Row 0 (all zeros) is used for brother=0 (pre-game); rows 1-3 are Julian, Phillip, Kevin respectively.

## Cross-Cutting Findings

- **Bravery double-duty**: Bravery serves as both experience (passive growth from kills) and active combat stat (reach + dodge). This means combat gets progressively easier as the player fights, in a compounding feedback loop.
- **Touch attack inconsistency**: Touch attack damage (5-7) exceeds sword damage (3-5), making wraiths, snakes, spiders, and loraii more dangerous per-hit than sword-wielding enemies. This may be intentional (supernatural fear factor) or a balancing oversight.
- **Weapon as damage**: The weapon code IS the damage. There is no separate damage lookup — `vitality -= wt`. This means carry capability (weapon code) and damage are the same integer.
- **Race 9 overload**: Race 9 is used for both the Necromancer (ENEMY type) and the witch's setfig race (0x89 = 0x80 | 9). The `dohit()` immunity check distinguishes them by checking `race == 9` (enemy) vs `race == 0x89` (setfig).
- **mixflag randomness**: `mixflag = rand()` with bit-checked variations — `fmain.c:2059`. Bit 2 randomizes weapon within the arms group; bit 1 randomizes enemy race between adjacent pairs. Disabled for xtype > 49 or xtype divisible by 4.
- **Kindness decay in combat**: Killing any SETFIG (except the witch) decreases `kind` by 3 — `fmain.c:2776`. This is a hidden NPC-killing penalty.
- **Battle end detection**: `aftermath()` fires when `battleflag` transitions from TRUE to FALSE — `fmain.c:2192`. No explicit experience or loot is given by aftermath — rewards come from `checkdead()` (brave++) and manual body search.
- **prq(7) and prq(4)**: These queue HUD status bar redraws — `fmain2.c:474`. prq(7) likely updates bravery/combat bars, prq(4) likely updates vitality bar. Called in `checkdead()` after stat changes.

## Unresolved

1. **Missile slot 0 dodge bias** (`fmain.c:2289`): The condition `(i != 0 || bitrand(512)>bv)` means only missile slot 0 has a dodge check. Other missile slots (1-5) always hit if in range. Talin's own comment `/* really?? */` at `fmain.c:2286` suggests uncertainty about this logic. It's unclear whether `i` was intended to represent something other than the missile slot index.

2. **Race 0x8a/0x8b full immunity**: Spectre and Ghost setfigs are completely immune to all damage with no message. It's unclear if this is intentional game design (supernatural beings can't be killed) or if there's a quest mechanic to make them vulnerable that we haven't traced.

3. **Sound sample mapping**: The `sample[]` array and `sample_size[]` arrays are loaded from disk. The exact mapping of sample indices 0-5 to audio filenames needs verification from the asset loading code.

4. **diroffs interpretation for fight sprites**: The fight sprite system uses `diroffs[d+8]` for base offsets {56,56,68,68,32,32,44,44}, but the mapping of sprite bank layout (why these specific offsets?) requires analysis of the shape data format.

5. **Orc weapon wt randomization**: `wt = rand4()` is set globally for ALL enemies in a spawn batch — `fmain.c:2059`. Then per-enemy: `if (mixflag & 4) wt = rand4()` re-randomizes — `fmain.c:2756`. The variable `wt` is a **global** (`fmain.c:602`) that persists between spawns. When `mixflag & 4` is NOT set, all enemies in a batch get the same weapon slot index. This batch-weapon-sharing behavior may be intentional (uniform squads) or accidental.

## Refinement Log
- 2026-04-05: Initial comprehensive discovery pass covering dohit, aftermath, checkdead, nearest_fig, calc_dist, trans_list, weapon/treasure tables, death system, and stat effects.
