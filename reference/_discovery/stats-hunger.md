# Discovery: Hunger, Fatigue & Player Stats System

**Status**: complete
**Investigated**: 2026-04-05
**Requested by**: orchestrator
**Prompt summary**: Trace the complete stats system including hunger, fatigue, four character stats, vitality/HP, eating, sleeping, stat changes, brother succession, and HUD display.

## Player Stats

All six core stats are declared together as global `short` variables:

- `fmain.c:565` — `short brave, luck, kind, wealth, hunger, fatigue;`

These are part of a contiguous block of saved variables starting at `map_x` (`fmain.c:557`). The save system writes 80 bytes starting from `&map_x` (`fmain2.c:1516`), which covers `map_x`, `map_y`, `hero_x`, `hero_y`, `safe_x`, `safe_y`, `safe_r`, `img_x`, `img_y` (18 bytes), `cheat1`, `riding`, `flying`, `wcarry`, `turtleprox`, `raftprox` (12 bytes), `brave`, `luck`, `kind`, `wealth`, `hunger`, `fatigue` (12 bytes), `brother`, `princess`, `hero_sector`, `hero_place` (8 bytes), `daynight`, `lightlevel`, `actor_file`, `set_file`, `active_carrier`, `xtype`, `leader`, `secret_timer`, `light_timer`, `freeze_timer`, `cmode`, `encounter_type` (24 bytes) = ~74+ bytes — all saved/loaded in one block.

Vitality is NOT a global — it's a per-actor field in `struct shape`:
- `ftale.h:65` — `short vitality;` in `struct shape`
- Hero vitality is `anim_list[0].vitality`

Additional related variables:
- `fmain.c:592` — `unsigned char goodfairy;` — good fairy resurrection tracker
- `fmain.c:599` — `short sleepwait;` — counter for bed detection
- `fmain.c:571` — `USHORT daynight, lightlevel;` — time-of-day drives hunger/fatigue ticks

## Hunger System

### Increment
Hunger increments by 1 every 128 game ticks, in the main loop idle handler:

- `fmain.c:2199-2201` — `if ((daynight & 127) == 0 && anim_list[0].vitality && anim_list[0].state != SLEEP) { hunger++; fatigue++; ... }`

Condition: only ticks when `(daynight & 127) == 0`, hero is alive (`vitality > 0`), and hero is NOT sleeping (`state != SLEEP`).

### Thresholds and Effects

| Hunger Value | Effect | Citation |
|---|---|---|
| 35 | `event(0)` — "% was getting rather hungry." | `fmain.c:2203` |
| 60 | `event(1)` — "% was getting very hungry." | `fmain.c:2204` |
| 90 | `event(4)` — "% was getting sleepy." AND `event(2)` — "% was starving!" (when `hunger > 90` and `(hunger & 7)==0`) | `fmain.c:2219`, `fmain.c:2209` |
| >100 | HP damage: `anim_list[0].vitality -= 2` (every 8 hunger ticks, when `(hunger & 7)==0` and `vitality > 5`) | `fmain.c:2207-2208` |
| >120 | Movement wobble: direction randomly shifted ±1 with 75% probability (`!(rand4())`) | `fmain.c:1442-1445` |
| >140 | Forced sleep: `event(24)` — "% passed out from hunger!", hunger reset to 130, state set to SLEEP | `fmain.c:2213-2215` |

Note: The `event(4)` message at hunger==90 says "% was getting sleepy" — this is the FATIGUE message (event 4), triggered at this hunger threshold. Event 2 ("% was starving!") fires when `hunger > 90` on every 8th hunger tick.

### Auto-Eating
When the hero enters a safe zone (`(daynight & 127) == 0` and various safe conditions), if `hunger > 30` and `stuff[24] > 0` (has Fruit):
- `fmain.c:2195-2196` — `stuff[24]--; hunger -= 30; event(37);` — "% ate one of his apples."

### Wobble Detail
- `fmain.c:1442-1445` — When walking and `hunger > 120`, with probability 3/4 (`!(rand4())` is false when rand4 returns 0), the direction is randomly shifted +1 or -1 (50/50 coin flip via `rand()&1`).

## Fatigue System

### Increment
Fatigue increments by 1 every 128 game ticks, alongside hunger:
- `fmain.c:2202` — `fatigue++;` (same condition as hunger: `(daynight & 127) == 0`, alive, not sleeping)

### Thresholds and Effects

| Fatigue Value | Effect | Citation |
|---|---|---|
| 70 | `event(3)` — "% was getting tired." | `fmain.c:2218` |
| >160 | HP damage: `anim_list[0].vitality -= 2` (every 8 hunger ticks, when `(hunger & 7)==0` and `vitality > 5`) — this is the SAME check as `hunger > 100` | `fmain.c:2207-2208` |
| >170 | Forced sleep: `event(12)` — "% just couldn't stay awake any longer!", state = SLEEP (only if `vitality <= 5`) | `fmain.c:2211-2212` |

Note on fatigue > 160 damage: The code at `fmain.c:2206-2208` reads:
```c
if (anim_list[0].vitality > 5)
{   if (hunger > 100 || fatigue > 160)
    {   anim_list[0].vitality-=2; prq(4); }
```
So either hunger > 100 OR fatigue > 160 triggers the -2 HP damage, but only when vitality > 5.

The fatigue > 170 forced sleep path at `fmain.c:2211` is in the `else` branch — meaning it only fires when `vitality <= 5`.

### Sleep Mechanics

**Voluntary Sleep (beds):**
- `fmain.c:1876-1886` — Inside buildings (region 8), standing on specific terrain tiles (161, 52, 162, 53) increments `sleepwait`. After 30 ticks on a bed:
  - If `fatigue < 50`: `event(25)` — "% is not sleepy." (no sleep)
  - If `fatigue >= 50`: `event(26)` — "% was tired, so he decided to lie down and sleep.", hero y-position OR'd with 0x1f, state = SLEEP

**Sleep Processing:**
- `fmain.c:2013-2020` — While in SLEEP state:
  - `daynight += 63` — time advances rapidly (63 extra ticks per frame)
  - `fatigue--` (if > 0) — fatigue decrements by 1 per frame
  - Wake conditions (any of):
    1. `fatigue == 0` — fully rested
    2. `fatigue < 30 && daynight > 9000 && daynight < 10000` — light sleep ends around morning
    3. `battleflag && (rand64() == 0)` — combat can wake you (1/64 chance per tick)
  - On waking: `state = STILL`, hero y-position AND'd with 0xFFE0 (snap to grid)

**Forced Sleep:** Two conditions trigger forced sleep:
1. Extreme fatigue (`> 170`) when HP ≤ 5: `fmain.c:2211-2212`
2. Extreme hunger (`> 140`): `fmain.c:2213-2215` — also resets hunger to 130

## eat() Function

Defined at `fmain2.c:1704-1707`:
```c
eat(amt)
{   hunger -= amt;
    if (hunger < 0) { hunger = 0; event(13); }
    else print("Yum!");
}
```

- Decrements hunger by `amt`
- If hunger drops below 0, clamps to 0 and prints `event(13)` — "% was feeling quite full."
- Otherwise prints "Yum!"

### Food Sources

| Source | Amount | Citation |
|---|---|---|
| Apple (ground pickup, index 148) | `eat(30)` — reduces hunger by 30 | `fmain.c:3167` |
| Apple (ground pickup, when `hunger < 15`) | `stuff[24]++; event(36)` — stored for later instead of eating | `fmain.c:3166` |
| Buy food from shopkeeper | `eat(50)` — reduces hunger by 50, costs 3 gold | `fmain.c:3433` |
| Auto-eat Fruit in safe zone | hunger -= 30 (direct, not via eat()) | `fmain.c:2196` |

Buy prices from `jtrans[]` at `fmain2.c:850`:
```c
char jtrans[] = { 0,3, 8,10, 11,15, 1,30, 2,45, 3,75, 13,20 };
```
Format: pairs of (item_index, cost). First pair: item 0 = food, cost 3. The BUY handler at `fmain.c:3428-3434` decodes this.

`stuff[24]` corresponds to `inv_list[24]` = "Fruit" (`fmain.c:403`).

## Vitality / HP System

### Max HP Formula
Max HP = `15 + brave/4`

Citations:
- `fmain.c:2042` — natural healing cap: `if (anim_list[0].vitality < (15+brave/4) ...`
- `fmain.c:2901` — revive: `an->vitality = (15+brave/4);`
- `fmain.c:3350-3351` — heal vial cap: `if (anim_list[0].vitality > (15+brave/4)) anim_list[0].vitality = (15+brave/4);`
- `fmain.c:3391` — priest healing: `anim_list[0].vitality = (15+brave/4);`

### Natural Healing
- `fmain.c:2041-2044` — Every 1024 game ticks (`(daynight & 0x3ff) == 0`), if alive and not dead, hero gains +1 vitality up to max HP.

### Heal Vial (Magic item, case 7)
- `fmain.c:3349-3352` — `anim_list[0].vitality += rand8()+4;` then capped at `15+brave/4`. Heals 4-11 HP.
- Prints "That feels a lot better!" if not already at max.

### Priest Healing
- `fmain.c:3390-3391` — Talking to a priest (setfig race 1) when kindness >= 10 and no writ: `anim_list[0].vitality = (15+brave/4);` — full heal to max HP.

### Damage Sources
- Combat hits: `fmain2.c:236` — `anim_list[j].vitality -= wt;` clamped to 0
- Hunger/fatigue: `fmain.c:2208` — `-2 HP` when `hunger > 100 || fatigue > 160`
- Drowning (environ == 30): `fmain.c:1850` — `-1 HP` per 8 ticks
- Lava (fiery_death zone): `fmain.c:1845-1846` — `-1 HP` per tick when environ > 2; instant death when environ > 15
- Rose protection in lava: `fmain.c:1844` — `if (i==0 && stuff[23]) an->environ = 0;` — Rose (stuff[23]) prevents lava damage

## Stat Changes

### Bravery
- **Kill enemy**: `fmain.c:2777` — `brave++;` (when a non-hero dies via `checkdead` with `i != 0`)
- **Wand of Death kill**: `fmain.c:3359` — `brave--;` (per target killed by wand — note this DECREASES bravery)
- **Initial values (per brother)**:
  - Julian: 35 (`fmain.c:2810`)
  - Phillip: 20 (`fmain.c:2811`)
  - Kevin: 15 (`fmain.c:2812`)

### Luck
- **Hero death**: `fmain.c:2777` — `luck -= 5;` (when hero dies in `checkdead` with `i == 0`)
- **Fall into pit**: `fmain.c:1771` — `luck -= 2;`
- **Sorceress talk**: `fmain.c:3402` — `if (luck < rand64()) luck += 5;` (probabilistic increase, easier when luck is low)
- **Resurrection check**: `fmain.c:1391` — `if (luck < 1 && goodfairy < 200)` triggers brother succession (`revive(TRUE)`)
- **Clamp**: `fmain2.c:461` — `if (luck < 0) luck = 0;` (on HUD redraw)
- **Initial values**:
  - Julian: 20 (`fmain.c:2810`)
  - Phillip: 35 (`fmain.c:2811`)
  - Kevin: 20 (`fmain.c:2812`)

### Kindness
- **Kill SETFIG** (non-0x89 race): `fmain.c:2776` — `kind -= 3;` (killing friendly NPCs)
- **Clamp**: `fmain.c:2778` — `if (kind < 0) kind = 0;`
- **Give gold to NPC**: `fmain.c:3496` — `if (rand64() > kind) kind++;` (probabilistic increase, easier when kind is low)
- **Wizard/priest checks**: `fmain.c:3380,3388` — kind < 10 triggers dismissive dialogue
- **Initial values**:
  - Julian: 15 (`fmain.c:2810`)
  - Phillip: 15 (`fmain.c:2811`)
  - Kevin: 35 (`fmain.c:2812`)

### Wealth
- **Loot gold from chests**: `fmain.c:3158` — `wealth += 50;` (gold pieces object)
- **Loot gold from containers**: `fmain.c:3213` — `wealth += 100;` (100 gold pieces)
- **Loot treasure from corpses**: `fmain.c:3279` — `if (j >= GOLDBASE) wealth += inv_list[j].maxshown;` (gold items add their maxshown value: 2, 5, 10, or 100)
- **Buy items**: `fmain.c:3431` — `wealth -= j;` (j = cost from jtrans table)
- **Give gold**: `fmain.c:3495` — `wealth -= 2;`
- **Princess rescue**: `fmain2.c:1600` — `wealth += 100;`
- **Shop check**: `fmain.c:3538` — `j=8; if (wealth>2) j = 10;` — give-gold option only enabled when wealth > 2
- **Initial values**:
  - Julian: 20 (`fmain.c:2810`)
  - Phillip: 15 (`fmain.c:2811`)
  - Kevin: 10 (`fmain.c:2812`)

## Stat Transfer (Brother Succession)

The `blist[]` array at `fmain.c:2809-2812` defines initial stats per brother:
```c
struct bro {
    char    brave,luck,kind,wealth;
    UBYTE   *stuff;
} blist[] = 
{   { 35,20,15,20,julstuff },   /* julian's attributes */
    { 20,35,15,15,philstuff },  /* phillip's attributes */
    { 15,20,35,10,kevstuff } }; /* kevin's attributes */
```

`revive(new)` at `fmain.c:2814` handles succession. When `new == TRUE` (new brother):

1. `brother` incremented: `fmain.c:2847` — `brother++;`
2. Previous brother's position saved as an object: `fmain.c:2837-2840`
3. New brother's base stats loaded: `fmain.c:2845-2846` — `brave = br->brave; luck = br->luck; kind = br->kind; wealth = br->wealth; stuff = br->stuff;`
4. Inventory zeroed: `fmain.c:2849` — `for (i=0; i<GOLDBASE; i++) stuff[i] = 0;`
5. Given a dirk: `fmain.c:2850` — `stuff[0] = an->weapon = 1;`
6. Timers reset: `fmain.c:2852` — `secret_timer = light_timer = freeze_timer = 0;`
7. Placed at safe starting location: `fmain.c:2853` — `safe_x = 19036; safe_y = 15755; region_num = safe_r = 3;`

After the brother-specific initialization (whether new or not):
- `fmain.c:2901` — `an->vitality = (15+brave/4);` — full HP
- `fmain.c:2906` — `hunger = fatigue = 0;` — fully fed and rested
- `fmain.c:2903` — `daynight = 8000; lightlevel = 300;` — reset to daytime

If `brother > 3` (all brothers dead): `fmain.c:2869` — `quitflag = TRUE;` — game over.

Note: Previous brother's inventory (`julstuff`, `philstuff`) can be recovered by finding their bones: `fmain.c:3173-3176` — `for (k=0; k<GOLDBASE; k++) { if (x==1) stuff[k] += julstuff[k]; else stuff[k] += philstuff[k]; }`

## HUD Display

The status bar is rendered via the print queue system (`ppick()` at `fmain2.c:441`):

- **prq(7)**: Draws all four stats — `fmain2.c:461-465`:
  ```
  Brv: <brave>   at position (14, 52)
  Lck: <luck>    at position (90, 52)
  Knd: <kind>    at position (168, 52)
  Wlth: <wealth> at position (321, 52)
  ```
  Also clamps luck: `if (luck < 0) luck = 0;`

- **prq(4)**: Draws vitality — `fmain2.c:458`:
  ```
  Vit: <vitality> at position (245, 52)
  ```

Hunger and fatigue are NOT displayed on the HUD — they are only communicated through event messages.

## Death and Resurrection

When hero vitality reaches 0, `checkdead(0, dtype)` is called:
- `fmain.c:2770-2780` — Sets state to DYING, goal to DEATH
- `luck -= 5` on hero death
- `event(dtype)` fires the death message (dtype: 5=combat, 6=drowning, 27=lava, etc.)

The resurrection sequence in the main loop (`fmain.c:1388-1398`):
1. `goodfairy == 1`: `revive(FALSE)` — resurrect at last safe point (same brother)
2. `goodfairy` counts down; at < 20, pause for glow effect
3. `luck < 1 && goodfairy < 200`: `revive(TRUE)` — brother succession
4. `state == FALL && goodfairy < 200`: `revive(FALSE)` — fall recovery
5. `goodfairy < 120`: fairy sprite animation
6. Otherwise: goodfairy decrements and fairy approaches

So resurrection vs. succession depends on luck: if `luck >= 1`, the good fairy revives the same character. If `luck < 1`, the next brother takes over.

## Cross-Cutting Findings

- **Hunger affects movement direction**: `fmain.c:1442-1445` — hunger > 120 causes direction wobble in the walking handler, not in the hunger system itself.
- **Rose prevents lava environment damage**: `fmain.c:1844` — `stuff[23]` (Rose) checked in the environmental damage handler, not in an inventory system.
- **Brave affects combat hit range**: `fmain.c:2249` — `bv = (brave/20)+5` for hero melee, `fmain.c:2283` — `bv = brave` for hero missile. Bravery modulates combat effectiveness beyond just max HP.
- **Brave affects enemy dodge**: `fmain.c:2260` — `if ((i==0 || rand256()>brave) && yd < bv && !freeze_timer)` — enemies hit the hero if `rand256() > brave`, so higher brave makes hero harder to hit by enemies.
- **Kindness gates NPC dialogue**: `fmain.c:3380,3388` — wizard and priest give different (worse) dialogue when `kind < 10`.
- **Luck gates sorceress benefit**: `fmain.c:3402` — `if (luck < rand64()) luck += 5` — probabilistic, so lower luck = higher chance of gain (self-balancing).
- **Wealth gates shop interaction**: `fmain.c:3538` — give-gold option only enabled when `wealth > 2`.
- **Wand of Death decreases bravery**: `fmain.c:3359` — `brave--` per kill, creating a tradeoff for using it.

## Unresolved

- **Innkeeper dialogue at fmain.c:3406-3408**: The innkeeper (setfig race 8) checks `fatigue < 5` and `dayperiod > 7` for different speeches, but what the actual speeches say (speak(12), speak(13), speak(14)) would need narr.asm speech table tracing.
- **Exact timing of hunger/fatigue ticks**: `daynight` increments roughly once per main loop iteration (with `daynight++` at `fmain.c:2023`), cycling 0-23999. So `(daynight & 127) == 0` fires every 128 iterations. The real-world rate depends on the game's frame rate, which is platform-dependent.
- **Event(14) "rested" message**: `event(14)` — "% was feeling quite rested." is declared in narr.asm but no code path was found that calls `event(14)`. It may be unused.
- **Sleep healing**: During sleep, fatigue decrements but there is no explicit vitality recovery code in the sleep handler. However, the natural healing at `fmain.c:2041-2044` fires every 1024 ticks regardless of sleep state. Since sleep advances daynight by 63 per frame, healing would occur approximately 63x faster during sleep (every ~16 frames instead of every 1024).

## Refinement Log
- 2026-04-05: Initial comprehensive discovery pass. All 12 questions answered with citations.
