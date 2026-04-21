# Shops — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c, fmain2.c
> Cross-refs: [RESEARCH §6](../RESEARCH.md#6-items--inventory), [RESEARCH §17](../RESEARCH.md#17-npc-interactions), [npc-dialogue.md](npc-dialogue.md#bartender_speech), [_discovery/npc-quests.md](../_discovery/npc-quests.md), [_discovery/npc-item-location-map.md](../_discovery/npc-item-location-map.md)

## Overview

*Faery Tale Adventure* has exactly one vendor NPC class — the **bartender**
(setfig index 8, race byte `0x88`). There is no separate weapon shop, armor
shop, inn-keeper, or general merchant; every commercial interaction in the
game flows through a single inline branch in `do_option`, the `CMODE_BUY`
case at `fmain.c:3424-3442`.

The transaction pipeline is:

1. **Dialogue** — the player first TALKs to the bartender; speech selection
   lives in [`bartender_speech`](npc-dialogue.md#bartender_speech) and is
   purely informational (no state change, no price quote).
2. **Menu entry** — the player selects the main menu's `Buy` slot (`hit==3`
   at `fmain.c:1327`, `cmode = BUY`) or presses one of the seven buy-menu
   hot-keys (`O R 8 C W B E`) registered at `fmain.c:541`. The buy-menu label
   list is `label5 = "Food ArrowVial Mace SwordBow  Totem"` (`fmain.c:500`),
   giving slots 5..11. The default `enabled[]` template `{… 10,10,10,10,10,
   10,10}` (`fmain.c:525`) marks every slot as immediate-action; no
   dynamic enable/disable is performed — all seven items are always listed
   even when the nearest actor is not a bartender.
3. **Purchase** — [`buy_dispatch`](#buy_dispatch) resolves the slot to
   an `(inventory_index, price)` row in `TABLE:jtrans`, checks `wealth > price`
   (strict), deducts gold, and applies the per-slot side effect.

**Vendor inventory (row map).** `TABLE:jtrans` is a 14-byte flat array of
7 `(i, j)` pairs at `fmain2.c:850`:

| Menu slot | Key | Label | `i` (inv_list index) | `j` (gold) | Side effect |
|---|---|---|---|---|---|
| 5 | `O` | Food    | 0  | 3  | `event(22)`; `eat(50)` — hunger decreases 50 |
| 6 | `R` | Arrow   | 8  | 10 | `stuff[8] += 10`; `event(23)` — 10-arrow bundle |
| 7 | `8` | Vial    | 11 | 15 | `stuff[11]++`; print "% bought a Glass Vial." |
| 8 | `C` | Mace    | 1  | 30 | `stuff[1]++`; print "% bought a Mace." |
| 9 | `W` | Sword   | 2  | 45 | `stuff[2]++`; print "% bought a Sword." |
| 10| `B` | Bow     | 3  | 75 | `stuff[3]++`; print "% bought a Bow." |
| 11| `E` | Totem   | 13 | 20 | `stuff[13]++`; print "% bought a Bird Totem." |

Item `i == 0` is a sentinel: inventory slot 0 is the Dirk (`inv_list[0]`) and
is not granted by the shop — instead the Food branch fires `eat(50)` and a
narration event. Item `i == 8` (arrows, `ARROWBASE-relative count`) is a
second sentinel: arrows are sold in ten-shot bundles rather than singletons.
All other purchases increment `stuff[i]` by one and narrate via
`"% bought a "` + `inv_list[i].name`.

**Ale / rest / sleep.** The TALK speech tree mentions "Have a drink!"
(`speak(14)`) and "do you just need lodging?" (`speak(12)`), but there is
**no code path** that sells ale or provides paid lodging. The player rests
by walking onto a sleeping-spot terrain tile; the sleep trigger at
`fmain.c:1877-1889` runs unconditionally once `sleepwait == 30` and is not
gated by proximity to a bartender or by a gold cost. The bartender's only
transactional effect is the seven-slot `buy_dispatch` branch below.

**Failure modes.**

- `nearest_person == 0` — silent break; no menu is redrawn.
- Nearest is not a bartender (`race != 0x88`) — silent break; `wealth` is
  untouched and no item is granted. The buy menu is therefore inert in front
  of any non-bartender target.
- `wealth <= price` — prints `"Not enough money!"` and breaks.
- `hit > 11` — `return` (not `break`); this short-circuits the normal
  post-dispatch `set_options()` refresh at `fmain.c:3507`.

## Symbols

No new locals beyond the function's declared parameters. Proposed SYMBOLS
additions (`BARTENDER_RACE`, `BUY_HIT_BASE`, `BUY_SENTINEL_FOOD`,
`BUY_SENTINEL_ARROWS`, `BUY_FOOD_HUNGER_DELTA`, `BUY_ARROW_BUNDLE_SIZE`,
`EVENT_BOUGHT_FOOD`, `EVENT_BOUGHT_ARROWS`, `PRQ_STATS_REFRESH`,
`TABLE:jtrans`, `TABLE:inv_list`, `nearest`, `inv_list` global) are listed
in the wave report.

## buy_dispatch

Source: `fmain.c:3424-3442`
Called by: `do_option` (`CMODE_BUY` case)
Calls: `nearest_person`, `anim_list`, `wealth`, `stuff`, `prq`, `event`, `eat`, `extract`, `print_cont`, `print`, `jtrans`, `inv_list`

```pseudo
def buy_dispatch(hit: int) -> None:
    """Resolve one CMODE_BUY slot to a price lookup, gold check, and inventory side effect."""
    nearest = nearest_person                              # fmain.c:3425 — global side-effect of nearest_fig
    if nearest == 0:                                      # fmain.c:3425 — no candidate target
        return
    if anim_list[nearest].race != 0x88:                   # fmain.c:3426 — 0x88 = bartender setfig race byte
        return
    if hit > 11:                                          # fmain.c:3427 — 11 = last BUY slot (Totem)
        return
    row = (hit - 5) * 2                                   # fmain.c:3428 — 5 = first BUY slot (Food); *2 = (i,j) pair stride
    i = jtrans[row]                                       # fmain.c:3429 — inv_list index (or sentinel 0/8)
    j = jtrans[row + 1]                                   # fmain.c:3429 — gold cost
    if wealth <= j:                                       # fmain.c:3430 — strict > check in source
        print("Not enough money!")                        # fmain.c:3440 — hard-coded denial string
        return
    wealth = wealth - j                                   # fmain.c:3431 — deduct price
    prq(7)                                                # fmain.c:3432 — 7 = stats-line refresh priority
    if i == 0:                                            # fmain.c:3433 — sentinel: Food slot
        event(22)                                         # narr.asm event_msg[22] — "% ate some food"
        eat(50)                                           # fmain.c:3433; fmain2.c:1706 — 50 = hunger delta for one meal
        return
    if i == 8:                                            # fmain.c:3434 — sentinel: arrow bundle (stuff[8] == arrows)
        stuff[i] = stuff[i] + 10                          # fmain.c:3434 — 10 = arrows per purchase
        event(23)                                         # narr.asm event_msg[23] — "% bought 10 arrows"
        return
    stuff[i] = stuff[i] + 1                               # fmain.c:3436 — single-item grant
    extract("% bought a ")                                # fmain.c:3436 — % expands to hero name
    print_cont(inv_list[i].name)                          # fmain.c:3437 — inv_list row name field
    print_cont(".")                                       # fmain.c:3437 — sentence terminator
```

## Notes

- `buy_dispatch` always falls through to `do_option`'s `set_options()` call at
  `fmain.c:3507` **except** when `hit > 11`, where the original source uses
  `return` rather than `break`. A re-implementation that replaces the
  enclosing `switch` with a function dispatch must preserve that asymmetry
  (no menu refresh on the bad-slot path).
- `wealth` is a signed `i32` (`fmain.c`); the `wealth > j` check at
  `fmain.c:3430` means a player with exactly `j` gold cannot afford a `j`-gold
  item. This is a deliberate 1-gold margin, not a bug — the lowest item costs
  3 gold so overflow is not a concern.
- The buy menu's `enabled[]` bytes are never updated by `set_options`
  (`fmain.c:3528-3544`); any BUY slot is always green even in front of a
  non-bartender. The player's only feedback for "wrong target" is silence
  from `buy_dispatch`.
- `eat(50)` at `fmain2.c:1704-1708` clamps `hunger` at 0 and fires a separate
  `event(13)` "starvation cleared" narration whenever the meal takes hunger
  below zero; this is an implicit chain reaction of the Food purchase.
- The `% bought a <Glass Vial/Mace/Sword/Bow/Bird Totem>` string uses the raw
  `inv_list[i].name` field unmodified; there is no "the" or plural handling.
  The Arrow branch is therefore a separate code path because "bought a Arrows"
  would be ungrammatical.
