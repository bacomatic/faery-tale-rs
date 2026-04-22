# Quests & Win Condition — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c, fmain2.c
> Cross-refs: [RESEARCH §6](../RESEARCH.md#6-items--inventory), [RESEARCH §18](../RESEARCH.md#18-quest-progression), [STORYLINE.md](../STORYLINE.md), [_discovery/npc-quests.md](../_discovery/npc-quests.md), [_discovery/quest-stat-items.md](../_discovery/quest-stat-items.md), [_discovery/win-condition.md](../_discovery/win-condition.md)

## Overview

Quest progression in *Faery Tale Adventure* is not driven by a central scripting
system. It is a sparse mesh of flag reads and writes spread across the
inventory-pickup, menu-dispatch, actor-tick, and extent-trigger pipelines. Seven
small resolver bodies capture every quest state transition:

1. `give_item_to_npc` — the `GIVE` submenu handler in `do_option`.
2. `check_quest_flag` — `stuff_flag`, the menu-enable helper that reads an
   inventory/quest-item slot.
3. `necromancer_death_drop` — the race-9 / race-0x89 branch of the
   `STATE_DYING → STATE_DEAD` transition in `actor_tick`, which drops the
   Talisman and the Golden Lasso.
4. `leave_item` — the one-slot ground-drop helper used by the death branch and
   by the Spectre-exchange branch of `give_item_to_npc`.
5. `rescue` — the princess-rescue cinematic invoked when the hero enters the
   princess extent.
6. `get_turtle` — the `USE Shell` handler that spawns a turtle carrier on the
   nearest deep-water tile.
7. `try_win_condition` — the Talisman-pickup post-condition inside the `TAKE`
   handler.
8. `end_game_sequence` — `win_colors`, the sunrise-and-win-picture finale.

All branching on `(NPC race, item)` lives inside `give_item_to_npc`; there is
no table-driven dispatcher in the source. Most other quest gates are checked
inline by the subsystem that owns them (combat, movement, doors) and are
documented in their respective logic files — this doc only captures the
bodies whose primary purpose is quest state, not combat or movement.

## Symbols

No new locals beyond each function's declared parameters. All global
identifiers resolve in [SYMBOLS.md](SYMBOLS.md) or in each function's `Calls:`
header. Proposed SYMBOLS additions (quest `stuff[]` slot constants, object IDs,
speech/placard indices, rescue reward constants) are listed in the wave report
— this doc therefore cites numeric literals inline.

## give_item_to_npc

Source: `fmain.c:3490-3508`
Called by: `option_handler` (via `do_option` `CMODE_GIVE` branch)
Calls: `speak`, `leave_item`, `rand64`, `gomenu`, `anim_list`, `stuff`, `wealth`

```pseudo
def give_item_to_npc(hit: int) -> None:
    """Dispatch the GIVE submenu on (slot hit, target race): gold, bone, or no-op."""
    # fmain.c:3491 — require a nearby actor cached by the last TALK/GIVE proximity scan.
    if nearest_person == 0:
        return
    k = anim_list[nearest_person].race                       # fmain.c:3492
    if hit == 5 and wealth > 2:                              # fmain.c:3493 — 5 = GIVE slot "Gold", 2 = min kept
        # Give gold: -2 wealth; probabilistic kindness bump; beggar gets goal-indexed prophecy.
        wealth = wealth - 2                                  # fmain.c:3495
        if rand64() > kind:                                  # fmain.c:3496 — rand64() in [0,63]
            kind = kind + 1
        if k == 0x8d:                                        # fmain.c:3498 — 0x8d = beggar setfig race
            speak(24 + anim_list[nearest_person].goal)       # fmain.c:3498 — 24..26 = beggar prophecies
        else:
            speak(50)                                        # fmain.c:3499 — 50 = generic "thank you"
    elif hit == 8 and stuff[29]:                             # fmain.c:3501 — 8 = GIVE "Bone", 29 = bone slot
        if k != 0x8a:                                        # fmain.c:3502 — 0x8a = spectre setfig race
            speak(21)                                        # fmain.c:3502 — 21 = "no use for it"
        else:
            speak(48)                                        # fmain.c:3503 — 48 = "take this crystal shard"
            stuff[29] = 0                                    # fmain.c:3503 — consume the bone
            leave_item(nearest_person, 140)                  # fmain.c:3503 — 140 = Crystal Shard object id
    gomenu(CMODE_ITEMS)                                      # fmain.c:3507 — return to the top-level ITEMS menu
```

**Notes on dead slots.** `GIVE` entries 6 (Book) and 7 (Writ) are reachable
from the menu when their `stuff_flag` byte is set, but this function has no
`hit == 6` or `hit == 7` branch — selecting either is a silent no-op that
simply falls through to `gomenu(CMODE_ITEMS)`. Per
[_discovery/npc-quests.md](../_discovery/npc-quests.md), `stuff[26]` (Book) is
hardcoded disabled by `set_options` and `stuff[28]` (Writ) is consumed only
via the passive priest-TALK check in `fmain.c:3383-3388`, not via GIVE.

## check_quest_flag

Source: `fmain2.c:1640-1648`
Called by: `set_options` (one call per inventory / quest slot; results wired
into `menus[*].enabled[*]`)
Calls: `stuff`

```pseudo
def check_quest_flag(slot: int) -> int:
    """Return a menu-enable byte reflecting whether stuff[slot] is nonzero."""
    # fmain2.c:1642-1647 — assembly helper: read stuff[slot] as u8; return 10 if nonzero, else 8.
    # Values encode MenuEntry.enabled bytes (see SYMBOLS §3.1):
    #   8  = ATYPE_IMMEDIATE, hidden  — slot exists but not drawn / not pickable
    #   10 = ATYPE_IMMEDIATE | visible — slot drawn and selectable
    if stuff[slot] != 0:                                     # fmain2.c:1644-1645 — tst.b (a0); beq 1$
        return 10                                            # fmain2.c:1646 — moveq #10, d0 (visible + IMMEDIATE)
    return 8                                                 # fmain2.c:1641 — default moveq #8, d0 (hidden)
```

**Usage.** Every inventory-backed menu slot runs this predicate. Examples from
`set_options` (see [menu-system.md](menu-system.md)):
`menus[CMODE_USE].enabled[8] = check_quest_flag(7)` gates the `USE Sunstone`
entry on `stuff[7]` (Sunstone); `menus[CMODE_GIVE].enabled[8] = check_quest_flag(29)`
gates `GIVE Bone` on `stuff[29]` (Bone). Because the return values are menu
bytes, `check_quest_flag` is both the quest-flag reader and the menu-visibility
producer.

## necromancer_death_drop

Source: `fmain.c:1749-1757`
Called by: `actor_tick` (inside the `STATE_DYING → STATE_DEAD` transition; runs
once per dying actor when its `tactic` countdown hits 0)
Calls: `leave_item`, `anim_list`

```pseudo
def necromancer_death_drop(i: int) -> None:
    """On final DYING tick: race 9 transforms to woodcutter+drops Talisman; race 0x89 drops Lasso."""
    an = anim_list[i]                                        # fmain.c:1748 — ref to the dying actor
    # fmain.c:1750-1755 — Necromancer (race 9) does not truly die. On the tick its DYING
    # countdown expires, it is rewritten as a race-10 woodcutter, re-vitalised to 10 HP,
    # moved back to STILL, disarmed, and it drops world object 139 (Talisman) at its feet.
    if an.race == 9:                                         # fmain.c:1751 — 9 = necromancer race
        an.race = 10                                         # fmain.c:1751 — 10 = woodcutter race
        an.vitality = 10                                     # fmain.c:1752 — revive with 10 HP
        an.state = STATE_STILL                               # fmain.c:1752
        an.weapon = 0                                        # fmain.c:1753 — disarmed (woodcutter)
        leave_item(i, 139)                                   # fmain.c:1754 — 139 = Talisman object id
    # fmain.c:1756 — Witch (0x89) stays dead; she just drops the Golden Lasso.
    if an.race == 0x89:                                      # fmain.c:1756 — 0x89 = witch setfig race
        leave_item(i, 27)                                    # fmain.c:1756 — 27 = Golden Lasso object id
```

**Pickup side.** The dropped world objects reach the `stuff[]` array only via
the generic `TAKE` handler's `itrans[]` walk at `fmain.c:3191-3196`. Object
139 maps to `stuff[22]` (Talisman) and object 27 maps to `stuff[5]` (Lasso)
per `fmain2.c:983-984`. The Talisman pickup then triggers
`try_win_condition`.

## leave_item

Source: `fmain2.c:1191-1195`
Called by: `necromancer_death_drop`, `give_item_to_npc`
Calls: `anim_list`, `ob_listg`

```pseudo
def leave_item(i: int, object: int) -> None:
    """Place a ground-state world object at actor i's feet using global slot 0."""
    # fmain2.c:1192-1194 — ob_listg[0] is reserved as a scratch drop slot; any prior
    # contents are overwritten. Ground position is the actor's (x, y+10).
    ob_listg[0].xc = anim_list[i].abs_x                      # fmain2.c:1192
    ob_listg[0].yc = anim_list[i].abs_y + 10                 # fmain2.c:1193 — 10 = Y offset to feet
    ob_listg[0].ob_id = object                               # fmain2.c:1194 — world object id
    ob_listg[0].ob_stat = 1                                  # fmain2.c:1195 — 1 = ground / pickable
```

## rescue

Source: `fmain2.c:1584-1603`
Called by: `actor_tick` (extent-trigger path at `fmain.c:2684-2685` when
`xtype == 83` and `ob_list8[9].ob_stat != 0`); also the `R` cheat key at
`fmain.c:1333`
Calls: `map_message`, `SetFont`, `placard_text`, `name`, `placard`, `Delay`, `SetAPen`, `RectFill`, `message_off`, `xfer`, `move_extent`, `speak`, `stuff`, `ob_list8`, `princess`, `rp`, `afont`, `wealth`

```pseudo
def rescue() -> None:
    """Princess-rescue cinematic: display placards, teleport to Marheim, grant writ + gold + keys."""
    # fmain2.c:1587 — the princess counter picks one of three rescue-text triplets.
    i = princess * 3                                         # 3 = placard_text offset stride per princess
    # fmain2.c:1586-1589 — part 1: rescue narrative (3 placards with name interpolation).
    map_message()
    SetFont(rp, afont)
    placard_text(8 + i)                                      # fmain2.c:1588 — 8 = PLACARD_RESCUE_BASE
    name()
    placard_text(9 + i)                                      # fmain2.c:1588 — 9 = rescue placard pt 2
    name()
    placard_text(10 + i)                                     # fmain2.c:1588 — 10 = rescue placard pt 3
    placard()
    Delay(380)                                               # fmain2.c:1589 — 380 ticks ~ 7.6 s at 50 Hz
    # fmain2.c:1590 — part 2: clear inner rectangle and show post-rescue placards 17, 18.
    SetAPen(rp, 0)
    RectFill(rp, 13, 13, 271, 107)                           # fmain2.c:1590 — placard inner bounds
    Delay(10)                                                # fmain2.c:1590 — 10 = short settle delay before text
    SetAPen(rp, 24)                                          # fmain2.c:1590 — 24 = placard text pen
    placard_text(17)                                         # fmain2.c:1591 — 17 = post-rescue pt 1
    name()
    placard_text(18)                                         # fmain2.c:1591 — 18 = post-rescue pt 2
    Delay(380)                                               # fmain2.c:1591 — 380 ticks ~ 7.6 s at 50 Hz
    message_off()                                            # fmain2.c:1592
    # fmain2.c:1594-1602 — side effects: advance counter, teleport hero, grant rewards.
    princess = princess + 1                                  # fmain2.c:1594
    xfer(5511, 33780, 0)                                     # fmain2.c:1595 — Marheim castle drop coords; flag 0 = no region reload
    move_extent(0, 22205, 21231)                             # fmain2.c:1596 — extent 0 = bird extent, relocated for next phase
    ob_list8[2].ob_id = 4                                    # fmain2.c:1597 — cast slot 2 from noble (6) to princess (4)
    stuff[28] = 1                                            # fmain2.c:1598 — grant Writ (slot 28)
    speak(18)                                                # fmain2.c:1599 — king's post-rescue line
    wealth = wealth + 100                                    # fmain2.c:1600 — 100 gold reward
    ob_list8[9].ob_stat = 0                                  # fmain2.c:1601 — clear princess-captive flag
    i = 16                                                   # fmain2.c:1602 — 16 = KEYBASE (first key slot)
    while i < 22:                                            # fmain2.c:1602 — 22 = one past last key slot
        stuff[i] = stuff[i] + 3                              # fmain2.c:1602 — +3 of each key type
        i = i + 1
```

**Princess counter.** `princess` is not reset by `revive`
([brother-succession](../_discovery/brother-succession.md)) or by any other
path. It can take values 0, 1, 2 across up to three successful rescues (one
per brother, gated by `ob_list8[9].ob_stat` being re-raised only in
`revive`). The value 3 is unreachable in normal play because
`ob_list8[9].ob_stat` is cleared at `fmain2.c:1601` and is only re-set to 3
inside `revive`, which runs a bounded number of times.

## get_turtle

Source: `fmain.c:3510-3517`
Called by: `do_option` (`CMODE_USE`, `hit == 6`, outside the turtle-eggs
exclusion rectangle at `fmain.c:3459-3461`)
Calls: `set_loc`, `px_to_im`, `move_extent`, `load_carrier`

```pseudo
def get_turtle() -> None:
    """USE Shell: find a deep-water tile within the encounter-ring and spawn the turtle carrier there."""
    # fmain.c:3511-3514 — try up to 25 random ring points around the hero (see encounters.md
    # set_loc) until one lands on terrain class 5 (very-deep water).
    i = 0                                                    # fmain.c:3511 — retry counter
    while i < 25:                                            # fmain.c:3511 — 25 = max placement attempts
        set_loc()
        if px_to_im(encounter_x, encounter_y) == 5:          # fmain.c:3513 — 5 = TERRAIN_WATER_VDEEP
            break
        i = i + 1
    if i == 25:                                              # fmain.c:3514 — all attempts failed
        return
    # fmain.c:3515-3516 — move the turtle extent (index 1) and async-load the turtle carrier (race code 5).
    move_extent(1, encounter_x, encounter_y)                 # fmain.c:3515 — extent 1 = turtle-eggs / turtle extent
    load_carrier(5)                                          # fmain.c:3516 — 5 = RIDING_RAFT / turtle carrier
```

**Exclusion rectangle.** The USE handler at `fmain.c:3459` short-circuits
before calling `get_turtle` when the hero is inside the axis-aligned box
`11194 < hero_x < 21373` and `10205 < hero_y < 16208` — the turtle-eggs
nesting area around the dragon coast. The check is in the menu handler, not
in `get_turtle` itself.

## try_win_condition

Source: `fmain.c:3244-3247`
Called by: `do_option` (at the `pickup:` label inside the `CMODE_ITEMS`
`hit == 8 TAKE` branch, after every successful container or ground pickup)
Calls: `map_message`, `SetFont`, `end_game_sequence`, `stuff`, `quitflag`, `rp`, `afont`

```pseudo
def try_win_condition() -> None:
    """Run once per pickup: if the Talisman is now in inventory, end the game in victory."""
    # fmain.c:3244 — win latch. Any nonzero value in slot 22 means the Talisman was just taken.
    if stuff[22] == 0:                                       # 22 = STUFF_TALISMAN slot
        return
    # fmain.c:3245-3246 — quitflag ends the main while(!quitflag) loop at fmain.c:1270;
    # viewstatus=2 switches the display off the playfield before win_colors paints over it.
    quitflag = True                                          # fmain.c:3245
    viewstatus = 2                                           # fmain.c:3245 — 2 = full-screen placard/map mode
    map_message()                                            # fmain.c:3246 — fade page to black
    SetFont(rp, afont)                                       # fmain.c:3246 — Amber font
    end_game_sequence()                                      # fmain.c:3246 — call win_colors()
```

**Loss counterpart.** The other path that sets `quitflag = True` is
`brother > 3` inside `revive` (`fmain.c:2872`), reached after the
`placard_text(5)` "And so ends our sad tale..." screen. That flow belongs to
the brother-succession doc and is not repeated here. The third setter is the
`SAVEX → Exit` menu option at `fmain.c:3466`.

## end_game_sequence

Source: `fmain2.c:1605-1636`
Called by: `try_win_condition`
Calls: `placard_text`, `name`, `placard`, `Delay`, `unpackbrush`, `LoadRGB4`, `screen_size`, `sun_colors`, `fader`, `blackcolors`, `fp_drawing`, `vp_page`, `vp_text`, `HIRES`, `SPRITES`, `VP_HIDE`

```pseudo
def end_game_sequence() -> None:
    """Display the win placard, load winpic, run a 55-frame sunrise fade, then black out."""
    # fmain2.c:1607 — victory placard: msg7 + name + msg7a, bordered, held ~1.6 s.
    placard_text(6)                                          # fmain2.c:1607 — 6 = win placard pt 1
    name()
    placard_text(7)                                          # 7 = win placard pt 2
    placard()
    Delay(80)                                                # fmain2.c:1607 — ~1.6 s at 50 Hz
    # fmain2.c:1608-1609 — load winpic IFF brush into the drawing page bitmap at (0,0).
    bm_draw = fp_drawing.ri_page.BitMap
    unpackbrush("winpic", bm_draw, 0, 0)
    # fmain2.c:1610-1613 — black both viewports, hide the HUD, and expand the playfield.
    LoadRGB4(vp_page, blackcolors, 32)                       # fmain2.c:1610 — 32 = palette entries
    LoadRGB4(vp_text, blackcolors, 32)                       # fmain2.c:1611 — 32 = palette entries
    vp_text.Modes = HIRES | SPRITES | VP_HIDE                # fmain2.c:1612 — hide the status bar
    screen_size(156)                                         # fmain2.c:1613 — 156 = expanded display height
    # fmain2.c:1614-1632 — 55-frame sunrise: slide a window across sun_colors[] into palette entries 2..27.
    i = 25                                                   # fmain2.c:1614 — 25 = start offset (-30..25 range)
    while i > -30:                                           # fmain2.c:1614 — -30 = sunrise-end offset
        fader[0] = 0                                         # fmain2.c:1616 — background always black
        fader[31] = 0                                        # fmain2.c:1616 — top entry always black
        fader[1] = 0xfff                                     # fmain2.c:1617 — entry 1 always white
        fader[28] = 0xfff                                    # fmain2.c:1617 — entry 28 always white
        j = 2
        while j < 28:                                        # fmain2.c:1618 — palette window [2..27]
            if i + j > 0:
                fader[j] = sun_colors[i + j]                 # fmain2.c:1619 — from sun_colors[] palette
            else:
                fader[j] = 0                                 # fmain2.c:1620 — pre-sunrise = black
            j = j + 1
        # fmain2.c:1623-1630 — entries 29, 30 fade horizon reds down as i drops below -14.
        if i > -14:                                          # fmain2.c:1623 — -14 = sun-above-horizon cutoff
            fader[29] = 0x800                                # fmain2.c:1624 — deep red
            fader[30] = 0x400                                # fmain2.c:1625 — darker red
        else:
            j = (i + 30) / 2                                 # fmain2.c:1628 — 30 = range offset
            fader[29] = 0x100 * j                            # fmain2.c:1629 — linear red ramp
            fader[30] = 0x100 * (j / 2)                      # fmain2.c:1630 — half-rate ramp
        LoadRGB4(vp_page, fader, 32)                         # fmain2.c:1631
        if i == 25:                                          # fmain2.c:1632 — first frame holds 60 ticks
            Delay(60)                                        # 60 = ~1.2 s sustain on initial frame
        Delay(9)                                             # fmain2.c:1633 — ~0.18 s per subsequent frame
        i = i - 1
    Delay(30)                                                # fmain2.c:1635 — ~0.6 s tail
    LoadRGB4(vp_page, blackcolors, 32)                       # fmain2.c:1636 — final blackout
```

**Loop exit.** `end_game_sequence` returns to `try_win_condition`, which
returns into the `CMODE_ITEMS` arm of `do_option`, which returns through
`option_handler` back into the main loop. The next `while (!quitflag)` check
at `fmain.c:1270` then fails because `try_win_condition` already set
`quitflag = True`, and the game proceeds to `stopscore()` and `close_all()`
at `fmain.c:2616-2620`.

## Notes

- **Stat gates not in this doc.** The desert-gate check
  (`stuff[STATBASE] < 5` at `fmain.c:1919` and the map-tile stamp at
  `fmain.c:3594-3596`), the lava-damage Rose bypass (`stuff[23]` at
  `fmain.c:1845`), the Crystal-Shard pass-wall (`stuff[30]` at
  `fmain.c:1609`), the Lasso-bird guard (`stuff[5]` at `fmain.c:1498`) and
  the Sunstone combat gate (`stuff[7]` at `fmain2.c:231-233`) all live in
  the subsystem that owns them — see
  [movement.md](movement.md), [combat.md](combat.md), and the door handler
  section of [game-loop.md](game-loop.md). Each such gate is a single `if`
  on a `stuff[]` slot; they are not extracted as named functions here.

- **NPC gift reveals.** The priest's first writ-gated gift (`speak(39)`,
  `ob_listg[10].ob_stat = 1` at `fmain.c:3384-3385`) and the sorceress's
  first-talk gift (`speak(45)`, `ob_listg[9].ob_stat = 1` at
  `fmain.c:3404-3405`) are ordinary branches inside the `CMODE_TALK`
  dispatcher. Wave 7 (dialogue) captures the full TALK dispatch; they are
  referenced here only to document the flow from Writ → statue reveal →
  ground pickup → `stuff[25]` increment.
