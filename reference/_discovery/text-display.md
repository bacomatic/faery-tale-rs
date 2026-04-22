# Discovery: Text & Message Display System

**Status**: complete
**Investigated**: 2026-04-06
**Requested by**: orchestrator
**Prompt summary**: Trace the full text rendering and message display pipeline — from low-level assembly routines to C wrappers, template substitution, location messaging, status bar/HUD, compass, and font loading.

## Display Architecture Overview

The game uses a split-screen Amiga display with two ViewPorts:

- **`vp_page`** — lo-res (288×140) playfield for the game world, positioned at top — `fmain.c:14` (`PAGE_HEIGHT 143`), `fmain.c:811-813`
- **`vp_text`** — hi-res (640×57) status bar at bottom — `fmain.c:16` (`TEXT_HEIGHT 57`), `fmain.c:815-818`

Key bitmap structures — `fmain.c:446-448`:
```c
struct BitMap *bm_page1, *bm_page2, *bm_text, *bm_lim, *bm_draw, *bm_source;
struct BitMap bm_scroll, pagea, pageb;
struct RastPort rp_map, rp_text, rp_text2, *rp;
```

- `bm_text` — 4-bitplane (16-color) 640×57 bitmap for the status bar — `fmain.c:828`
- `bm_scroll` — 1-bitplane 640×57 bitmap used as the text scrolling area — `fmain.c:832`, plane shared with `bm_text->Planes[0]` at `fmain.c:938`
- `rp_map` — RastPort for drawing on the playfield pages
- `rp_text` — RastPort for the scrolling text area (initially `wb_bmap`, then switched to `&bm_scroll` at `fmain.c:1167`)
- `rp_text2` — RastPort for the hi-res status bar labels/menus (BitMap = `bm_text` at `fmain.c:835`)
- `rp` — global pointer swapped between `rp_map` and `rp_text` as needed

Bitmap plane layout for `bm_text` — `fmain.c:871-874`:
```c
bm_text->Planes[0] = wb_bmap->Planes[0];
bm_text->Planes[1] = wb_bmap->Planes[1];
bm_text->Planes[2] = bm_text->Planes[0] + (TEXT_HEIGHT*80);
bm_text->Planes[3] = bm_text->Planes[1] + (TEXT_HEIGHT*80);
```

The status bar text area (`vp_text`) starts hidden (`VP_HIDE`) and is revealed after game initialization — `fmain.c:818`, `fmain.c:1257`.

## Font System

Two fonts are used:

1. **Topaz 8** (`tfont`) — ROM font, loaded via `OpenFont(&topaz_ta)` — `fmain.c:650`, `fmain.c:779`
   - Used for: status bar labels, menu text (`rp_text2`), map-mode text (`rp_map`)
   - Applied at init: `SetFont(&rp_text,tfont); SetFont(&rp_text2,tfont); SetFont(&rp_map,tfont);` — `fmain.c:779-781`

2. **Amber 9** (`afont`) — custom disk font loaded from `fonts/Amber/9` — `fmain.c:774-782`
   ```c
   if ((seg = LoadSeg("fonts/Amber/9")) == NULL) return 15;
   font = (struct DiskFontHeader *) ((seg<<2)+8);
   afont = &(font->dfh_TF);
   ```
   - Loaded via `LoadSeg` (not `OpenDiskFont`), cast through `DiskFontHeader` struct
   - Used for: in-game scrolling messages and map-message/placard text
   - Applied when scrolling text is established: `SetFont(rp,afont); SetAPen(rp,10); SetBPen(rp,11);` — `fmain.c:1168`
   - The font file at [game/fonts/Amber/9](game/fonts/Amber/9) is the Amiga font data

The standalone [text.c](text.c) is a separate test program (not part of the game build) that tests disk font rendering via the Intuition/GfxBase API. Its `infont` references `"sapphire.font"`, size 19 — `text.c:154`. This is unrelated to the game's font system.

## _prdec (Decimal Number Printing) — fsubs.asm:342-378

Prints a decimal number to the current RastPort.

**Signature**: `prdec(value, length)` where value is the number and length is the number of digits to display.

**Code path** — `fsubs.asm:342-377`:
1. Saves all registers — `fsubs.asm:343`
2. Loads value from stack into `d0` — `fsubs.asm:344`
3. Calls `ion6` to convert number to ASCII in `_numbuf` — `fsubs.asm:345`
4. `ion6` (`fsubs.asm:367-377`): iterates 10 digits, divides by 10 repeatedly, stores ASCII digits (`$30` + remainder) right-to-left in `numbuf[0..9]`, space-fills ($20) leading positions
5. After conversion, adjusts pointer: `add #10,a0` to point past the buffer, then subtracts the requested display length — `fsubs.asm:347-348`
6. Calls `GfxBase->Text(rp, buffer, length+1)` — `fsubs.asm:350-353`

The `numbuf` is an 11-byte buffer declared at `fmain.c:492`: `char numbuf[11] = { 0,0,0,0,0,0,0,0,0,0,' '};`

**Usage examples**:
- `prdec(anim_list[0].vitality,3)` — display vitality with 3 digits — `fmain2.c:461`
- `prdec(brave,3)` — display bravery stat — `fmain2.c:464`

## _placard (Fractal Line Pattern) — fsubs.asm:382-475

**NOT a text display routine.** Despite appearing near text routines, `_placard` draws a decorative fractal/recursive line pattern on the `rp_map` RastPort. It is used as a visual effect during story screens.

**Code path** — `fsubs.asm:382-475`:
1. Uses `rp_map` (not the global `rp`) — `fsubs.asm:389`
2. Starts at origin `(12, 0)` — `fsubs.asm:392-393`
3. Outer loop `i` = 16 iterations — `fsubs.asm:395`
4. Inner loop `j` = 15 iterations — `fsubs.asm:397`
5. At each step, applies offsets from `xmod`/`ymod` tables (±4 pixel deltas) — `fsubs.asm:381`
   - `xmod` = `{-4,-4,-4,0,0,0,4,4,0,-4,0,4,4,0,0,0}` — `fsubs.asm:381`
   - `ymod` = `{0,0,0,4,4,4,0,0,-4,0,-4,0,0,4,4,4}` — `fsubs.asm:382`
6. `k` loop (5 iterations) draws Move/Draw pairs for mirrored line segments — `fsubs.asm:417-454`
7. Uses color 1 (pen) for most iterations, color 24 when `k==0` — `fsubs.asm:411-414`
8. Draws lines reflected across multiple axes (creating a symmetric fractal pattern):
   - Original at `(xorg, yorg)` to `(dx, dy)`
   - Mirrored at `(284-xorg, 124-yorg)` to `(284-dx, 124-dy)` — center mirror
   - Rotated at `(16+yorg, 12-xorg)` to `(16+dy, 12-dx)` — 90° rotation
   - Rotated at `(268-yorg, 112+xorg)` to `(268-dy, 112+dx)` — 270° rotation

Called during story sequences and the intro: `placard()` after `placard_text()` — e.g., `fmain.c:2869-2870`

## _placard_text (Story Text via SSP) — narr.asm:235-248

Indexes into a table of 20 story message pointers and calls `_ssp` to display them.

**Code path** — `narr.asm:235-248`:
```asm
_placard_text
    move.l  4(sp),d0          ; get message index
    add.w   d0,d0             ; index * 4 (pointer table)
    add.w   d0,d0
    lea     mst,a0            ; message table base
    add.l   (a0,d0),a0        ; add offset to get message address
    move.l  a0,4(sp)          ; replace argument on stack
    jmp     _ssp              ; tail-call to scrolling string print
```

The message table `mst` at `narr.asm:248-250` contains offsets to 20 messages (msg1–msg12):
- msg1 (index 0): Julian's quest intro — `narr.asm:252-259`
- msg2 (index 1): Julian's failure — `narr.asm:261-264`
- msg3 (index 2): Phillip sets out — `narr.asm:266-269`
- msg4 (index 3): Phillip's failure — `narr.asm:271-274`
- msg5 (index 4): Kevin sets out — `narr.asm:276-283`
- msg6 (index 5): Game over — `narr.asm:284-287`
- msg7/msg7a (index 6-7): Victory/Talisman recovered — `narr.asm:288-296`
- msg8/msg8a/msg8b (index 8-10): Princess Katra rescue text — `narr.asm:298-305`
- msg9/msg9a/msg9b (index 11-13): Princess Karla rescue text — `narr.asm:307-314`
- msg10/msg10a/msg10b (index 14-16): Princess rescue third variant — `narr.asm:316-322`
- msg11/msg11a (index 17-18): After seeing princess safely home — `narr.asm:330-335`
- msg12 (index 19): Copy protection intro — `narr.asm:337-347`

Comments at `narr.asm:1-2` specify line width constraints:
- Max 36 characters for scroll text
- Max 29 characters for placard text

## _ssp (Scrolling String Print) — fsubs.asm:497-536

Prints a string with embedded XY positioning commands to the current RastPort via `rp`.

**Constant**: `XY equ 128` — `fsubs.asm:228`. Byte value 128 ($80) is the positioning escape code.

**Signature**: `ssp(string)` — takes a pointer to a formatted string.

**Code path** — `fsubs.asm:497-536`:
1. Loads string pointer from stack, saves registers — `fsubs.asm:497-499`
2. Uses global `_rp` as the RastPort — `fsubs.asm:500`
3. **Main loop** (`ssp10`, `fsubs.asm:501`):
   - Read first byte into `d0` — `fsubs.asm:503`
   - If byte == 0 (`ETX`): exit — `fsubs.asm:504`
   - If byte == 128 (`XY`): goto `setxy` — `fsubs.asm:505-506`
   - Otherwise: **print text segment** — `fsubs.asm:508-522`
     - Scan forward from current position counting printable bytes (not 0, not high-bit set) — `fsubs.asm:512-515`
     - Call `GfxBase->Text(rp, buffer, count)` to render the segment — `fsubs.asm:516-518`
     - Advance pointer past the printed segment — `fsubs.asm:519`
     - Loop back to `ssp10` — `fsubs.asm:520`
4. **`setxy`** handler (`fsubs.asm:526-531`):
   - Read next byte as X position — `fsubs.asm:527`
   - Read next byte as Y position — `fsubs.asm:528`
   - X is **doubled** (`add.w d0,d0`) — `fsubs.asm:529` — because ssp is used with lo-res coordinates but hi-res offsets can be encoded at half
   - Call `GfxBase->Move(rp, x*2, y)` — `fsubs.asm:530-531`
   - Loop back to `ssp10` — `fsubs.asm:531`

**String format**: A sequence of segments where each segment is either:
- A printable ASCII string (bytes 1-127 excluding 128), terminated by 0 or a byte with high bit set
- Or `XY, x_half, y` — a positioning command where x is stored as half the actual pixel coordinate

**Example** — `fsubs.asm:236-241`:
```asm
_titletext  dc.b  XY,(160-26*4)/2,33,$22,"The Faery Tale Adventure",$22
            dc.b  XY,(160-30*4)/2,79,"Animation, Programming & Music"
            dc.b  XY,(160-2*4)/2,90,"by"
            dc.b  XY,(160-12*4)/2,101,"David Joiner"
            dc.b  XY,(160-30*4)/2,160,"Copyright 1986 MicroIllusions "
```
Usage: `ssp(titletext)` — `fmain.c:1163`

## Text Positioning (_move, _text) — fsubs.asm:477-495

### _move — fsubs.asm:477-485
Thin wrapper around Amiga GfxBase `Move()`.

```asm
_move
    movem.l a0-a2/d0-d1,-(sp)
    move.l  4+20(sp),d0       ; x
    move.l  8+20(sp),d1       ; y
    move.l  _rp,a1             ; global RastPort
    move.l  _GfxBase,a6
    jsr     Move(a6)
    movem.l (sp)+,a0-a2/d0-d1
    rts
```
**Signature**: `move(x, y)` — positions the drawing cursor in the current global `rp`.

### _text — fsubs.asm:487-495
Thin wrapper around Amiga GfxBase `Text()`.

```asm
_text
    movem.l a0-a6/d0-d7,-(sp)
    move.l  60+4(sp),a0        ; string pointer
    move.l  60+8(sp),d0        ; length
    move.l  _rp,a1             ; global RastPort
    move.l  _GfxBase,a6
    jsr     Text(a6)
    movem.l (sp)+,a0-a6/d0-d7
    rts
```
**Signature**: `text(string, length)` — renders `length` characters at current cursor position.

Both use the global `_rp` pointer, which is the C variable `rp` declared at `fmain.c:448`.

## Print Queue (prq / ppick) — fmain2.c:434-470

A deferred print system that queues small integer commands for processing during idle time.

### Data Structures — fmain2.c:434-435:
```c
char print_que[32];     // circular buffer of command bytes
short prec=0, pplay=0;  // record (write) and play (read) indices
```

### prq (Enqueue) — fmain2.c:473-488
Inline assembly function. Enqueues a single byte command:
```asm
_prq
    movem.l d0/d1/a1,-(sp)
    move.w  _prec,d0
    move.w  _pplay,d1
    addq    #1,d0
    and.w   #31,d0           ; wrap at 32
    cmp.w   d0,d1            ; full check
    beq.s   prqx             ; if full, drop command
    move.w  _prec,d1
    move.w  d0,_prec         ; advance write pointer
    lea     _print_que,a1
    move.b  3+4+12(sp),(a1,d1)  ; store command byte
prqx
    movem.l (sp)+,d0/d1/a1
    rts
```

### ppick (Dequeue & Execute) — fmain2.c:443-470
Called from the main game loop during idle frames (`fmain.c:2013`). Processes one command per call:

```c
ppick()
{   register long p;
    if (prec == pplay) Delay(1);   // nothing queued → yield
    else
    {   p = print_que[pplay];
        pplay = (pplay + 1) & 31;  // advance read pointer
        switch (p) {
        case 2: // debug: coords + memory
            print("Coords = "); prdec(hero_x,6); prdec(hero_y,6);
            print("Memory Available: "); prdec(AvailMem(0),6);
            break;
        case 3: // debug: position info
            print("You are at: ");
            prdec(hero_x/256,3); prdec(hero_y/256,3);
            print("H/Sector = "); prdec(hero_sector,3);
            text(" Extent = ",10); prdec(xtype,2);
            break;
        case 4: // vitality display
            move(245,52);
            text("Vit:",4); prdec(anim_list[0].vitality,3);
            break;
        case 5: // refresh menu options
            print_options(); break;
        case 7: // full stat display
            if (luck < 0) luck = 0;
            move(14,52); text("Brv:",4); prdec(brave,3);
            move(90,52); text("Lck:",4); prdec(luck,3);
            move(168,52); text("Knd:",4); prdec(kind,3);
            move(321,52); text("Wlth:",5); prdec(wealth,3);
            break;
        case 10: // "Take What?" prompt
            print("Take What?"); break;
        }
    }
}
```

**Command codes**:
| Code | Action | Citation |
|------|--------|----------|
| 2 | Debug: coords + available memory | fmain2.c:449-451 |
| 3 | Debug: position, sector, extent | fmain2.c:452-456 |
| 4 | Display vitality at (245,52) | fmain2.c:457-459 |
| 5 | Refresh `print_options()` | fmain2.c:460 |
| 7 | Display all stats (Brv/Lck/Knd/Wlth) | fmain2.c:461-466 |
| 10 | Print "Take What?" | fmain2.c:467 |

## C Wrappers (print, print_cont) — fmain2.c:495-505

### print() — fmain2.c:495-500
Scrolls the text area up by 10 pixels, then prints a string on the bottom line:
```c
print(str) register char *str;
{   register long l;
    l = 0; while (str[l]) l++;
    ScrollRaster(rp,0,10,TXMIN,TYMIN,TXMAX,TYMAX);
    move(TXMIN,42);
    text(str,l);
}
```
- Manually computes string length (no `strlen`)
- `ScrollRaster(rp,0,10,...)` scrolls the text region up by 10 pixels
- Region bounds: `TXMIN=16`, `TYMIN=5`, `TXMAX=400`, `TYMAX=44` — `fmain2.c:490-493`
- New text rendered at position `(16, 42)` — the bottom of the scroll area

### print_cont() — fmain2.c:502-505
Continues printing on the same line (no scroll):
```c
print_cont(str) register char *str;
{   register long l = 0;
    while (str[l]) l++;
    text(str,l);
}
```

Both use the global `rp` pointer (which should be `rp_text` / `&bm_scroll` during gameplay — set at `fmain.c:1167`). Text colors: pen 10 (foreground), pen 11 (background), JAM2 mode — `fmain.c:1168`.

## Template Substitution (extract) — fmain2.c:515-548

Performs word-wrapping and `%` → hero name substitution.

**Signature**: `extract(start)` — takes a null-terminated message string.

**Code path** — `fmain2.c:515-548`:
1. Uses local buffer `mesbuf[200]` — `fmain2.c:509`
2. Scans input character by character, max 37 chars per line — `fmain2.c:523`
3. On space: records wrap point — `fmain2.c:525`
4. On null (0): ends processing — `fmain2.c:526`
5. On carriage return (13): forces line break — `fmain2.c:527`
6. On `%`: substitutes `datanames[brother-1]` (hero name) — `fmain2.c:528-530`
   - `datanames[] = { "Julian","Phillip","Kevin" }` — `fmain.c:604`
7. At wrap boundary (37 chars or wrap point found): calls `print(lstart)` to output the line — `fmain2.c:535`
8. Continues until entire string is processed or line count exceeds 38 — `fmain2.c:536`

## Message Dispatch (msg, speak, event) — fmain2.c:550-577

Three inline-assembly message dispatch functions that index into null-terminated string tables and call `extract()`:

### _msg — fmain2.c:567-571
```asm
_msg
    move.l  4(sp),a0           ; string table base
    move.l  8(sp),d0           ; message index
```
Generic: takes an explicit string table and index.

### _speak — fmain2.c:562-564
```asm
_speak
    lea     _speeches,a0       ; NPC speech table (narr.asm:351)
    move.l  4(sp),d0           ; speech index
    bra     msg1
```
Uses the `_speeches` table for NPC dialogue.

### _event — fmain2.c:557-560
```asm
_event
    lea     _event_msg,a0      ; event message table (narr.asm:10)
    move.l  4(sp),d0           ; event index
    bra     msg1
```
Uses the `_event_msg` table for gameplay events (hunger, drowning, etc.).

### Common handler (`msg1`) — fmain2.c:572-577:
```asm
msg1    beq     msgx               ; if index == 0, skip to extract
1$      tst.b   (a0)+              ; scan for null terminator
        bne.s   1$                 ; keep scanning
        subq.w  #1,d0              ; decrement index
        bne.s   1$                 ; loop until we reach desired message
msgx    move.l  a0,4(sp)           ; replace arg with found string pointer
        bra     _extract           ; tail-call to extract()
```
Skips `d0` null-terminated strings to find the requested message, then tail-calls `extract()`.

### Message Tables:
- `_event_msg` — `narr.asm:10-30`: hunger, drowning, burning, turned to stone, journey start, sleeping, etc.
- `_speeches` — `narr.asm:351+`: NPC dialogue (ogre, goblin, wraith, skeleton, snake, bartender, guard, king, princess, etc.)
- `_place_msg` — `narr.asm:164-195`: outdoor location names ("% came to the city of Marheim.")
- `_inside_msg` — `narr.asm:199-223`: indoor location names ("% came to a small chamber.")

## Location Messages (find_place, map_message) — fmain.c:2653-2680, fmain2.c:601-619

### find_place() — fmain.c:2653
Called from the main loop (`fmain.c:2051`) to detect location changes.

**Code path** — `fmain.c:2653-2680`:
1. Gets `hero_sector` (terrain sector id, masked to 0-255) — `fmain.c:2655`
2. Selects table based on region:
   - `region_num > 7`: uses `inside_tbl` / `inside_msg` — `fmain.c:2659`
   - Otherwise: uses `place_tbl` / `place_msg` — `fmain.c:2660`
3. Searches the terrain-to-message lookup table (`_place_tbl` or `_inside_tbl` at `narr.asm:100-148`):
   - Each entry is 3 bytes: `{min_sector, max_sector, message_index}` — `narr.asm:100`
   - Scans until `hero_sector` falls within `[tbl[0], tbl[1]]` — `fmain.c:2663`
4. Special case for mountains (message 4) varies by region — `fmain.c:2668-2671`
5. If message index changed and non-zero: calls `msg(ms, i)` to display — `fmain.c:2675`

### map_message() — fmain2.c:601-613
Switches display to expanded map-message mode (fullscreen text overlay on game world):
```c
map_message()
{   fade_down();
    rp = &rp_map;
    rp_map.BitMap = fp_drawing->ri_page->BitMap;
    SetDrMd(rp,JAM1);
    SetAPen(rp,24);
    SetRast(rp,0);                    // clear screen
    vp_text.Modes = HIRES | SPRITES | VP_HIDE;  // hide status bar
    stillscreen();
    LoadRGB4(&vp_page,pagecolors,32);
    viewstatus = 2;
}
```
- Fades display down, clears the playfield, hides the status bar
- Sets `rp` to `rp_map` for drawing on playfield
- Sets `viewstatus = 2` — signals the main loop to wait 200 ticks before transitioning

### message_off() — fmain2.c:615-620
Returns to normal gameplay display:
```c
message_off()
{   fade_down();
    rp = &rp_text;
    vp_text.Modes = HIRES | SPRITES;  // show status bar
    pagechange();
    viewstatus = 3;
}
```
- Restores `rp` to `rp_text`, re-shows status bar, sets `viewstatus = 3` for fade-back

### name() — fmain2.c:593
Simply prints the current brother's name without a newline:
```c
name()
{   print_cont(datanames[brother-1]); }
```

### announce_container() — fmain2.c:579-584
Prints "`<name>` found `<container>` containing ":
```c
announce_container(s) char *s;
{   print(datanames[brother-1]);
    print_cont(" found ");
    print_cont(s);
    print_cont(" containing ");
}
```

### announce_treasure() — fmain2.c:586-590
Prints "`<name>` found `<item>`":
```c
announce_treasure(s) char *s;
{   print(datanames[brother-1]);
    print_cont(" found ");
    print_cont(s);
}
```

## Status Bar / HUD

### Layout
The status bar occupies `vp_text` — a 640×57 hi-res viewport below the playfield. Color palette loaded from `textcolors[]` (20 colors) — `fmain.c:476-479`:
```c
USHORT textcolors[] = {
    0x000, 0xFFF, 0xC00, 0xF60, 0x00f, 0xc0f, 0x090, 0xFF0,
    0xf90, 0xf0c, 0xA50, 0xFDB, 0xEB7, 0xCCC, 0x888, 0x444,
    0x000, 0xDB0, 0x740, 0xC70
};
```

### Stat Display
Stats are displayed via `prq(7)` and `prq(4)` commands processed by `ppick()`:

- **prq(7)** — Full stat line at y=52:
  - `Brv:<value>` at x=14 — bravery
  - `Lck:<value>` at x=90 — luck
  - `Knd:<value>` at x=168 — kindness
  - `Wlth:<value>` at x=321 — wealth
  - All rendered with `text()` and `prdec()` — `fmain2.c:461-466`

- **prq(4)** — Vitality display at position (245, 52):
  - `Vit:<value>` — `fmain2.c:457-459`

### Menu/Option Display — fmain.c:3053-3098

`print_options()` renders the right-side menu labels on `rp_text2` (the hi-res status bitmap):
- Iterates enabled menu options for current `cmode` — `fmain.c:3055-3068`
- Options arranged in 2 columns: x=430 (even) and x=482 (odd) — `fmain.c:3064`
- Rows spaced 9 pixels apart, starting at y=8 — `fmain.c:3065`
- Each label is 6 characters wide — `fmain.c:3066-3067`
- Colors vary by mode (USE, FILE, KEYS, SAVEX) — `fmain.c:3076-3082`

`propt(j,pena)` (`fmain.c:3073-3096`) renders individual option labels:
- Sets foreground/background pen based on mode and option type
- Uses `Text(&rp_text2, label, 5)` for the 5-character labels — `fmain.c:3091`

## drawcompass() — fmain2.c:351-365

Draws the compass direction indicator on the hi-res status bar using blitter operations.

**Data**: Two 48×24 pixel compass bitmaps stored as raw bitplane data in assembly — `fsubs.asm:249-277`:
- `_hinor` — base compass image (all directions normal) — `fsubs.asm:249-260`
- `_hivar` — compass with highlighted direction segments — `fsubs.asm:262-275`

These are copied into chip RAM at init: `nhinor = into_chip(&hinor,(16*16)); nhivar = into_chip(&hivar,(16*16));` — `fmain.c:944-945`

**Compass region table** — `fmain2.c:332-344`:
```c
struct compvars {
    UBYTE xrect, yrect;
    UBYTE xsize, ysize;
} comptable[10] = {
    {  0, 0, 16,8 },      // dir 0: NW
    { 16, 0, 16,9 },      // dir 1: N
    { 32, 0, 16,8 },      // dir 2: NE
    { 30, 8, 18,8 },      // dir 3: E
    { 32,16, 16,8 },      // dir 4: SE
    { 16,13, 16,11 },     // dir 5: S
    {  0,16, 16,8 },      // dir 6: SW
    {  0, 8, 18,8 },      // dir 7: W
    {  0, 0,  1,1 },      // dir 8: (null)
    {  0, 0,  1,1 }       // dir 9: (null)
};
```

**Code path** — `fmain2.c:351-365`:
1. Looks up rectangle for `dir` from `comptable` — `fmain2.c:353-356`
2. Sets `bm_source->Planes[2] = nhinor` (normal compass) — `fmain2.c:357`
3. Blits entire 48×24 normal compass to `bm_text` at (567, 15) — `fmain2.c:358`
4. If `dir < 9`: sets `bm_source->Planes[2] = nhivar` (highlighted) — `fmain2.c:360`
5. Blits only the highlighted region subset on top — `fmain2.c:361`

**Caller**: Called from the joystick/input handler in `fsubs.asm:1582`: `jsr _drawcompass` within `_decode_mouse`.

Compass position on status bar: pixel (567, 15) — right side of the 640-wide hi-res display.

## viewstatus State Machine

The `viewstatus` variable (`fmain.c:583`) controls display mode transitions:

| Value | Meaning | Citation |
|-------|---------|----------|
| 0 | Normal gameplay display | fmain.c:1994, fmain.c:2614 |
| 1 | Flashing/picking mode (menu selection active) | fmain.c:1367-1368 |
| 2 | Map-message mode (fullscreen text, wait 200 ticks) | fmain2.c:613, fmain.c:1365-1366 |
| 3 | Returning from message mode (fade back) | fmain2.c:620, fmain.c:2614 |
| 4 | Picking mode variant | fmain.c:1367 |
| 98 | First rebuild pass after corruption | fmain.c:1993-1994 |
| 99 | Full screen rebuild needed | fmain.c:1989-1993 |

Transition: 99 → 98 → 0 (two-frame rebuild) — `fmain.c:1993-1994`

## Cross-Cutting Findings

- `_rp` (the global RastPort pointer) is written to by many subsystems — combat, text, map display. Any code that changes `rp` affects where text goes. Key switches: `rp = &rp_map` for map drawing (fmain.c:1152), `rp = &rp_text` for scrolling text (fmain.c:1167, fmain2.c:617).
- The `prq()` queue (32-entry circular buffer) silently drops commands when full — `fmain2.c:480-481`. This means rapid stat changes during combat could lose display updates.
- `extract()` has a fixed 200-byte buffer (`mesbuf[200]`) — `fmain2.c:509`. Long messages with name substitution could theoretically overflow, though the 37-char line limit with 3 brothers makes this improbable.
- The `%` substitution in `extract()` does NOT update the per-line character counter `i` properly — it increments `i` inside the substitution loop (`fmain2.c:529`) but starts from the current position, potentially allowing lines slightly longer than 37 chars with long names.
- `map_message()` hides the status bar (`VP_HIDE`) and draws text directly on the playfield bitmap. The `afont` (Amber 9) is explicitly set for this mode — `fmain.c:2860`, `fmain2.c:1586`.
- The compass bitmaps (`hinor`/`hivar`) are stored inline in the assembly source as raw longword bitmap data and must be copied to chip RAM before use — `fmain.c:944-945`.
- `bm_source` is a 3-bitplane 64×24 bitmap (`fmain.c:833`) reused for both compass and sprite source blitting. `Planes[2]` is swapped between `nhinor` and `nhivar` during compass rendering — only plane 2 differs between normal and highlighted.
- The `_ion` function at `fsubs.asm:364` provides a standalone C-callable entry point: `ion(value)` that converts a number to ASCII in `numbuf` without printing. `_prdec` calls `ion6` directly and then prints.

## Unresolved

None — all questions from the prompt have been answered with direct source citations.

## Refinement Log
- 2026-04-06: Initial discovery pass — full trace of all 12 requested topics
