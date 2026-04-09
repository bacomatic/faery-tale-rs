# Discovery: NPC Dialogue & Quest Logic

**Status**: complete
**Investigated**: 2026-04-05
**Requested by**: orchestrator
**Prompt summary**: Trace the complete NPC dialogue system, quest progression logic, speech tables, Talk/Give mechanics, rescue sequence, shop system, and all message tables.

## setfig_table

Defined at `fmain.c:22-36`. Each entry has `{cfile_entry, image_base, can_talk}`.

| Index | Comment | cfile_entry | image_base | can_talk | Race code (0x80+idx) |
|-------|---------|-------------|------------|----------|----------------------|
| 0  | wizard    | 13 | 0 | 1 (yes) | 0x80 |
| 1  | priest    | 13 | 4 | 1 (yes) | 0x81 |
| 2  | guard     | 14 | 0 | 0 (no)  | 0x82 |
| 3  | guard (back) | 14 | 1 | 0 (no) | 0x83 |
| 4  | princess  | 14 | 2 | 0 (no)  | 0x84 |
| 5  | king      | 14 | 4 | 1 (yes) | 0x85 |
| 6  | noble     | 14 | 6 | 0 (no)  | 0x86 |
| 7  | sorceress | 14 | 7 | 0 (no)  | 0x87 |
| 8  | bartender | 15 | 0 | 0 (no)  | 0x88 |
| 9  | witch     | 16 | 0 | 0 (no)  | 0x89 |
| 10 | spectre   | 16 | 6 | 0 (no)  | 0x8a |
| 11 | ghost     | 16 | 7 | 0 (no)  | 0x8b |
| 12 | ranger    | 17 | 0 | 1 (yes) | 0x8c |
| 13 | beggar    | 17 | 4 | 1 (yes) | 0x8d |

**`can_talk`** controls whether the NPC enters TALKING state with a 15-tick animation timer when addressed — `fmain.c:3376-3379`. Note: `can_talk` does NOT gate whether the NPC responds; all 14 types have speech dispatch code. `can_talk` only triggers the talking animation.

**`cfile_entry`** indexes into `cfiles[]` — `fmain2.c:640-660` — to determine which shape file to load for the NPC's sprite. Entries 13-17 are setfig shape files (wizard/priest, royal set, bartender, witch/spectre/ghost, ranger/beggar).

**Race code**: When instantiated in `set_objects()` — `fmain2.c:1280` — setfigs get `an->race = id + 0x80` (the setfig index OR'd with 0x80). This distinguishes them from enemy `race` values (0-9).

**Vitality**: Each setfig gets `an->vitality = 2 + id*2` — `fmain2.c:1274`. So wizard=2, priest=4, ... beggar=28.

**Goal field**: Set to the object list index `i` within the current list — `fmain2.c:1275`. This matters for wizards (selects speech 27+goal), rangers (selects speech 53+goal), and beggars (selects speech 24+goal). Different instances of the same NPC type placed at different positions in the object list give different hints.

## Speech Index

Source: `narr.asm:347-end` (label `_speeches`). Speeches are 0-indexed, null-terminated strings.

| Index | Speaker | Text (abbreviated) | Trigger |
|-------|---------|---------------------|---------|
| 0 | Ogre | "% attempted to communicate with the Ogre but a guttural snarl..." | Talk to enemy race 0 |
| 1 | Orc | '"Human must die!" said the goblin-man.' | Talk to enemy race 1 |
| 2 | Wraith | '"Doom!" wailed the wraith.' | Talk to enemy race 2 |
| 3 | Skeleton | 'A clattering of bones was the only reply.' | Talk to enemy race 3 |
| 4 | Snake | '% knew that it is a waste of time to talk to a snake.' | Talk to enemy race 4 |
| 5 | Salamander | '...' | Talk to enemy race 5 |
| 6 | Loraii | 'There was no reply.' | Talk to enemy race 6 |
| 7 | Necromancer | '"Die, foolish mortal!" he said.' | Talk to enemy race 7 |
| 8 | Generic | '"No need to shout, son!" he said.' | Yell (hit==5) when target is close (dist < 35) — `fmain.c:3374` |
| 9 | Ranger | '"Nice weather we're having, isn't it?"' | Talk to ranger in region 2 (was generic, speech 22 override used instead) — unused directly via `speak(9)` in TALK handler |
| 10 | Ranger | '"Good luck, sonny!" said the ranger. "Hope you win!"' | Ranger goal-based: speak(53+goal) maps here for goal offset |
| 11 | Ranger | '"If you need to cross the lake..."' | Ranger goal-based hint |
| 12 | Bartender | '"Would you like to buy something?"' | Talk to bartender when time-of-day fatigue < 5 — `fmain.c:3406` is wrong — actually: `if (fatigue < 5) speak(13); else if (dayperiod > 7) speak(12); else speak(14)`. So speak(12) fires when `fatigue >= 5 && dayperiod > 7`. |
| 13 | Bartender | '"Good Morning." said the tavern keeper...' | Talk to bartender when `fatigue < 5` — `fmain.c:3405` |
| 14 | Bartender | '"Have a drink!" said the tavern keeper.' | Talk to bartender when `fatigue >= 5 && dayperiod <= 7` — `fmain.c:3407` |
| 15 | Guard | '"State your business!" ... "My business is with the king."' | Talk to guard (setfig index 2 or 3) — `fmain.c:3394` |
| 16 | Princess | '"Please, sir, rescue me..."' | Talk to princess (setfig index 4) when `ob_list8[9].ob_stat` is set — `fmain.c:3397`; also auto-speak on proximity — `fmain.c:2099` |
| 17 | King | '"I cannot help you, young man..."' | Talk to king (setfig index 5) when `ob_list8[9].ob_stat` is set — `fmain.c:3398` |
| 18 | King | '"Here is a writ designating you as my official agent..."' | Called from `rescue()` — `fmain2.c:1599` — after rescuing a princess |
| 19 | King/Priest | '"I'm afraid I cannot help you... already gave the golden statue..."' | Talk to priest when `stuff[28]` (writ) but `ob_listg[10].ob_stat` already == 1 — `fmain.c:3387` |
| 20 | Noble | '"If you could rescue the king's daughter..."' | Talk to noble (setfig index 6) — `fmain.c:3396` |
| 21 | Generic | '"Sorry, I have no use for it."' | Give bone to non-spectre — `fmain.c:3502` |
| 22 | Ranger | '"The dragon's cave is directly north of here."' | Talk to ranger in region 2 — `fmain.c:3411` |
| 23 | Beggar | '"Alms! Alms for the poor!"' | Talk to beggar (setfig 13) — `fmain.c:3414`; also auto-speak on proximity — `fmain.c:2097` |
| 24 | Beggar | '"I have a prophecy for you... seek two women, one Good, one Evil."' | Give gold to beggar, speak(24+goal) for goal=0 — `fmain.c:3498` |
| 25 | Beggar | '"Lovely Jewels, glint in the night - give to us the gift of Sight!"' | Give gold to beggar, goal=1 |
| 26 | Beggar | '"Where is the hidden city?..."' | Give gold to beggar, goal=2 |
| 27 | Wizard | '"Kind deeds could gain thee a friend from the sea."' | Talk to wizard, kind >= 10, goal=0 — `fmain.c:3381` speak(27+an->goal) |
| 28 | Wizard | '"Seek the place that is darker than night..."' | Wizard goal=1 |
| 29 | Wizard | '"Like the eye itself, a crystal Orb can help to find things concealed."' | Wizard goal=2 |
| 30 | Wizard | '"The Witch lives in the dim forest of Grimwood..."' | Wizard goal=3 |
| 31 | Wizard | '"Only the light of the Sun can destroy the Witch's Evil."' | Wizard goal=4 |
| 32 | Wizard | '"The maiden you seek lies imprisoned in an unreachable castle..."' | Wizard goal=5 |
| 33 | Wizard | '"Tame the golden beast and no mountain may deny you!..."' | Wizard goal=6 |
| 34 | Wizard | '"Just what I needed!" he said.' | Wizard goal=7 |
| 35 | Wizard | '"Away with you, young ruffian!"...' | Talk to wizard when `kind < 10` — `fmain.c:3380` |
| 36 | Priest/Cleric | '"You must seek your enemy on the spirit plane..."' | Talk to priest, kind >= 10, no writ, daynight%3==0 — `fmain.c:3392` |
| 37 | Priest/Cleric | '"When you wish to travel quickly, seek the power of the Stones."' | Priest, daynight%3==1 |
| 38 | Priest/Cleric | '"Since you are brave of heart, I shall Heal all your wounds."' | Priest, daynight%3==2; also heals player to max vitality — `fmain.c:3392-3394` |
| 39 | Priest/Cleric | '"Ah! You have a writ from the king. Here is one of the golden statues..."' | Talk to priest with stuff[28] (writ) and ob_listg[10].ob_stat==0 — `fmain.c:3384-3385` (first time only) |
| 40 | Priest/Cleric | '"Repent, Sinner!..."' | Talk to priest when `kind < 10` — `fmain.c:3389` |
| 41 | DreamKnight | '"Ho there, young traveler!"... "None may enter the sacred shrine..."' | Auto-speak on proximity with dknight (encounter race 7) — `fmain.c:2101` |
| 42 | DreamKnight | '"Your prowess in battle is great."... "You have earned the right to enter..."' | DreamKnight death — `checkdead()` when `an->race == 7` — `fmain.c:2775` |
| 43 | Necromancer | '"So this is the so-called Hero..."' | Auto-speak on proximity — `fmain.c:2100` |
| 44 | Necromancer | '% gasped. The Necromancer had been transformed...' | Necromancer death: when dying state completes (`an->race == 0x09`), race changes to 10 (woodcutter), vitality restored to 10, Talisman dropped — `fmain.c:1751-1756` |
| 45 | Sorceress | '"%." said the Sorceress. "Welcome. Here is one of the five golden figurines..."' | Talk to sorceress (setfig 7) when `ob_listg[9].ob_stat` == 0 (first visit) — `fmain.c:3400-3401`. Sets ob_listg[9].ob_stat = 1. |
| 46 | Witch | '"Look into my eyes and Die!!" hissed the witch.' | Talk to witch (setfig 9) — `fmain.c:3408`; also auto-speak on proximity — `fmain.c:2099` (race 0x89) |
| 47 | Spectre | '"HE has usurped my place... Bring me bones of the ancient King..."' | Talk to spectre (setfig 10) — `fmain.c:3409` |
| 48 | Spectre | '% gave him the ancient bones... "Take this crystal shard."' | Give bone to spectre — `fmain.c:3503`. Clears stuff[29], drops crystal shard (object 140). |
| 49 | Ghost | '"%..." I am the ghost of your dead brother. Find my bones...' | Talk to ghost (setfig 11) — `fmain.c:3410` |
| 50 | Generic | '% gave him some gold coins. "Why, thank you, young sir!"' | Give gold to non-beggar — `fmain.c:3499` |
| 51 | Generic | '"Sorry, but I have nothing to sell."' | (Referenced in narr.asm but no speak(51) call found in code) |
| 52 | Empty | (null byte — empty string) | (no speak(52) call found) |
| 53 | Ranger | '"The dragon's cave is east of here."' | Ranger outside region 2, goal=0: speak(53+0) — `fmain.c:3413` |
| 54 | Ranger | '"The dragon's cave is west of here."' | Ranger goal=1: speak(53+1) |
| 55 | Ranger | '"The dragon's cave is south of here."' | Ranger goal=2: speak(53+2) |
| 56 | Turtle | '"Oh, thank you for saving my eggs, kind man!"...' | Talk to carrier (turtle) when `active_carrier==5` and `stuff[6]==0` — `fmain.c:3419-3420`. Gives sea shell: stuff[6]=1. |
| 57 | Turtle | '"Just hop on my back if you need a ride somewhere."' | Talk to turtle when `stuff[6]!=0` (already has shell) — `fmain.c:3419` |
| 58 | Witch/Necro | '"Stupid fool, you can't hurt me with that!"' | Hit witch or necromancer with weapon < bow (weapon < 4) and without sunstone (stuff[7]==0 for witch 0x89) — `fmain2.c:231-233` |
| 59 | Necromancer | '"Your magic won't work here, fool!"' | Use magic when `extn->v3 == 9` (necromancer's extent region) — `fmain.c:3305` |
| 60 | Witch | 'The Sunstone has made the witch vulnerable!' | USE sunstone (hit==8) when `witchflag` is set — `fmain.c:3462` |

## Talk System

### Entry Point
Talk action handled in `do_option()` — `fmain.c:3367-3422`, triggered from TALK menu mode.

### TALK Submenu
Defined at `fmain.c:497`: `label3[] = "Yell Say  Ask  "` — three options:
- **hit==5 (Yell)**: `nearest_fig(1,100)` — searches 100-unit radius. If found within 35 units, speak(8) "No need to shout!" — `fmain.c:3374`.
- **hit==6 (Say)**: `nearest_fig(1,50)` — searches 50-unit radius.
- **hit==7 (Ask)**: Same as Say — `nearest_fig(1,50)`.

All three share the same dispatch logic after range check — `fmain.c:3368`. There is NO separate "Ask" handler. The three options are functionally identical except Yell has double range and a "too close to shout" check.

### Dispatch Logic

1. If `nearest == 0` → break (no target) — `fmain.c:3369`
2. If target is DEAD → break — `fmain.c:3370`
3. **SETFIG**: Switch on `k = an->race & 0x7f` (setfig index) — `fmain.c:3375-3415`
   - can_talk NPCs enter TALKING state for 15 ticks — `fmain.c:3377-3378`
   - Each NPC type has specific speech logic (see Speech Index above)
4. **CARRIER** with `active_carrier == 5` (turtle): Shell dialogue — `fmain.c:3418-3421`
5. **ENEMY**: `speak(an->race)` — enemy race indexes directly into speech table — `fmain.c:3422`

### Proximity Auto-Speech
Certain NPCs auto-speak when the player is nearby, independent of the Talk menu — `fmain.c:2094-2102`:
- Beggar (0x8d) → speak(23)
- Witch (0x89) → speak(46)
- Princess (0x84) → speak(16) — only if `ob_list8[9].ob_stat` set
- Necromancer (race 9) → speak(43)
- DreamKnight (race 7) → speak(41)

This triggers once per NPC (`last_person` tracking prevents re-triggering for the same NPC).

### Wizard Speech Selection
Wizard speech depends on `kind` stat AND the wizard's `goal` field (set from object list position):
- `kind < 10` → speak(35) "Away with you, ruffian!" — `fmain.c:3380`
- `kind >= 10` → speak(27 + an->goal) — `fmain.c:3381`
  - goal 0 → speak(27): "Kind deeds could gain thee a friend from the sea."
  - goal 1 → speak(28): "Seek the place darker than night..."
  - goal 2 → speak(29): Crystal Orb hint
  - goal 3 → speak(30): Witch location
  - goal 4 → speak(31): Sun defeats Witch
  - goal 5 → speak(32): Princess in unreachable castle
  - goal 6 → speak(33): Tame the golden beast (bird/lasso)
  - goal 7 → speak(34): "Just what I needed!"

### Priest Speech Selection
Three progression stages — `fmain.c:3382-3394`:
1. If `stuff[28]` (has writ):
   - If `ob_listg[10].ob_stat == 0`: speak(39) — gives golden statue, sets ob_listg[10].ob_stat = 1
   - Else: speak(19) — "already gave the statue"
2. If `kind < 10`: speak(40) — "Repent, Sinner!"
3. If `kind >= 10`: speak(36 + daynight%3) — rotating hints. Also heals player to max vitality. The three speeches:
   - daynight%3==0 → speak(36): Spirit plane warning
   - daynight%3==1 → speak(37): Stone circle hint
   - daynight%3==2 → speak(38): "I shall Heal all your wounds"

### Bartender Speech Selection
Three conditions — `fmain.c:3405-3407`:
- `fatigue < 5` → speak(13): "Good Morning. Hope you slept well."
- `fatigue >= 5 && dayperiod > 7` → speak(12): "Would you like to buy something? Or do you just need lodging?"
- else → speak(14): "Have a drink!"

## Give System

### Entry Point
GIVE menu mode in `do_option()` — `fmain.c:3490-3508`.

### Available Items
Menu label `labelA[] = "Gold Book Writ Bone "` — `fmain.c:506`.

Enabled status set in `set_options()` — `fmain.c:3538-3542`:
- **Gold** (hit==5): enabled if `wealth > 2`
- **Book** (hit==6): hardcoded to 8 (disabled) — ALWAYS DISABLED
- **Writ** (hit==7): enabled if `stuff[28] != 0` — menu enabled, but NO handler code for this hit value exists
- **Bone** (hit==8): enabled if `stuff[29] != 0`

### Give Gold (hit==5) — `fmain.c:3493-3500`
- Requires `nearest_person != 0` and `wealth > 2`
- Deducts 2 gold from wealth
- If `rand64() > kind` → `kind++` (kindness stat increase, probabilistic)
- If target is Beggar (race 0x8d) → speak(24 + an->goal) — beggar-specific prophecy based on object list position
- Else → speak(50) — generic "thank you" for gold

### Give Bone (hit==8) — `fmain.c:3501-3503`
- Requires `stuff[29] != 0` (has bone)
- If target is NOT Spectre (race != 0x8a) → speak(21) "Sorry, I have no use for it."
- If target IS Spectre (race 0x8a):
  - speak(48) — "Good! Take this crystal shard."
  - `stuff[29] = 0` — bone consumed
  - `leave_item(nearest_person, 140)` — drops crystal shard (object 140) at spectre's location

### Give Writ (hit==7) — No handler
The writ menu option is enabled when the player has a writ, but no code processes this action. The writ is checked passively during Talk interactions with the priest (setfig 1).

### Give Book (hit==6) — Disabled
Book is permanently disabled in the menu. Never usable.

## Quest Progression

### Quest Items (stuff[] indices)
From `inv_list[]` — `fmain.c:382-417` and `fmain.c:426-430`:

| stuff[] | Item | Usage |
|---------|------|-------|
| 0-4 | Dirk/Mace/Sword/Bow/Wand | Weapons |
| 5 | Golden Lasso | Capture bird: if `active_carrier == 11` and bird nearby, `stuff[5] = 1` — `fmain.c:1294`. Used to ride bird: `if (raftprox && wcarry == 3 && stuff[5])` — `fmain.c:1498` |
| 6 | Sea Shell | Obtained from turtle dialogue, `stuff[6] = 1` — `fmain.c:3420` |
| 7 | Sun Stone | Anti-witch item. Without it (`stuff[7] == 0`), melee damage to witch (race 0x89) is blocked — `fmain2.c:233`. With `witchflag` set, USE sunstone → speak(60) — `fmain.c:3462` |
| 8 | Arrows | Ammunition for bow |
| 9-15 | Blue Stone through Jade Skull | Magic items (MAGICBASE=9) |
| 16-21 | Gold/Green/Blue/Red/Grey/White Key | Door keys (KEYBASE=16) |
| 22 | Talisman | **WIN CONDITION**: When picked up, `quitflag = TRUE; viewstatus = 2` → triggers `win_colors()` — `fmain.c:3244-3246`. Dropped by necromancer on death — `fmain.c:1754` |
| 23 | Rose | Passwall through terrain type 12 (obstructions?): `if (stuff[23]) an->environ = 0` — `fmain.c:1845`. No, wrong — that's lava immunity. Actually `stuff[30]` is the passwall: `if (stuff[30] && j==12) goto newloc` — `fmain.c:1609`. Rose may protect from lava: `if (i==0 && stuff[23]) an->environ = 0` — `fmain.c:1845`. |
| 24 | Fruit (Apple) | Auto-eaten when hunger > 30 and in safe zone — `fmain.c:2197-2198`. Can be manually eaten via pickup: `if (hunger < 15) { stuff[24]++; event(36); } else eat(30)` — `fmain.c:3187-3188` |
| 25 | Gold Statue (STATBASE) | 5 needed to enter desert/Azal — gate check at `fmain.c:1919` and `fmain.c:3594` |
| 26 | Book | Menu item, always disabled in GIVE |
| 27 | Herb | (standard inventory item, no special quest logic found) |
| 28 | Writ | From king after rescue. Priest checks to give gold statue — `fmain.c:3383-3386` |
| 29 | Bone | Giveable to spectre for crystal shard — `fmain.c:3501-3503`. Found in underground: `ob_list9[8]` at `fmain2.c:1166`: `{3723,39340,(128+10),1}` — object ID 128+10=138, the king's bone |
| 30 | Shard (Crystal Shard) | Passwall item: `if (stuff[30] && j==12) goto newloc` — `fmain.c:1609`. Allows walking through terrain type 12 pixels. |

### Key Quest Flags (ob_listg / ob_list8)

**ob_list8[9].ob_stat** — Princess capture flag:
- Set to 3 (active setfig) when new brother starts — `fmain.c:2843`
- Checked for princess/king dialogue gating — `fmain.c:2099,3397,3398`
- Checked to trigger princess rescue — `fmain.c:2684`
- Cleared to 0 after rescue — `fmain2.c:1601`

**ob_listg[9].ob_stat** — Sorceress gold statue:
- Starts at 0 (not given) — `fmain2.c:1010`
- On first talk: set to 1, sorceress gives gold statue — `fmain.c:3401`
- Grants luck boost on subsequent talks: `if (luck < rand64()) luck += 5` — `fmain.c:3400`

**ob_listg[10].ob_stat** — Priest gold statue:
- Starts at 0 (not given) — `fmain2.c:1011`
- On first talk with writ: set to 1, priest gives statue speak(39) — `fmain.c:3384-3385`
- Already given: speak(19) — `fmain.c:3387`

**ob_listg[5].ob_stat** — Spectre NPC visibility:
- Toggles between 2 (visible, not talkable?) and 3 (active setfig) based on light level — `fmain.c:2027-2028`
- `lightlevel < 40` → ob_stat = 3 (visible as setfig)
- `lightlevel >= 40` → ob_stat = 2 (inventory/ground state = hidden)

**ob_listg[6-8]** — Three gold statues placed in world (ob_stat = 1 = ground):
- Index 6: seahold (11092, 38526)
- Index 7: ogre den (25737, 10662)
- Index 8: octal room (2910, 39023)

**ob_listg[1-2]** — Dead brother bones (ob_id=28 = bones object):
- Positions filled in during brother succession — `fmain.c:2838-2839`
- ob_stat = 1 when placed (ground item)

**ob_listg[3-4]** — Ghost brothers:
- Initialized with ob_id=11, ob_stat=0 (nonexistent)
- On brother death: `ob_listg[brother+2].ob_stat = 3` (becomes active setfig) — `fmain.c:2841`
- When dead brother's bones are picked up: `ob_listg[3].ob_stat = ob_listg[4].ob_stat = 0` (ghosts disappear) — `fmain.c:3174`

### Gold Statue Sources (5 needed for desert gate)
1. Sorceress (first talk) — `fmain.c:3400-3401`, implicit statue grant via ob_listg[9]
2. Priest (talk with writ) — `fmain.c:3384-3385`, via ob_listg[10]
3. Seahold — ob_listg[6] at (11092, 38526), ob_stat=1 (pickup from ground)
4. Ogre Den — ob_listg[7] at (25737, 10662), ob_stat=1
5. Octal Room — ob_listg[8] at (2910, 39023), ob_stat=1

**Note**: Sorceress and Priest give statues through dialogue (their ob_listg entries track "already given") but the actual `stuff[STATBASE]++` increment is NOT visible in the Talk handler code for these two. The sorceress handler sets `ob_listg[9].ob_stat = 1` but doesn't explicitly do `stuff[25]++`. The three ground-placed statues at indices 6-8 ARE picked up through normal object pickup (their ob_id is STATUE). This needs verification — the increment for sorceress/priest statues may be implicit through a mechanism not traced here.

### Quest State Gates

1. **Desert/Azal gate**: `stuff[STATBASE] < 5` blocks:
   - Door traversal to desert terrain — `fmain.c:1919`: `if (d->type == DESERT && (stuff[STATBASE]<5)) break;`
   - Map tile overwrite hides Azal city — `fmain.c:3594-3596`: sets map tiles to 254 (impassable)
   
2. **Pax zones** (weapon prevention):
   - King's castle: xtype==81 → `event(15)` "Even % would not be stupid enough to draw weapon in here." — `fmain.c:1413`
   - Sorceress area: xtype==82 → `event(16)` "A great calming influence..." — `fmain.c:1414`

3. **Witch invulnerability**: Without `stuff[7]` (Sun Stone), melee hits on witch (race 0x89) are blocked with `speak(58)` — `fmain2.c:231-233`. The condition: `anim_list[0].weapon < 4 && (race == 9 || (race == 0x89 && stuff[7] == 0))`

4. **Necromancer invulnerability**: Damage to necromancer (race 9) blocked if `weapon < 4` (need bow or wand) — `fmain2.c:231-232`

5. **Spectre/Ghost invulnerability**: `dohit()` returns early for race 0x8a (spectre) and 0x8b (ghost) — they cannot be damaged — `fmain2.c:234`

6. **Magic blocked in necromancer arena**: `if (extn->v3 == 9)` → speak(59) — `fmain.c:3305`

7. **Crystal Shard passwall**: `stuff[30]` allows walking through terrain type 12 — `fmain.c:1609`

8. **Rose lava protection**: `if (i==0 && stuff[23]) an->environ = 0` — `fmain.c:1845` — prevents lava environment damage to player

9. **Golden Lasso + Bird**: Need `stuff[5]` (lasso) to ride bird (carrier 11) — `fmain.c:1498`

### Win Condition
1. Kill Necromancer (race 9, 50 HP, in final arena extent) — `fmain.c:1751-1756`
2. On necromancer death: transforms to race 10 (woodcutter), drops Talisman (object 139)
3. Pick up Talisman → `stuff[22]` set → `quitflag = TRUE; viewstatus = 2` → `win_colors()` — `fmain.c:3244-3247`
4. Win sequence: `placard_text(6)` + `name()` + `placard_text(7)` + win picture — `fmain2.c:1605-1607`

### Game Over (All Brothers Dead)
- `brother` increments past 3 → `placard_text(5)` → `quitflag = TRUE` — `fmain.c:2870-2872`
- Placard text 5 = "So Kevin took up the quest..." then text 6 = "And so ends our sad tale... Stay at Home!"

## rescue() — Princess Rescue Mechanics

Source: `fmain2.c:1584-1604`

### Trigger
When player enters princess extent (xtype == 83) AND `ob_list8[9].ob_stat` is set — `fmain.c:2684-2685`:
```
if (xtype == 83 && ob_list8[9].ob_stat)
{   rescue(); flag = 0; goto findagain;  }
```

Princess extent defined at `fmain.c:346`: `{10820,35646, 10877,35670, 83, 1, 1, 0}`.

### Sequence
1. `map_message()` — fade to map display
2. `i = princess * 3` — calculate placard text offset
3. Display three placard texts: `placard_text(8+i)`, `name()`, `placard_text(9+i)`, `name()`, `placard_text(10+i)` — rescue narrative (princess-specific text)
4. `placard()` then `Delay(380)` — display and wait
5. Clear screen, then display `placard_text(17)` + `name()` + `placard_text(18)` — "After seeing the princess safely... set out on his quest."
6. `Delay(380)` then `message_off()`
7. `princess++` — increment princess counter
8. `xfer(5511,33780,0)` — teleport to coordinates (near king's castle)
9. `move_extent(0,22205,21231)` — move bird extent(?)
10. `ob_list8[2].ob_id = 4` — change a building interior NPC: slot 2 (originally noble, id=6) becomes princess (id=4)
11. `stuff[28] = 1` — give writ
12. `speak(18)` — king says "Here is a writ designating you as my official agent..."
13. `wealth += 100` — 100 gold reward
14. `ob_list8[9].ob_stat = 0` — clear princess rescue flag (princess no longer captive)
15. `for (i=16; i<22; i++) stuff[i] += 3` — give 3 of each key type (Gold, Green, Blue, Red, Grey, White)

### Princess Counter and Texts
`princess` variable — `fmain.c:568` — starts at 0, incremented each rescue.

| princess value | placard texts used | Princess name |
|----------------|-------------------|---------------|
| 0 | 8, 8a, 8b (msg8-msg8b) | Katra |
| 1 | 9, 9a, 9b (msg9-msg9b) | Karla (Katra's sister) |
| 2 | 10, 10a, 10b (msg10-msg10b) | Kandy (Katra's and Karla's sister) |

After rescue texts: placard_text(17) = msg11, placard_text(18) = msg11a — "After seeing the princess safely to her home city... once more set out on his quest."

## Shop System

### Bartender Identification
Bartenders have setfig index 8, race 0x88. Shop menu (BUY) only works with race 0x88 — `fmain.c:3426`.

### jtrans Table
Defined at `fmain2.c:850`:
```
char jtrans[] = { 0,3, 8,10, 11,15, 1,30, 2,45, 3,75, 13,20 };
```
Format: pairs of `{stuff_index, price}`:

| Menu hit | jtrans index | stuff[] idx | Price | Item |
|----------|-------------|-------------|-------|------|
| 5 (Food) | 0,1 | 0 | 3 | Food (special: eat, don't add to inventory) |
| 6 (Arrow) | 2,3 | 8 | 10 | Arrows (+10) |
| 7 (Vial) | 4,5 | 11 | 15 | Glass Vial |
| 8 (Mace) | 6,7 | 1 | 30 | Mace |
| 9 (Sword) | 8,9 | 2 | 45 | Sword |
| 10 (Bow) | 10,11 | 3 | 75 | Bow |
| 11 (Totem) | 12,13 | 13 | 20 | Bird Totem |

### BUY Logic — `fmain.c:3424-3443`
1. Check `nearest_person != 0` and `anim_list[nearest].race == 0x88` (bartender)
2. If `hit > 11` → return (out of range)
3. Convert hit to jtrans index: `hit = (hit - 5) * 2`
4. `i = jtrans[hit++]; j = jtrans[hit]` — get item index and price
5. If `wealth > j`:
   - Deduct price from wealth
   - If i==0 (Food): `event(22)` "% bought some food and ate it." + `eat(50)`
   - If i==8 (Arrows): `stuff[8] += 10; event(23)` "% bought some arrows."
   - Else: `stuff[i]++; extract("% bought a "); print_cont(inv_list[i].name)`
6. If not enough money: `print("Not enough money!")`

### Non-Bartender BUY
If target is not bartender (race != 0x88), BUY case breaks with no action. The `speak(51)` "Sorry, but I have nothing to sell." is defined in narr.asm but no code path calls it.

## Message Tables

### _event_msg — `narr.asm:11-74`
Called via `event(n)` — `fmain2.c:554-558`. Indexed by number, 0-based.

| Index | Text | Usage |
|-------|------|-------|
| 0 | "% was getting rather hungry." | Hunger warning 1 |
| 1 | "% was getting very hungry." | Hunger warning 2 |
| 2 | "% was starving!" | Hunger warning 3 |
| 3 | "% was getting tired." | Fatigue warning 1 |
| 4 | "% was getting sleepy." | Fatigue warning 2 |
| 5 | "% was hit and killed!" | Death by combat — `checkdead(i,5)` |
| 6 | "% was drowned in the water!" | Death by drowning — `checkdead(i,6)` |
| 7 | "% was burned in the lava." | Death by lava (mild) |
| 8 | "% was turned to stone by the witch." | Witch petrification |
| 9 | "% started the journey in his home village of Tambry" | Brother start 1 |
| 10 | "as had his brother before him." | Brother start 2 (second brother) |
| 11 | "as had his brothers before him." | Brother start 3 (third brother) |
| 12 | "% just couldn't stay awake any longer!" | Forced sleep |
| 13 | "% was feeling quite full." | Ate food |
| 14 | "% was feeling quite rested." | Rested |
| 15 | "Even % would not be stupid enough to draw weapon in here." | King's castle pax zone — xtype==81, `fmain.c:1413` |
| 16 | "A great calming influence comes over %..." | Sorceress pax zone — xtype==82, `fmain.c:1414` |
| 17 | "% picked up a scrap of paper." | Paper pickup — `fmain.c:3167` |
| 18 | 'It read: "Find the turtle!"' | Paper content (region ≤ 7) — `fmain.c:3168` |
| 19 | 'It read: "Meet me at midnight at the Crypt. Signed, the Wraith Lord."' | Paper content (region > 7) — `fmain.c:3168` |
| 20 | "% looked around but discovered nothing." | Look action, nothing found — `fmain.c:3297` |
| 21 | "% does not have that item." | Missing item |
| 22 | "% bought some food and ate it." | Shop: buy food — `fmain.c:3433` |
| 23 | "% bought some arrows." | Shop: buy arrows — `fmain.c:3434` |
| 24 | "% passed out from hunger!" | Hunger collapse |
| 25 | "% is not sleepy." | Not tired enough to sleep |
| 26 | "% was tired, so he decided to lie down and sleep." | Going to sleep |
| 27 | "% perished in the hot lava!" | Death by lava (severe) — `checkdead(i,27)` — `fmain.c:1847` |
| 28 | "It was midnight." | Time announcement — dayperiod 0, `fmain.c:2033` |
| 29 | "It was morning." | Time — dayperiod 4 |
| 30 | "It was midday." | Time — dayperiod 6 |
| 31 | "Evening was drawing near." | Time — dayperiod 9 |
| 32 | "Ground is too hot for swan to land." | Bird landing on lava — `fmain.c:1419` |
| 33 | "Flying too fast to dismount." | Bird dismount while fast — actually from narr.asm text |
| 34 | '"They're all dead!" he cried.' | All enemies killed in battle — `fmain.c:3362` after MAGIC kill spell |
| 35 | "No time for that now!" | Combat prevents action — `fmain.c:3293` (searching body during combat) |
| 36 | "% put an apple away for later." | Apple pickup (hunger < 15) — `fmain.c:3187` |
| 37 | "% ate one of his apples." | Auto-eat apple in safe zone — `fmain.c:2198` |
| 38 | "% discovered a hidden object." | Look action found object — `fmain.c:3296` |

### _placard_text — `narr.asm:230-343`
Called via `placard_text(n)` — `narr.asm:224-229`. Uses XY positioning with formatted text. 0-indexed.

| Index | Label | Content | When displayed |
|-------|-------|---------|----------------|
| 0 | msg1 | Julian's quest start: "Rescue the Talisman!" Mayor's plea | Start of game / Julian's intro — `fmain.c:2864` |
| 1 | msg2 | Julian failed: "his luck had run out" | Julian dies, Phillip starts — `fmain.c:2866` (`brother==2`) |
| 2 | msg3 | Phillip sets out | Phillip starts (same as above, `placard_text(1)` then this in sequence — wait no, `placard_text(1)` is msg2 and `placard_text(3)` is msg4) |
| 3 | msg4 | Phillip failed: "Phillip's cleverness could not save him" | Phillip dies, Kevin starts — `fmain.c:2867` |
| 4 | msg5 | Kevin takes quest: "Young and inexperienced..." | Kevin starts — computed from `placard_text(3)` mapping... Actually indexing: brother==3 → `placard_text(3)` = msg4, then... |
| 5 | msg6 | Game over: "And so ends our sad tale... Stay at Home!" | All brothers dead — `fmain.c:2870` |
| 6 | msg7 | Win text part 1: "Having defeated the villainous Necromancer..." | Win sequence — `fmain2.c:1607` |
| 7 | msg7a | Win text part 2: "returned to Marheim where he wed the princess..." | Win sequence — `fmain2.c:1607` |
| 8 | msg8 | Rescue princess 1 (Katra) part 1 | rescue() with princess==0 — `fmain2.c:1588` |
| 9 | msg8a | Rescue Katra part 2: "had rescued Katra, Princess of Marheim..." | rescue() |
| 10 | msg8b | Rescue Katra part 3: "knew that his quest could not be forsaken." | rescue() |
| 11 | msg9 | Rescue princess 2 (Karla) part 1 | rescue() with princess==1 |
| 12 | msg9a | Rescue Karla part 2 | rescue() |
| 13 | msg9b | Rescue Karla part 3 | rescue() |
| 14 | msg10 | Rescue princess 3 (Kandy) part 1 | rescue() with princess==2 |
| 15 | msg10a | Rescue Kandy part 2 | rescue() |
| 16 | msg10b | Rescue Kandy part 3 | rescue() |
| 17 | msg11 | Post-rescue: "After seeing the princess safely to her home city..." | rescue() — `fmain2.c:1591` |
| 18 | msg11a | Post-rescue: "once more set out on his quest." | rescue() |
| 19 | msg12 | Copy protection: "So... You, game seeker, would guide the brothers..." | `copy_protect_junk()` — `fmain2.c:1316-1317` |

### _question — `narr.asm:57-73`
Copy protection riddles. Called via `question(n)` — `narr.asm:57-65`. Index 0-7.

| Index | Text | Answer |
|-------|------|--------|
| 0 | "To Quest for the...?" | LIGHT |
| 1 | "Make haste, but take...?" | HEED |
| 2 | "Scorn murderous...?" | DEED |
| 3 | "Summon the...?" | SIGHT |
| 4 | "Wing forth in...?" | FLIGHT |
| 5 | "Hold fast to your...?" | CREED |
| 6 | "Defy Ye that...?" | BLIGHT |
| 7 | "In black darker than...?" | NIGHT |

Answers defined at `fmain2.c:1307`: `char *answers[] = { "LIGHT","HEED","DEED","SIGHT","FLIGHT","CREED","BLIGHT","NIGHT" };`

### _place_msg — `narr.asm:130-181`
Region entry messages. 0-indexed. Triggered by `find_place()` when entering a new named region.

| Index | Text |
|-------|------|
| 0 | (null — no message) |
| 1 | (null — do not change) |
| 2 | "% returned to the village of Tambry." |
| 3 | "% came to Vermillion Manor." |
| 4 | "% reached the Mountains of Frost" |
| 5 | "% reached the Plain of Grief." |
| 6 | "% came to the city of Marheim." |
| 7 | "% came to the Witch's castle." |
| 8 | "% came to the Graveyard." |
| 9 | "% came to a great stone ring." |
| 10 | "% came to a watchtower." |
| 11 | "% traveled to the great Bog." |
| 12 | "% came to the Crystal Palace." |
| 13 | "% came to mysterious Pixle Grove." |
| 14 | "% entered the Citadel of Doom." |
| 15 | "% entered the Burning Waste." |
| 16 | "% found an oasis." |
| 17 | "% came to the hidden city of Azal." |
| 18 | "% discovered an outlying fort." |
| 19 | "% came to a small keep." |
| 20 | "% came to an old castle." |
| 21 | "% came to a log cabin." |
| 22 | "% came to a dark stone tower." |
| 23 | "% came to an isolated cabin." |
| 24 | "% came to the Tombs of Hemsath." |
| 25 | "% reached the Forbidden Keep." |
| 26 | "% found a cave in the hillside." |

Lookup via `_place_tbl` — `narr.asm:76-104` — maps sector number ranges to place_msg indices.

### _inside_msg — `narr.asm:199-221`
Interior location messages. Same structure as place_msg but for building interiors.

| Index | Text |
|-------|------|
| 0 | (null) |
| 1 | (null) |
| 2 | "% came to a small chamber." |
| 3 | "% came to a large chamber." |
| 4 | "% came to a long passageway." |
| 5 | "% came to a twisting tunnel." |
| 6 | "% came to a forked intersection." |
| 7 | "He entered the keep." |
| 8 | "He entered the castle." |
| 9 | "He entered the castle of King Mar." |
| 10 | "He entered the sanctuary of the temple." |
| 11 | "% entered the Spirit Plane." |
| 12 | "% came to a large room." |
| 13 | "% came to an octagonal room." |
| 14 | "% traveled along a stone corridor." |
| 15 | "% came to a stone maze." |
| 16 | "He entered a small building." |
| 17 | "He entered the building." |
| 18 | "He entered the tavern." |
| 19 | "He went inside the inn." |
| 20 | "He entered the crypt." |
| 21 | "He walked into the cabin." |
| 22 | "He unlocked the door and entered." |

Lookup via `_inside_tbl` — `narr.asm:105-129` — maps interior sector numbers to inside_msg indices.

## extract() — Template Substitution

Source: `fmain2.c:514-554`.

### Mechanism
- Scans input string character by character
- `%` is replaced with current brother's name from `datanames[brother-1]` — `fmain2.c:528`
  - `datanames[] = { "Julian","Phillip","Kevin" }` — `fmain.c:604`
- `\r` (char 13) forces a line break
- `\0` (null) terminates the string
- Word-wraps at column 37 (max line width for scroll display)
- Output printed via `print()` line by line

### Related Functions (inline assembly) — `fmain2.c:554-572`
- **`event(n)`**: `lea _event_msg,a0` → skip `n` null-terminated strings → call `_extract`
- **`speak(n)`**: `lea _speeches,a0` → skip `n` null-terminated strings → call `_extract`
- **`msg(start,n)`**: same skip-n logic on arbitrary start pointer → call `_extract`

All three ultimately call `_extract` (the C `extract()` function) with a pointer to the appropriate string.

### placard_text / ssp
- `placard_text(n)` — `narr.asm:224-229`: uses a jump table (`mst`) to locate the nth placard message, then calls `_ssp` (screen string print) which renders XY-positioned text on the placard display.
- `question(n)` — `narr.asm:57-65`: similar jump table for copy protection questions, calls `_print_cont`.

## Unresolved

1. **Sorceress/Priest gold statue increment**: The sorceress talk handler (case 7) sets `ob_listg[9].ob_stat = 1` but no explicit `stuff[STATBASE]++` is visible. Similarly for priest (case 1) — sets `ob_listg[10].ob_stat = 1`. The five ground-placed statues (ob_listg[6-8]) have STATUE as their ob_id and would be picked up via normal object pickup. But how do sorceress and priest statue gifts increment `stuff[25]`? Possibly through a mechanism in `set_objects()` or `change_object()` not traced here. The `stuff[STATBASE] < 5` gate checks suggest all 5 must be in the stuff array.

2. **`ob_list8[2].ob_id = 4` in rescue()**: This changes slot 2 (originally noble, ob_id=6) to ob_id=4 (princess). Why? Possibly to place a rescued princess in the castle after rescue. But the noble is at position (5592,33764) which is in the king's castle area.

3. **`move_extent(0,22205,21231)` in rescue()**: What extent is being moved and to where? Extent index 0 is the bird extent. Moving the bird after rescue — possibly to reposition the bird for the player's next phase of the quest.

4. **Speak(51) "Sorry, nothing to sell"**: Defined in narr.asm but no `speak(51)` call found in any source file. May be unreachable/unused content.

5. **Speak(44) Necromancer transformation**: The necromancer death triggers speak(42) via `checkdead()` (DreamKnight death speech), not speak(44). Speech 44 ("The Necromancer had been transformed into a normal man") — no `speak(44)` call found. The necromancer transforms to race 10 (woodcutter) at `fmain.c:1751-1753` but the transformation narration appears to be unused.

6. **Book item (stuff[26])**: Always disabled in GIVE menu. No specific quest usage found. What was its intended purpose?

7. **Herb item (stuff[27])**: No special quest-related code paths found beyond basic inventory handling.

8. **Writ in GIVE menu**: The writ can be enabled in the GIVE submenu via `set_options()` — `fmain.c:3541` — but the GIVE handler has no code for hit==7. Was this intended as a "show writ to guard" mechanic that was never implemented?

9. **Speech 9-11 vs ranger handler**: Speeches 9-11 are labeled as "woodcutter messages" in narr.asm comments but are not used by the ranger Talk handler. The ranger handler uses `speak(22)` for region 2, then `speak(53+goal)` for others. Speeches 9-11 might be early/alternate ranger dialogue that was superseded.

## Cross-Cutting Findings

- **stuff[30] (Crystal Shard) in movement system** (`fmain.c:1609`): Checked during walking collision — passwall through terrain type 12. This is a movement subsystem check for a quest item.
- **stuff[23] (Rose) in damage system** (`fmain.c:1845`): Checked during lava environment damage calculation — protects player from fire.
- **stuff[7] (Sun Stone) in combat dohit()** (`fmain2.c:231-233`): Checked during melee damage application — gates damage to witch.
- **stuff[5] (Golden Lasso) in carrier movement** (`fmain.c:1498`): Checked in the bird carrier animation loop — gates bird riding.
- **ob_list8[9].ob_stat in extent system** (`fmain.c:2684`): Princess capture flag checked during extent traversal, not in dialogue system.
- **witchflag in safe zone tracking** (`fmain.c:2190`): Witch presence blocks safe zone updates — dying near the witch means respawning far away.
- **Spectre visibility toggled by daylight** (`fmain.c:2027-2028`): Time-of-day subsystem controls whether spectre NPC is visible/interactable.
- **Necromancer transforms to woodcutter on death** (`fmain.c:1751-1753`): The dying animation completion handler doubles as a quest state transition — race change from 9→10 with vitality restored.
- **Brother succession writes quest objects** (`fmain.c:2838-2843`): The revive/death system creates dead brother bones and ghost NPCs in the global object list.

## Refinement Log

- 2026-04-05: Initial comprehensive discovery pass. Traced all TALK, GIVE, BUY handlers. Indexed all speeches, event messages, placard texts, and place/inside messages. Identified quest gates and progression flags.
