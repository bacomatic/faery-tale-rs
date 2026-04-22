# Input Handling — Logic Spec

> Fidelity: behavioral  |  Source files: fsubs.asm, fmain.c
> Cross-refs: [RESEARCH §4](../RESEARCH.md#4-input-system), [_discovery/input-handling.md](../_discovery/input-handling.md), [logic/menu-system.md](menu-system.md)

## Overview

The input subsystem sits upstream of `key_dispatch` (see
[menu-system.md#key_dispatch](menu-system.md#key_dispatch)). It installs a
priority-51 Amiga `input.device` handler that pre-empts Intuition, converts raw
keyboard, mouse-button and timer events into the 7-bit codes defined in
[SYMBOLS.md §7](SYMBOLS.md#7-keycode-values), and enqueues them into a 128-byte
circular buffer on `handler_data`. The main loop drains that buffer one code
per frame via `getkey` and computes the current movement direction via
`decode_mouse`, which fuses mouse-button, hardware-joystick and latched-keydir
sources through the `TABLE:movement_course_map` lookup.

Five functions define the full path: `add_device` installs the handler,
`wrap_device` tears it down, `handler_interface` runs inside the
`input.device` chain to populate the buffer and the mouse/qualifier state,
`getkey` is the game-side consumer, and `decode_mouse` resolves the per-tick
movement direction used by the player-control, AI and compass-render phases.

Routing of the codes returned by `getkey` into menu actions and `keydir` /
`keyfight` latches is owned by `key_dispatch` in menu-system.md and is not
duplicated here.

## Symbols

No new locals are declared in this file. All identifiers resolve to entries in
[SYMBOLS.md](SYMBOLS.md), the `handler_data` fields enumerated in
[_discovery/input-handling.md §2](../_discovery/input-handling.md#2-struct-in_work-ftaleh110-119),
and the `event` fields `type`, `qualifier`, `rawkey`, `dx`, `dy`, `next`
(Amiga `struct InputEvent`: `ie_Class=4(a0)`, `ie_Code=6(a0)`,
`ie_Qualifier=8(a0)`, `ie_X=10(a0)`, `ie_Y=12(a0)`,
`ie_NextEvent=0(a0)`).

## add_device

Source: `fmain.c:3017-3036`
Called by: `entry point`
Calls: `CreatePort`, `CreateStdIO`, `DeletePort`, `OpenDevice`, `DoIO`, `handler_interface`

```pseudo
def add_device() -> bool:
    """Install the priority-51 input handler into the Amiga input.device chain."""
    handler_data.laydown = 0                                    # fmain.c:3020
    handler_data.pickup = 0                                     # fmain.c:3020
    inputDevPort = CreatePort(0, 0)                             # fmain.c:3021
    if inputDevPort == 0:
        return False
    inputRequestBlock = CreateStdIO(inputDevPort)               # fmain.c:3022
    if inputRequestBlock == 0:
        DeletePort(inputDevPort)                                # fmain.c:3023
        return False
    handlerStuff.is_Data = handler_data                         # fmain.c:3025
    handlerStuff.is_Code = handler_interface                    # fmain.c:3026
    handlerStuff.is_Node.ln_Pri = 51                            # fmain.c:3027 — above Intuition's 50
    error = OpenDevice("input.device", 0, inputRequestBlock, 0) # fmain.c:3029
    if error != 0:
        return False
    inputRequestBlock.io_Command = IND_ADDHANDLER               # fmain.c:3032
    inputRequestBlock.io_Data = handlerStuff                    # fmain.c:3033
    DoIO(inputRequestBlock)                                     # fmain.c:3034
    return True
```

## wrap_device

Source: `fmain.c:3038-3046`
Called by: `entry point`
Calls: `DoIO`, `CloseDevice`, `DeleteStdIO`, `DeletePort`

```pseudo
def wrap_device() -> None:
    """Detach the input handler and release its device resources."""
    inputRequestBlock.io_Command = IND_REMHANDLER               # fmain.c:3040
    inputRequestBlock.io_Data = handlerStuff                    # fmain.c:3041
    DoIO(inputRequestBlock)                                     # fmain.c:3042
    CloseDevice(inputRequestBlock)                              # fmain.c:3043
    DeleteStdIO(inputRequestBlock)                              # fmain.c:3044
    DeletePort(inputDevPort)                                    # fmain.c:3045
```

## handler_interface

Source: `fsubs.asm:63-218`
Called by: `entry point`
Calls: `handle_rawkey`, `handle_rawmouse`, `update_pointer`

```pseudo
def handler_interface(events: object) -> int:
    """Input-device callback: walk event chain, translate keys, update mouse."""
    event = events
    while event != 0:
        etype = event.type                                      # fsubs.asm:71 — ie_Class at offset 4
        if etype == INPUT_EVENT_TIMER:                          # fsubs.asm:73, INPUT_EVENT_TIMER = 6
            if handler_data.ticker == 16:                       # fsubs.asm:74 — 16-tick heartbeat threshold
                event.type = INPUT_EVENT_RAWKEY                 # fsubs.asm:77, RAWKEY = 1
                event.rawkey = 0xE0                             # fsubs.asm:78 — undefined scancode $60 | up-bit
                handler_data.ticker = 0                         # fsubs.asm:79
            else:
                handler_data.ticker += 1                        # fsubs.asm:76
        if event.type == INPUT_EVENT_RAWKEY:                    # fsubs.asm:84, RAWKEY = 1
            handle_rawkey(event)
        elif event.type == INPUT_EVENT_RAWMOUSE:                # fsubs.asm:112, RAWMOUSE = 2
            handle_rawmouse(event)
        elif event.type == INPUT_EVENT_DISKIN:                  # fsubs.asm:160, DISKIN = $10
            handler_data.newdisk = 1
        update_pointer(event)
        event = event.next
    return 0
```

## handle_rawkey

Source: `fsubs.asm:84-111`
Called by: `handler_interface`
Calls: `keybuf_push`, `TABLE:keytrans`

```pseudo
def handle_rawkey(event: object) -> None:
    """Translate a RAWKEY, nullify it, enqueue into the 128-byte ring buffer."""
    qual = event.qualifier                                      # fsubs.asm:90
    if (qual & 0x200) != 0:                                     # fsubs.asm:91 — IEQUALIFIER_REPEAT bit 9
        return
    raw = event.rawkey                                          # fsubs.asm:94
    up_bit = raw & KEY_UP_BIT                                   # fsubs.asm:95, KEY_UP_BIT = 0x80
    scancode = raw & KEY_CODE_MASK                              # fsubs.asm:96, KEY_CODE_MASK = 0x7f
    if scancode > KEYTRANS_MAX_SCANCODE:                        # fsubs.asm:97, KEYTRANS_MAX_SCANCODE = 0x5A
        return
    event.type = 0                                              # fsubs.asm:100 — consume before Intuition sees it
    code = keytrans[scancode] | up_bit                          # fsubs.asm:102-104, TABLE:keytrans
    keybuf_push(code)                                           # fsubs.asm:105-112
```

## handle_rawmouse

Source: `fsubs.asm:112-157`
Called by: `handler_interface`
Calls: `keybuf_push`

```pseudo
def handle_rawmouse(event: object) -> None:
    """Detect left-button transitions; synthesize menu-slot codes in the strip."""
    new_qual = event.qualifier                                  # fsubs.asm:115
    changed = handler_data.qualifier ^ new_qual                 # fsubs.asm:117
    if (changed & MOUSE_LEFT_BUTTON) != 0:                      # fsubs.asm:118, MOUSE_LEFT_BUTTON = 0x4000
        if (new_qual & MOUSE_LEFT_BUTTON) == 0:
            prev = handler_data.lastmenu                        # fsubs.asm:123
            if prev != 0:
                keybuf_push(prev | KEY_UP_BIT)                  # fsubs.asm:125-126
                handler_data.lastmenu = 0
        else:
            sx = handler_data.xsprite                           # fsubs.asm:131
            sy = handler_data.ysprite                           # fsubs.asm:132
            if sx >= MOUSE_MENU_X_LO and sx <= MOUSE_MENU_X_HI: # fsubs.asm:134-137 — X range 215..265
                row = (sy - MOUSE_MENU_Y_TOP) // 9              # fsubs.asm:139-141 — Y_TOP=144, 9 px per row
                if row >= 0:
                    code = MOUSE_MENU_BASE + row * 2            # fsubs.asm:142-144, MOUSE_MENU_BASE = 0x61
                    if sx >= MOUSE_MENU_X_SPLIT:                # fsubs.asm:146, X_SPLIT = 240
                        code = code + 1
                    keybuf_push(code)                           # fsubs.asm:149-155
                    handler_data.lastmenu = code                # fsubs.asm:156
    handler_data.qualifier = new_qual                           # fsubs.asm:157
```

## keybuf_push

Source: `fsubs.asm:105-113`
Called by: `handle_rawkey`, `handle_rawmouse`
Calls: `none`

```pseudo
def keybuf_push(code: u8) -> None:
    """Append one translated code to the 128-byte circular buffer; drop on overflow."""
    nxt = (handler_data.laydown + 1) & 0x7F                     # fsubs.asm:109-110 — 128-slot wrap mask
    if nxt == handler_data.pickup:
        return
    handler_data.keybuf[handler_data.laydown] = code
    handler_data.laydown = nxt
```

## update_pointer

Source: `fsubs.asm:163-200`
Called by: `handler_interface`
Calls: `MoveSprite`

```pseudo
def update_pointer(event: object) -> None:
    """Accumulate mouse deltas, clamp to the status-bar strip, move the sprite."""
    x = handler_data.xsprite + event.dx                         # fsubs.asm:163-166 — ie_X at offset 10
    y = handler_data.ysprite + event.dy                         # fsubs.asm:164-167 — ie_Y at offset 12
    if x < 5:                                                   # fsubs.asm:169-175 — sprite X clamp lo
        x = 5                                                   # fsubs.asm:169 — min X = 5
    if x > 315:                                                 # fsubs.asm:172 — clamp hi
        x = 315                                                 # fsubs.asm:172 — max X = 315
    if y < 147:                                                 # fsubs.asm:177 — status-bar Y top
        y = 147                                                 # fsubs.asm:177 — min Y = 147
    if y > 195:                                                 # fsubs.asm:180 — status-bar Y bottom
        y = 195                                                 # fsubs.asm:180 — max Y = 195
    handler_data.xsprite = x
    handler_data.ysprite = y
    if handler_data.pbase == 0:
        return
    MoveSprite(handler_data.gbase, handler_data.pbase, handler_data.vbase, x * 2, y - 143)  # fsubs.asm:186-198 — vp_text offset 143
```

## getkey

Source: `fsubs.asm:281-295`
Called by: `process_input_key`
Calls: `none`

```pseudo
def getkey() -> u8:
    """Pop one code from the 128-byte ring buffer; 0 if empty."""
    rd = handler_data.pickup                                    # fsubs.asm:286
    if rd == handler_data.laydown:                              # fsubs.asm:287 — buffer empty
        return 0
    code = handler_data.keybuf[rd]                              # fsubs.asm:289
    handler_data.pickup = (rd + 1) & 0x7F                       # fsubs.asm:290-292 — 128-slot wrap
    return code
```

## decode_mouse

Source: `fsubs.asm:1488-1590`
Called by: `game_tick` (phase 8)
Calls: `decode_mouse_strip`, `decode_joystick`, `decode_keydir`, `drawcompass`

```pseudo
def decode_mouse() -> None:
    """Resolve the frame's movement direction: mouse > joystick > keydir."""
    qual = handler_data.qualifier                               # fsubs.asm:1491
    if (qual & MOUSE_BUTTON_MASK) != 0:                         # fsubs.asm:1493, MOUSE_BUTTON_MASK = 0x6000
        dir = decode_mouse_strip()
    else:
        dir = decode_joystick()
        if dir == DIR_NONE:
            dir = decode_keydir()
    if dir != oldir:                                            # fsubs.asm:1579
        oldir = dir                                             # fsubs.asm:1581
        drawcompass(dir)                                        # fsubs.asm:1583
```

## decode_mouse_strip

Source: `fsubs.asm:1497-1530`
Called by: `decode_mouse`
Calls: `none`

```pseudo
def decode_mouse_strip() -> i16:
    """Partition the status-bar cursor into a 3x3 compass grid."""
    sx = handler_data.xsprite                                   # fsubs.asm:1497
    sy = handler_data.ysprite                                   # fsubs.asm:1498
    if sx <= MOUSE_DIR_COLS_X_LO:                               # fsubs.asm:1501 — X_LO = 265
        return DIR_NONE
    if sx < MOUSE_DIR_COL_MID_LO:                               # fsubs.asm:1509 — COL_MID_LO = 292
        up = DIR_NW                                             # fsubs.asm:1505
        ctr = DIR_W                                             # fsubs.asm:1506
        dn = DIR_SW                                             # fsubs.asm:1507
    elif sx > MOUSE_DIR_COL_MID_HI:                             # fsubs.asm:1515 — COL_MID_HI = 300
        up = DIR_NE                                             # fsubs.asm:1511
        ctr = DIR_E                                             # fsubs.asm:1512
        dn = DIR_SE                                             # fsubs.asm:1513
    else:
        up = DIR_N                                              # fsubs.asm:1517
        ctr = DIR_NONE                                          # fsubs.asm:1518
        dn = DIR_S                                              # fsubs.asm:1519
    if sy < MOUSE_DIR_ROW_MID_LO:                               # fsubs.asm:1522 — ROW_MID_LO = 166
        return up
    if sy > MOUSE_DIR_ROW_MID_HI:                               # fsubs.asm:1525 — ROW_MID_HI = 174
        return dn
    return ctr
```

## decode_joystick

Source: `fsubs.asm:1533-1562`
Called by: `decode_mouse`
Calls: `read_u8`, `TABLE:movement_course_map`

```pseudo
def decode_joystick() -> i16:
    """Read CIA joystick port 1 at $dff00c; map (xjoy, yjoy) via com2."""
    raw_y = read_u8(JOY1DAT_HIGH)                               # fsubs.asm:1533 — $dff000 + 12
    left_bit = (raw_y >> 1) & 1                                 # fsubs.asm:1534-1536
    fwd_bit = raw_y & 1                                         # fsubs.asm:1537
    yaxis = fwd_bit ^ left_bit                                  # fsubs.asm:1538 — 0 or 1 per axis bit
    raw_x = read_u8(JOY1DAT_LOW)                                # fsubs.asm:1540 — $dff000 + 13
    right_bit = (raw_x >> 1) & 1                                # fsubs.asm:1541-1543
    back_bit = raw_x & 1                                        # fsubs.asm:1544
    xaxis = back_bit ^ right_bit                                # fsubs.asm:1545
    xjoy = xaxis - left_bit                                     # fsubs.asm:1547
    yjoy = yaxis - fwd_bit                                      # fsubs.asm:1548
    if xjoy == 0 and yjoy == 0:
        return DIR_NONE
    idx = 4 + yjoy * 3 + xjoy                                   # fsubs.asm:1554-1558 — 4 = center of com2[9]
    keydir = 1
    return com2[idx]                                            # fsubs.asm:1559-1560
```

## decode_keydir

Source: `fsubs.asm:1567-1580`
Called by: `decode_mouse`
Calls: `none`

```pseudo
def decode_keydir() -> i16:
    """Convert a latched 20..29 keypad code into a 0..9 direction."""
    k = keydir - 20                                             # fsubs.asm:1567-1568 — 20 = KEY_KEYDIR_LO
    if k < 0 or k >= 10:                                        # fsubs.asm:1569-1572 — 10 = range width
        keydir = 0
        return DIR_NONE
    return k
```

## Notes

- `keybuf_push` is factored out of the pseudo-code here for readability; in the
  1987 source it is inlined twice inside `handler_interface` (`fsubs.asm:105-112`
  for RAWKEY, `fsubs.asm:149-156` for RAWMOUSE) with the mouse-branch additionally
  storing `lastmenu` after the enqueue.
- `TABLE:keytrans` (`fsubs.asm:221-226`, 91 bytes) and the keypad → direction
  encoding are documented in
  [_discovery/input-handling.md §4](../_discovery/input-handling.md#4-keytrans-table-fsubsasm221-226).
- The `MOUSE_LEFT_BUTTON` mask in `handle_rawmouse` is the left-button bit.
  `decode_mouse` gates on `MOUSE_BUTTON_MASK` (`0x6000`), which additionally
  accepts the right button. This is deliberate: either button forces the
  mouse-strip direction source, while only the left button synthesizes
  menu-slot codes.
- `keyfight` (the `'0'` hold latch) and the `keydir = key` latch on keypad
  codes `20..29` live in `key_dispatch`, not here. This file's `decode_keydir`
  only consumes the latched value.
