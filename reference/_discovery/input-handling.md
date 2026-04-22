# Discovery: Input Handling (Keyboard, Joystick, Mouse)

**Status**: complete
**Investigated**: 2026-04-05
**Requested by**: orchestrator
**Prompt summary**: Trace the complete input system — handler installation, interrupt handler, joystick/keyboard/mouse processing, keyboard buffer, key translation, and shortcut tables.

## 1. Handler Installation: `add_device()` / `wrap_device()`

### `add_device()` — fmain.c:3017-3036

Installs a custom input event handler into the Amiga input.device chain:

1. Clears the keyboard buffer: `handler_data.laydown = handler_data.pickup = 0` — fmain.c:3020
2. Creates a message port: `inputDevPort = CreatePort(0,0)` — fmain.c:3021
3. Creates a standard I/O request: `inputRequestBlock = CreateStdIO(inputDevPort)` — fmain.c:3022
4. Fills in the `struct Interrupt handlerStuff`:
   - `is_Data = (APTR)&handler_data` — the `struct in_work` instance (fmain.c:3025)
   - `is_Code = (void (*)())HandlerInterface` — the assembly handler entry point (fmain.c:3026)
   - `is_Node.ln_Pri = 51` — priority 51, higher than Intuition's 50, so this handler sees events first (fmain.c:3027)
5. Opens the input device: `OpenDevice("input.device",0,...)` — fmain.c:3029
6. Sends `IND_ADDHANDLER` command to register the handler — fmain.c:3032-3035

### `wrap_device()` — fmain.c:3038-3046

Teardown in reverse order:
1. Sends `IND_REMHANDLER` to deregister — fmain.c:3040-3042
2. `CloseDevice()` — fmain.c:3043
3. `DeleteStdIO()` / `DeletePort()` — fmain.c:3044-3045

### Initialization Context

Before `add_device()` is called (fmain.c:785-788):
- `handler_data.xsprite = handler_data.ysprite = 320` — initial cursor position
- `handler_data.gbase = GfxBase` — GfxBase pointer for sprite movement
- `handler_data.pbase = 0` — no sprite initially (set to `&pointer` later at fmain.c:1258)

After display setup (fmain.c:943):
- `handler_data.vbase = &vp_text` — ViewPort for MoveSprite calls

### Variable Declarations — fmain.c:3008-3010

```c
struct MsgPort *inputDevPort;
struct IOStdReq *inputRequestBlock;
struct Interrupt handlerStuff;
```

## 2. `struct in_work` — ftale.h:110-119

The handler's private data area, passed as `a1` to the interrupt handler:

| Offset | Field | Type | Purpose |
|--------|-------|------|---------|
| 0 | `xsprite` | short | Current mouse pointer X position (pixel), clamped 5–315 |
| 2 | `ysprite` | short | Current mouse pointer Y position (pixel), clamped 147–195 |
| 4 | `qualifier` | short | Last input event qualifier word (button/modifier state) |
| 6 | `laydown` | UBYTE | Keyboard buffer write pointer (0–127, wraps with AND $7f) |
| 7 | `pickup` | UBYTE | Keyboard buffer read pointer (0–127, wraps with AND $7f) |
| 8 | `newdisk` | char | Set to 1 on DISKIN event (disk inserted) |
| 9 | `lastmenu` | char | Last mouse-click menu character; cleared on up-click |
| 10 | `gbase` | struct GfxBase* | GfxBase pointer (for MoveSprite) |
| 14 | `pbase` | struct SimpleSprite* | Pointer to SimpleSprite (NULL = no sprite updates) |
| 18 | `vbase` | struct ViewPort* | ViewPort for MoveSprite |
| 22 | `keybuf[128]` | unsigned char[] | 128-byte circular keyboard buffer |
| 150 | `ticker` | short | Timer heartbeat counter (0–16, wraps) |

**Byte offsets are derived from assembly access patterns:**
- Offsets 0–9 are confirmed by the assembly equates `xsprite equ 0`, `ysprite equ 2`, `qualifier equ 4` (fsubs.asm:60-62) and field access at offsets 6 (laydown), 7 (pickup), 8 (newdisk), 9 (lastmenu).
- `gbase` at offset 10 = `10(a1)` in `nosprite` block — fsubs.asm:195
- `pbase` at offset 14 = `14(a1)` tested for NULL — fsubs.asm:191
- `vbase` at offset 18 = `18(a1)` loaded for MoveSprite — fsubs.asm:196
- `keybuf` at offset 22 = `22(a1,d1.w)` — fsubs.asm:101, 144
- `ticker` at offset 22+128=150 = `22+128(a1)` — fsubs.asm:74-76

## 3. `_HandlerInterface` — The Interrupt Handler (fsubs.asm:63-218)

Entry: `a0` = input event chain pointer, `a1` = `&handler_data`.

The handler iterates through the linked list of input events (`ie_NextEvent` at offset 0) and processes each based on its type byte at offset 4:

### 3.1 TIMER Events (type 6) — fsubs.asm:72-80

A heartbeat mechanism:
- Checks `ticker` field (offset 22+128 of `handler_data`): if it reaches 16, resets to 0 and synthesizes a fake RAWKEY event with keycode `$80+$60` (key-up for scancode $60 = undefined). The translated value for scancode $5A+ is 0, so this effectively generates a null heartbeat key-up.
- Otherwise increments `ticker` by 1.

Purpose: generates periodic synthetic events so the game loop doesn't stall waiting for real input.

### 3.2 RAWKEY Events (type 1) — fsubs.asm:82-110

Keyboard processing:
1. Reads qualifier word at offset 8 of the event — fsubs.asm:90
2. Checks bit 9 (repeat flag); if set, **ignores the event** — fsubs.asm:91-92
3. Reads raw keycode at offset 6 of the event — fsubs.asm:94
4. Saves bit 7 (up/down flag) in `d1` — fsubs.asm:95
5. Masks to 7-bit scancode: `AND #$7f` — fsubs.asm:96
6. Ignores scancodes > $5A (undefined keys) — fsubs.asm:97-98
7. **Nullifies the event** by setting type to $0000, so Intuition doesn't see it — fsubs.asm:100
8. Translates via `keytrans` table: `d0 = keytrans[scancode]` — fsubs.asm:102
9. Restores up/down bit: `d0 = translated_key | (original_bit7)` — fsubs.asm:103-104
10. Writes to circular `keybuf`: `keybuf[laydown] = d0` — fsubs.asm:106-107
11. Increments `laydown`, wraps to 0–127 — fsubs.asm:109-110
12. Checks for overflow (laydown == pickup → drop the key) — fsubs.asm:111-112

### 3.3 RAWMOUSE Events (type 2) — fsubs.asm:114-159

Mouse button processing:
1. Reads new qualifier (`d1`) from event offset 8 — fsubs.asm:115
2. XORs with old qualifier to detect changes: `d0 = old XOR new` — fsubs.asm:117
3. Tests bit 14 (`$4000` = left mouse button) for change — fsubs.asm:118
4. If left button **released** (was down, now up):
   - Reads `lastmenu` (offset 9) — fsubs.asm:123
   - If non-zero, ORs with $80 (marks as key-up) and queues it in keybuf — fsubs.asm:125-126
   - Clears `lastmenu` — fsubs.asm:127
5. If left button **pressed** (was up, now down):
   - Reads `xsprite`/`ysprite` — fsubs.asm:131-132
   - Tests if click is in the menu area (X: 215–265) — fsubs.asm:134-137
   - If outside menu area: `d2 = 9` (no direction) — fsubs.asm:130
   - If in menu area, computes menu row: `(ysprite - 144) / 9 + 'a'` — fsubs.asm:139-142
   - Left column (X < 240): character stays as-is — fsubs.asm:144
   - Right column (X >= 240): character incremented by 1 — fsubs.asm:147-148
   - Result queued into keybuf and saved as `lastmenu` — fsubs.asm:149-156
6. Stores new qualifier into `handler_data.qualifier` — fsubs.asm:157

### 3.4 DISKIN Events (type $10) — fsubs.asm:160-161

Sets `handler_data.newdisk = 1` — a disk-inserted flag read by the game loop.

### 3.5 Mouse Position Update (all events) — fsubs.asm:163-200

For every event (regardless of type), the handler updates the mouse pointer position:
1. Reads current `xsprite`/`ysprite` — fsubs.asm:163-164
2. Adds delta from event (`ie_X` at offset 10, `ie_Y` at offset 12) — fsubs.asm:166-167
3. Clamps X to 5–315, Y to 147–195 — fsubs.asm:169-180
4. Stores back to `xsprite`/`ysprite` — fsubs.asm:182-183
5. If `pbase` (SimpleSprite) is non-NULL, calls `MoveSprite(gbase, pbase, x*2, y-143)` — fsubs.asm:184-199
   - Note: X is doubled (`add.w d0,d0`) for hi-res sprite positioning
   - Y offset -143 maps from screen coordinates to ViewPort-relative

### 3.6 Event Chain Traversal — fsubs.asm:202-210

After processing, follows `ie_NextEvent` (offset 0) to next event. Returns 0 in `d0` (consumes all events from the chain since it nullifies RAWKEY events and processes the rest).

## 4. `keytrans` Table — fsubs.asm:221-226

Translates Amiga raw scancodes (0–$5A) to the game's internal key codes. The table is 91 bytes:

### Row 0: Scancodes $00–$0F
```
`  1  2  3  4  5  6  7  8  9  0  -  =  \  ?  0
```
(Last byte '0' = keypad 0)

### Row 1: Scancodes $10–$1F
```
Q  W  E  R  T  Y  U  I  O  P  {  }  ?  26 25 24
```
Scancodes $1D/$1E/$1F map to control codes 26/25/24 = cursor right/cursor up/cursor left (numpad 6/8/4 equivalents).

### Row 2: Scancodes $20–$2F
```
A  S  D  F  G  H  J  K  L  :  ?  ?  ?  27 29 23
```
$2D/$2E/$2F → 27/29/23 = direction codes mapped to numpad keys.

### Row 3: Scancodes $30–$3F
```
?  Z  X  C  V  B  N  M  ,  .  ?  ?  .  20 21 22
```
$3D/$3E/$3F → 20/21/22 = direction codes (numpad 1/2/3 equivalents).

### Row 4: Scancodes $40–$4F
```
$20(space) $08(backspace) $09(tab) $0D(enter) $0D(enter2) $1B(esc) $7F(del)
0  0  0  $2D(-)  0  1  2  3  4
```
$4C/$4D/$4E/$4F → 1/2/3/4 = cursor keys mapped to direction codes (up/down/right/left).

### Row 5: Scancodes $50–$5A
```
10 11 12 13 14 15 16 17 18 19 0
```
$50–$59 → 10–19 = function keys F1–F10, mapped to codes 10–19.
$5A → 0 (unused).

### Direction Code Summary

The numpad and cursor keys translate to direction values 20–29 (stored in key buffer). These are consumed in the game loop at fmain.c:1288:
```c
if (key >= 20 && key <= 29) { keydir = key; }
```
And in `_decode_mouse` (fsubs.asm:1568-1574):
```asm
move.w  _keydir,d2    ; if key >= 20 && key < 30 then dir = key-20
sub.w   #20,d2
```

**Direction key mappings (keytrans → direction code):**

| Numpad Key | Scancode | keytrans value | Direction (value-20) | Compass |
|------------|----------|---------------|---------------------|---------|
| Numpad 7 | $3D | 20 | 0 | NW |
| Numpad 8 | $3E | 21 | 1 | N |
| Numpad 9 | $3F | 22 | 2 | NE |
| Numpad 4 | $1F | 24 (note: stored as keytrans value) | — | — |
| Numpad 6 | $1D | 26 | — | — |

Wait — let me re-check. The numpad keys on Amiga have these scancodes:
- Numpad 1 = $1D, 2 = $1E, 3 = $1F, 4 = $2D, 5 = $2E, 6 = $2F, 7 = $3D, 8 = $3E, 9 = $3F, 0 = $0F

Actually, standard Amiga scancodes for the numpad are:
- $0F = Numpad 0
- $1D = Numpad 1, $1E = Numpad 2, $1F = Numpad 3
- $2D = Numpad 4, $2E = Numpad 5, $2F = Numpad 6
- $3D = Numpad 7, $3E = Numpad 8, $3F = Numpad 9

Checking keytrans at those offsets:

| Scancode | Offset in table | keytrans value | Interpretation |
|----------|----------------|---------------|----------------|
| $0F | 15 | '0' (ASCII 48) | Numpad 0 → 0 key (fight key) |
| $1D | 29 | 26 | Numpad 1 → direction code 26 → dir 6 (SW) |
| $1E | 30 | 25 | Numpad 2 → direction code 25 → dir 5 (S) |
| $1F | 31 | 24 | Numpad 3 → direction code 24 → dir 4 (SE)... |

Hmm, these don't subtract to clean direction indices. Let me reconsider. The values 20–29 are consumed directly by `decode_mouse`. Let me re-examine the actual table more carefully.

**Corrected numpad mapping** — keytrans values at numpad scancode offsets:

| Scancode | Numpad Key | keytrans byte | As direction (val-20) |
|----------|-----------|--------------|----------------------|
| $0F | 0 | `'0'` (48) | Not a direction — fight toggle |
| $1D | 1 | 26 | 6 = SW |
| $1E | 2 | 25 | 5 = S |
| $1F | 3 | 24 | 4 = SE |
| $2D | 4 | 27 | 7 = W |
| $2E | 5 | 29 | 9 = center (stop) |
| $2F | 6 | 23 | 3 = E |
| $3D | 7 | 20 | 0 = NW |
| $3E | 8 | 21 | 1 = N |
| $3F | 9 | 22 | 2 = NE |

And cursor keys:

| Scancode | Key | keytrans byte | As direction (val-20) |
|----------|-----|--------------|----------------------|
| $4C | Up | 1 | Not in 20-29 range |
| $4D | Down | 2 | Not in 20-29 range |
| $4E | Right | 3 | Not in 20-29 range |
| $4F | Left | 4 | Not in 20-29 range |

Cursor keys translate to 1–4, which are NOT direction codes (those are 20–29). Values 1–4 are used as cheat movement keys (fmain.c:1339-1342).

## 5. `_getkey` — Keyboard Buffer Read (fsubs.asm:281-295)

```asm
_getkey
    lea     _handler_data,a1
    clr.l   d0
    clr.w   d1
    move.b  7(a1),d1        ; get pickup pointer
    cmp.b   6(a1),d1        ; if same as laydown → buffer empty
    beq.s   getkeyx         ; return 0
    move.b  22(a1,d1.w),d0  ; get key from buffer
    addq    #1,d1
    and.b   #$7f,d1         ; wrap to 0-127
    move.b  d1,7(a1)        ; update pickup pointer
getkeyx
    rts
```

Returns 0 if buffer empty, or the next translated keycode (with bit 7 = up/down flag). Called from the game loop at fmain.c:1278: `key = getkey();`

The buffer is a 128-byte circular FIFO with `laydown` (write) and `pickup` (read) pointers, both masked to 7 bits.

## 6. `_decode_mouse` — Direction Decoder (fsubs.asm:1488-1590)

Called every frame from the game loop (fmain.c:1376: `decode_mouse()`). Determines the current movement direction from three possible sources, in priority order:

### 6.1 Mouse Direction (highest priority) — fsubs.asm:1492-1529

If either mouse button is held (`qualifier & $6000 != 0`):
- Reads `xsprite`/`ysprite` from `handler_data`
- If X ≤ 265: direction = 9 (no direction / center) — the cursor is over the text/menu area
- If X > 265: maps the cursor position to one of 9 compass zones using a 3×3 grid:

| X Range | Column |
|---------|--------|
| 266–291 | Left (NW/W/SW) |
| 292–299 | Middle (N/center/S) |
| 300+ | Right (NE/E/SE) |

| Y Range | Row |
|---------|-----|
| < 166 | Top (NW/N/NE) |
| 166–174 | Middle (W/center/E) |
| > 174 | Bottom (SW/S/SE) |

Resulting direction grid:
```
 0(NW)  1(N)  2(NE)
 7(W)   9(c)  3(E)
 6(SW)  5(S)  4(SE)
```
This matches the `com2` direction encoding.

### 6.2 Joystick Direction — fsubs.asm:1531-1560

If no mouse buttons are held, reads hardware joystick registers:
- `$dff00c` (JOY1DAT, offset 12 from $dff000) — fsubs.asm:1533
- `$dff00d` does NOT exist; note the actual code reads offset 13, which is the high byte of `$dff00c`. But wait — looking more carefully:

Actually, the code reads:
```asm
lea     $dff000,a0
move.b  12(a0),d0       ; JOY1DAT high byte (Y axis data)
...
move.b  13(a0),d2       ; JOY1DAT low byte (X axis data)
```

Register `$dff00c` = JOY1DAT (16-bit). Reading `12(a0)` = high byte (Y movement), `13(a0)` = low byte (X movement).

Decoding logic for each axis (Y-axis shown, X identical):
```asm
move.b  d0,d1       ; copy
lsr.b   #1,d0       ; shift right to get bit 1
and.b   #1,d0       ; isolate (left/up component)
and.b   #1,d1       ; isolate bit 0 (forward component)
eor.b   d0,d1       ; XOR gives direction: 0=neutral, 1=one way, depends on bits
```

Y-axis from high byte: d0 = bit 9 (left), d1 = bit 8 XOR bit 9 (forward)
X-axis from low byte: d2 = bit 1 (right), d3 = bit 0 XOR bit 1 (back)

Then:
```asm
sub.b   d0,d2       ; xjoy = right - left: -1, 0, or +1
sub.b   d1,d3       ; yjoy = back - forward: -1, 0, or +1
```

If both zero (joystick centered), falls through to keyboard decoding.

Otherwise computes index: `4 + xjoy*1 + yjoy*3` giving values 0–8, used to index `com2`:
```asm
moveq   #4,d0
add.b   d3,d0       ; + yjoy
add.b   d3,d0       ; + yjoy (x2)
add.b   d3,d0       ; + yjoy (x3) → d0 = 4 + yjoy*3
add.b   d2,d0       ; + xjoy     → d0 = 4 + yjoy*3 + xjoy
```

Also sets `_keydir = 1` when joystick input detected — fsubs.asm:1561.

### 6.3 `com2` Table — fsubs.asm:1486

```
com2: dc.b 0,1,2,7,9,3,6,5,4
```

Maps the formula `4 + yjoy*3 + xjoy` to compass direction:

| yjoy\xjoy | -1 | 0 | +1 |
|---|---|---|---|
| -1 | com2[0]=0(NW) | com2[1]=1(N) | com2[2]=2(NE) |
| 0 | com2[3]=7(W) | com2[4]=9(center) | com2[5]=3(E) |
| +1 | com2[6]=6(SW) | com2[7]=5(S) | com2[8]=4(SE) |

### 6.4 Keyboard Direction (lowest priority) — fsubs.asm:1565-1576

If joystick is centered:
```asm
move.w  _keydir,d2       ; check if keydir has a direction stored
sub.w   #20,d2           ; subtract 20 to get 0-9 direction
bmi.s   decodenull       ; if negative, no direction
cmp.w   #10,d2           ; if >= 10, no direction
bge.s   decodenull
bra.s   setcomp          ; use this direction
```

If no valid keyboard direction:
```asm
decodenull:
    moveq   #9,d2            ; direction = 9 (center/none)
    clr.w   _keydir          ; clear keydir
```

### 6.5 Compass Update — fsubs.asm:1578-1585

If direction changed from `_oldir`:
```asm
cmp.w   _oldir,d2        ; if dir != oldir
beq.s   setcompx
move.w  d2,_oldir        ; update oldir
jsr     _drawcompass      ; redraw compass UI element
```

Calls `drawcompass(dir)` (fmain2.c:351-365) which blits the appropriate compass highlight from the `nhivar` bitmap onto the status bar.

### 6.6 State Variables

- `_keydir` (fmain.c:1009): `short keydir = 0` — current keyboard-initiated direction (20-29 range, or 0)
- `_oldir` (fmain.c:1008): `short oldir = 9` — last decoded direction, used to avoid redundant compass redraws
- `_keyfight` (fmain.c:1009): `short keyfight` — keyboard fight mode toggle (set by '0' key)

## 7. Mouse Qualifier Bits Usage in Game Loop

The `handler_data.qualifier` field stores the Amiga input qualifier word. Key bits used:

| Bit | Hex Mask | Meaning | Where Used |
|-----|----------|---------|------------|
| 13 | $2000 | Right mouse button | fmain.c:1409 — triggers combat/action: `handler_data.qualifier & 0x2000` |
| 14 | $4000 | Left mouse button | fmain.c:1447 — triggers walking: `handler_data.qualifier & 0x4000` |
| Both | $6000 | Any button down | fsubs.asm:1494 — `_decode_mouse` uses mouse-based direction if either button held |

Also at fmain.c:1409, the fire button from joystick port 2 is read directly from CIA-A:
```c
BYTE *pia = (BYTE *)0xbfe001;
// ...
(*pia & 128) == 0  // bit 7 of $bfe001 = joystick port 2 fire button (active low)
```

The combat trigger condition (fmain.c:1409) is:
```c
handler_data.qualifier & 0x2000  ||  keyfight  ||  (*pia & 128) == 0
```
(Right mouse button OR keyboard fight mode OR joystick fire button)

The walk trigger condition (fmain.c:1447) is:
```c
handler_data.qualifier & 0x4000  ||  keydir
```
(Left mouse button OR keyboard direction active)

## 8. `letter_list[38]` — Keyboard Shortcuts (fmain.c:537-556)

A table of 38 entries mapping translated key characters to menu actions. Struct definition at fmain.c:535-536:
```c
struct letters { char letter, menu, choice; }
```

Full table:

| # | Key | Menu | Choice | Action |
|---|-----|------|--------|--------|
| 0 | 'I' | ITEMS(0) | 5 | Items menu |
| 1 | 'T' | ITEMS(0) | 6 | Take |
| 2 | '?' | ITEMS(0) | 7 | Look |
| 3 | 'U' | ITEMS(0) | 8 | Use |
| 4 | 'G' | ITEMS(0) | 9 | Give |
| 5 | 'Y' | TALK(2) | 5 | Yell |
| 6 | 'S' | TALK(2) | 6 | Say |
| 7 | 'A' | TALK(2) | 7 | Ask |
| 8 | ' ' | GAME(4) | 5 | Pause |
| 9 | 'M' | GAME(4) | 6 | Music toggle |
| 10 | 'F' | GAME(4) | 7 | Sound toggle |
| 11 | 'Q' | GAME(4) | 8 | Quit |
| 12 | 'L' | GAME(4) | 9 | Load |
| 13 | 'O' | BUY(3) | 5 | Buy item (Food/Arrows) |
| 14 | 'R' | BUY(3) | 6 | Buy item (Arrow/Vial) |
| 15 | '8' | BUY(3) | 7 | Buy item |
| 16 | 'C' | BUY(3) | 8 | Buy item (Mace) |
| 17 | 'W' | BUY(3) | 9 | Buy item (Sword) |
| 18 | 'B' | BUY(3) | 10 | Buy item (Bow) |
| 19 | 'E' | BUY(3) | 11 | Buy item (Totem) |
| 20 | 'V' | SAVEX(5) | 5 | Save |
| 21 | 'X' | SAVEX(5) | 6 | Exit |
| 22 | 10 (F1) | MAGIC(1) | 5 | Magic spell 1 |
| 23 | 11 (F2) | MAGIC(1) | 6 | Magic spell 2 |
| 24 | 12 (F3) | MAGIC(1) | 7 | Magic spell 3 |
| 25 | 13 (F4) | MAGIC(1) | 8 | Magic spell 4 |
| 26 | 14 (F5) | MAGIC(1) | 9 | Magic spell 5 |
| 27 | 15 (F6) | MAGIC(1) | 10 | Magic spell 6 |
| 28 | 16 (F7) | MAGIC(1) | 11 | Magic spell 7 |
| 29 | '1' | USE(8) | 0 | Use item slot 1 |
| 30 | '2' | USE(8) | 1 | Use item slot 2 |
| 31 | '3' | USE(8) | 2 | Use item slot 3 |
| 32 | '4' | USE(8) | 3 | Use item slot 4 |
| 33 | '5' | USE(8) | 4 | Use item slot 5 |
| 34 | '6' | USE(8) | 5 | Use item slot 6 |
| 35 | '7' | USE(8) | 6 | Use item slot 7 |
| 36 | 'K' | USE(8) | 7 | Use item slot 8 (Key) |
| 37 | (end) | — | — | — |

Note: The `#define LMENUS 38` at fmain.c:533 sets the table size. Function keys F1-F7 (keytrans values 10–16) map to magic spells.

### Key Processing Flow — fmain.c:1278-1355

The game loop processes keys in this priority:
1. `getkey()` → returns 0 or translated key with up/down bit — fmain.c:1278
2. If in view mode and not paused: any key-down dismisses the view — fmain.c:1283-1284
3. Direction keys (20–29): sets `keydir` on down, clears on up — fmain.c:1288-1289
4. '0' key: toggles `keyfight` — fmain.c:1290-1291
5. Mouse-click menu characters (>= 0x61): processes menu selections — fmain.c:1301-1333
6. Cheat keys (if `cheat1` flag set): 'R', '=', various controls — fmain.c:1335-1342
7. KEYS mode: digits 1-6 use specific key items — fmain.c:1343-1346
8. If space or not paused: scans `letter_list` for matching shortcut — fmain.c:1347-1355

## 9. Mouse Menu Click System

The interrupt handler (fsubs.asm:129-157) converts left mouse button clicks in the menu region into synthetic keypress characters:

**Menu region**: X: 215–265, Y: 144+ (status bar area)

**Conversion formula** (fsubs.asm:139-148):
```
row = (ysprite - 144) / 9
column = (xsprite >= 240) ? 1 : 0
character = row * 2 + 'a' + column
```

This generates characters starting at 'a' (0x61), incrementing by 2 per row (left column = even, right column = odd). These are queued in keybuf and consumed in the game loop at fmain.c:1301: `else if ((key & 0x7f) >= 0x61)`.

**On button release** (fsubs.asm:123-127): the `lastmenu` character is replayed with bit 7 set (key-up), allowing the game loop to detect the release and un-highlight the option.

## 10. `drawcompass()` — fmain2.c:351-365

Uses a `comptable[10]` lookup (fmain2.c:336-347) defining blit rectangles for each of the 10 direction states (0–8 = compass directions, 9 = center/none):

| Dir | xrect | yrect | xsize | ysize | Compass |
|-----|-------|-------|-------|-------|---------|
| 0 | 0 | 0 | 16 | 8 | NW |
| 1 | 16 | 0 | 16 | 9 | N |
| 2 | 32 | 0 | 16 | 8 | NE |
| 3 | 30 | 8 | 18 | 8 | E |
| 4 | 32 | 16 | 16 | 8 | SE |
| 5 | 16 | 13 | 16 | 11 | S |
| 6 | 0 | 16 | 16 | 8 | SW |
| 7 | 0 | 8 | 18 | 8 | W |
| 8 | 0 | 0 | 1 | 1 | (unused) |
| 9 | 0 | 0 | 1 | 1 | center (1×1 = no highlight) |

The compass is rendered by:
1. Blitting `nhinor` (normal compass, no highlight) to screen position (567, 15) — fmain2.c:359
2. If dir < 9, blitting `nhivar` (variant/highlight) at the appropriate sub-rectangle — fmain2.c:361-362

Both `nhinor` and `nhivar` are chip-memory copies of `_hinor` and `_hivar` data from fsubs.asm:259-306. The compass is a 48×24 pixel image in bitplane 2.

## Cross-Cutting Findings

- **Timer heartbeat in input handler** (fsubs.asm:72-80): The TIMER event processing generates synthetic key events to keep the game loop responsive. The `ticker` field at offset 150 counts to 16, then generates a fake $E0 key-up (scancode $60 with up-bit). This is not documented elsewhere and ensures the input chain doesn't starve.

- **CIA-A direct read for fire button** (fmain.c:1272, 1409): The joystick fire button is NOT read through the input handler — it's read directly from the CIA-A PRA register at `$bfe001`, bit 7 (active low). This bypasses the input.device entirely.

- **Fight mode persistence** (fmain.c:1290-1291): The '0' key (numpad 0, scancode $0F → keytrans '0') sets `keyfight = TRUE` on key-down and clears on key-up, providing a keyboard alternative to holding the fire button.

- **Hunger affects direction** (fmain.c:1442-1445): When `hunger > 120`, the walking direction is randomly perturbed by ±1 with 25% probability (`!rand4()`). This cross-cuts the input system with the survival mechanic.

- **Mouse sprite clamping** (fsubs.asm:169-180): The Y-axis clamp of 147–195 confines the pointer to the status bar area (48 pixel range). The X-axis clamp of 5–315 covers nearly the full 320-pixel width. This means the mouse pointer never enters the playfield area — it's status-bar only.

- **Key repeat suppression** (fsubs.asm:91-92): Bit 9 of the qualifier is the repeat flag. All repeated keys are silently dropped, meaning the game receives only key-down and key-up transitions.

## Unresolved

- **`_ion` symbol purpose**: Declared `public` at fsubs.asm:50 alongside `_HandlerInterface`, but its definition at fsubs.asm:357 was not fully traced. It may be related to interrupt enable/disable.

## Verified

- **Numpad keytrans mapping**: Confirmed against standard Amiga keyboard scancodes ($1D-$1F = Numpad 1-3, $2D-$2F = Numpad 4-6, $3D-$3F = Numpad 7-9). The direction encoding produces a correct numpad-to-compass layout:
  ```
  7=NW(20)  8=N(21)   9=NE(22)
  4=W(27)   5=stop(29) 6=E(23)
  1=SW(26)  2=S(25)   3=SE(24)
  ```

## Refinement Log

- 2026-04-05: Initial comprehensive discovery pass covering all 9 requested topics.
