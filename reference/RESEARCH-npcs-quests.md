# Game Mechanics Research — NPCs, Quests & Stats

NPC dialogue/quests, hunger/fatigue/stats, brother succession, and win condition.

> **Citation format**: `file.c:LINE` or `file.c:START-END`. Speech references: `speak(N)`.
> Split from [RESEARCH.md](RESEARCH.md). See the hub document for the full section index.

---

## 13. NPC Dialogue & Quests

### 13.1 Setfig NPCs

Defined at `fmain.c:22-36`, the `setfig_table[]` maps 14 NPC types:

| Index | NPC | cfile | image_base | can_talk | Race |
|-------|-----|-------|------------|----------|------|
| 0 | Wizard | 13 | 0 | Yes | 0x80 |
| 1 | Priest | 13 | 4 | Yes | 0x81 |
| 2 | Guard | 14 | 0 | No | 0x82 |
| 3 | Guard (back) | 14 | 1 | No | 0x83 |
| 4 | Princess | 14 | 2 | No | 0x84 |
| 5 | King | 14 | 4 | Yes | 0x85 |
| 6 | Noble | 14 | 6 | No | 0x86 |
| 7 | Sorceress | 14 | 7 | No | 0x87 |
| 8 | Bartender | 15 | 0 | No | 0x88 |
| 9 | Witch | 16 | 0 | No | 0x89 |
| 10 | Spectre | 16 | 6 | No | 0x8a |
| 11 | Ghost | 16 | 7 | No | 0x8b |
| 12 | Ranger | 17 | 0 | Yes | 0x8c |
| 13 | Beggar | 17 | 4 | Yes | 0x8d |

`can_talk` controls only the TALKING state visual effect (15-tick timer) — `fmain.c:3376-3379`. While `tactic` counts down from 15 to 0, each render tick randomly selects between two adjacent sprite images (`dex += rand2()` — `fmain.c:1556`) producing a flickering jitter, not a true animation sequence. When the timer expires the NPC returns to STILL (`fmain.c:1557`). All 14 types have speech dispatch code regardless.

Race code: `id + 0x80` (OR'd with 0x80 in `set_objects` — `fmain2.c:1280`). Vitality: `2 + id*2` (`fmain2.c:1274`). The `goal` field stores the object list index — `fmain2.c:1275` — which selects variant dialogue for wizards, rangers, and beggars.

### 13.2 Talk System

Entry point: `do_option()` at `fmain.c:3367-3422`.

**Submenu** (`fmain.c:497`): `"Yell Say  Ask  "` — three options (hit values 5, 6, 7):

| Option | Range | Special Behavior |
|--------|-------|-----------------|
| Yell (hit=5) | `nearest_fig(1,100)` — 100 units | If target within 35 units: `speak(8)` "No need to shout!" — `fmain.c:3374` |
| Say (hit=6) | `nearest_fig(1,50)` — 50 units | Standard dispatch |
| Ask (hit=7) | `nearest_fig(1,50)` — 50 units | Functionally identical to Say |

All three share the same dispatch logic after the range check. There is no separate Ask handler.

#### TALKING Visual Effect

Before speech dispatch, if the target is a SETFIG with `can_talk=1`, the NPC enters the TALKING state for 15 ticks (`fmain.c:3376-3377`). Only 5 of 14 NPC types have `can_talk=1`: **Wizard**, **Priest**, **King**, **Ranger**, and **Beggar**. During each render tick, the sprite image randomly toggles between two adjacent frames (`dex += rand2()` — `fmain.c:1556`), producing a visual flicker. When the timer expires, the NPC returns to STILL (`fmain.c:1557`). This is purely cosmetic — all 14 types receive speech text regardless of `can_talk`.

#### Dispatch Logic (`fmain.c:3375-3422`)

1. If no target or target is DEAD → break
2. **SETFIG**: switch on `k = an->race & 0x7f` (setfig index)
3. **CARRIER** with `active_carrier == 5` (turtle): shell dialogue — `fmain.c:3418-3421`
4. **ENEMY**: `speak(an->race)` — enemy race directly indexes the speech table — `fmain.c:3422`

#### NPC-Specific Speech

**Wizard** (`fmain.c:3380-3381`):
- `kind < 10` → `speak(35)` — "Away with you, ruffian!"
- `kind >= 10` → `speak(27 + an->goal)` — goal selects from 8 different hints (speak 27–34)

**Priest** (`fmain.c:3382-3394`):
- Has writ (`stuff[28]`): if `ob_listg[10].ob_stat == 0` → `speak(39)`, gives gold statue, sets stat=1. Else → `speak(19)` — "already gave the statue"
- `kind < 10` → `speak(40)` — "Repent, Sinner!"
- `kind >= 10` → `speak(36 + daynight%3)` — rotating hints. Also heals player to max vitality (`15 + brave/4`) — `fmain.c:3390-3391`

**Bartender** (`fmain.c:3405-3407`):
- `fatigue < 5` → `speak(13)` — "Good Morning"
- `fatigue >= 5 && dayperiod > 7` → `speak(12)` — "Would you like to buy something?"
- Else → `speak(14)` — "Have a drink!"

**Other NPCs**: Guard → `speak(15)`, Princess → `speak(16)` (gated by `ob_list8[9].ob_stat`), King → `speak(17)` (same gate), Noble → `speak(20)`, Sorceress → `speak(45)` (first visit gives statue, luck boost on returns — `fmain.c:3400-3402`), Witch → `speak(46)`, Spectre → `speak(47)`, Ghost → `speak(49)`, Ranger → region-based (`speak(22)` in region 2, else `speak(53+goal)` — `fmain.c:3411-3413`), Beggar → `speak(23)` — `fmain.c:3414`.

#### Proximity Auto-Speech (`fmain.c:2094-2102`)

Certain NPCs speak automatically when near the player, independent of the Talk menu:
- Beggar (0x8d) → `speak(23)`
- Witch (0x89) → `speak(46)`
- Princess (0x84) → `speak(16)` (only if `ob_list8[9].ob_stat` set)
- Necromancer (race 9) → `speak(43)`
- DreamKnight (race 7) → `speak(41)`

Tracked by `last_person` to prevent re-triggering for the same NPC.

### 13.3 Give System

Entry point: `do_option()` at `fmain.c:3490-3508`.

Menu: `"Gold Book Writ Bone "` (`fmain.c:506`).

| Action | Condition | Effect | Citation |
|--------|-----------|--------|----------|
| Gold (hit=5) | `wealth > 2` | `wealth -= 2`. If `rand64() > kind` → `kind++`. Beggar: `speak(24+goal)`. Others: `speak(50)`. | `fmain.c:3493-3500` |
| Book (hit=6) | ALWAYS DISABLED | Hardcoded disabled in `set_options` — `fmain.c:3540` | `fmain.c:3540` |
| Writ (hit=7) | `stuff[28] != 0` | **No handler code** — menu enabled but no processing. Writ is checked passively during Priest Talk. | — |
| Bone (hit=8) | `stuff[29] != 0` | Non-spectre: `speak(21)`. Spectre (0x8a): `speak(48)`, `stuff[29]=0`, drops crystal shard (140). | `fmain.c:3501-3503` |

### 13.4 Quest Progression Flags

Quest state is tracked via object list `ob_stat` fields, not dedicated flag variables:

| Flag | Meaning | Set | Cleared |
|------|---------|-----|---------|
| `ob_list8[9].ob_stat` | Princess captive | `revive(TRUE)` → 3 (`fmain.c:2843`) | `rescue()` → 0 (`fmain2.c:1601`) |
| `ob_listg[9].ob_stat` | Sorceress statue given | First talk → 1 (`fmain.c:3403`) | — |
| `ob_listg[10].ob_stat` | Priest statue given | Writ presented → 1 (`fmain.c:3384-3385`) | — |
| `ob_listg[5].ob_stat` | Spectre visible | `lightlevel < 40` → 3, else → 2 (`fmain.c:2027-2028`) | — |
| `ob_listg[1-2].ob_stat` | Dead brother bones | Brother death → 1 (`fmain.c:2839`) | Bones picked up → 0 (implicit) |
| `ob_listg[3-4].ob_stat` | Ghost brothers | Brother death → 3 (`fmain.c:2841`) | Bones picked up → 0 (`fmain.c:3174`) |

#### Gold Statue Sources (5 needed for desert gate)

| # | Source | Mechanism |
|---|--------|-----------|
| 1 | Sorceress (first talk) | `ob_listg[9].ob_stat` → 1 — `fmain.c:3400-3401` |
| 2 | Priest (talk with writ) | `ob_listg[10].ob_stat` → 1 — `fmain.c:3384-3385` |
| 3 | Seahold (ob_listg[6]) | Ground pickup at (11092, 38526) |
| 4 | Ogre Den (ob_listg[7]) | Ground pickup at (25737, 10662) |
| 5 | Octal Room (ob_listg[8]) | Ground pickup at (2910, 39023) |

**Note**: The sorceress and priest talk handlers set `ob_listg[].ob_stat = 1` but no explicit `stuff[25]++` is visible in the talk code. The three ground statues are picked up through normal `itrans` object pickup. Whether the dialogue-given statues increment `stuff[25]` through an untraced mechanism requires further investigation; see [P21](PROBLEMS.md).

#### Quest State Gates

| Gate | Condition | Citation |
|------|-----------|----------|
| Desert/Azal entrance | `stuff[STATBASE] < 5` blocks DESERT doors | `fmain.c:1919` |
| Azal city map | `stuff[25] < 5` → tiles overwritten to 254 | `fmain.c:3594-3596` |
| King's castle pax | `xtype == 81` → `event(15)` — weapon sheathed | `fmain.c:1413` |
| Sorceress pax | `xtype == 82` → `event(16)` — calming influence | `fmain.c:1414` |
| Witch invulnerability | `weapon < 4 && (race==9 \|\| (race==0x89 && stuff[7]==0))` | `fmain2.c:231-233` |
| Necromancer invulnerability | `weapon < 4` blocks damage to race 9 | `fmain2.c:231-232` |
| Spectre/Ghost immunity | Absolute — `dohit()` returns early for 0x8a/0x8b | `fmain2.c:234` |
| Magic blocked in necro arena | `extn->v3 == 9` → `speak(59)` | `fmain.c:3305` |
| Crystal shard passwall | `stuff[30] && j==12` → bypass terrain 12 | `fmain.c:1609` |
| Rose lava protection | `stuff[23]` → `environ = 0` | `fmain.c:1844` |
| Golden lasso + bird | `stuff[5]` needed to ride bird carrier | `fmain.c:1498` |

### 13.5 `rescue()` — Princess Rescue

Defined at `fmain2.c:1584-1604`. Triggered when the player enters the princess extent (`xtype == 83`) and `ob_list8[9].ob_stat` is set — `fmain.c:2684-2685`.

Sequence:
1. Display rescue narrative: `placard_text(8 + princess*3)` through three princess-specific texts — `fmain2.c:1588-1591`
2. `princess++` — increment counter — `fmain2.c:1594`
3. Teleport to king's castle: `xfer(5511, 33780, 0)` — `fmain2.c:1596`
4. `ob_list8[2].ob_id = 4` — place a princess NPC in the castle — `fmain2.c:1597`
5. `stuff[28] = 1` — give writ — `fmain2.c:1598`
6. `speak(18)` — king gives writ speech — `fmain2.c:1599`
7. `wealth += 100` — gold reward — `fmain2.c:1600`
8. `ob_list8[9].ob_stat = 0` — clear princess captive flag — `fmain2.c:1601`
9. `stuff[16..21] += 3` — give 3 of each key type — `fmain2.c:1602`

Three princesses: Katra (princess=0), Karla (princess=1), Kandy (princess=2). The `princess` counter persists across brother succession.

### 13.6 Shop System

**Bartender identification**: setfig index 8, race `0x88`. BUY menu only works with race `0x88` — `fmain.c:3426`.

**`jtrans[]`** (`fmain2.c:850`) defines 7 purchasable items as `{stuff_index, price}` pairs:

| Item | Price | Effect | Citation |
|------|-------|--------|----------|
| Food | 3 | `eat(50)` — reduces hunger by 50 | `fmain.c:3433-3434` |
| Arrows | 10 | `stuff[8] += 10` | `fmain.c:3435` |
| Vial | 15 | `stuff[11]++` | `fmain.c:3436` |
| Mace | 30 | `stuff[1]++` | `fmain.c:3436` |
| Sword | 45 | `stuff[2]++` | `fmain.c:3436` |
| Bow | 75 | `stuff[3]++` | `fmain.c:3436` |
| Totem | 20 | `stuff[13]++` | `fmain.c:3436` |

Menu: `"Food ArrowVial Mace SwordBow  Totem"` (`fmain.c:501`). Purchase requires `wealth > price` — `fmain.c:3430`.

### 13.7 Win Condition

1. Kill Necromancer (race 9) — requires bow or wand (`weapon >= 4`)
2. On necromancer death: transforms to race 10 (woodcutter), talisman (139) dropped — `fmain.c:1751-1756`
3. Pick up talisman → `stuff[22]` → `quitflag = TRUE; viewstatus = 2` → `win_colors()` — `fmain.c:3244-3247`
4. Win sequence: `placard_text(6)`, `name()`, `placard_text(7)` + win picture — `fmain2.c:1605-1607`

### 13.8 Message Tables

#### Event Messages (`narr.asm:11-74`)

Called via `event(n)` — `fmain2.c:554-558`. Key entries:

| Index | Text | Usage |
|-------|------|-------|
| 0–2 | Hunger warnings ("rather hungry", "very hungry", "starving!") | `fmain.c:2203-2209` |
| 3–4 | Fatigue warnings ("getting tired", "getting sleepy") | `fmain.c:2218-2219` |
| 5–7 | Death messages (combat, drowning, lava) | `checkdead()` dtype |
| 12 | "couldn't stay awake any longer!" | Forced sleep — `fmain.c:2211` |
| 15–16 | Pax zone messages | `fmain.c:1413-1414` |
| 17–19 | Paper pickup and regionally-variant text | `fmain.c:3163-3168` |
| 22–23 | Shop purchases | `fmain.c:3433-3434` |
| 24 | "passed out from hunger!" | `fmain.c:2213` |
| 27 | "perished in the hot lava!" | `fmain.c:1847` |
| 28–31 | Time-of-day announcements | `fmain.c:2033` |
| 36–37 | Fruit pickup/auto-eat | `fmain.c:3166`, `fmain.c:2196` |

#### Speech Table (`narr.asm:347+`)

0-indexed, null-terminated strings. Entries 0–7 are enemy talk responses (directly indexed by race). Entries 8–60 cover NPC dialogue, sub-indexed by NPC type and conditional state. See [§13.2](#132-talk-system) for the full dispatch logic.

#### Placard Texts (`narr.asm:230-343`)

Formatted narrative screens with XY positioning. Entries 0–5 cover brother succession/game-over, 6–7 the win sequence, 8–18 princess rescues (3 sets of 3 texts + 2 shared post-rescue texts), and 19 the copy protection prompt.

---


## 14. Hunger, Fatigue & Stats

### 14.1 Player Stats Overview

Six core stats are declared as global `short` variables at `fmain.c:565`:

```c
short brave, luck, kind, wealth, hunger, fatigue;
```

These are part of a contiguous saved block starting at `map_x` (`fmain.c:557`), serialized as 80 bytes via `saveload((void *)&map_x, 80)` (`fmain2.c:1516`).

> **Compiler caveat:** The source code assumes these variables are laid out contiguously in memory in declaration order. Disassembly of the `fmain` executable in this repository shows the Aztec C compiler scattered them across BSS — `map_y` is 2 bytes *below* `map_x`, `hero_x`/`hero_y` are 154+ bytes *above*, and `cheat1` is 474 bytes below. The 80-byte `saveload` call therefore captures `map_x` plus 78 bytes of unrelated variables. Save files from the original shipped release (tested with `game/A.faery` created from a release disk image) are correct, indicating the release was built with a compiler version that preserved declaration order. The `fmain` in this repository appears to have been compiled later with an optimizing toolchain that reorders globals. See [PROBLEMS.md §P21](PROBLEMS.md) for the full analysis.

Vitality is per-actor (`struct shape.vitality` at `ftale.h:65`), NOT a global. The hero's vitality is `anim_list[0].vitality`.

### 14.2 Hunger System

#### Increment

Hunger increases by 1 every 128 game ticks — `fmain.c:2199-2201`:

```c
if ((daynight & 127) == 0 && anim_list[0].vitality &&
        anim_list[0].state != SLEEP)
{   hunger++;
```

Conditions: `(daynight & 127) == 0`, hero alive, not sleeping.

#### Thresholds

| Hunger | Effect | Citation |
|--------|--------|----------|
| 35 | `event(0)` — "getting rather hungry" | `fmain.c:2203` |
| 60 | `event(1)` — "getting very hungry" | `fmain.c:2204` |
| >90 | `event(2)` — "starving!" (every 8th tick when `(hunger & 7)==0`) | `fmain.c:2209` |
| >100 | HP damage: `vitality -= 2` (every 8th tick, only when `vitality > 5`) | `fmain.c:2207-2208` |
| >120 | Movement wobble: direction shifted ±1 with 75% probability | `fmain.c:1442-1445` |
| >140 | Forced sleep: `event(24)` "passed out!", hunger reset to 130, `state = SLEEP` | `fmain.c:2213-2215` |

#### Auto-Eating

When `(daynight & 127) == 0` in a safe zone, if `hunger > 30` and `stuff[24] > 0` (has Fruit): `stuff[24]--; hunger -= 30; event(37)` — `fmain.c:2195-2196`.

### 14.3 Fatigue System

#### Increment

Fatigue increases by 1 alongside hunger, same conditions — `fmain.c:2202`.

#### Thresholds

| Fatigue | Effect | Citation |
|---------|--------|----------|
| 70 | `event(3)` — "getting tired" | `fmain.c:2218` |
| 90 | `event(4)` — "getting sleepy" | `fmain.c:2219` |
| >160 | HP damage: `vitality -= 2` (same check as hunger >100) | `fmain.c:2207-2208` |
| >170 | Forced sleep: `event(12)` (only when `vitality <= 5`) | `fmain.c:2211-2212` |

The HP damage at `fmain.c:2206-2208` fires when **either** `hunger > 100` OR `fatigue > 160`, each 8th hunger tick:

```c
if (anim_list[0].vitality > 5)
{   if (hunger > 100 || fatigue > 160)
    {   anim_list[0].vitality -= 2; prq(4); }
```

The forced-sleep at `fatigue > 170` is in the `else` branch — only fires when `vitality <= 5`.

### 14.4 Sleep Mechanics

**Voluntary sleep** (`fmain.c:1876-1886`): Inside buildings (region 8), standing on bed terrain tiles (161, 52, 162, 53) increments `sleepwait`. After 30 ticks:
- `fatigue < 50` → `event(25)` "not sleepy" — no sleep
- `fatigue >= 50` → `event(26)` "decided to lie down and sleep", `state = SLEEP`

**Sleep processing** (`fmain.c:2013-2020`): While sleeping:
- `daynight += 63` — time advances rapidly
- `fatigue--` (if > 0) — fatigue decrements per frame
- Wake conditions: `fatigue == 0`, or `fatigue < 30 && daynight ∈ [9000,10000)` (morning), or combat with 1/64 chance per tick
- On waking: `state = STILL`, Y-position snapped to grid

### 14.5 `eat()` Function

Defined at `fmain2.c:1704-1707`:

```c
eat(amt)
{   hunger -= amt;
    if (hunger < 0) { hunger = 0; event(13); }
    else print("Yum!");
}
```

| Food Source | Amount | Citation |
|-------------|--------|----------|
| Pickup fruit (hungry) | `eat(30)` | `fmain.c:3167` |
| Buy food from shop | `eat(50)` | `fmain.c:3433` |
| Auto-eat fruit in safe zone | `hunger -= 30` (direct, not via `eat()`) | `fmain.c:2196` |

When hunger < 15, picked-up fruit is stored instead: `stuff[24]++; event(36)` — `fmain.c:3166`.

### 14.6 Vitality / HP

**Max HP formula**: `15 + brave/4`

Used at:
- Natural healing cap — `fmain.c:2042`
- Revive — `fmain.c:2901`
- Heal vial cap — `fmain.c:3350-3351`
- Priest healing — `fmain.c:3391`

**Natural healing**: Every 1024 ticks (`(daynight & 0x3ff) == 0`), hero gains +1 vitality up to max — `fmain.c:2041-2044`. During sleep, `daynight` advances by 63 per frame, so healing occurs approximately 63× faster.

**Heal vial**: `vitality += rand8() + 4` (4–11 HP), capped at max — `fmain.c:3349-3352`.

**Priest healing**: Full heal to `15 + brave/4` — `fmain.c:3390-3391`. Requires `kind >= 10`.

**Damage sources**:
- Combat hits: `vitality -= wt` — `fmain2.c:236`
- Hunger/fatigue: `-2 HP` per tick — `fmain.c:2208`
- Drowning (`environ == 30`): `-1 HP` per 8 ticks — `fmain.c:1850`
- Lava (`environ > 2`): `-1 HP` per tick; instant death at `environ > 15` — `fmain.c:1845-1846`
- Rose (`stuff[23]`) prevents lava damage by forcing `environ = 0` — `fmain.c:1844`

### 14.7 Stat Changes

#### Bravery

| Change | Condition | Citation |
|--------|-----------|----------|
| +1 | Kill any non-hero actor | `fmain.c:2777` |
| −1 | Per target killed by Jade Skull (Wand of Death) | `fmain.c:3359` |

Initial values: Julian=35, Phillip=20, Kevin=15 — `fmain.c:2810-2812`.

Bravery also affects combat: hero melee hit range = `(brave/20) + 5` (`fmain.c:2249`), hero missile bravery = full `brave` value (`fmain.c:2283`), and enemies must roll `rand256() > brave` to hit the hero (`fmain.c:2260`).

#### Luck

| Change | Condition | Citation |
|--------|-----------|----------|
| −5 | Hero death | `fmain.c:2777` |
| −2 | Fall into pit | `fmain.c:1771` |
| +5 (probabilistic) | Sorceress talk: `if (luck < rand64()) luck += 5` | `fmain.c:3402` |
| Clamped ≥ 0 | On HUD redraw | `fmain2.c:461` |

Initial values: Julian=20, Phillip=35, Kevin=20. Luck < 1 after death triggers brother succession instead of fairy rescue — `fmain.c:1391`.

#### Kindness

| Change | Condition | Citation |
|--------|-----------|----------|
| −3 | Kill non-witch setfig | `fmain.c:2776` |
| +1 (probabilistic) | Give gold: `if (rand64() > kind) kind++` | `fmain.c:3496` |
| Clamped ≥ 0 | In `checkdead` | `fmain.c:2778` |

Initial values: Julian=15, Phillip=15, Kevin=35. Below 10, wizards and priests give dismissive dialogue — `fmain.c:3380`, `fmain.c:3388`.

#### Wealth

| Change | Condition | Citation |
|--------|-----------|----------|
| +50 | Loot gold bag (MONEY) | `fmain.c:3158` |
| +100 | Container gold | `fmain.c:3213` |
| +100 | Princess rescue reward | `fmain2.c:1600` |
| variable | Corpse loot (`inv_list[j].maxshown` for gold items) | `fmain.c:3279` |
| −price | Buy item from shop | `fmain.c:3431` |
| −2 | Give gold to NPC | `fmain.c:3495` |

Initial values: Julian=20, Phillip=15, Kevin=10.

### 14.8 HUD Display

Rendered via the print queue (`ppick()` at `fmain2.c:441`):

- **prq(7)**: `Brv`, `Lck`, `Knd`, `Wlth` — `fmain2.c:461-465`
- **prq(4)**: `Vit` — `fmain2.c:458`

Hunger and fatigue are **not** displayed on the HUD — communicated only through event messages.

---


## 15. Brother Succession

### 15.1 `checkdead` — Death Detection

Defined at `fmain.c:2769-2781`. Triggers when `an->vitality < 1` and actor is not already DYING/DEAD:

```c
an->vitality = 0; an->tactic = 7;
an->goal = DEATH; an->state = DYING;
```

Hero-specific effects (`i == 0`): `event(dtype)` (death message), `luck -= 5`, `setmood(TRUE)` — `fmain.c:2777`.

NPC effects (`i != 0`): if SETFIG and not witch (0x89) → `kind -= 3`; if DreamKnight (race 7) → `speak(42)`. Always `brave++` — `fmain.c:2775-2777`.

Death types: 5=combat, 6=drowning, 27=lava. Transition DYING→DEAD occurs when `tactic` counts to 0 — `fmain.c:1747-1748`.

### 15.2 Fairy Rescue Mechanism

Defined at `fmain.c:1388-1403`. Activates when hero's state is DEAD or FALL. Uses `goodfairy` counter (`unsigned char`, starts at 0, declared `fmain.c:592`).

Timeline after hero enters DEAD/FALL state:

| `goodfairy` Range | Behavior |
|-------------------|----------|
| 255–200 (frames 2–57) | **Death sequence plays**. No code branches match — the death animation and death song always play fully before any rescue decision. |
| 199–120 (frames 58–137) | **Luck gate**: `luck < 1` → `revive(TRUE)` (brother succession). FALL state → `revive(FALSE)` (non-lethal recovery). If `luck >= 1`: no visible effect, countdown continues. Since luck cannot change during DEAD state, this gate is effectively a one-time decision at the moment goodfairy first drops below 200. |
| 119–20 (frames 138–237) | Fairy sprite visible, flying toward hero. Only reached if `luck >= 1`. `battleflag = FALSE`. AI suspended (`fmain.c:2112`). |
| 19–2 (frames 238–255) | Resurrection glow effect. |
| 1 (frame 256) | `revive(FALSE)` — fairy rescues hero, same brother continues. |

**Key insight**: `checkdead()` subtracts `luck -= 5` on hero death. The luck gate at `goodfairy < 200` is deliberately positioned after the death animation completes (255→200), so the player always sees the full death sequence before the outcome is determined. Luck cannot change during DEAD state — `checkdead` is guarded by `state != DYING && state != DEAD` (`fmain.c:2772`), pit fall luck loss requires movement (`fmain.c:1771`), and sorceress luck gain requires TALK interaction (`fmain.c:3402`). So if luck ≥ 1 when the gate first fires, the fairy is guaranteed. The system is fully deterministic with no random element. FALL state always gets `revive(FALSE)` regardless of luck (pit falls are non-lethal) — `fmain.c:1392`.

### 15.3 `blist` — Brother Base Stats

Defined at `fmain.c:2806-2812`:

| Brother | brave | luck | kind | wealth | stuff | Starting HP (`15+brave/4`) |
|---------|-------|------|------|--------|-------|---------------------------|
| Julian (`blist[0]`) | 35 | 20 | 15 | 20 | `julstuff` | 23 |
| Phillip (`blist[1]`) | 20 | 35 | 15 | 15 | `philstuff` | 20 |
| Kevin (`blist[2]`) | 15 | 20 | 35 | 10 | `kevstuff` | 18 |

Each brother has an independent 35-byte inventory array (`ARROWBASE=35`): `julstuff`, `philstuff`, `kevstuff` — `fmain.c:432`.

Design: Julian is the strongest fighter (highest bravery/HP), Phillip has the most fairy rescues available (highest luck — 6 rescues vs. 3 for the others), Kevin is the diplomat (highest kindness, but weakest combatant).

### 15.4 `revive()` — Resurrection and Succession

Defined at `fmain.c:2814-2911`. `revive(new)` where `new=TRUE` means brother succession, `new=FALSE` means fairy rescue/fall recovery.

#### Common Setup (both paths)

- `anim_list[1]` placed as RAFT, `anim_list[2]` as SETFIG (reset carriers)
- `handler_data.laydown = handler_data.pickup = 0`
- `battleflag = goodfairy = mdex = 0`

#### New Brother Path (`new == TRUE`)

1. **Place dead brother ghost** (`fmain.c:2837-2841`): Only for brothers 1 (Julian) and 2 (Phillip) — Kevin has no successor.
   - `ob_listg[brother].xc/yc = hero_x/hero_y; ob_stat = 1` — bones at death location
   - `ob_listg[brother+2].ob_stat = 3` — ghost setfig activated

2. **Load new brother stats** (`fmain.c:2843-2847`):
   - `ob_list8[9].ob_stat = 3` — re-enable princess as captive
   - Load stats from `blist[brother]`: `brave, luck, kind, wealth`
   - `stuff` pointer switches to new brother's inventory array
   - `brother++`

3. **Clear inventory** (`fmain.c:2849-2850`): Zero first 31 slots (`GOLDBASE`). Give one Dirk: `stuff[0] = an->weapon = 1`.

4. **Reset position/timers** (`fmain.c:2852-2853`): `secret_timer = light_timer = freeze_timer = 0`. Spawn at `(19036, 15755)` in region 3 (Tambry area).

5. **Display placard** — brother-specific narrative text:
   - Brother 1 (Julian): `placard_text(0)` — "Rescue the Talisman!"
   - Brother 2 (Phillip): `placard_text(1)` + `placard_text(2)` — Julian failed / Phillip sets out
   - Brother 3 (Kevin): `placard_text(3)` + `placard_text(4)` — Phillip failed / Kevin takes quest

6. **Load sprites**: `shape_read()` → `read_shapes(brother-1)` for correct character sprite — `fmain.c:2882`.

7. **Journey message** (`fmain.c:2885-2892`): `event(9)` — "started the journey in Tambry", with brother-specific suffix (`event(10)` or `event(11)` for subsequent brothers).

#### Fairy Rescue Path (`new == FALSE`)

Skips ghost placement, stat/inventory reset, and placard text. Hero respawns at current `safe_x, safe_y` with current stats. Only vitality, hunger, and fatigue are reset.

#### Common Finalization (both paths) (`fmain.c:2896-2907`)

- Position: `hero_x = safe_x, hero_y = safe_y`
- Vitality: `15 + brave/4` (full HP)
- Time: `daynight = 8000, lightlevel = 300` (daytime)
- `hunger = fatigue = 0`
- `an->state = STILL; an->race = -1`

### 15.5 `mod1save` — Inventory Serialization

Defined at `fmain.c:3621-3630`. Serializes all three brothers' inventory arrays (35 bytes each) and the missile list. After loading, `stuff = blist[brother-1].stuff` reassigns the active inventory pointer.

### 15.6 Picking Up Dead Brother's Bones

When a living brother picks up bones (ob_id 28) — `fmain.c:3173-3177`:

- Both ghost setfigs cleared: `ob_listg[3].ob_stat = ob_listg[4].ob_stat = 0`
- Dead brother's inventory merged: `for (k=0; k<GOLDBASE; k++) stuff[k] += dead_brother_stuff[k]`
- Index 1 = Julian's stuff, index 2 = Phillip's stuff

### 15.7 Game Over

When `brother > 3` (all three dead) — `fmain.c:2870-2872`:
- `placard_text(5)`: "And so ends our sad tale. The Lesson of the Story: Stay at Home!"
- `quitflag = TRUE`
- `Delay(500)` — 10-second pause (500 ticks at 50Hz)

### 15.8 What Persists Across Brothers

| Persists | Resets |
|----------|--------|
| Princess counter (`princess`) | Stats (loaded fresh from `blist`) |
| Quest flags (`ob_listg`, `ob_list8` stats) | Inventory (zeroed; only a Dirk given) |
| Object world state (all `ob_list` data) | Position (back to Tambry 19036, 15755) |
| `dstobs[]` distribution flags | Hunger / fatigue (→ 0) |
| | Timers (secret, light, freeze → 0) |
| | `daynight` (→ 8000), `lightlevel` (→ 300) |

---


## 16. Win Condition & Princess Rescue

### 16.1 Princess Rescue Trigger

Each brother's journey can include rescuing a captured princess. The trigger is extent-based — `fmain.c:2684-2685`:

```c
if (xtype == 83 && ob_list8[9].ob_stat)
{   rescue(); flag = 0; goto findagain; }
```

The princess extent (index 6) covers coordinates 10820–10877, 35646–35670 — `fmain.c:346`. The flag `ob_list8[9].ob_stat` must be nonzero (set to 3 by `revive()` at `fmain.c:2843`). A cheat shortcut exists: `'R' && cheat1` at `fmain.c:1333`.

### 16.2 `rescue()` Function

Defined at `fmain2.c:1584-1603`. Performs the full princess rescue sequence:

1. `map_message()` + `SetFont(rp,afont)` — enter fullscreen text mode with Amber font.
2. Compute text offset `i = princess*3` — indexes princess-specific placard text (`fmain2.c:1587`).
3. Display rescue story: `placard_text(8+i)`, `name()`, `placard_text(9+i)`, `name()`, `placard_text(10+i)`, then `placard()` + `Delay(380)` (~7.6 sec).
4. Clear inner rectangle, display post-rescue text (`placard_text(17)` + `name()` + `placard_text(18)`), `Delay(380)`.
5. `message_off()` — restore normal display.
6. `princess++` — advance counter (`fmain2.c:1594`).
7. `xfer(5511,33780,0)` — teleport hero near king's castle (`fmain2.c:1595`).
8. `move_extent(0,22205,21231)` — reposition bird extent (`fmain2.c:1596`).
9. `ob_list8[2].ob_id = 4` — place rescued princess NPC in castle (`fmain2.c:1597`).
10. `stuff[28] = 1` — give Writ item (`fmain2.c:1598`).
11. `speak(18)` — king says "Here is a writ designating you as my official agent…"
12. `wealth += 100` — reward 100 gold (`fmain2.c:1600`).
13. `ob_list8[9].ob_stat = 0` — clear princess-captured flag (`fmain2.c:1601`).
14. `for (i=16; i<22; i++) stuff[i] += 3` — give +3 of each key type (`fmain2.c:1602`).

### 16.3 Princess Counter

Declared at `fmain.c:568` as `short princess`. There are exactly three princesses:

| `princess` | Name | Placard Text Indices (8+i×3) |
|------------|------|------------------------------|
| 0 | Katra | 8, 9, 10 — `narr.asm:298-305` |
| 1 | Karla | 11, 12, 13 — `narr.asm:307-314` |
| 2 | Kandy | 14, 15, 16 — `narr.asm:316-322` |

The counter persists across brother succession — `revive()` at `fmain.c:2830-2850` does NOT reset `princess`. However, `ob_list8[9].ob_stat` IS reset to 3 during `revive()` (`fmain.c:2843`), enabling each new brother to trigger a rescue with different text. After `princess >= 3`, no further rescues can fire because `ob_list8[9].ob_stat` stays 0 after the third `rescue()` call.

### 16.4 Necromancer and the Talisman

The Necromancer is the final boss — race 9, 50 HP, arms 5 (wand) — `fmain.c:62`. Its extent is at coordinates 9563–10144, 33883–34462 — `fmain.c:343`.

On death (`fmain.c:1750-1755`):
- Transforms to Woodcutter: `an->race = 10`, `an->vitality = 10`, `an->state = STILL`, `an->weapon = 0`.
- Drops the Talisman: `leave_item(i, 139)`.

World object 139 maps to `stuff[22]` via the `itrans` lookup — `fmain2.c:983`.

Proximity speech: `speak(43)` when near — `fmain.c:2099-2100`.

### 16.5 Win Check and Victory Sequence

After every item pickup, the win condition fires — `fmain.c:3244-3247`:

```c
if (stuff[22])
{   quitflag = TRUE; viewstatus = 2;
    map_message(); SetFont(rp,afont); win_colors();
}
```

The `win_colors()` function at `fmain2.c:1605-1636` plays the victory sequence:

1. Display victory placard: `placard_text(6)` + `name()` + `placard_text(7)` — "Having defeated the villanous Necromancer and recovered the Talisman…" (`narr.asm:288-296`). `placard()` + `Delay(80)`.
2. Load win picture: `unpackbrush("winpic", bm_draw, 0, 0)` — IFF image from `game/winpic`.
3. Black out both viewports and hide HUD: `vp_text.Modes = HIRES | SPRITES | VP_HIDE`.
4. Expand playfield: `screen_size(156)`.
5. Sunrise animation — 55 frames (i=25 down to −29): slides a window across the `sun_colors[]` gradient table (`fmain2.c:1569-1578`, 53 entries of 12-bit RGB values). Colors 2–27 fade in progressively, colors 29–30 transition through reds. First frame pauses 60 ticks; subsequent frames at 9 ticks each. Total: ~555 ticks ≈ 11.1 seconds.
6. Final pause `Delay(30)`, then blackout via `LoadRGB4(&vp_page, blackcolors, 32)`.

### 16.6 `quitflag` and Game Termination

Declared at `fmain.c:590`. Controls the main loop `while (!quitflag)` at `fmain.c:1270`.

| Trigger | Citation | Meaning |
|---------|----------|---------|
| `quitflag = FALSE` | `fmain.c:1269` | Reset at game start |
| `quitflag = TRUE` | `fmain.c:2872` | All brothers dead — game over |
| `quitflag = TRUE` | `fmain.c:3245` | Talisman picked up — victory |
| `quitflag = TRUE` | `fmain.c:3466` | SAVEX menu → Exit option |

After the loop exits: `stopscore()` at `fmain.c:2616`, then `close_all()` at `fmain.c:2619-2620`.

### 16.7 Complete Quest Chain

1. **Rescue princess** (up to 3 times): Enter princess extent with `ob_list8[9].ob_stat` set. Rewards: Writ, 100 gold, +3 of each key, bird repositioned.
2. **Show Writ to Priest**: Talk to priest (setfig race 1) with `stuff[28]` — `fmain.c:3383-3386`. Priest speaks(39), reveals a golden statue (`ob_listg[10].ob_stat = 1`).
3. **Collect 5 golden figurines** (`stuff[25]`): Sorceress gives one via `speak(45)` at `fmain.c:3404-3406`; priest gives one with writ; others from world objects.
4. **Defeat the Necromancer**: Only bow (weapon 4) or wand (weapon 5) can damage due to combat immunity. On death → drops Talisman (world object 139).
5. **Pick up the Talisman**: `stuff[22]` set → `quitflag = TRUE` → `win_colors()` victory sequence.

---

