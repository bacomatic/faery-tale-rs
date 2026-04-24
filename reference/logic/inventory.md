# Inventory — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c, fmain2.c
> Cross-refs: [RESEARCH §10](../RESEARCH.md#10-inventory--items), [_discovery/inventory.md](../_discovery/inventory.md), [quests.md](quests.md), [shops.md](shops.md), [carrier-transport.md](carrier-transport.md), [doors.md](doors.md), [menu-system.md](menu-system.md)

## Overview

The player inventory is the `stuff[]` byte array: slots `0..GOLDBASE-1` are real
carryables, gold slots sit at `GOLDBASE..ARROWBASE-1` as display-only rows, and
the scratch slot `stuff[ARROWBASE]` accumulates quiver-of-arrows pickups inside
a single TAKE action (`fmain.c:3151,3243`). All item acquisition flows through
one dispatcher — the `TAKE` case of `do_option` at `fmain.c:3149-3248` — which
decides per-object whether to consume gold, eat a fruit, swallow a scrap-of-paper
quest trigger, open a container, recover a dead brother's stash, or run the
generic `itrans[]` world-object-to-slot lookup. The companion branch at
`fmain.c:3249-3282` runs when the nearest figure is an actor rather than a loose
object: it pulls the victim's weapon into `stuff[0..4]` and rolls a treasure
drop from `TABLE:treasure_probs` into either `stuff[]` or `wealth`.

There is **no DROP command** in the shipped game. The `ITEMS` submenu exposes
`List`, `Take`, `Look`, `Use`, `Give` and nothing else (`fmain.c:497`); items
leave `stuff[]` only by consumption (weapons keep their count on display but
`stuff[8]` arrows decrement per shot, `stuff[9..15]` magic items decrement per
use, `stuff[16..21]` keys decrement per successful unlock, `stuff[24]` fruit
decrements on eat) or by the three `leave_item` paths documented in
[quests.md#leave_item](quests.md#leave_item). This logic doc therefore covers
TAKE, body-search, the `USE` submenu dispatcher, and the `LOOK` hidden-object
reveal. The `MAGIC` submenu dispatcher and the full magic-item catalog have
their own logic doc — see [magic.md](magic.md). Weapon equipping is documented
inside `use_dispatch`; key-on-door unlock is deferred to
[doors.md#use_key_on_door](doors.md#use_key_on_door); Sea Shell carrier spawn
is deferred to [quests.md#get_turtle](quests.md#get_turtle); shop purchase is
deferred to [shops.md#buy_dispatch](shops.md#buy_dispatch); GIVE is deferred to
[quests.md#give_item_to_npc](quests.md#give_item_to_npc).

## Symbols

No new locals beyond each function's declared parameters. Global identifiers
resolve in [SYMBOLS.md](SYMBOLS.md) or in each function's `Calls:` header.
Numeric literals that are `stuff[]` slot indices, world-object ids, or
narr.asm speech indices carry inline citations because the corresponding named
constants are not yet registered. Proposed SYMBOLS additions
(`itrans`, `inv_list`, `stone_list`, `anix2`, `secx`, `secy`, `rp_map`,
`planes`, `JAM1`, stuff-slot and world-object id constants) are listed in the
wave report.

## take_command

Source: `fmain.c:3149-3248`
Called by: `option_handler` (via `do_option` `CMODE_ITEMS` branch on `hit == 6`)
Calls: `prq`, `nearest_fig`, `announce_treasure`, `announce_container`, `print`, `print_cont`, `event`, `eat`, `rand4`, `rand8`, `change_object`, `map_message`, `SetFont`, `end_game_sequence`, `search_body`, `itrans`, `inv_list`

```pseudo
def take_command(hit: int) -> None:
    """Resolve nearest object, apply per-type pickup or container roll, retire the world object, then check the win condition."""
    prq(7)                                                   # fmain.c:3151 — 7 = HUD stats refresh code
    nearest_fig(0, 30)                                       # fmain.c:3152 — 30 = scan radius in pixels
    stuff[35] = 0                                            # fmain.c:3151 — 35 = ARROWBASE; clear quiver accumulator
    if nearest == 0:                                         # fmain.c:3154 — nothing in range
        prq(10)                                              # fmain.c:3285 — 10 = "Take What?" prompt
        return
    if anim_list[nearest].type != OBJECTS:                   # fmain.c:3155 — OBJECTS=1; actor, not loose object
        search_body(nearest)                                 # fmain.c:3249-3282 — delegate to body-search branch
        return
    j = anim_list[nearest].index                             # fmain.c:3158 — world object id (low byte of index)
    x = anim_list[nearest].vitality                          # fmain.c:3159 — bones owner id (1=Julian, 2=Phillip)
    taken = 0                                                # set to 1 by every branch that reaches the pickup epilogue
    if j == 13:                                              # fmain.c:3160 — 13 = MONEY object id
        announce_treasure("50 gold pieces")                  # fmain.c:3161
        wealth = wealth + 50                                 # fmain.c:3162 — 50 = gold per money pile
        taken = 1
    elif j == 20:                                            # fmain.c:3164 — 20 = SCRAP of paper
        event(17)                                            # fmain.c:3166 — 17 = scrap-found event
        if region_num > 7:                                   # fmain.c:3167 — 7 = boundary between early / late regions
            event(19)                                        # fmain.c:3167 — 19 = late-game scrap text
        else:
            event(18)                                        # fmain.c:3167 — 18 = early-game scrap text
        taken = 1                                            # fmain.c:3168 — goto pickup
    elif j == 148:                                           # fmain.c:3170 — 148 = FRUIT object id
        if hunger < 15:                                      # fmain.c:3171 — 15 = satiation floor for stashing
            stuff[24] = stuff[24] + 1                        # fmain.c:3171 — 24 = STUFF_FOOD slot
            event(36)                                        # fmain.c:3171 — 36 = "picked up some fruit"
        else:
            eat(30)                                          # fmain.c:3172 — 30 = hunger reduction per fruit
        taken = 1                                            # fmain.c:3173 — goto pickup
    elif j == 102:                                           # fmain.c:3175 — 102 = TURTLE (turtle egg)
        return                                               # fmain.c:3175 — break: cannot be taken
    elif j == 28:                                            # fmain.c:3177 — 28 = bones world object id
        announce_treasure("his brother's bones.")            # fmain.c:3178
        ob_listg[3].ob_stat = 0                              # fmain.c:3179 — 3 = Julian's ghost setfig slot
        ob_listg[4].ob_stat = 0                              # fmain.c:3179 — 4 = Phillip's ghost setfig slot
        k = 0                                                # fmain.c:3180 — loop over real-inventory slots
        while k < 31:                                        # fmain.c:3180 — 31 = GOLDBASE (real stuff stops here)
            if x == 1:                                       # fmain.c:3181 — 1 = Julian
                stuff[k] = stuff[k] + julstuff[k]            # fmain.c:3181 — merge Julian's stash
            else:
                stuff[k] = stuff[k] + philstuff[k]           # fmain.c:3182 — merge Phillip's stash
            k = k + 1
        taken = 1
    elif j == 15:                                            # fmain.c:3186 — 15 = CHEST
        announce_container("a chest")
    elif j == 14:                                            # fmain.c:3187 — 14 = URN
        announce_container("a brass urn")
    elif j == 16:                                            # fmain.c:3188 — 16 = SACKS
        announce_container("some sacks")
    elif j == 29:                                            # fmain.c:3189 — 29 = empty chest
        return                                               # fmain.c:3189 — already-opened chest: bail
    elif j == 31:                                            # fmain.c:3192 — 31 = FOOTSTOOL
        return                                               # fmain.c:3192 — cannot be taken
    else:
        k = 0                                                # fmain.c:3193 — itrans[] pair-walk index
        matched = 0
        while itrans[k] != 0:                                # fmain.c:3193 — 0,0 sentinel ends the table
            if j == itrans[k]:                               # fmain.c:3194 — world id match
                i = itrans[k + 1]                            # fmain.c:3195 — resolved stuff[] index
                stuff[i] = stuff[i] + 1                      # fmain.c:3196
                announce_treasure("a ")                      # fmain.c:3197
                print_cont(inv_list[i].name)                 # fmain.c:3198
                print_cont(".")                              # fmain.c:3199
                matched = 1                                  # fmain.c:3200 — goto pickup
                break
            k = k + 2                                        # fmain.c:3193 — itrans walks pairs
        if matched == 0:                                     # fmain.c:3204 — unknown object id: silent bail
            return
        taken = 1                                            # fmain.c:3200 — goto pickup
    # Container contents roll — entered only via the CHEST / URN / SACKS branches above, which left taken=0.
    if taken == 0:
        k = rand4()                                          # fmain.c:3208 — 0..3 selects loot branch
        if k == 0:                                           # fmain.c:3209 — empty container
            print("nothing.")
        elif k == 1:                                         # fmain.c:3211 — single item
            i = rand8() + 8                                  # fmain.c:3213 — 8 = base index; rand8 ∈ 0..7 ⇒ 8..15
            if i == 8:                                       # fmain.c:3214 — 8 = arrows row ⇒ promote to quiver
                i = 35                                       # fmain.c:3214 — 35 = ARROWBASE (quiver alias)
            print("a ")
            print_cont(inv_list[i].name)
            print_cont(".")
            stuff[i] = stuff[i] + 1                          # fmain.c:3217
        elif k == 2:                                         # fmain.c:3218 — paired loot
            i = rand8() + 8                                  # fmain.c:3220 — first item 8..15
            if i == 8:                                       # fmain.c:3221 — arrows row ⇒ gold instead
                i = 34                                       # fmain.c:3221 — 34 = GOLDBASE+3 (100-gold row)
                wealth = wealth + 100                        # fmain.c:3221 — 100 = gold grant
            else:
                print_cont(" a")
            print(inv_list[i].name)
            print_cont(" and a ")
            k = rand8() + 8                                  # fmain.c:3225 — second item 8..15, distinct
            while k == i:
                k = rand8() + 8                              # fmain.c:3225 — reroll on collision
            if k == 8:                                       # fmain.c:3226 — arrows row ⇒ quiver alias
                k = 35                                       # fmain.c:3226 — 35 = ARROWBASE
            print_cont(inv_list[k].name)
            if i < 31:                                       # fmain.c:3228 — 31 = GOLDBASE; skip gold accumulator
                stuff[i] = stuff[i] + 1
            stuff[k] = stuff[k] + 1                          # fmain.c:3229
        elif k == 3:                                         # fmain.c:3230 — triple-of-a-kind
            i = rand8() + 8                                  # fmain.c:3232 — base roll
            if i == 8:                                       # fmain.c:3233 — arrows row ⇒ 3 random keys
                print("3 keys.")
                n = 0
                while n < 3:                                 # fmain.c:3235 — 3 = key-count grant
                    i = rand8() + 16                         # fmain.c:3236 — 16 = KEYBASE
                    if i == 22:                              # fmain.c:3237 — 22 outside key range ⇒ Gold Key
                        i = 16                               # fmain.c:3237 — 16 = KEYBASE
                    if i == 23:                              # fmain.c:3238 — 23 outside range ⇒ Grey Key
                        i = 20                               # fmain.c:3238 — 20 = Grey Key slot
                    stuff[i] = stuff[i] + 1
                    n = n + 1
            else:
                print("3 ")
                print_cont(inv_list[i].name)
                print_cont("s.")
                stuff[i] = stuff[i] + 3                      # fmain.c:3244 — 3 = triple-stack grant
    # Common epilogue (labelled `pickup:` in the C source) — retire the world object, bank quiver arrows, test win.
    change_object(nearest, 2)                                # fmain.c:3249 — 2 = OB_STAT_TAKEN
    stuff[8] = stuff[8] + stuff[35] * 10                     # fmain.c:3250 — 8 = arrows slot, 35 = ARROWBASE, 10 = arrows-per-quiver
    if stuff[22] != 0:                                       # fmain.c:3251 — 22 = Talisman slot; try_win_condition
        quitflag = True                                      # fmain.c:3252
        viewstatus = 2                                       # fmain.c:3252 — 2 = VIEWSTATUS_PLACARD
        map_message()                                        # fmain.c:3253
        SetFont(rp, afont)                                   # fmain.c:3253
        end_game_sequence()                                  # fmain.c:3253 — win_colors()
```

**Dispatch ordering.** Five type-specific branches (`MONEY`, `SCRAP`, `FRUIT`,
turtle eggs, bones) run before the container / `itrans[]` walk. The SCRAP and
FRUIT branches skip the `itrans[]` walk by setting `taken=1` and falling
through to the epilogue (labelled `pickup:` in the C source at `fmain.c:3240`).
The bones branch transfers by copying `julstuff[]` or `philstuff[]` wholesale
and reaches the epilogue the same way. The container branches (`CHEST`,
`URN`, `SACKS`) set an announcement string but leave `taken=0`, so the
`rand4()` loot block runs afterwards before the shared epilogue.

**Container reroll corners.** The `rand8() + 8` draws are clamped to slots
`8..15`; an `8` (arrows row) is promoted to the quiver alias (slot 35) under
`k == 1`, to a 100-gold grant under `k == 2`, and to a triple-key burst under
`k == 3`. The `i == 22` / `i == 23` fixups in the key-burst are deliberate:
`rand8()+16` can exceed the key range because `rand8()` returns `0..7` but the
only valid keys sit at `16..21` (6 entries); `22 → 16` and `23 → 20` rebind
the two over-limit rolls back into range without re-rolling.

**Talisman latch.** Any pickup path reaching the epilogue — including
container rolls, `itrans[]` matches, and the brother-bones merge (which can
transfer a Talisman in slot 22 from a dead brother to the current one) —
re-tests `stuff[22] != 0` and triggers the win sequence at `fmain.c:3245-3247`.

## search_body

Source: `fmain.c:3249-3282`
Called by: `take_command`
Calls: `extract`, `print`, `print_cont`, `prdec`, `rand8`, `event`, `inv_list`, `treasure_probs`, `encounter_chart`

```pseudo
def search_body(target: int) -> None:
    """TAKE against a live-but-frozen or dead actor: loot weapon, then roll treasure drop."""
    an = anim_list[target]                                   # fmain.c:3249
    if an.weapon < 0:                                        # fmain.c:3249 — -1 means already looted
        return
    if an.vitality != 0 and freeze_timer == 0:               # fmain.c:3250 — not dead and not frozen
        event(35)                                            # fmain.c:3283 — 35 = "it is still alive!" message
        return
    extract("% searched the body and found")                 # fmain.c:3253 — % = current brother's name
    print("")                                                # fmain.c:3254 — newline
    i = an.weapon                                            # fmain.c:3255 — 1..5 or 0
    if i > 5:                                                # fmain.c:3255 — out-of-range defensive clamp
        i = 0
    if i != 0:                                               # fmain.c:3256 — victim had a weapon
        print_cont("a ")
        print_cont(inv_list[i - 1].name)                     # fmain.c:3258 — inv_list row i-1 (Dirk..Wand)
        stuff[i - 1] = stuff[i - 1] + 1                      # fmain.c:3259 — add weapon to hero inventory
        if i > anim_list[0].weapon:                          # fmain.c:3260 — auto-equip if better than current
            anim_list[0].weapon = i
        if i == 4:                                           # fmain.c:3261 — 4 = Bow: grant ammo and short-circuit
            print_cont(" and ")
            j = rand8() + 2                                  # fmain.c:3263 — 2..9 extra arrows
            prdec(j, 1)                                      # fmain.c:3264 — print decimal
            print_cont(" Arrows.")
            stuff[8] = stuff[8] + j                          # fmain.c:3266 — 8 = arrows slot
            an.weapon = -1                                   # fmain.c:3267 — mark body as looted
            return
    an.weapon = -1                                           # fmain.c:3270 — mark body as looted
    j = an.race                                              # fmain.c:3271
    if j & 0x80:                                             # fmain.c:3272 — 0x80 = SETFIG_RACE_BIT; setfigs never drop treasure
        j = 0
    else:
        j = encounter_chart[j].treasure * 8 + rand8()        # fmain.c:3274 — 8 = TREASURE_PROBS_COLUMNS
        j = treasure_probs[j]                                # fmain.c:3275 — inv_list row or 0
    if j != 0:                                               # fmain.c:3276 — non-zero slot means loot
        if i != 0:                                           # fmain.c:3277 — chain onto weapon line
            print_cont(" and ")
        if j < 31:                                           # fmain.c:3278 — 31 = GOLDBASE
            print_cont("a ")
        print_cont(inv_list[j].name)
        if j >= 31:                                          # fmain.c:3280 — gold row
            wealth = wealth + inv_list[j].maxshown           # fmain.c:3280 — maxshown doubles as gold value
        else:
            stuff[j] = stuff[j] + 1                          # fmain.c:3281
    elif i == 0:                                             # fmain.c:3282 — no weapon + no treasure
        print_cont("nothing")
    print_cont(".")
```

## use_dispatch

Source: `fmain.c:3444-3467`
Called by: `option_handler` (via `do_option` `CMODE_USE` branch)
Calls: `gomenu`, `extract`, `speak`, `get_turtle`

```pseudo
def use_dispatch(hit: int) -> None:
    """Dispatch the USE submenu: equip a weapon, open the KEYS submenu, summon a turtle, or consult Sun/Book."""
    # Menu label order at fmain.c:502 is: Dirk Mace Sword Bow Wand Lasso Shell Key Sun Book.
    # Slots 0..4 are weapons; 5 Lasso (no explicit handler); 6 Shell; 7 Keys submenu; 8 Sun; 9 Book.
    if hit == 7:                                             # fmain.c:3446 — 7 = KEYS submenu slot
        if hitgo:                                            # fmain.c:3446 — hitgo reflects menus[USE].enabled[7] bit 1
            gomenu(CMODE_KEYS)                               # fmain.c:3446 — open Keys submenu
        else:
            extract("% has no keys!")                        # fmain.c:3446 — % = brother name
        return
    if hit < 5:                                              # fmain.c:3449 — 5 = first non-weapon slot
        if hitgo:                                            # fmain.c:3450 — owner has the weapon
            anim_list[0].weapon = hit + 1                    # fmain.c:3450 — weapon codes: 1 Dirk..5 Wand
        else:
            extract("% doesn't have one.")                   # fmain.c:3451
    if hit == 6 and hitgo:                                   # fmain.c:3457 — 6 = Sea Shell; hitgo gates ownership
        # Swamp-rectangle veto (turtle-eggs nesting box near the dragon coast).
        if hero_x < 21373 and hero_x > 11194 and hero_y < 16208 and hero_y > 10205:  # fmain.c:3459
            gomenu(CMODE_ITEMS)                              # fmain.c:3466 — fall through to ITEMS without spawning
            return
        get_turtle()                                         # fmain.c:3461 — see quests.md#get_turtle
    if hit == 8 and witchflag:                               # fmain.c:3462 — 8 = Sun Stone slot; witch is active
        speak(60)                                            # fmain.c:3462 — 60 = witch-unmasking line
    gomenu(CMODE_ITEMS)                                      # fmain.c:3466 — always return to top ITEMS menu
```

**Dead slots.** `hit == 5` (Lasso) falls through every branch: the lasso has no
USE effect because mounting is handled passively by
[carrier-transport.md#carrier_tick](carrier-transport.md#carrier_tick) at
`fmain.c:1498`, which reads `stuff[5]`. `hit == 9` (Book) also has no branch —
selecting it simply returns to the ITEMS menu. `menus[USE].enabled[9]` is set
to `10` unconditionally (label7 at `fmain.c:502`, enable vector at
`fmain.c:530`), so the slot is always selectable even though it does nothing.

**Consumption side.** Weapon USE is non-consuming: `stuff[0..4]` counts are
kept as display flags and are decremented only by arrow shots (`stuff[8]`) or
per-door key spends (`stuff[16..21]` in `use_key_on_door`). Only the Sea Shell
path has a consuming effect, and that consumption happens inside `get_turtle`
(extent move) rather than in `use_dispatch`.

## look_command

Source: `fmain.c:3286-3295`
Called by: `option_handler` (via `do_option` `CMODE_ITEMS` branch on `hit == 7`)
Calls: `calc_dist`, `change_object`, `event`, `anix2`

```pseudo
def look_command() -> None:
    """LOOK: reveal every hidden-state object (race==0) within 40 px of the hero."""
    flag = 0                                                 # fmain.c:3286 — "any reveal this tick?" latch
    i = 0
    while i < anix2:                                         # fmain.c:3288 — iterate actor+object table
        an = anim_list[i]
        # race==0 on an OBJECTS entry marks a hidden object (ob_stat==5 in the world-object backing row).
        # change_object(i, 1) promotes it to ob_stat=1 (visible, pickable).
        if an.type == OBJECTS and an.race == 0 and calc_dist(i, 0) < 40:  # fmain.c:3289 — 40 = reveal radius
            flag = 1
            change_object(i, 1)                              # fmain.c:3290 — 1 = OB_STAT_VISIBLE
        i = i + 1
    if flag != 0:                                            # fmain.c:3294 — at least one reveal
        event(38)                                            # fmain.c:3294 — 38 = "you find something hidden"
    else:
        event(20)                                            # fmain.c:3294 — 20 = "you find nothing"
```

## Notes

- **`nearest` vs `nearest_person`.** TAKE uses the radius-30 `nearest_fig(0, 30)`
  result in the global `nearest`; the GIVE / TALK submenus use the radius-50
  scan stored in `nearest_person`. See [quests.md#give_item_to_npc](quests.md#give_item_to_npc)
  for the nearest_person counterpart.
- **No DROP command.** The `ITEMS` submenu label string at `fmain.c:497` lists
  exactly `List Take Look Use  Give`; there is no drop or leave option. The
  three in-game "drop" behaviours — Necromancer / Witch death drops
  (`fmain.c:1754,1756`), Bone→Shard exchange at the Spectre
  (`fmain.c:3503`), and the princess-rescue object re-seating
  (`fmain2.c:1596-1601`) — all route through
  [quests.md#leave_item](quests.md#leave_item) or direct `ob_list*[]` writes.
- **Quiver accumulator.** `stuff[35]` (`ARROWBASE`) is set to 0 at the top of
  TAKE (`fmain.c:3151`) and folded into `stuff[8]` in the epilogue
  (`fmain.c:3250` reads `stuff[ARROWBASE] * 10`). The only writer that
  targets `stuff[35]` is the `itrans[]` match for world object `QUIVER = 11`
  (`fmain2.c:981`) — pairing arrows as quivers of ten.
- **Menu refresh.** `take_command`, `search_body`, `look_command`, and
  `use_dispatch` all rely on the outer `set_options()` call at
  `fmain.c:3514` to recompute `menus[*].enabled[]` after any `stuff[]`
  mutation. The `MAGIC` submenu dispatcher
  ([magic.md#magic_dispatch](magic.md#magic_dispatch)) calls `set_options()`
  directly only on the transition to zero, because the displayed row
  changes from selectable to greyed out at that moment.
