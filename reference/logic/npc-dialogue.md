# NPC Dialogue — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c, narr.asm
> Cross-refs: [RESEARCH §13](../RESEARCH.md#13-npc-dialogue--quests), [STORYLINE.md](../STORYLINE.md), [_discovery/npc-quests.md](../_discovery/npc-quests.md), [quests.md](quests.md#give_item_to_npc), [game-loop.md](game-loop.md#no_motion_tick)

## Overview

`CMODE_TALK` (Yell / Say / Ask) and proximity auto-speech form the conversation
half of NPC interaction; `CMODE_GIVE` (quest-item transfer) is the acceptance
half and is already captured by
[`give_item_to_npc`](quests.md#give_item_to_npc). This file documents only the
dialogue side.

The TALK handler at `fmain.c:3367-3423` is a monolithic `switch` on the low
seven bits of the target's race. Six NPC types (wizard, priest, sorceress,
bartender, ranger, carrier/turtle) have non-trivial speech selection; the rest
are flat `speak(N)` calls. The non-trivial bodies are extracted as helpers
here (`wizard_hint`, `priest_speech`, `bartender_speech`, `ranger_hint`); the
sorceress and turtle branches stay inline in `talk_dispatch` because each is a
single `if` and also writes quest-flag state.

Proximity auto-speech (`proximity_auto_speak`) is the passive counterpart: it
runs every time the map is stationary and the closest actor changes, and
fires a one-shot greeting for five NPC races. It shares the narr.asm speech
table with `talk_dispatch` but is not triggered by any menu action.

## Symbols

No new locals beyond each function's declared parameters. All other names
resolve in [SYMBOLS.md](SYMBOLS.md). Proposed SYMBOLS additions (`setfig_table`,
`fatigue`, `dayperiod`, `nearest`, speech-range constants, NPC-kindness gate,
talk-tactic timer, setfig case-index constants) are listed in the wave report.

## talk_dispatch

Source: `fmain.c:3367-3423`
Called by: `option_handler` (via `do_option` `CMODE_TALK` branch)
Calls: `nearest_fig`, `speak`, `wizard_hint`, `priest_speech`, `bartender_speech`, `ranger_hint`, `prq`, `rand64`, `anim_list`, `setfig_table`, `stuff`, `ob_list8`, `ob_listg`

```pseudo
def talk_dispatch(hit: int) -> None:
    """Dispatch CMODE_TALK (Yell/Say/Ask) to the target's NPC-specific speech selector."""
    # hit encodings come from labels3[] at fmain.c:497: 5 = Yell, 6 = Say, 7 = Ask.
    # Yell doubles the search radius and fires a "no need to shout" cutoff.
    if hit == 5:                                         # fmain.c:3368 — 5 = TALK Yell slot
        j = nearest_fig(1, 100)                          # fmain.c:3368 — 100 px yell radius
    else:
        j = nearest_fig(1, 50)                           # fmain.c:3368 — 50 px say/ask radius
    if nearest == 0:                                     # fmain.c:3369 — no target within range
        return
    an = anim_list[nearest]                              # fmain.c:3370
    if an.state == STATE_DEAD:                           # fmain.c:3371 — corpses do not reply
        return
    if an.type == SETFIG:                                # fmain.c:3372 — humanoid NPC
        if hit == 5 and j < 35:                          # fmain.c:3373 — 5 = Yell, 35 px = shout-too-close
            speak(8)                                     # narr.asm — "No need to shout, son!"
            return
        k = an.race & 0x7f                               # fmain.c:3374 — 0x7f strips SETFIG_RACE_BIT
        if setfig_table[k].can_talk:                     # fmain.c:3375 — only 5 setfigs animate
            an.state = STATE_TALKING                     # fmain.c:3376
            an.tactic = 15                               # fmain.c:3377 — 15-tick talking-animation timer
        match k:
            case 0:                                      # fmain.c:3380 — wizard
                wizard_hint(an)
            case 1:                                      # fmain.c:3382 — priest
                priest_speech()
            case 2:                                      # fmain.c:3395 — guard (front)
                speak(15)                                # narr.asm — "State your business!"
            case 3:                                      # fmain.c:3396 — guard (back)
                speak(15)                                # narr.asm — "State your business!"
            case 4:                                      # fmain.c:3397 — princess
                if ob_list8[9].ob_stat != 0:             # fmain.c:3397 — 9 = princess-captive slot
                    speak(16)                            # narr.asm — "Please, sir, rescue me..."
            case 5:                                      # fmain.c:3398 — king
                if ob_list8[9].ob_stat != 0:             # fmain.c:3398 — princess-captive flag
                    speak(17)                            # narr.asm — "I cannot help you..."
            case 6:                                      # fmain.c:3399 — noble
                speak(20)                                # narr.asm — "If you could rescue the king's daughter..."
            case 7:                                      # fmain.c:3400 — sorceress
                if ob_listg[9].ob_stat != 0:             # fmain.c:3401 — 9 = sorceress-statue slot; already given
                    if luck < rand64():                  # fmain.c:3402 — rand64() in [0,63]
                        luck = luck + 5                  # fmain.c:3402 — 5 = luck boost per repeat visit
                else:
                    speak(45)                            # narr.asm — "Welcome. Here is one of the five golden figurines..."
                    ob_listg[9].ob_stat = 1              # fmain.c:3403 — 1 = ground/pickable statue
                prq(7)                                   # fmain.c:3404 — 7 = gift-voice priority
            case 8:                                      # fmain.c:3406 — bartender
                bartender_speech()
            case 9:                                      # fmain.c:3409 — witch
                speak(46)                                # narr.asm — "Look into my eyes and Die!!"
            case 10:                                     # fmain.c:3410 — spectre
                speak(47)                                # narr.asm — "HE has usurped my place..."
            case 11:                                     # fmain.c:3411 — ghost
                speak(49)                                # narr.asm — "I am the ghost of your dead brother..."
            case 12:                                     # fmain.c:3412 — ranger
                ranger_hint(an)
            case 13:                                     # fmain.c:3415 — beggar
                speak(23)                                # narr.asm — "Alms! Alms for the poor!"
        return
    if an.type == CARRIER and active_carrier == 5:       # fmain.c:3418 — 5 = RIDING_TURTLE / turtle carrier
        if stuff[6] != 0:                                # fmain.c:3419 — 6 = Sea Shell slot
            speak(57)                                    # narr.asm — "Just hop on my back..."
        else:
            stuff[6] = 1                                 # fmain.c:3420 — grant Sea Shell
            speak(56)                                    # narr.asm — "Oh, thank you for saving my eggs..."
        return
    if an.type == ENEMY:                                 # fmain.c:3422
        speak(an.race)                                   # narr.asm — speeches 0..9 keyed by enemy race
```

**Notes.**

- The Yell-cutoff `speak(8)` at `fmain.c:3373` exits the TALK handler entirely
  and does **not** enter the NPC-type switch. A target close enough to Yell
  never animates, regardless of `setfig_table[k].can_talk`.
- `can_talk` (column 2 of `setfig_table`) only gates the animation and the
  15-tick `tactic` timer. Every setfig still has a speech branch in the
  switch; the animation simply does not play for guards, princess, king,
  noble, sorceress, bartender, witch, spectre, or ghost.
- The sorceress branch writes a quest flag (`ob_listg[9].ob_stat`) and
  rewards luck on repeat visits — it is kept inline because it is the only
  setfig case that mutates multiple globals.
- `speak(an.race)` for ENEMY exploits the 1:1 mapping of enemy races (0..9)
  to speech indices 0..9 — this is why the narr table's first ten rows are
  the enemy banter lines. Woodcutter (race 10) falls out of `speak`'s
  defined range but is never addressed via TALK (setfig types 0..13 capture
  all on-map humanoids).

## wizard_hint

Source: `fmain.c:3380-3381`
Called by: `talk_dispatch`
Calls: `speak`

```pseudo
def wizard_hint(an: Shape) -> None:
    """Wizard speech: kind<10 scolds, otherwise goal-indexed hint."""
    if kind < 10:                                        # fmain.c:3380 — 10 = wizard kindness gate
        speak(35)                                        # narr.asm — "Away with you, young ruffian!"
        return
    speak(27 + an.goal)                                  # fmain.c:3381 — 27..34 = wizard goal-hint base
```

**Goal indexing.** The wizard's `goal` field is set at spawn from the object
list slot (`fmain2.c:1275`), so eight distinct wizards placed in the object
tables yield eight distinct hints (`speak(27)` through `speak(34)`), one per
quest objective. See [_discovery/npc-quests.md](../_discovery/npc-quests.md#speech-index)
for the full 27..34 speech table.

## priest_speech

Source: `fmain.c:3382-3394`
Called by: `talk_dispatch`
Calls: `speak`, `prq`, `stuff`, `ob_listg`, `anim_list`

```pseudo
def priest_speech() -> None:
    """Priest speech: writ path (gives statue once), kind<10 rebuke, else heal + rotating hint."""
    if stuff[28] != 0:                                   # fmain.c:3383 — 28 = Writ slot
        if ob_listg[10].ob_stat == 0:                    # fmain.c:3384 — 10 = priest-statue slot; not yet given
            speak(39)                                    # narr.asm — "Ah! You have a writ... Here is one of the golden statues..."
            ob_listg[10].ob_stat = 1                     # fmain.c:3385 — 1 = ground/pickable statue
        else:
            speak(19)                                    # narr.asm — "already gave the golden statue"
        return
    if kind < 10:                                        # fmain.c:3388 — 10 = priest kindness gate
        speak(40)                                        # narr.asm — "Repent, Sinner!"
        return
    # Kindly visitor with no writ: rotating daily hint + free heal.
    speak(36 + (daynight % 3))                           # fmain.c:3390 — 36..38 = rotating priest hints
    anim_list[0].vitality = 15 + brave / 4               # fmain.c:3391 — 15 = heal baseline, /4 bravery bonus
    prq(4)                                               # fmain.c:3392 — 4 = heal-voice priority
```

**Heal ceiling.** The priest sets vitality absolutely to `15 + brave/4`, not
capped against `anim_list[0].vitality`'s current value. For a low-bravery
hero this can actually *lower* HP if the hero is currently full. The
three-way daily hint (`daynight % 3`) persists across game loads because
`daynight` is saved (see [save-load](../_discovery/save-load.md)).

## bartender_speech

Source: `fmain.c:3406-3408`
Called by: `talk_dispatch`
Calls: `speak`

```pseudo
def bartender_speech() -> None:
    """Bartender speech: fatigue<5 morning greeting; else time-of-day split."""
    if fatigue < 5:                                      # fmain.c:3406 — 5 = rested-sleep threshold
        speak(13)                                        # narr.asm — "Good Morning..."
        return
    if dayperiod > 7:                                    # fmain.c:3407 — 7 = evening dayperiod cutoff
        speak(12)                                        # narr.asm — "Would you like to buy something?"
        return
    speak(14)                                            # narr.asm — "Have a drink!"
```

**Branch coverage.** The three cases are mutually exclusive: rested hero
hears the morning greeting regardless of time; tired hero at night hears the
shop prompt; tired hero in daytime hears the drink pitch. The actual shop
menu (`CMODE_BUY`) is opened by a separate action and its transactions are
documented in [RESEARCH §13.6](../RESEARCH.md#136-shop-system).

## ranger_hint

Source: `fmain.c:3412-3414`
Called by: `talk_dispatch`
Calls: `speak`

```pseudo
def ranger_hint(an: Shape) -> None:
    """Ranger speech: region-2 override, otherwise goal-indexed dragon-cave hint."""
    if region_num == 2:                                  # fmain.c:3412 — 2 = Grimwood region with cave hint
        speak(22)                                        # narr.asm — "The dragon's cave is directly north of here."
        return
    speak(53 + an.goal)                                  # fmain.c:3413 — 53..55 = ranger goal-hint base
```

**Goal indexing.** As with wizards, each ranger's spawn-time `goal` selects
one of three directional hints (`speak(53)`, `speak(54)`, `speak(55)`). The
region-2 ranger bypasses this and always delivers `speak(22)`.

## proximity_auto_speak

Source: `fmain.c:2094-2103`
Called by: `no_motion_tick` (Phase 14, see [game-loop.md](game-loop.md#no_motion_tick))
Calls: `speak`, `anim_list`, `ob_list8`

```pseudo
def proximity_auto_speak() -> None:
    """Fire a one-shot greeting when nearest_person changes; suppress repeats via last_person."""
    k = anim_list[nearest_person].race                   # fmain.c:2094
    if nearest_person == 0:                              # fmain.c:2095 — no target cached this tick
        return
    if k == last_person:                                 # fmain.c:2095 — already greeted this NPC
        return
    match k:
        case 0x8d:                                       # fmain.c:2097 — 0x8d = beggar setfig race
            speak(23)                                    # narr.asm — "Alms! Alms for the poor!"
        case 0x89:                                       # fmain.c:2098 — 0x89 = witch setfig race
            speak(46)                                    # narr.asm — "Look into my eyes and Die!!"
        case 0x84:                                       # fmain.c:2099 — 0x84 = princess setfig race
            if ob_list8[9].ob_stat != 0:                 # fmain.c:2099 — 9 = princess-captive slot
                speak(16)                                # narr.asm — "Please, sir, rescue me..."
        case 9:                                          # fmain.c:2100 — 9 = RACE_NECROMANCER
            speak(43)                                    # narr.asm — "So this is the so-called Hero..."
        case 7:                                          # fmain.c:2101 — 7 = RACE_DKNIGHT
            speak(41)                                    # narr.asm — "Ho there, young traveler!"
    last_person = k                                      # fmain.c:2103 — suppress repeats of this race
```

**Triggering.** `nearest_person` is recomputed by `sort_sprites`
(Phase 19, see [game-loop.md](game-loop.md#sort_sprites)) as the closest
non-hero actor within 50 px of the hero. `proximity_auto_speak` then
compares the freshly-cached race against `last_person` and fires at most
one `speak` per tick. The suppression is **keyed on race**, not on actor
index, so leaving and re-entering the same NPC's range re-fires the
speech only if a different race was encountered in between.

**Scope.** Only five races have auto-speech hooks. Every other setfig and
enemy race is silent on proximity — hero must open the TALK menu to
address them.

## Notes

- **GIVE dispatch** is not re-documented here. The `CMODE_GIVE` branch at
  `fmain.c:3490-3505` dispatches gold-to-beggar (`speak(24 + goal)`),
  bone-to-spectre (`speak(48)` + shard drop), generic gold thanks
  (`speak(50)`), and no-match silence. See
  [`give_item_to_npc`](quests.md#give_item_to_npc) for the pseudo-code.
- **Guard behavior.** Cases 2 and 3 (front / back guards) both fire
  `speak(15)`. The back-guard variant exists for sprite orientation, not
  for dialogue differentiation.
- **Witch / spectre / ghost** cases in TALK are cosmetically symmetric with
  their proximity greetings, but the witch's auto-speech fires whenever
  she is the nearest actor, whereas the TALK `speak(46)` fires only when
  the player explicitly selects Yell/Say/Ask — they share speech index 46.
- **Beggar parallel.** The beggar's TALK speech (`speak(23)` at
  `fmain.c:3415`) is identical to her proximity greeting. The
  goal-indexed prophecies (`speak(24 + goal)`) are gated behind GIVE Gold,
  not TALK; see `give_item_to_npc`.
- **Healer timing.** The priest's heal at `fmain.c:3391` runs inside
  `priest_speech`, which is invoked from inside `talk_dispatch`. Because
  `talk_dispatch` runs during menu dispatch (not Phase 14), the heal takes
  effect on the same tick the menu slot is clicked.
