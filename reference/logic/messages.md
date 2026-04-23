# Complete Message String Reference

All text strings defined in `narr.asm`. This document is the authoritative
source for implementors who do not have access to the 1987 assembler source.
It covers every indexed table (`_event_msg`, `_speeches`, `_place_msg`,
`_inside_msg`), the copy-protection question table (`_question`), and the
full-screen placard messages (`_placard_text`). Hardcoded strings embedded
in `fmain.c` and `fmain2.c` are documented separately in
[dialog_system.md](dialog_system.md#hardcoded-scroll-messages--complete-reference).

## Substitution conventions

- `%` anywhere in a string is replaced at display time with the current
  brother's name: `Julian` (brother 1), `Phillip` (brother 2), or `Kevin`
  (brother 3). Substitution is performed by `extract()` (`fmain2.c:510`).
- A `chr(13)` byte embedded mid-string forces a scroll-line break. The two
  segments before and after are printed as separate scroll lines.
- Strings in the **placard** tables use a different rendering path (`_ssp`)
  which interprets coordinate-byte headers; those strings are already
  split into positioned lines — see [§5 Placard screens](#5-placard-screens).

---

## 1. Event messages — `_event_msg` (`narr.asm:11`)

Called via `event(N)` (`fmain2.c:556`). Printed to the HI scroll area via
`extract()`, so `%` is substituted and word-wrap is applied.

| N | Text | Primary trigger |
|---|------|----------------|
| 0 | `% was getting rather hungry.` | `hunger == 35` (`fmain.c:2203`) |
| 1 | `% was getting very hungry.` | `hunger == 60` (`fmain.c:2204`) |
| 2 | `% was starving!` | `hunger > 90` (`fmain.c:2209`) |
| 3 | `% was getting tired.` | `fatigue == 70` (`fmain.c:2218`) |
| 4 | `% was getting sleepy.` | `hunger == 90` (fatigue branch, `fmain.c:2219`) |
| 5 | `% was hit and killed!` | `checkdead()` — mortal blow (`fmain2.c:246`) |
| 6 | `% was drowned in the water!` | `checkdead()` — drowning (`fmain2.c:246`) |
| 7 | `% was burned in the lava.` | `checkdead()` — lava (`fmain2.c:246`) |
| 8 | `% was turned to stone by the witch.` | `checkdead()` — witch gaze (`fmain2.c:246`) |
| 9 | `% started the journey in his home village of Tambry` | Brother succession — start of journey; see note below |
| 10 | `as had his brother before him.` | Brother succession — brother 2 (`fmain.c:2890`) |
| 11 | `as had his brothers before him.` | Brother succession — brother 3 (`fmain.c:2891`) |
| 12 | `% just couldn't stay awake any longer!` | `hunger > 90`, forced sleep (`fmain.c:2212`) |
| 13 | `% was feeling quite full.` | `eat()` — hunger drops to 0 or below (`fmain2.c:1706`) |
| 14 | `% was feeling quite rested.` | *(no call site found in source — may be unused)* |
| 15 | `Even % would not be stupid enough to draw weapon in here.` | Entering temple/holy zone while armed (`fmain.c:1413`) |
| 16 | `A great calming influence comes over %, preventing him from drawing his weapon.` | Entering dragon shrine / forbidden zone (`fmain.c:1414`) |
| 17 | `% picked up a scrap of paper.` | Picking up `SCRAP` object (`fmain.c:3161`) |
| 18 | `It read: "Find the turtle!"` | Scrap pickup in regions 0–7 (`fmain.c:3162`) |
| 19 | `It read: "Meet me at midnight at the Crypt. Signed, the Wraith Lord."` | Scrap pickup in regions 8–9 (`fmain.c:3162`) |
| 20 | `% looked around but discovered nothing.` | LOOK with no hidden objects in range (`fmain.c:3297`) |
| 21 | `% does not have that item.` | MAGIC or USE with item not in inventory (`fmain.c:3303`) |
| 22 | `% bought some food and ate it.` | BUY food from tavern keeper (`fmain.c:3433`) |
| 23 | `% bought some arrows.` | BUY arrows from tavern keeper (`fmain.c:3434`) |
| 24 | `% passed out from hunger!` | Starvation collapse (`fmain.c:2214`) |
| 25 | `% is not sleepy.` | REST command — `fatigue < 50` (`fmain.c:1881`) |
| 26 | `% was tired, so he decided to lie down and sleep.` | REST command — `fatigue >= 50` (`fmain.c:1883`) |
| 27 | `% perished in the hot lava!` | Lava death — `fiery_death` flag set (`fmain.c:1418`) |
| 28 | `It was midnight.` | `dayperiod == 0` transition (`fmain.c:2032`) |
| 29 | `It was morning.` | `dayperiod == 4` transition (`fmain.c:2033`) |
| 30 | `It was midday.` | `dayperiod == 6` transition (`fmain.c:2034`) |
| 31 | `Evening was drawing near.` | `dayperiod == 9` transition (`fmain.c:2035`) |
| 32 | `Ground is too hot for swan to land.` | Attempting to dismount swan over lava (`fmain.c:1418`, `event(32)`) |
| 33 | `Flying too fast to dismount.` | Attempting to dismount swan at speed (`fmain.c:1427`) |
| 34 | `"They're all dead!" he cried.` | MAGIC spell kills all enemies in encounter (`fmain.c:3362`) |
| 35 | `No time for that now!` | TAKE attempted on a living actor (during combat, `fmain.c:3285`) |
| 36 | `% put an apple away for later.` | Picking up food when not hungry (`fmain.c:3166`) |
| 37 | `% ate one of his apples.` | Auto-eating food in safe zone (`fmain.c:2196`) |
| 38 | `% discovered a hidden object.` | LOOK reveals a hidden object (`fmain.c:3297`) |

**Notes on combined event sequences:**

`event(9)` does **not** end with a period — it is always followed immediately by
one of:
- `print_cont(".")` — brother 1 (Julian, `fmain.c:2889`)
- `event(10)` — brother 2 (Phillip, `fmain.c:2890`)
- `event(11)` — brother 3 (Kevin, `fmain.c:2891`)

This produces three complete messages:
- Julian: `Julian started the journey in his home village of Tambry.`
- Phillip: `Phillip started the journey in his home village of TambryAs had his brother before him.`
- Kevin: `Kevin started the journey in his home village of TambryAs had his brothers before him.`

No call site for `event(14)` was found in `fmain.c` or `fmain2.c` — the
"quite rested" string appears to be unused in the shipped game.

---

## 2. NPC speeches — `_speeches` (`narr.asm:351`)

Called via `speak(N)` (`fmain2.c:561`). Printed through `extract()` to the
HI scroll area. `%` is replaced with the brother name. `chr(13)` forces a
line break within the speech.

| N | Text | Speaker / condition |
|---|------|---------------------|
| 0 | `% attempted to communicate with the Ogre but a guttural snarl was the only response.` | Enemy type 0 (Ogre) — `speak(an->race)` (`fmain.c:3422`) |
| 1 | `"Human must die!" said the goblin-man.` | Enemy type 1 (Orc/Goblin-man) |
| 2 | `"Doom!" wailed the wraith.` | Enemy type 2 (Wraith) |
| 3 | `A clattering of bones was the only reply.` | Enemy type 3 (Skeleton) |
| 4 | `% knew that it is a waste of time to talk to a snake.` | Enemy type 4 (Snake) |
| 5 | `...` | Enemy type 5 (Salamander) |
| 6 | `There was no reply.` | Enemy type 6 (Loraii) |
| 7 | `"Die, foolish mortal!" he said.` | Enemy type 7 (Necromancer) — proximity trigger (`fmain.c:2100`) or enemy TALK |
| 8 | `"No need to shout, son!" he said.` | TALK to any setfig while yelling / too close (`fmain.c:3373`) |
| 9 | `"Nice weather we're having, isn't it?" queried the ranger.` | Ranger (setfig 12) goal 0 (`fmain.c:3413`, `speak(53+an->goal)` → 53) |
| 10 | `"Good luck, sonny!" said the ranger. "Hope you win!"` | Ranger goal 1 (`speak(54)`) |
| 11 | `"If you need to cross the lake" said the ranger, "There's a raft just north of here."` | Ranger goal 2 (`speak(55)`) — region 2 |
| 12 | `"Would you like to buy something?" said the tavern keeper. "Or do you just need lodging for the night?"` | Bartender (setfig 8) — `dayperiod > 7` (`fmain.c:3407`) |
| 13 | `"Good Morning." said the tavern keeper. "Hope you slept well."` | Bartender — `fatigue < 5` (`fmain.c:3406`) |
| 14 | `"Have a drink!" said the tavern keeper."` | Bartender — default (`fmain.c:3408`) |
| 15 | `"State your business!" said the guard.`↵`"My business is with the king." stated %, respectfully.` | Guard (setfig 2 or 3) (`fmain.c:3396`) |
| 16 | `"Please, sir, rescue me from this horrible prison!" pleaded the princess.` | Princess (setfig 4) — `ob_list8[9].ob_stat != 0` (`fmain.c:3397`) |
| 17 | `"I cannot help you, young man." said the king. "My armies are decimated, and I fear that with the loss of my children, I have lost all hope."` | King (setfig 5) — `ob_list8[9].ob_stat != 0` (`fmain.c:3398`) |
| 18 | `"Here is a writ designating you as my official agent. Be sure and show this to the Priest before you leave Marheim.` | King — delivered via `speak(18)` after princess/king cutscene (`fmain2.c:1599`) |
| 19 | `"I'm afraid I cannot help you, young man. I already gave the golden statue to the other young man.` | Priest (setfig 1) — writ presented but `ob_listg[10].ob_stat` already set (`fmain.c:3385`) |
| 20 | `"If you could rescue the king's daughter," said Lord Trane, "The King's courage would be restored."` | Noble (setfig 6) (`fmain.c:3399`) |
| 21 | `"Sorry, I have no use for it."` | Attempting to GIVE bones to any NPC who doesn't accept them (`fmain.c:3502`) |
| 22 | `"The dragon's cave is directly north of here." said the ranger."` | Ranger (setfig 12) in region 2 (`fmain.c:3412`) |
| 23 | `"Alms! Alms for the poor!"` | Beggar (setfig 13) proximity trigger or TALK (`fmain.c:3415`, `fmain.c:2097`) |
| 24 | `"I have a prophecy for you, m'lord." said the beggar. "You must seek two women, one Good, one Evil."` | Beggar GIVE response — goal 0 (`fmain.c:3498`, `speak(24+an->goal)`) |
| 25 | `"Lovely Jewels, glint in the night - give to us the gift of Sight!" he said.` | Beggar GIVE — goal 1 |
| 26 | `"Where is the hidden city? How can you find it when you cannot even see it?" said the beggar.` | Beggar GIVE — goal 2 |
| 27 | `"Kind deeds could gain thee a friend from the sea."` | Wizard (setfig 0) — `kind >= 10`, goal 0 (`speak(27+an->goal)`, `fmain.c:3381`) |
| 28 | `"Seek the place that is darker than night - There you shall find your goal in sight!" said the wizard, cryptically.` | Wizard — goal 1 |
| 29 | `"Like the eye itself, a crystal Orb can help to find things concealed."` | Wizard — goal 2 |
| 30 | `"The Witch lives in the dim forest of Grimwood, where the very trees are warped to her will. Her gaze is Death!"` | Wizard — goal 3 |
| 31 | `"Only the light of the Sun can destroy the Witch's Evil."` | Wizard — goal 4 |
| 32 | `"The maiden you seek lies imprisoned in an unreachable castle surrounded by unclimbable mountains."` | Wizard — goal 5 |
| 33 | `"Tame the golden beast and no mountain may deny you! But what rope could hold such a creature?"` | Wizard — goal 6 |
| 34 | `"Just what I needed!" he said.` | Wizard — goal 7 |
| 35 | `"Away with you, young ruffian!" said the Wizard. "Perhaps you can find some small animal to torment if that pleases you!"` | Wizard — `kind < 10` (`fmain.c:3380`) |
| 36 | `"You must seek your enemy on the spirit plane. It is hazardous in the extreme. Space may twist, and time itself may run backwards!"` | Priest (setfig 1) healing — `daynight % 3 == 0` (`speak(36+(daynight%3))`, `fmain.c:3390`) |
| 37 | `"When you wish to travel quickly, seek the power of the Stones." he said.` | Priest — `daynight % 3 == 1` |
| 38 | `"Since you are brave of heart, I shall Heal all your wounds."`↵`Instantly % felt much better.` | Priest — `daynight % 3 == 2`; vitality is also restored (`fmain.c:3391`) |
| 39 | `"Ah! You have a writ from the king. Here is one of the golden statues of Azal-Car-Ithil. Find all five and you'll find the vanishing city."` | Priest — writ presented first time; `ob_listg[10].ob_stat` set to 1 (`fmain.c:3385`) |
| 40 | `"Repent, Sinner! Thou art an uncouth brute and I have no interest in your conversation!"` | Priest — `kind < 10` and no writ (`fmain.c:3388`) |
| 41 | `"Ho there, young traveler!" said the black figure. "None may enter the sacred shrine of the People who came Before!"` | Dark Knight (race 7) proximity trigger (`fmain.c:2101`) |
| 42 | `"Your prowess in battle is great." said the Knight of Dreams. "You have earned the right to enter and claim the prize."` | Dark Knight after combat victory (`fmain.c:2775`) |
| 43 | `"So this is the so-called Hero who has been sent to hinder my plans. Simply Pathetic. Well, try this, young Fool!"` | Necromancer (race 9) proximity trigger (`fmain.c:2100`) |
| 44 | `% gasped. The Necromancer had been transformed into a normal man. All of his evil was gone.` | Necromancer death / quest completion (used in win sequence) |
| 45 | `"%." said the Sorceress. "Welcome. Here is one of the five golden figurines you will need."`↵`"Thank you." said %.` | Sorceress (setfig 7) — first visit; `ob_listg[9].ob_stat` set to 1 (`fmain.c:3403`) |
| 46 | `"Look into my eyes and Die!!" hissed the witch.`↵`"Not a chance!" replied %` | Witch (setfig 9) TALK or proximity (`fmain.c:3409`, `fmain.c:2098`) |
| 47 | `The Spectre spoke. "HE has usurped my place as lord of undead. Bring me bones of the ancient King and I'll help you destroy him."` | Spectre (setfig 10) (`fmain.c:3410`) |
| 48 | `% gave him the ancient bones.`↵`"Good! That spirit now rests quietly in my halls. Take this crystal shard."` | Giving ancient bones to Spectre (`fmain.c:3503`) |
| 49 | `"%..." said the apparition. "I am the ghost of your dead brother. Find my bones -- there you will find some things you need.` | Ghost (setfig 11) (`fmain.c:3411`) |
| 50 | `% gave him some gold coins. `↵`"Why, thank you, young sir!"` | GIVE gold to any non-beggar NPC (`fmain.c:3499`) |
| 51 | `"Sorry, but I have nothing to sell."` | BUY attempted near a non-tavern-keeper setfig (proximity buy with wrong race) |
| 52 | *(empty — no text)* | `speak(52)` — no-op; empty slot (`narr.asm`, between speeches 51 and 53) |
| 53 | `"The dragon's cave is east of here." said the ranger."` | Ranger (setfig 12) outside region 2 — goal 0 (`speak(53+an->goal)`, `fmain.c:3413`) |
| 54 | `"The dragon's cave is west of here." said the ranger."` | Ranger — goal 1 |
| 55 | `"The dragon's cave is south of here." said the ranger."` | Ranger — goal 2 |
| 56 | `"Oh, thank you for saving my eggs, kind man!" said the turtle. "Take this seashell as a token of my gratitude."` | Turtle carrier — first encounter, shell not yet held (`fmain.c:3420`) |
| 57 | `"Just hop on my back if you need a ride somewhere." said the turtle.` | Turtle — shell already in inventory (`fmain.c:3419`) |
| 58 | `"Stupid fool, you can't hurt me with that!"` | Witch or Necromancer — struck with wrong weapon (`fmain2.c:234`) |
| 59 | `"Your magic won't work here, fool!"` | Attempting to use magic inside the Necromancer's arena — xtype 9 (`fmain.c:3304`) |
| 60 | `The Sunstone has made the witch vulnerable!` | Using Sun Stone while witch is present — `witchflag` set (`fmain.c:3462`) |

---

## 3. Place-name messages — `_place_msg` (`narr.asm:164`)

Called via `msg(place_msg, N)` from the location-detection loop
(`fmain.c:2672`). The index N is looked up in `_place_tbl`
(`narr.asm:86`): the table maps sector-id ranges to message indices.
Index 0 and 1 are sentinel values — no message is printed for them.

| N | Text |
|---|------|
| 0 | *(no message — sentinel)* |
| 1 | *(no message — do not change)* |
| 2 | `% returned to the village of Tambry.` |
| 3 | `% came to Vermillion Manor.` |
| 4 | `% reached the Mountains of Frost` |
| 5 | `% reached the Plain of Grief.` |
| 6 | `% came to the city of Marheim.` |
| 7 | `% came to the Witch's castle.` |
| 8 | `% came to the Graveyard.` |
| 9 | `% came to a great stone ring.` |
| 10 | `% came to a watchtower.` |
| 11 | `% traveled to the great Bog.` |
| 12 | `% came to the Crystal Palace.` |
| 13 | `% came to mysterious Pixle Grove.` |
| 14 | `% entered the Citadel of Doom.` |
| 15 | `% entered the Burning Waste.` |
| 16 | `% found an oasis.` |
| 17 | `% came to the hidden city of Azal.` |
| 18 | `% discovered an outlying fort.` |
| 19 | `% came to a small keep.` |
| 20 | `% came to an old castle.` |
| 21 | `% came to a log cabin.` |
| 22 | `% came to a dark stone tower.` |
| 23 | `% came to an isolated cabin.` |
| 24 | `% came to the Tombs of Hemsath.` |
| 25 | `% reached the Forbidden Keep.` |
| 26 | `% found a cave in the hillside.` |

**Note on message 4:** The source string at `narr.asm:169` is missing its
trailing period: `"% reached the Mountains of Frost"` (no `.`). This is
faithful to the original.

---

## 4. Interior messages — `_inside_msg` (`narr.asm:199`)

Called via `msg(inside_msg, N)` from the same location-detection loop.
Index N is looked up in `_inside_tbl` (`narr.asm:117`). Index 0 and 1 are
sentinels.

| N | Text |
|---|------|
| 0 | *(no message — sentinel)* |
| 1 | *(no message — do not change)* |
| 2 | `% came to a small chamber.` |
| 3 | `% came to a large chamber.` |
| 4 | `% came to a long passageway.` |
| 5 | `% came to a twisting tunnel.` |
| 6 | `% came to a forked intersection.` |
| 7 | `He entered the keep.` |
| 8 | `He entered the castle.` |
| 9 | `He entered the castle of King Mar.` |
| 10 | `He entered the sanctuary of the temple.` |
| 11 | `% entered the Spirit Plane.` |
| 12 | `% came to a large room.` |
| 13 | `% came to an octagonal room.` |
| 14 | `% traveled along a stone corridor.` |
| 15 | `% came to a stone maze.` |
| 16 | `He entered a small building.` |
| 17 | `He entered the building.` |
| 18 | `He entered the tavern.` |
| 19 | `He went inside the inn.` |
| 20 | `He entered the crypt.` |
| 21 | `He walked into the cabin.` |
| 22 | `He unlocked the door and entered.` |

**Note on messages 7–10, 16–22:** These use the fixed pronoun `"He"` rather
than `%`. No substitution is performed — the text is unconditional regardless
of which brother is active. This is faithful to the original.

---

## 5. Copy-protection questions — `_question` (`narr.asm:63`)

Called via `question(N)` (`fmain2.c:1317`). There are 8 questions
(indices 0–7). Each is printed via `print_cont()` directly — no `%`
substitution, no word-wrap scroll. They appear on the copy-protection
input screen. The correct answers come from a manual look-up table not
reproduced here.

| N | Question text |
|---|--------------|
| 0 | `To Quest for the...?` |
| 1 | `Make haste, but take...?` |
| 2 | `Scorn murderous...?` |
| 3 | `Summon the...?` |
| 4 | `Wing forth in...?` |
| 5 | `Hold fast to your...?` |
| 6 | `Defy Ye that...?` |
| 7 | `In black darker than...?` |

---

## 6. Placard screens — `_placard_text` (`narr.asm:235`)

Placard messages are rendered via `placard_text(N)` onto the full-screen map
overlay (not the HI scroll area) using a byte-stream protocol with coordinate
headers of the form `chr(128), x/2, y`. The caller inserts hero names by
calling `name()` between segments; no `%` substitution happens at this layer.

**The authoritative list of all 20 placard messages — including indices,
assembly labels, rendered `(x, y)` pen positions, per-line text fragments,
calling-site context, and calling-sequence patterns — lives in
[placard.md § Message table](placard.md#message-table).** Do not duplicate
that table here; edits and cross-references should target `placard.md`.

Notable source-level typos documented in `placard.md` rather than patched in
the source (which is read-only):

- `msg7` at `narr.asm:294` spells "villanous" (one `l`) — faithful to author
  intent; preserved in the spec.
- `msg8a` at `narr.asm:307` contains a stray comma (`"their love for each, "`)
  that produces the rendered glitch "each, other,". This is a typo against
  author intent (the parallel msg9a / msg10a have `"each other,"`). The
  `placard.md` table deliberately omits the stray comma. See
  [placard.md § Deliberate deviation from source: the msg8a typo](placard.md#deliberate-deviation-from-source-the-msg8a-typo).
