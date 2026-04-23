# Dialog System — Logic Spec

> Fidelity: behavioral  |  Source files: fmain.c, fmain2.c, narr.asm
> Cross-refs: [RESEARCH §7](../RESEARCH.md#7-npc-dialogue), [_discovery/npc-quests.md](../_discovery/npc-quests.md), [messages.md](messages.md), [inventory.md](inventory.md), [menu-system.md](menu-system.md)

## Overview

Text output to the HI scroll area is the game's primary feedback channel. All
such output passes through one of four primitives — `print`, `print_cont`,
`extract`, `prdec` — which all write through the global `rp` (rastport pointer)
to the scroll bitmap. `print()` scrolls the area up 10 pixels before drawing;
`print_cont()` appends in-place without scrolling; `extract()` filters `%` →
current brother name then calls `print` repeatedly for word-wrapping up to 37
chars per line; `prdec()` appends a zero-padded decimal number via `print_cont`.

Message sources fall into two tiers:

1. **narr.asm messages** — indexed null-terminated strings printed via the
   assembly stubs `_speak`, `_event`, and `_msg`. Full string content for
   every index is tabulated in [messages.md](messages.md):
   - `speak(N)` — indexes `_speeches` table (NPC dialogue, `narr.asm:349–end`);
     see [messages.md §2](messages.md#2-npc-speeches--_speeches-narrasm351).
   - `event(N)` — indexes `_event_msg` table (world-event notifications,
     `narr.asm:11–55`); see [messages.md §1](messages.md#1-event-messages--_event_msg-narrasm11).
   - `msg(table, N)` — indexes `_place_msg` or `_inside_msg` (location
     announcements, `narr.asm:164–222`); see
     [messages.md §3](messages.md#3-place-name-messages--_place_msg-narrasm164) and
     [messages.md §4](messages.md#4-interior-messages--_inside_msg-narrasm199).

2. **Hardcoded string literals** — string literals or `extract()`-template
   strings embedded directly in `fmain.c` and `fmain2.c`. These have no index
   and no narr.asm entry; they are the primary concern of this document.

Item names used in composed messages are drawn from `inv_list[].name`
(`fmain.c:381–423`). Brother names are drawn from `datanames[]`
(`fmain.c:604`): `"Julian"`, `"Phillip"`, `"Kevin"`.

### Scroll area geometry

Defined in `fmain2.c:486–489`:

| Constant | Value | Meaning |
|----------|-------|---------|
| `TXMIN`  | 16    | Left edge of scroll raster clip |
| `TXMAX`  | 400   | Right edge |
| `TYMIN`  | 5     | Top edge |
| `TYMAX`  | 44    | Bottom edge |

`print()` calls `ScrollRaster(rp, 0, 10, TXMIN, TYMIN, TXMAX, TYMAX)` then
moves to `(TXMIN, 42)` before drawing — one text row per call.

## Symbols

Locals are scoped to each function below. Global identifiers referenced here:
`rp` (current rastport), `stuff[]` (inventory array), `wealth`, `hunger`,
`brother`, `datanames[]`, `inv_list[]`, `anim_list[]`, `region_num`,
`nearest`, `nearest_person`. All resolve in [SYMBOLS.md](SYMBOLS.md).

## print

Source: `fmain2.c:495-501`
Called by: all message-composition sites in `fmain.c` and `fmain2.c`
Calls: `ScrollRaster`, `move`, `text`, `TXMIN`, `TXMAX`, `TYMIN`, `TYMAX`

```pseudo
def print(str: str) -> None:
    """Scroll the HI area up one row then draw str at the bottom line."""
    l = 0
    while str[l] != '\0':
        l = l + 1
    ScrollRaster(rp, 0, 10, TXMIN, TYMIN, TXMAX, TYMAX)      # fmain2.c:498 — scroll 10 pixels up
    move(TXMIN, 42)                                           # 42 = bottom text baseline
    text(str, l)
```

## print_cont

Source: `fmain2.c:503-507`
Called by: all message-composition sites that continue on the same line
Calls: `text`, `strlen`

```pseudo
def print_cont(str: str) -> None:
    """Append str to the current cursor position without scrolling."""
    l = 0
    while str[l] != '\0':
        l = l + 1
    text(str, l)

```

## prdec

Source: `fmain2.c` (assembly stub, equivalent behavior)
Called by: `ppick`, `aftermath`, body-search composition
Calls: `text`, `format_zero_padded`

```pseudo
def prdec(value: int, width: int) -> None:
    """Append value as a zero-padded decimal of length width, via print_cont."""
    # Implementation detail: format value into a zero-padded buffer of length width
    # then call text(buf, width). The exact formatting routine is in the assembly
    # driver; any zero-padded decimal renderer is conformant.
    buf = format_zero_padded(value, width)                   # platform-supplied decimal formatter
    text(buf, width)
```

## extract

Source: `fmain2.c:510-546`
Called by: `take_command`, `use_dispatch`, `key_dispatch`, `buy_dispatch`, body-search composition
Calls: `print`, `datanames`, `read_next`, `append_str`, `append_char`, `row_up_to`, `row_from`, `strlen_nm`

```pseudo
def extract(start: str) -> None:
    """Word-wrap start onto scroll rows, substituting % with the brother's name."""
    # Uses a character-pointer model: start advances through the source string.
    # mbuf accumulates the current output row; lstart marks the row's beginning.
    # lbreak tracks the last space seen (word-wrap point).
    mbuf = mesbuf                                             # fmain2.c:510 — static 200-byte row accumulator
    mbuf_len = 0                                              # length of current row buffer
    lstart_off = 0                                            # offset in mesbuf of current row start
    i = 0
    while True:
        lbreak = -1
        while i < 37:                                         # fmain2.c:516 — 37 = max chars per row before forced wrap
            c = read_next(start)                              # advance source pointer, return char
            if c == ' ':
                lbreak = mbuf_len
            if c == '\0':
                lbreak = mbuf_len
                i = 1000                                      # fmain2.c:519 — flag: string ended
            if c == '\r':
                lbreak = mbuf_len
                break
            if c == '%':
                nm = datanames[brother - 1]                   # fmain2.c:524 — substitute brother name
                append_str(mbuf, nm)
                mbuf_len = mbuf_len + strlen_nm(nm)
                i = i + strlen_nm(nm)
            else:
                append_char(mbuf, c)
                mbuf_len = mbuf_len + 1
            i = i + 1
        if lbreak >= 0:                                       # fmain2.c:530 — found a wrap point
            print(row_up_to(mbuf, lstart_off, lbreak))
            if i > 38:                                        # fmain2.c:533 — 38 = end-of-string sentinel check
                break
            lstart_off = lstart_off + lbreak + 1             # advance past the null terminator
            i = mbuf_len - lbreak
        else:                                                 # fmain2.c:538 — no wrap point; flush whole row
            print(row_from(mbuf, lstart_off))
            if i > 38:                                        # fmain2.c:541 — 38 = end-of-string sentinel check
                break
            lstart_off = mbuf_len
            i = 0
```

## announce_treasure

Source: `fmain2.c:586-590`
Called by: `take_command`
Calls: `print`, `print_cont`, `datanames`

```pseudo
def announce_treasure(s: str) -> None:
    """Print "{name} found {s}" to the scroll area."""
    print(datanames[brother - 1])
    print_cont(" found ")
    print_cont(s)
```

## announce_container

Source: `fmain2.c:579-584`
Called by: `take_command`
Calls: `print`, `print_cont`, `datanames`

```pseudo
def announce_container(s: str) -> None:
    """Print "{name} found {s} containing " to the scroll area."""
    print(datanames[brother - 1])
    print_cont(" found ")
    print_cont(s)
    print_cont(" containing ")
```

## aftermath

Source: `fmain2.c:1259-1278`
Called by: main combat completion path (`fmain.c:2187`)
Calls: `print`, `print_cont`, `prdec`, `get_turtle`

```pseudo
def aftermath() -> None:
    """Summarise battle outcome to the scroll area after combat ends."""
    dead = 0
    flee = 0
    i = ENEMY_ACTOR_START                                     # fmain2.c:1263 — actors 0=hero, 1=swan, 2=raft; enemies start at 3
    while i < anix:
        if anim_list[i].type != ENEMY:
            pass
        elif anim_list[i].state == STATE_DEAD:
            dead += 1
        elif anim_list[i].goal == GOAL_FLEE:
            flee += 1
        i = i + 1

    if anim_list[0].vitality < 1:                            # fmain2.c:1268 — hero is dead; no message
        pass
    elif anim_list[0].vitality < 5 and dead > 0:             # fmain2.c:1269 — barely survived
        print("Bravely done!")
    elif xtype < 50:                                          # fmain2.c:1271 — 50 = outdoor encounter threshold
        if dead > 0:
            print("")
            prdec(dead, 1)
            print_cont("foes were defeated in battle.")
        if flee > 0:
            print("")
            prdec(flee, 1)
            print_cont("foes fled in retreat.")

    if turtle_eggs:
        get_turtle()
```

## ppick (print queue dispatcher)

Source: `fmain2.c:437-471`
Called by: main game loop (dequeues from `print_que[]`)
Calls: `print`, `print_cont`, `prdec`, `text`, `move`, `print_options`

The print queue (`prq(N)`) defers HUD-panel and scroll-area updates to the
main loop so they can be batched. Only codes 4, 5, 7 update the HUD panel
via `text()` / `move()` without scrolling. Codes 2, 3, 10 write to the
scroll area.

```pseudo
def ppick() -> None:
    """Dequeue and execute one pending print-queue entry."""
    if prec == pplay:
        return
    p = print_que[pplay]
    pplay = (pplay + 1) & 31                                 # fmain2.c:441 — 31 = queue mask; wraps at 32
    match p:
        case 2:                                               # debug: coordinates (cheat key '=')
            print("Coords = ")
            prdec(hero_x, 6)
            prdec(hero_y, 6)
            print("Memory Available: ")
            prdec(avail_mem(), 6)
        case 3:                                               # debug: location (cheat key ctrl-S)
            print("You are at: ")
            prdec(hero_x // 256, 3)
            prdec(hero_y // 256, 3)
            print("H/Sector = ")
            prdec(hero_sector, 3)
            text(" Extent = ", 10)                           # fmain2.c:455 — runs without scrolling
            prdec(xtype, 2)
        case 4:                                               # HUD vitality panel refresh
            move(245, 52)
            text("Vit:", 4)
            prdec(anim_list[0].vitality, 3)
        case 5:                                               # options menu refresh
            print_options()
        case 7:                                               # HUD stat panel refresh
            if luck < 0:
                luck = 0
            move(14, 52);  text("Brv:", 4); prdec(brave, 3)
            move(90, 52);  text("Lck:", 4); prdec(luck, 3)
            move(168, 52); text("Knd:", 4); prdec(kind, 3)
            move(321, 52); text("Wlth:", 5); prdec(wealth, 3)
        case 10:                                              # fmain2.c:467 — TAKE with nothing nearby
            print("Take What?")
```

## Hardcoded scroll messages — complete reference

These messages are emitted by hardcoded string literals. None have a `speak()`
or `event()` index. An implementation must render all of them to be
behaviourally faithful.

### Door / key feedback (`fmain.c`)

| Source | Text | Condition |
|--------|------|-----------|
| `fmain.c:1117` | `"It opened."` | Key matched door; door tile replaced |
| `fmain.c:1122` | `"It's locked."` | Bumped locked door and no key matched (`!bumped && !keytype`) |

### Arrows / bow (`fmain.c`)

| Source | Text | Condition |
|--------|------|-----------|
| `fmain.c:1694` | `"No Arrows!"` | Bow selected as weapon; `stuff[8] == 0` |

### TAKE — treasure pickup composition (`fmain.c`)

All fragments are assembled via `announce_treasure()`, `announce_container()`,
`print()`, `print_cont()`, `prdec()`. `%` is substituted with the current
brother name at composition time.

| Source | Composed text | Condition |
|--------|--------------|-----------|
| `fmain.c:3157` | `"{name} found 50 gold pieces"` | Money object (`j == MONEY`) |
| `fmain.c:3173` | `"{name} found his brother's bones."` | Dead brother bones object (`j == 28`) |
| `fmain.c:3181` | `"{name} found a chest containing "` | Chest object (`j == 0x0f`); followed by contents |
| `fmain.c:3182` | `"{name} found a brass urn containing "` | Urn object (`j == 0x0e`); followed by contents |
| `fmain.c:3183` | `"{name} found some sacks containing "` | Sacks object (`j == 0x10`); followed by contents |
| `fmain.c:3191–3193` | `"{name} found a {item_name}."` | Single known item found in container via `itrans[]` |
| `fmain.c:3202` | `"nothing."` | Container empty; `rand4() == 0` |
| `fmain.c:3207–3208` | `"a {item_name}."` | One random item; `rand4() == 1` |
| `fmain.c:3214–3219` | `"{item_name} and a {item_name2}"` or `"{item_name} a {item_name2}"` | Two random items; `rand4() == 2` |
| `fmain.c:3226` | `"3 keys."` | Three random keys; `rand4() == 3`, `rand8() == 8` |
| `fmain.c:3235–3236` | `"3 {item_name}s."` | Three of same item; `rand4() == 3` |

### TAKE — body search composition (`fmain.c:3251-3283`)

Built from `extract("% searched the body and found")` then `print_cont` /
`prdec` fragments. `%` is substituted with the brother name.

| Fragment source | Text fragment | Condition |
|-----------------|--------------|-----------|
| `fmain.c:3251` | `"% searched the body and found"` | Always; opens the body-search line |
| `fmain.c:3255` | `"a "` | Weapon found (`i > 0`) |
| `fmain.c:3256` | `{weapon_name}` | Drawn from `inv_list[i-1].name` |
| `fmain.c:3260` | `" and "` | Weapon found and bow (weapon `i==4`) |
| `fmain.c:3261` | `{N}` | `prdec(rand8()+2, 1)` — arrow count (`2..9`) |
| `fmain.c:3262` | `" Arrows."` | Bow found; arrow bonus appended |
| `fmain.c:3276` | `" and "` | Second item follows first |
| `fmain.c:3277` | `"a "` | Treasure item is countable (`j < GOLDBASE`) |
| `fmain.c:3278` | `{treasure_name}` | Drawn from `inv_list[j].name` via `treasure_probs[]` |
| `fmain.c:3282` | `"nothing"` | No weapon and no treasure |
| `fmain.c:3283` | `"."` | Always; closes the body-search line |

### USE menu (`fmain.c`)

| Source | Text | Condition |
|--------|------|-----------|
| `fmain.c:3352` | `"That feels a lot better!"` | Healing potion used; vitality not yet at maximum |
| `fmain.c:3436–3437` | `"% bought a {item_name}."` | `extract("% bought a ")` + `inv_list[i].name` + `"."` — shop purchase (non-food, non-arrows) |
| `fmain.c:3440` | `"Not enough money!"` | BUY: `wealth <= j` (item cost not met) |
| `fmain.c:3450` | `"% has no keys!"` | `extract("% has no keys!")` — USE > Keys, `hitgo == 0` |
| `fmain.c:3455` | `"% doesn't have one."` | `extract("% doesn't have one.")` — USE weapon not in inventory |
| `fmain.c:3483–3485` | `"% tried a {key_name} but it didn't fit."` | `extract("% tried a ")` + `inv_list[KEYBASE+hit].name` + `" but it didn't"` + `print("fit.")` — key tried on all adjacent tiles, none matched |

### Battle aftermath (`fmain2.c`)

| Source | Text | Condition |
|--------|------|-----------|
| `fmain2.c:262` | `"Bravely done!"` | Hero `vitality < 5` after combat and at least one kill |
| `fmain2.c:265–266` | `"{N} foes were defeated in battle."` | `prdec(dead,1)` + `print_cont(…)`; `xtype < 50` |
| `fmain2.c:269–270` | `"{N} foes fled in retreat."` | `prdec(flee,1)` + `print_cont(…)`; `xtype < 50` |

### Eating (`fmain2.c`)

| Source | Text | Condition |
|--------|------|-----------|
| `fmain2.c:1707` | `"Yum!"` | Food eaten; `hunger >= 0` after adjustment (non-overflow path) |

### Save / load system (`fmain2.c`)

These messages appear only on floppy-drive configurations. Hard-drive installs
skip the disk-prompt paths entirely.

| Source | Text | Condition |
|--------|------|-----------|
| `fmain2.c:1498` | `"Insert a writable disk in ANY drive."` | Save: no writable floppy found after checking df0:, df1:, df2: |
| `fmain2.c:1499` | `"Aborted."` | User cancelled the disk-insert wait |
| `fmain2.c:1532` | `"ERROR: Couldn't save game."` | `svflag==1` and `Open()` failed or `IoErr()` set after write |
| `fmain2.c:1533` | `"ERROR: Couldn't load game."` | `svflag==0` and `Open()` failed or `IoErr()` set after read |
| `fmain2.c:1538` | `"Please insert GAME disk."` | Post-load: waiting for `df0:winpic` to become readable (disk 1 not present) |
