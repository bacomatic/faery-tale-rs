# Inventory — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c, fmain2.c
> Cross-refs: [RESEARCH §10](../RESEARCH.md#10-inventory--items), [_discovery/inventory.md](../_discovery/inventory.md), [quests.md](quests.md), [shops.md](shops.md), [carrier-transport.md](carrier-transport.md), [doors.md](doors.md), [menu-system.md](menu-system.md), [sprite-rendering.md#render_inventory_items_page](sprite-rendering.md#render_inventory_items_page)

## Overview

The player inventory is the `stuff[]` byte array: slots `0..GOLDBASE-1` are real
carryables, gold slots sit at `GOLDBASE..ARROWBASE-1` as display-only rows, and
the scratch slot `stuff[ARROWBASE]` accumulates quiver-of-arrows pickups inside
a single TAKE action (`fmain.c:3150,3243`). All item acquisition flows through
one dispatcher — the `TAKE` case of `do_option` at `fmain.c:3147-3287` — which
decides per-object whether to consume gold, eat a fruit, swallow a scrap-of-paper
quest trigger, open a container, recover a dead brother's stash, or run the
generic `itrans[]` world-object-to-slot lookup. The companion branch at
`fmain.c:3249-3285` runs when the nearest figure is an actor rather than a loose
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
[quests.md#give_item_to_npc](quests.md#give_item_to_npc). The full-screen
items-page render that fires on `ITEMS->List` (`hit == 5`) is documented in
[sprite-rendering.md#render_inventory_items_page](sprite-rendering.md#render_inventory_items_page);
it blits each populated `inv_list[]` row as a 16-pixel-wide icon repeated
`stuff[j]` times.

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

Source: `fmain.c:3147-3287` (full `hit == 6` switch-case body; OBJECT-pickup branch is `3148-3247`, body-search siblings extend through `3285`)
Called by: `option_handler` (via `do_option` `CMODE_ITEMS` branch on `hit == 6`; `hit` is consumed by the dispatcher and is not a parameter of the body)
Calls: `prq`, `nearest_fig`, `announce_treasure`, `announce_container`, `print`, `print_cont`, `event`, `eat`, `rand4`, `rand8`, `change_object`, `map_message`, `SetFont`, `end_game_sequence`, `search_body`, `itrans`, `inv_list`
Reads: `nearest`, `anim_list`, `region_num`, `hunger`, `julstuff`, `philstuff`, `itrans`, `inv_list`, `stuff[22]`, `rp`, `afont`
Writes: `stuff[]` (all carryable + gold + ARROWBASE rows), `wealth`, `ob_listg[3].ob_stat`, `ob_listg[4].ob_stat`, `quitflag`, `viewstatus`

```pseudo
def take_command() -> None:
    """Resolve nearest object, apply per-type pickup or container roll, retire the world object, then check the win condition."""
    prq(7)                                                   # fmain.c:3148 — 7 = HUD stats refresh code
    nearest_fig(0, 30)                                       # fmain.c:3149 — 30 = scan radius in pixels
    stuff[ARROWBASE] = 0                                     # fmain.c:3150 — clear per-action quiver accumulator
    if nearest == 0:                                         # fmain.c:3151 — `if (nearest)` falsy branch
        prq(10)                                              # fmain.c:3287 — 10 = "Take What?" prompt
        return
    if anim_list[nearest].type != OBJECTS:                   # fmain.c:3152 — OBJECTS=1; actor, not loose object
        search_body(nearest)                                 # fmain.c:3249-3285 — delegate to body-search branch
        return
    j = anim_list[nearest].index                             # fmain.c:3154 — world object id (low byte of index)
    x = anim_list[nearest].vitality & VITALITY_OWNER_MASK    # fmain.c:3155 — bones owner id; mask required, see SYMBOLS §4.1 Shape.vitality
    taken = 0                                                # taken=1 means this branch handled pickup itself; the rand4 container-loot block is skipped. The pickup epilogue runs unconditionally.
    if j == 13:                                              # fmain.c:3156 — 13 = MONEY object id (0x0d)
        announce_treasure("50 gold pieces")                  # fmain.c:3157
        wealth = wealth + 50                                 # fmain.c:3158 — 50 = gold per money pile
        taken = 1
    elif j == 20:                                            # fmain.c:3160 — 20 = SCRAP of paper (0x14)
        event(17)                                            # fmain.c:3161 — 17 = "% picked up a scrap of paper."
        if region_num > LATE_GAME_REGION_THRESHOLD:          # fmain.c:3162 — see SYMBOLS LATE_GAME_REGION_THRESHOLD
            event(19)                                        # fmain.c:3162 — 19 = Wraith Lord crypt rendezvous hint (late-game)
        else:
            event(18)                                        # fmain.c:3162 — 18 = "Find the turtle!" hint (early-game)
        taken = 1                                            # fmain.c:3163 — goto pickup
    elif j == 148:                                           # fmain.c:3165 — 148 = FRUIT object id
        if hunger < 15:                                      # fmain.c:3166 — 15 = satiation floor for stashing
            stuff[24] = stuff[24] + 1                        # fmain.c:3166 — 24 = STUFF_FOOD slot
            event(36)                                        # fmain.c:3166 — 36 = "% put an apple away for later."
        else:
            eat(30)                                          # fmain.c:3167 — 30 = hunger reduction per fruit
        taken = 1                                            # fmain.c:3168 — goto pickup
    elif j == 102:                                           # fmain.c:3171 — 102 = TURTLE (turtle egg)
        return                                               # fmain.c:3171 — break: cannot be taken
    elif j == BONES_OBJ_ID:                                  # fmain.c:3172 — 28
        announce_treasure("his brother's bones.")            # fmain.c:3173
        # Both ghost setfigs retire on either pickup — observable in-game; not a typo. See Notes.
        ob_listg[3].ob_stat = 0                              # fmain.c:3174 — 3 = Julian's ghost setfig slot
        ob_listg[4].ob_stat = 0                              # fmain.c:3174 — 4 = Phillip's ghost setfig slot
        k = 0                                                # fmain.c:3175 — loop over real-inventory slots
        while k < GOLDBASE:                                  # fmain.c:3175 — real stuff stops at gold rows
            if x == BONES_OWNER_JULIAN:                      # fmain.c:3176
                stuff[k] = stuff[k] + julstuff[k]            # fmain.c:3176 — merge Julian's stash
            else:                                            # default branch covers BONES_OWNER_PHILLIP
                stuff[k] = stuff[k] + philstuff[k]           # fmain.c:3177 — merge Phillip's stash
            k = k + 1
        taken = 1
    elif j == 15:                                            # fmain.c:3181 — 15 = CHEST (0x0f)
        announce_container("a chest")
    elif j == 14:                                            # fmain.c:3182 — 14 = URN (0x0e)
        announce_container("a brass urn")
    elif j == 16:                                            # fmain.c:3183 — 16 = SACKS (0x10)
        announce_container("some sacks")
    elif j == 29:                                            # fmain.c:3184 — 29 = empty chest (0x1d)
        return                                               # fmain.c:3184 — already-opened chest: bail
    elif j == 31:                                            # fmain.c:3186 — 31 = FOOTSTOOL
        return                                               # fmain.c:3186 — cannot be taken
    else:
        k = 0                                                # fmain.c:3187 — itrans[] pair-walk index
        matched = 0
        while itrans[k] != 0:                                # fmain.c:3187 — 0,0 sentinel ends the table
            if j == itrans[k]:                               # fmain.c:3188 — world id match
                i = itrans[k + 1]                            # fmain.c:3189 — resolved stuff[] index
                stuff[i] = stuff[i] + 1                      # fmain.c:3190
                announce_treasure("a ")                      # fmain.c:3191
                print_cont(inv_list[i].name)                 # fmain.c:3192
                print_cont(".")                              # fmain.c:3193
                matched = 1                                  # fmain.c:3194 — goto pickup
                break
            k = k + 2                                        # fmain.c:3187 — itrans walks pairs
        if matched == 0:                                     # fmain.c:3197 — unknown object id: silent bail
            return
        taken = 1                                            # fmain.c:3194 — goto pickup
    # Container contents roll — entered only via the CHEST / URN / SACKS branches above, which left taken=0.
    if taken == 0:
        k = rand4()                                          # fmain.c:3201 — 0..3 selects loot branch
        if k == 0:                                           # fmain.c:3202 — empty container
            print("nothing.")
        elif k == 1:                                         # fmain.c:3204 — single item
            i = rand8() + 8                                  # fmain.c:3205 — 8 = base index; rand8 ∈ 0..7 ⇒ 8..15
            if i == 8:                                       # fmain.c:3206 — 8 = arrows row ⇒ promote to quiver
                i = ARROWBASE                                # fmain.c:3206 — quiver alias slot
            print("a ")
            print_cont(inv_list[i].name)
            print_cont(".")
            stuff[i] = stuff[i] + 1                          # fmain.c:3209
        elif k == 2:                                         # fmain.c:3211 — paired loot
            i = rand8() + 8                                  # fmain.c:3212 — first item 8..15
            if i == 8:                                       # fmain.c:3213 — arrows row ⇒ gold instead
                i = GOLDBASE + 3                             # fmain.c:3213 — 100-gold row
                wealth = wealth + 100                        # fmain.c:3213 — 100 = gold grant
            else:
                print_cont(" a")                             # fmain.c:3214
            print(inv_list[i].name)                          # fmain.c:3215
            print_cont(" and a ")                            # fmain.c:3216
            k = rand8() + 8                                  # fmain.c:3217 — second item 8..15, distinct
            while k == i:
                k = rand8() + 8                              # fmain.c:3217 — reroll on collision
            if k == 8:                                       # fmain.c:3218 — arrows row ⇒ quiver alias
                k = ARROWBASE                                # fmain.c:3218 — quiver alias slot
            print_cont(inv_list[k].name)                     # fmain.c:3219
            if i < GOLDBASE:                                 # fmain.c:3220 — skip gold accumulator
                stuff[i] = stuff[i] + 1
            stuff[k] = stuff[k] + 1                          # fmain.c:3221
        elif k == 3:                                         # fmain.c:3223 — triple-of-a-kind
            i = rand8() + 8                                  # fmain.c:3224 — base roll
            if i == 8:                                       # fmain.c:3225 — arrows row ⇒ 3 random keys
                print("3 keys.")                             # fmain.c:3226
                n = 0
                while n < 3:                                 # fmain.c:3227 — 3 = key-count grant
                    i = rand8() + KEYBASE                    # fmain.c:3228
                    if i == 22:                              # fmain.c:3229 — 22 outside key range ⇒ Gold Key
                        i = 16                               # fmain.c:3229 — 16 = KEYBASE
                    if i == 23:                              # fmain.c:3230 — 23 outside range ⇒ Grey Key
                        i = 20                               # fmain.c:3230 — 20 = Grey Key slot
                    stuff[i] = stuff[i] + 1                  # fmain.c:3231
                    n = n + 1
            else:
                print("3 ")                                  # fmain.c:3235
                print_cont(inv_list[i].name)                 # fmain.c:3236
                print_cont("s.")                             # fmain.c:3236
                stuff[i] = stuff[i] + 3                      # fmain.c:3237 — 3 = triple-stack grant
    # Common epilogue (labelled `pickup:` at fmain.c:3241) — retire the world object, bank quiver arrows, test win.
    change_object(nearest, 2)                                # fmain.c:3242 — 2 = OB_STAT_TAKEN
    stuff[8] = stuff[8] + stuff[ARROWBASE] * 10              # fmain.c:3243 — 8 = arrows slot, 10 = arrows-per-quiver
    if stuff[22] != 0:                                       # fmain.c:3244 — 22 = Talisman slot; try_win_condition
        quitflag = True                                      # fmain.c:3245
        viewstatus = 2                                       # fmain.c:3245 — 2 = VIEWSTATUS_PLACARD
        map_message()                                        # fmain.c:3246
        SetFont(rp, afont)                                   # fmain.c:3246
        end_game_sequence()                                  # fmain.c:3246 — win_colors()
```

**Dispatch ordering.** Five type-specific branches (`MONEY`, `SCRAP`, `FRUIT`,
turtle eggs, bones) run before the container / `itrans[]` walk. The SCRAP and
FRUIT branches skip the `itrans[]` walk by setting `taken=1` and falling
through to the epilogue (labelled `pickup:` in the C source at `fmain.c:3241`).
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
re-tests `stuff[22] != 0` and triggers the win sequence at `fmain.c:3244-3247`.

## search_body

Source: `fmain.c:3249-3285` (the `else if (... weapon < 0)` / `else if (... vitality == 0 || freeze_timer)` / `else event(35)` sibling chain of the OBJECT-pickup branch)
Called by: `take_command`
Calls: `extract`, `print`, `print_cont`, `prdec`, `rand8`, `event`, `inv_list`, `treasure_probs`, `encounter_chart`
Reads: `anim_list`, `freeze_timer` (set by the Gold Ring time-stop spell — see [magic.md#magic_dispatch](magic.md#magic_dispatch)), `inv_list`, `treasure_probs`, `encounter_chart`
Writes: `anim_list[target].weapon` (sets `-1` to mark looted), `anim_list[0].weapon` (auto-equip on upgrade), `stuff[]`, `wealth`

```pseudo
def search_body(target: int) -> None:
    """TAKE against a live-but-frozen or dead actor: loot weapon, then roll treasure drop."""
    an = anim_list[target]                                   # fmain.c:3249 — alias for the rest of the body
    if an.weapon < 0:                                        # fmain.c:3249 — -1 means already looted
        return
    if an.vitality != 0 and freeze_timer == 0:               # fmain.c:3250 — not dead and not frozen
        event(35)                                            # fmain.c:3285 — 35 = "it is still alive!" message
        return
    extract("% searched the body and found")                 # fmain.c:3251 — % = current brother's name
    print("")                                                # fmain.c:3252 — newline
    i = an.weapon                                            # fmain.c:3253 — 1..5 or 0
    if i > 5:                                                # fmain.c:3253 — out-of-range defensive clamp
        i = 0
    if i != 0:                                               # fmain.c:3254 — victim had a weapon
        print_cont("a ")                                     # fmain.c:3255
        print_cont(inv_list[i - 1].name)                     # fmain.c:3256 — inv_list row i-1 (Dirk..Wand)
        stuff[i - 1] = stuff[i - 1] + 1                      # fmain.c:3257 — add weapon to hero inventory
        if i > anim_list[0].weapon:                          # fmain.c:3258 — auto-equip if better than current
            anim_list[0].weapon = i                          # fmain.c:3258
        if i == 4:                                           # fmain.c:3259 — 4 = Bow: grant ammo and short-circuit
            print_cont(" and ")                              # fmain.c:3260
            j = rand8() + 2                                  # fmain.c:3261 — 2..9 extra arrows
            prdec(j, 1)                                      # fmain.c:3261 — print decimal
            print_cont(" Arrows.")                           # fmain.c:3262
            stuff[8] = stuff[8] + j                          # fmain.c:3263 — 8 = arrows slot
            an.weapon = -1                                   # fmain.c:3264 — mark body as looted
            return
    an.weapon = -1                                           # fmain.c:3268 — mark body as looted
    j = an.race                                              # fmain.c:3269
    if j & 0x80:                                             # fmain.c:3270 — 0x80 = SETFIG_RACE_BIT; setfigs never drop treasure
        j = 0
    else:
        j = encounter_chart[j].treasure * 8 + rand8()        # fmain.c:3272 — 8 = TREASURE_PROBS_COLUMNS
        j = treasure_probs[j]                                # fmain.c:3273 — inv_list row or 0
    if j != 0:                                               # fmain.c:3275 — non-zero slot means loot
        if i != 0:                                           # fmain.c:3276 — chain onto weapon line
            print_cont(" and ")
        if j < GOLDBASE:                                     # fmain.c:3277 — gold rows skip the article
            print_cont("a ")
        print_cont(inv_list[j].name)                         # fmain.c:3278
        if j >= GOLDBASE:                                    # fmain.c:3279 — gold row
            wealth = wealth + inv_list[j].maxshown           # fmain.c:3279 — maxshown doubles as gold value
        else:
            stuff[j] = stuff[j] + 1                          # fmain.c:3280
    elif i == 0:                                             # fmain.c:3282 — no weapon + no treasure
        print_cont("nothing")
    print_cont(".")                                          # fmain.c:3283
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

Source: `fmain.c:3289-3298`
Called by: `option_handler` (via `do_option` `CMODE_ITEMS` branch on `hit == 7`)
Calls: `calc_dist`, `change_object`, `event`, `anix2`

```pseudo
def look_command() -> None:
    """LOOK: reveal every hidden-state object (race==0) within 40 px of the hero."""
    flag = 0                                                 # fmain.c:3290 — "any reveal this tick?" latch
    i = 0
    while i < anix2:                                         # fmain.c:3292 — iterate actor+object table
        an = anim_list[i]
        # race==0 on an OBJECTS entry marks a hidden object (ob_stat==5 in the world-object backing row).
        # change_object(i, 1) promotes it to ob_stat=1 (visible, pickable).
        if an.type == OBJECTS and an.race == 0 and calc_dist(i, 0) < 40:  # fmain.c:3293 — 40 = reveal radius
            flag = 1
            change_object(i, 1)                              # fmain.c:3294 — 1 = OB_STAT_VISIBLE
        i = i + 1
    if flag != 0:                                            # fmain.c:3297 — at least one reveal
        event(38)                                            # fmain.c:3297 — 38 = "you find something hidden"
    else:
        event(20)                                            # fmain.c:3297 — 20 = "you find nothing"
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
- **Bones retire both ghosts.** Picking up *either* brother's bones zeros
  both `ob_listg[3].ob_stat` and `ob_listg[4].ob_stat` in a single chained
  assignment (`fmain.c:3174`),
  retiring both ghost setfigs at once. This is observable in-game (find
  Julian's bones → Phillip's ghost also stops appearing) and is intentional;
  the live brother's identity for the *stash merge* is still selected by
  `vitality & VITALITY_OWNER_MASK` per the Shape contract in
  [SYMBOLS.md §4.1](SYMBOLS.md#41-shape--actor-record-ftaleh56-67-ftalei5-22).
- **Quiver accumulator.** `stuff[35]` (`ARROWBASE`) is set to 0 at the top of
  TAKE (`fmain.c:3150`) and folded into `stuff[8]` in the epilogue
  (`fmain.c:3243` reads `stuff[ARROWBASE] * 10`). The only writer that
  targets `stuff[35]` is the `itrans[]` match for world object `QUIVER = 11`
  (`fmain2.c:981`) — pairing arrows as quivers of ten.
- **Menu refresh.** `take_command`, `search_body`, `look_command`, and
  `use_dispatch` all rely on the outer `set_options()` call at
  `fmain.c:3514` to recompute `menus[*].enabled[]` after any `stuff[]`
  mutation. The `MAGIC` submenu dispatcher
  ([magic.md#magic_dispatch](magic.md#magic_dispatch)) calls `set_options()`
  directly only on the transition to zero, because the displayed row
  changes from selectable to greyed out at that moment.
