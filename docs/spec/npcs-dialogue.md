## 13. NPCs & Dialogue

### 13.1 NPC Types (setfig_table)

14 NPC types defined in `setfig_table[]` (`fmain.c:22-36`):

| Index | NPC | cfile | image_base | can_talk | Race | Vitality |
|-------|-----|-------|------------|----------|------|----------|
| 0 | Wizard | 13 | 0 | Yes | 0x80 | 2 |
| 1 | Priest | 13 | 4 | Yes | 0x81 | 4 |
| 2 | Guard | 14 | 0 | No | 0x82 | 6 |
| 3 | Guard (back) | 14 | 1 | No | 0x83 | 8 |
| 4 | Princess | 14 | 2 | No | 0x84 | 10 |
| 5 | King | 14 | 4 | Yes | 0x85 | 12 |
| 6 | Noble | 14 | 6 | No | 0x86 | 14 |
| 7 | Sorceress | 14 | 7 | No | 0x87 | 16 |
| 8 | Bartender | 15 | 0 | No | 0x88 | 18 |
| 9 | Witch | 16 | 0 | No | 0x89 | 20 |
| 10 | Spectre | 16 | 6 | No | 0x8a | 22 |
| 11 | Ghost | 16 | 7 | No | 0x8b | 24 |
| 12 | Ranger | 17 | 0 | Yes | 0x8c | 26 |
| 13 | Beggar | 17 | 4 | Yes | 0x8d | 28 |

- Race code: `id + 0x80` (OR'd in `set_objects` — `fmain2.c:1280`).
- Vitality: `2 + id*2` (`fmain2.c:1274`).
- `goal` field: stores the object list index (`fmain2.c:1275`), selecting variant dialogue for wizards, rangers, and beggars.
- `can_talk`: enables the TALKING state visual flicker (15-tick timer — `fmain.c:3376-3379`). While `tactic` counts down from 15 to 0, each render tick picks between two adjacent sprite frames via `dex += rand2()` (`fmain.c:1556`), producing a random flicker — **not** a scripted mouth/animation sequence. When the timer expires the NPC returns to STILL (`fmain.c:1557`). Only 5 of 14 types set `can_talk=1`: Wizard, Priest, King, Ranger, Beggar. All 14 types have speech dispatch code regardless of this flag.

### 13.2 Talk System

Entry point: `do_option()` — `fmain.c:3367-3422`.

**Submenu** (`fmain.c:497`): `"Yell Say  Ask  "` — three options:

| Option | Hit Value | Range | Special |
|--------|-----------|-------|---------|
| Yell | 5 | `nearest_fig(1,100)` — 100 units | If target within 35 units: `speak(8)` ("No need to shout!") |
| Say | 6 | `nearest_fig(1,50)` — 50 units | Standard dispatch |
| Ask | 7 | `nearest_fig(1,50)` — 50 units | Functionally identical to Say |

**Dispatch logic** (`fmain.c:3375-3422`):

1. If no target found or target is DEAD → break (no speech).
2. **SETFIG NPC**: switch on `k = an->race & 0x7f` (setfig index) — see §13.3.
3. **CARRIER** with `active_carrier == 5` (turtle): shell dialogue — see §13.7.
4. **ENEMY**: `speak(an->race)` — enemy race directly indexes speech table — see §13.8.

### 13.3 NPC Speech Dispatch

**Wizard** (index 0 — `fmain.c:3380-3381`):
- `kind < 10` → `speak(35)` ("Away with you, ruffian!")
- `kind >= 10` → `speak(27 + an->goal)` — goal selects from 8 hints:

| Goal | Speech | Hint |
|------|--------|------|
| 0 | speak(27) | "Kind deeds could gain thee a friend from the sea" (Turtle) |
| 1 | speak(28) | "Seek the place darker than night" |
| 2 | speak(29) | "Crystal Orb helps find concealed things" |
| 3 | speak(30) | "The Witch lives in the dim forest of Grimwood" |
| 4 | speak(31) | "Only the light of the Sun can destroy the Witch's Evil" |
| 5 | speak(32) | "Maiden imprisoned in unreachable castle" |
| 6 | speak(33) | "Tame the golden beast" (Swan) |
| 7 | speak(34) | "Just what I needed!" |

Wizard locations by object list:

| Region | Object List Entry | Goal |
|--------|-------------------|------|
| 0 (Snow) | ob_list0 | 0–2 (3 wizards) |
| 2 (Swamp) | ob_list2[0], ob_list2[1] | 0, 1 |
| 3 (Tambry) | ob_list3[2] | 2 |
| 5 (Farm/City) | ob_list5[3], ob_list5[4] | 3, 4 |
| 8 (Indoors) | ob_list8[5], ob_list8[6] | 5, 6 |
| 9 (Underground) | ob_list9[6] | 6 |

**Priest** (index 1 — `fmain.c:3382-3394`):
1. Has Writ (`stuff[28]`):
   - If `ob_listg[10].ob_stat == 0` → `speak(39)` ("Here is a golden statue"), set `ob_listg[10].ob_stat = 1`.
   - Else → `speak(19)` ("Already gave the statue").
2. `kind < 10` → `speak(40)` ("Repent, Sinner!").
3. `kind >= 10` → `speak(36 + daynight%3)` (rotating advice) AND heal to `15 + brave/4`:
   - `daynight%3 == 0` → speak(36): "Seek enemy on spirit plane"
   - `daynight%3 == 1` → speak(37): "Seek power of the Stones"
   - `daynight%3 == 2` → speak(38): "I shall Heal all your wounds"

All three speeches (36–38) trigger healing; the rotation selects only the text.

**Guard** (indices 2–3 — `fmain.c:3394`): `speak(15)` ("State your business!").

**Princess** (index 4 — `fmain.c:3397`): `speak(16)` ("Please, sir, rescue me...") — only when `ob_list8[9].ob_stat` is set.

**King** (index 5 — `fmain.c:3398`): `speak(17)` ("I cannot help you, young man") — only when `ob_list8[9].ob_stat` is set. No dialogue when flag is clear.

**Noble** (index 6 — `fmain.c:3396`): `speak(20)` ("If you could rescue the king's daughter...").

**Sorceress** (index 7 — `fmain.c:3400-3405`):
- First visit (`ob_listg[9].ob_stat == 0`): `speak(45)` ("Welcome. Here is one of the five golden figurines"), set `ob_listg[9].ob_stat = 1`.
- Return visits: no speech. Silent luck boost: if `luck < rand64()` then `luck += 5`.

**Bartender** (index 8 — `fmain.c:3405-3407`):
- `fatigue < 5` → `speak(13)` ("Good Morning").
- `fatigue >= 5 && dayperiod > 7` → `speak(12)` ("Would you like to buy something?").
- Else → `speak(14)` ("Have a drink!").

**Witch** (index 9 — `fmain.c:3408`): `speak(46)` ("Look into my eyes and Die!!").

**Spectre** (index 10 — `fmain.c:3409`): `speak(47)` ("HE has usurped my place... Bring me bones of the ancient King").

**Ghost** (index 11 — `fmain.c:3410`): `speak(49)` ("I am the ghost of your dead brother. Find my bones...").

**Ranger** (index 12 — `fmain.c:3411-3413`):
- `region_num == 2` (swamp) → `speak(22)` ("Dragon's cave is directly north").
- Else → `speak(53 + an->goal)`:
  - goal 0 → speak(53): "Dragon's cave is east"
  - goal 1 → speak(54): "Dragon's cave is west"
  - goal 2 → speak(55): "Dragon's cave is south"

Rangers appear only in ob_list0 (snow, 3 rangers) and ob_list2 (swamp, 1 ranger).

**Beggar** (index 13 — `fmain.c:3414`): `speak(23)` ("Alms! Alms for the poor!").

### 13.4 Proximity Auto-Speech

Certain NPCs speak automatically when near the player (`fmain.c:2094-2102`), independent of the Talk menu. Tracked by `last_person` to prevent re-triggering for the same NPC.

| Race | NPC | Speech | Condition |
|------|-----|--------|-----------|
| 0x8d | Beggar | `speak(23)` | Always |
| 0x89 | Witch | `speak(46)` | Always |
| 0x84 | Princess | `speak(16)` | Only if `ob_list8[9].ob_stat` set |
| 9 | Necromancer | `speak(43)` | Always (extent entry) |
| 7 | DreamKnight | `speak(41)` | Always (extent entry) |

DreamKnight death speech: `speak(42)` ("You have earned the right to enter...") — `fmain.c:2775`.

### 13.5 Give System

Entry point: `do_option()` — `fmain.c:3490-3508`.

Menu (`fmain.c:506`): `"Gold Book Writ Bone "` — four options (hit values 5–8):

| Option | Hit | Condition | Effect |
|--------|-----|-----------|--------|
| Gold | 5 | `wealth > 2` | `wealth -= 2`. If `rand64() > kind` → `kind++`. Beggar: `speak(24 + goal)`. Others: `speak(50)`. |
| Book | 6 | ALWAYS DISABLED | Hardcoded disabled in `set_options` (`fmain.c:3540`). |
| Writ | 7 | `stuff[28] != 0` | Menu enabled but **no processing handler**. Writ is checked passively during Priest Talk. |
| Bone | 8 | `stuff[29] != 0` | Non-spectre: `speak(21)` ("no use for it"). Spectre (0x8a): `speak(48)` ("Take this crystal shard"), `stuff[29] = 0`, drops crystal shard (object 140). |

**Beggar give-gold prophecies** (`speak(24 + goal)`):

| Goal | Speech | Prophecy |
|------|--------|----------|
| 0 | speak(24) | "Seek two women, one Good, one Evil" |
| 1 | speak(25) | "Jewels, glint in the night — gift of Sight" |
| 2 | speak(26) | "Where is the hidden city?" |
| 3 | speak(27) | **Bug**: overflows to first wizard hint ("Kind deeds...") — `ob_list3[3]` has `goal=3` |

### 13.6 Shop System

**Bartender identification**: setfig index 8, race `0x88`. BUY menu only activates with race `0x88` (`fmain.c:3426`).

**`jtrans[]`** (`fmain2.c:850`) defines 7 purchasable items as `{stuff_index, price}` pairs:

| # | Item | Price | Effect | Menu Text |
|---|------|-------|--------|-----------|
| 1 | Food | 3 | `eat(50)` — reduces hunger by 50 | Food |
| 2 | Arrows | 10 | `stuff[8] += 10` | Arrow |
| 3 | Vial | 15 | `stuff[11]++` (Glass Vial / healing potion) | Vial |
| 4 | Mace | 30 | `stuff[1]++` | Mace |
| 5 | Sword | 45 | `stuff[2]++` | Sword |
| 6 | Bow | 75 | `stuff[3]++` | Bow |
| 7 | Totem | 20 | `stuff[13]++` (Bird Totem) | Totem |

Menu string (`fmain.c:501`): `"Food ArrowVial Mace SwordBow  Totem"`.

Purchase requires `wealth > price` (`fmain.c:3430`). On purchase, `wealth -= price`.

### 13.7 Carrier Dialogue (Turtle)

When `active_carrier == 5` (turtle carrier active — `fmain.c:3418-3421`):

- If `stuff[6] == 0` (no sea shell): `speak(56)` ("Thank you for saving my eggs!"), set `stuff[6] = 1`.
- If `stuff[6] != 0` (has shell): `speak(57)` ("Hop on my back for a ride").

### 13.8 Enemy Speech

When talking to enemies, `speak(an->race)` — the race value directly indexes the speech table (`fmain.c:3422`):

| Race | Enemy | Speech |
|------|-------|--------|
| 0 | Ogre | speak(0): "A guttural snarl was the only reply." |
| 1 | Orc | speak(1): "Human must die!" |
| 2 | Wraith | speak(2): "Doom!" |
| 3 | Skeleton | speak(3): "A clattering of bones" |
| 4 | Snake | speak(4): "A waste of time to talk to a snake" |
| 5 | Salamander | speak(5): "..." |
| 6 | Spider | speak(6): "There was no reply." |
| 7 | DKnight | speak(7): "Die, foolish mortal!" |

Note: `narr.asm` labels speak(6) as "loraii" and speak(7) as "necromancer", reflecting an earlier race table. Loraii (race 8) and Necromancer (race 9) have special auto-speak handlers that preempt the generic talk path, so misaligned speeches at indices 8–9 are never heard in normal gameplay.

### 13.9 Message Tables

**Event messages** (`narr.asm:11-74`, called via `event(n)` — `fmain2.c:554-558`):

| Index | Text | Usage |
|-------|------|-------|
| 0–2 | Hunger warnings ("rather hungry", "very hungry", "starving!") | `fmain.c:2203-2209` |
| 3–4 | Fatigue warnings ("getting tired", "getting sleepy") | `fmain.c:2218-2219` |
| 5–7 | Death messages (combat, drowning, lava) | `checkdead()` dtype |
| 12 | "couldn't stay awake any longer!" | Forced sleep |
| 15–16 | Pax zone messages (castle, sorceress) | `fmain.c:1413-1414` |
| 17–19 | Paper pickup (regionally-variant text) | `fmain.c:3163-3168` |
| 22–23 | Shop purchase messages | `fmain.c:3433-3434` |
| 24 | "passed out from hunger!" | `fmain.c:2213` |
| 27 | "perished in the hot lava!" | `fmain.c:1847` |
| 28–31 | Time-of-day announcements | `fmain.c:2033` |
| 32 | "Ground is too hot for swan to land" | Swan dismount blocked in lava |
| 33 | "Flying too fast to dismount" | Swan dismount blocked at speed |
| 36–37 | Fruit pickup/auto-eat | `fmain.c:3166`, `fmain.c:2196` |

**Placard texts** (`narr.asm:230-343`): formatted narrative screens with XY positioning.

| Index | Content |
|-------|---------|
| 0 | Julian's quest begins ("Rescue the Talisman!") |
| 1–2 | Julian falls, Phillip starts |
| 3–4 | Phillip falls, Kevin starts |
| 5 | Game over ("Stay at Home!") |
| 6–7 | Victory sequence ("Having defeated the Necromancer...") |
| 8–10 | Princess Katra rescue |
| 11–13 | Princess Karla rescue |
| 14–16 | Princess Kandy rescue |
| 17–18 | Shared post-rescue text (all princesses) |
| 19 | Copy protection prompt |

---


