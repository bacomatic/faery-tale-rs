# Menu System — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c, fsubs.asm
> Cross-refs: [RESEARCH §13](../RESEARCH.md#18-menu-system), [_discovery/menu-system.md](../_discovery/menu-system.md)

## Overview

The menu system is the hero's one and only command channel. A `menus[10]` array
holds the ten menu modes (ITEMS, MAGIC, TALK, BUY, GAME, SAVEX, KEYS, GIVE, USE,
FILE); the currently visible one is indexed by `cmode`. Each menu entry carries
an `enabled` byte whose low two bits are render flags (selected / visible) and
whose upper six bits encode an action type (`MENU_ATYPE_MASK`). The action type
determines what happens when a user clicks or presses the bound shortcut key —
ranging from plain navigation to a toggle flip or a one-shot action dispatch.

Two functions define the full input path. `key_dispatch` consumes one code
from the input ring buffer (populated by the rawkey/mouse handler in
`fsubs.asm`) and routes it to transient-screen dismissal, the keydir/keyfight
latches, the menu-click handler, a KEYS-submenu digit shortcut, or a letter
shortcut looked up in `TABLE:letter_list`. `option_handler` is the shared
final stage for both mouse clicks and most letter shortcuts: it reads
`enabled[hit] & MENU_ATYPE_MASK` and branches on the action type.

## Symbols

No new locals are declared in this file; all identifiers used below resolve to
entries in [SYMBOLS.md](SYMBOLS.md) or to the function-local bindings shown in
each pseudo block.

## option_handler

Source: `fmain.c:1302-1330`
Called by: `key_dispatch`
Calls: `propt`, `do_option`, `prq`

```pseudo
def option_handler(key: int, inum: int) -> None:
    """Handle one menu-slot mouse event; branch on enabled[hit] action type."""
    # fmain.c:1303 — clamp the screen slot into the 12-wide real_options[] range
    inum = clamp(inum, 0, 11)                                   # fmain.c:1303
    hit = real_options[inum]                                    # fmain.c:1306
    if (key & KEY_UP_BIT) != 0:
        # fmain.c:1307-1308 — mouse-up: un-highlight the slot and stop
        if hit >= 0 and hit < menus[cmode].num:
            propt(inum, 0)
        return
    hitgo = True                                                # fmain.c:1311
    if hit < 0 or hit >= menus[cmode].num:
        return
    atype = menus[cmode].enabled[hit] & MENU_ATYPE_MASK         # fmain.c:1313
    if atype != ATYPE_IMMEDIATE:
        handler_data.lastmenu = 0                               # fmain.c:1314
    if atype == ATYPE_TOGGLE:
        # fmain.c:1315-1319 — flip the selected bit, redraw, fire action
        menus[cmode].enabled[hit] ^= MENU_FLAG_SELECTED
        propt(inum, menus[cmode].enabled[hit] & MENU_FLAG_SELECTED)
        do_option(hit)
        return
    if atype == ATYPE_IMMEDIATE:
        # fmain.c:1320-1323 — highlight, fire action; caller un-highlights on mouse-up
        propt(inum, 1)
        do_option(hit)
        return
    if atype == ATYPE_ONESHOT:
        # fmain.c:1324-1328 — force selected bit on, highlight, fire
        menus[cmode].enabled[hit] |= MENU_FLAG_SELECTED
        propt(inum, 1)
        do_option(hit)
        return
    # fmain.c:1329 — atype == ATYPE_NAV (or anything else)
    notpause = (menus[CMODE_GAME].enabled[5] & MENU_FLAG_SELECTED) == 0  # fmain.c:1282, slot 5 = Pause
    if notpause and atype == ATYPE_NAV and hit < 5:             # fmain.c:1329, top-bar is slots 0..4
        cmode = hit
        prq(5)                                                  # fmain.c:1329, prq(5) = request menu redraw
        return
    # fmain.c:1330 — fallback: just reassert the current highlight
    propt(inum, menus[cmode].enabled[hit] & MENU_FLAG_SELECTED)
```

## key_dispatch

Source: `fmain.c:1278-1363`
Called by: `entry point`
Calls: `option_handler`, `do_option`, `gomenu`, `print_options`, `cheat_key`, `TABLE:letter_list`

```pseudo
def key_dispatch(key: int) -> None:
    """Route one decoded code from the input ring buffer."""
    # fmain.c:1278 — empty ring buffer
    if key == 0:
        return
    notpause = (menus[CMODE_GAME].enabled[5] & MENU_FLAG_SELECTED) == 0  # fmain.c:1282, slot 5 = Pause
    # fmain.c:1283-1285 — any key-down dismisses a transient full-screen view
    if viewstatus != 0 and notpause:
        if (key & KEY_UP_BIT) == 0:
            viewstatus = 99                                     # fmain.c:1285, 99 = corrupt/redraw sentinel
        return
    if player.state == STATE_DEAD:                              # fmain.c:1286 — ignore input while dead
        return
    # fmain.c:1287-1290 — numeric-keypad keydir latch
    if key >= KEY_KEYDIR_LO and key <= KEY_KEYDIR_HI:
        keydir = key
        return
    if (key & KEY_CODE_MASK) == keydir:
        keydir = 0                                              # fmain.c:1290 — up event releases keydir
        return
    # fmain.c:1291-1292 — '0' held-down fight mode; up event releases
    if key == KEY_FIGHT_DOWN:
        keyfight = True
        return
    if (key & KEY_CODE_MASK) == KEY_FIGHT_DOWN:
        keyfight = False
        return
    # fmain.c:1294-1343 — debug cheats (gated by cheat1). See RESEARCH §19; folded here for brevity.
    if cheat1 and cheat_key(key):
        return
    # fmain.c:1305 — synthetic menu-slot codes from the mouse branch of the input handler
    if (key & KEY_CODE_MASK) >= MOUSE_MENU_BASE:
        option_handler(key, (key & KEY_CODE_MASK) - MOUSE_MENU_BASE)
        return
    # fmain.c:1341-1344 — KEYS submenu shortcut: digits '1'..'6' select a key slot
    if cmode == CMODE_KEYS and (key & KEY_UP_BIT) == 0:
        if key >= KEY_DIGIT_1 and key <= KEY_DIGIT_6:
            do_option(key - KEY_DIGIT_1 + 5)                    # fmain.c:1342, slots 5..10 hold key items
        else:
            gomenu(CMODE_ITEMS)                                 # fmain.c:1343 — any other key exits KEYS
        return
    # fmain.c:1345 — space always fires (e.g. to unpause); other letters only when not paused
    if key != KEY_SPACE and not notpause:
        return
    # fmain.c:1346-1358 — letter-shortcut lookup
    for entry in letter_list:
        if entry.letter != key:
            continue
        menu = entry.menu
        if menu == CMODE_SAVEX and cmode != CMODE_SAVEX:
            return                                              # fmain.c:1350 — Save/Exit only from SAVEX
        cmode = menu
        hit = entry.choice
        hitgo = menus[cmode].enabled[hit] & MENU_FLAG_VISIBLE   # fmain.c:1354
        atype = menus[cmode].enabled[hit] & MENU_ATYPE_MASK     # fmain.c:1355
        if atype == ATYPE_TOGGLE:
            menus[cmode].enabled[hit] ^= MENU_FLAG_SELECTED     # fmain.c:1356
        do_option(hit)
        print_options()
        return
```
