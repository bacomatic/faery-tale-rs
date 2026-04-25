# Placard — Logic Spec

> Fidelity: behavioral  |  Source files: fsubs.asm, narr.asm, fmain.c, fmain2.c
> Cross-refs: [RESEARCH §20](../RESEARCH.md#20-visual-effects), [visual-effects.md#map_message](visual-effects.md#map_message), [messages.md §6](messages.md#6-placard-screens--_placard_text-narrasm235)

## Overview

The placard system consists of two independent primitives that work together. `placard_text(N)` renders a pre-composed, multi-line narrative message onto the full-screen RastPort established by `map_message`: it looks up slot N in the `mst` dispatch table (`narr.asm:237`) and delegates to `ssp`, which walks an encoded byte string calling `Move` for XY positioning escapes (byte `0x80` followed by x/2 and y bytes) and `Text` for literal character runs. Callers interleave `placard_text()` calls with `name()` — which prints the current hero's name at the current pen position via `print_cont` — to splice the hero's name into the narrative mid-sentence. Each `placard_text` call inherits the pen position from whatever call preceded it, so multi-part messages (e.g. the princess-rescue sequence) chain seamlessly.

`placard()` is separate: it draws the animated Greek-key (meander) decorative border that frames the text. It is a pure 68000 assembly routine (`fsubs.asm:387`) that issues Amiga `SetAPen`/`Move`/`Draw` calls directly to `rp_map`, the RastPort whose BitMap pointer was redirected by `map_message` to the currently-displayed playfield page. There is no double-buffer flip between any two `Draw` calls, so every stroke is immediately visible on screen. Each line segment is drawn **five times** in a tight k-loop: four passes in color 1 (white) followed by one pass in color 24 (brown-red). This produces the observable "trace white → settle red" flash for each stroke as the pattern extends along the frame — no `Delay` is needed. The completed border is drawn entirely in brown-red. Four line pairs fill all four sides of the frame simultaneously from a single advancing polyline origin: pairs A and B are 180°-symmetric about the pixel coordinate (142, 62) and fill the left and right edges; pairs C and D are a 90° axis swap of A and B (also 180°-symmetric about (142, 62)) and fill the top and bottom edges.

Related effects: `map_message` and `message_off` (which put the screen into and out of full-screen placard mode) are documented in [visual-effects.md#map_message](visual-effects.md#map_message). The death and victory calling sequences that drive these functions are in [brother-succession.md#revive](brother-succession.md#revive) and [quests.md#end_game_sequence](quests.md#end_game_sequence). The copy-protection question caller (`fmain.c:1235`) also uses `placard_text(19)` to display the quiz lead-in.

## Symbols

Constants local to this subsystem (not in SYMBOLS.md):

- `XY: u8 = 128` — `narr.asm:230` — position-escape byte in encoded message strings.
- `ETX: u8 = 0` — `narr.asm:231` — string terminator (same value as ASCII NUL).

Tables and globals required in SYMBOLS.md (see proposed additions at end of this file):

- `mst` — `narr.asm:237` — 20-entry pc-relative offset table; each entry resolves a `placard_text(N)` index to one of the `msg*` byte strings below.
- `xmod` — `fsubs.asm:384` — 16-entry signed x-delta table used by `placard()`.
- `ymod` — `fsubs.asm:385` — 16-entry signed y-delta table used by `placard()`.
- `rp_map` — `fsubs.asm:380` — full-screen RastPort; its BitMap is redirected to the live playfield bitmap by `map_message` before any placard routine is called.

Globals already in [SYMBOLS.md](SYMBOLS.md) used here: `rp`.

## ssp

Source: `fsubs.asm:497-530`
Called by: `placard_text` (tail-call), `print_cont`
Calls: `Move`, `Text`, `rp`

The encoding format used by all `msg*` strings:
- A byte equal to `XY` (128) introduces a position command: the next byte is `x/2` (x coordinate stored halved to fit in one unsigned byte; doubled on read), the byte after is `y`. The cursor is moved to `(x, y)` without printing.
- Any run of bytes with values 1–127 is a literal text run; the run ends at the first byte that is 0 (ETX) or ≥ 128 (another XY or ETX variant).
- A byte equal to 0 (ETX) terminates the entire message.

After a text run ends, the terminating byte (XY or ETX) is re-read on the next loop iteration, so XY and ETX act as both run terminators and control codes.

```pseudo
def ssp(msg: bytes) -> None:
    """Walk an XY-positioned encoded byte string, issuing Move for XY escapes and Text for runs."""
    XY = 128                                             # narr.asm:230 — position-escape byte
    p = 0                                                # fsubs.asm:497 — byte index into msg
    while True:                                          # fsubs.asm:502 — ssp10 main loop
        ch = msg[p]                                      # fsubs.asm:503
        p = p + 1
        if ch == 0:                                      # fsubs.asm:504 — ETX = end of message
            return
        if ch == XY:                                     # fsubs.asm:505 — position escape
            x = msg[p] * 2                               # fsubs.asm:536 — stored halved; double → pixels
            y = msg[p + 1]                               # fsubs.asm:537 — y stored directly
            p = p + 2
            Move(rp, x, y)                               # fsubs.asm:539
        else:
            p = p - 1                                    # fsubs.asm:511 — back up: current byte opens a text run
            start = p
            length = 0
            while True:                                  # fsubs.asm:514 — ssp20 length scan
                b = msg[start + length]
                if b == 0:                               # zero = ETX, end of run
                    break
                if b >= XY:                              # fsubs.asm:516 — high-bit byte ends run
                    break
                length = length + 1
            Text(rp, msg[start:start + length])          # fsubs.asm:519-521 — draw the text run
            p = start + length                           # fsubs.asm:523 — advance past text; terminator re-read next iteration
```

## placard_text

Source: `narr.asm:235-248`
Called by: `revive` (`fmain.c:2862-2868`), `rescue` (`fmain2.c:1588-1591`), `win_colors` (`fmain2.c:1607`), `copy_protect_junk` setup (`fmain.c:1235`)
Calls: `ssp`, `TABLE:mst`

```pseudo
def placard_text(n: i32) -> None:
    """Dispatch to message n in mst[] and render it via ssp."""
    msg = mst[n]                                         # narr.asm:239-244 — pc-relative offset table lookup
    return ssp(msg)                                      # narr.asm:246 — tail call: jmp _ssp
```

### Message table

This is the authoritative list of all 20 placard messages. Each row shows the exact `(x, y)` pen-move escapes and literal text fragments a port must emit, in order. All coordinates are in the 320×200 display-pixel space of `rp_map`. Messages whose content is just an XY position command followed immediately by ETX (shown as "*(position only)*") move the pen without printing any text; the caller then invokes `name()` to print the hero's name at that position before the next `placard_text` appends more text at the current pen.

**X-coordinate rounding.** In the source, each XY x-byte is written as the expression `x/2`, evaluated at assembly time using 68000 assembler integer division. `ssp` multiplies the stored byte by 2 on read (`fsubs.asm:536`), so any **odd** x in the source collapses to `x−1` at render time. The author-intended coordinates and the actually-rendered coordinates therefore differ on five messages:

| Source expression | Stored byte | Rendered x | Affects |
|-------------------|-------------|------------|---------|
| `21/2` | 10 | **20** | msg8, msg8a, msg8b, msg9, msg9a, msg9b, msg10, msg10a, msg10b |
| `25/2` | 12 | **24** | msg6 line 2 |
| `35/2` | 17 | **34** | msg11 lines 2–5, msg11a |
| `71/2` | 35 | **70** | msg11 line 1 |
| `x/2` (even x) | x/2 | x | all other lines (no rounding) |

The table below lists the **rendered** `(x, y)` — the values a port must pass to its equivalent of `Move()`.

| N  | Asm label | Context | `(x, y)` sequence → text fragment (`name()` splice inserted by caller) |
|----|-----------|---------|------------------------------------------------------------------------|
|  0 | `msg1`    | Game start — Julian's intro, brother 1 only (`fmain.c:2862`) | `(20, 28)` `   "Rescue the Talisman!"` · `(20, 39)` `was the Mayor's plea.` · `(20, 50)` `"Only the Talisman can` · `(20, 61)` `protect our village from` · `(20, 72)` `the evil forces of the` · `(20, 83)` `night." And so Julian` · `(20, 94)` `set out on his quest to` · `(20, 105)` `recover it.` |
|  1 | `msg2`    | Julian's death card (`fmain.c:2866`) | `(24, 44)` `Unfortunately for Julian,` · `(24, 55)` `his luck had run out.` · `(24, 66)` `Many months passed and` · `(24, 77)` `Julian did not return...` |
|  2 | `msg3`    | Phillip set-out text, shown after Julian's card is cleared (`fmain.c:2877`) | `(40, 44)` `So Phillip set out,` · `(40, 55)` `determined to find his` · `(40, 66)` `brother and complete` · `(40, 77)` `the quest.` |
|  3 | `msg4`    | Phillip's death card (`fmain.c:2866`) | `(24, 44)` `But sadly, Phillip's` · `(24, 55)` `cleverness could not save` · `(24, 66)` `him from the same fate` · `(24, 77)` `as his older brother.` |
|  4 | `msg5`    | Kevin set-out text, shown after Phillip's card (`fmain.c:2877`) | `(30, 30)` `So Kevin took up the` · `(30, 41)` `quest, risking all, for` · `(30, 52)` `the village had grown` · `(30, 63)` `desperate. Young and` · `(30, 74)` `inexperienced, his` · `(30, 85)` `chances did not look` · `(30, 96)` `good.` |
|  5 | `msg6`    | All three brothers dead — final game-over card (`fmain.c:2868`) | `(20, 31)` `And so ends our sad tale.` · `(24, 45)` `The Lesson of the Story:` · `(66, 88)` `Stay at Home!` |
|  6 | `msg7`    | Victory part 1 — followed by `name()` (`fmain2.c:1607`) | `(28, 32)` `Having defeated the` · `(28, 43)` `villanous Necromancer` · `(28, 54)` `and recovered the` · `(28, 65)` `Talisman, ` |
|  7 | `msg7a`   | Victory part 2 — appended after hero name | `(28, 76)` `returned to Marheim` · `(28, 87)` `where he wed the` · `(28, 98)` `princess...` |
|  8 | `msg8`    | Katra rescue — positions pen for `name()` (`fmain2.c:1588`, `princess==0`) | `(20, 26)` *(position only)* |
|  9 | `msg8a`   | Katra rescue — appended after hero name; followed by `name()` | ` had rescued Katra,` (at last pen position) · `(20, 37)` `Princess of Marheim.` · `(20, 48)` `Though they had pledged` · `(20, 59)` `their love for each ` · `(20, 70)` `other, ` |
| 10 | `msg8b`   | Katra rescue — appended after second hero name | ` knew that` (at last pen position) · `(20, 81)` ` his quest could not` · `(20, 92)` `be forsaken.` |
| 11 | `msg9`    | Karla rescue — positions pen for `name()` (`fmain2.c:1588`, `princess==1`) | `(20, 33)` *(position only)* |
| 12 | `msg9a`   | Karla rescue — appended after hero name; followed by `name()` | ` had rescued Karla` (at last pen position) · `(20, 44)` `(Katra's sister), Princess` · `(20, 55)` `of Marheim. Though they` · `(20, 66)` `had pledged their love` · `(20, 77)` `for each other, ` |
| 13 | `msg9b`   | Karla rescue — appended after second hero name | `(20, 88)` `knew that his quest` · `(20, 99)` `could not be forsaken.` |
| 14 | `msg10`   | Kandy rescue — positions pen for `name()` (`fmain2.c:1588`, `princess==2`) | `(20, 26)` *(position only)* |
| 15 | `msg10a`  | Kandy rescue — appended after hero name; followed by `name()` | ` had rescued Kandy` (at last pen position) · `(20, 37)` `(Katra's and Karla's` · `(20, 48)` `sister), Princess of` · `(20, 59)` `Marheim. Though they had` · `(20, 70)` `pledged their love for` · `(20, 81)` `each other, ` |
| 16 | `msg10b`  | Kandy rescue — appended after second hero name | ` knew ` (at last pen position) · `(20, 92)` `that his quest could` · `(20, 103)` `not be forsaken.` |
| 17 | `msg11`   | Post-rescue departure — followed by `name()` (`fmain2.c:1591`) | `(70, 37)` `After seeing the` · `(34, 48)` `princess safely to her` · `(34, 59)` `home city, and with a` · `(34, 70)` `king's gift in gold,` · `(34, 81)` *(position only)* |
| 18 | `msg11a`  | Post-rescue departure — appended after hero name | ` once more set` (at last pen position) · `(34, 92)` `out on his quest.` |
| 19 | `msg12`   | Copy-protection quiz lead-in (`fmain.c:1235`) | `(128, 19)` `So...` · `(34, 65)` `You, game seeker, would guide the` · `(10, 75)` `brothers to their destiny? You would` · `(10, 85)` `aid them and give directions? Answer,` · `(10, 95)` `then, these three questions and prove` · `(10, 105)` `your fitness to be their advisor:` |

Messages `msg7`, `msg8a`, `msg9a`, `msg10a`, and `msg11` end with `", ",ETX` (literal text followed by ETX, no final XY). The next `placard_text` or `name()` call therefore resumes drawing at the pen position left by the preceding `Text()` call — the trailing text is **not** positioned by a subsequent XY. Similarly, `msg8b`, `msg9b`, `msg10b`, and `msg11a` open with a literal text fragment (`" knew that"`, `knew that his quest` [positioned by its own XY, unlike 8b/10b], `" knew "`, `" once more set"`) that is drawn at the pen position inherited from the preceding `name()`.

### Deliberate deviation from source: the msg8a typo

> **Warning to future agents:** the rendered text for `msg8a` in the table above **intentionally omits a stray comma** that is present in the original source. **Do not "fix" the table by re-adding it.** See details below.

`narr.asm:307` contains the byte sequence `"their love for each, "` — an extra comma after `each` that does not appear in the parallel msg9a (`"for each other, "`, `narr.asm:317`) or msg10a (`"each other, "`, `narr.asm:326`). Rendered in sequence with the following line (`narr.asm:308`, `"other, "`), the source bytes would display as:

```
their love for each, other,
```

This is a typo in the 1987 source: the author's intent, consistent with the two parallel princess messages, was `their love for each other,`. Because `placard.md` is a **logic spec describing author intent** (not a byte-for-byte dump of `narr.asm`), the rendered-text column above shows the intended text (`for each ` / `other,`) with the stray comma removed.

- **Do not edit `narr.asm`** — source files are read-only in this repository. The typo remains in the source bytes.
- **Do not re-add the stray comma to this spec** — the deviation is deliberate and matches author intent.
- **Ports that want pixel-accurate fidelity** to the original Amiga display must still reproduce the typo on-screen; ports that prefer author intent can drop it. This spec's table shows the author-intent form.

### Multi-part rescue sequences

`placard_text` for princess-rescue cutscenes uses `princess * 3` as an index offset into the message table to select the right princess (0 = Katra, 1 = Karla, 2 = Kandy). The calling pattern is always:

```text
placard_text(8 + princess*3)   → positions pen (XY+ETX only)
name()                          → prints hero name at that position
placard_text(9 + princess*3)   → prints rescue body text
name()                          → prints hero name again mid-sentence
placard_text(10 + princess*3)  → prints closing clause
placard()                       → draws the border
```

Between the two `placard_text` calls in the successor-brother sequence (`fmain.c:2875`), callers issue `SetAPen(rp, 0); RectFill(rp, 13, 13, 271, 107)` to erase the inner text rectangle (coordinates 13–271 × 13–107) before rendering the next card.

### Placard calling sequences

The placard messages are sequenced by callers in `fmain2.c:1586–1607` and `fmain.c:1235, 2862–2877`:

**Copy protection** (`fmain.c:1235`):
- `placard_text(19)` → copy-protection lead-in, then three `question(j)` calls.

**Brother succession** (`fmain2.c:1588`, princess-rescue cutscene for departing brother):
- Let `i = (brother - 2) * 3` (0 for Phillip, 3 for Kevin) — but note the callsite in `fmain2.c:1588` actually indexes by `princess` field (0=Katra, 1=Karla, 2=Kandy). See `rescue` pseudo in fmain2.c.
- `placard_text(8+i)`, `name()`, `placard_text(9+i)`, `name()`, `placard_text(10+i)`.

**King–princess cutscene** (`fmain2.c:1591`):
- `placard_text(17)`, `name()`, `placard_text(18)` — quest resumption.

**Brother-intro / death sequence** (`fmain.c:2862–2877`):
- `placard_text(0)` — Julian intro (brother 1, first game start).
- `placard_text(1)` — Julian failure (brother 1 death).
- `placard_text(2)` — Phillip intro (shown after Julian's card is cleared).
- `placard_text(3)` — Phillip failure (brother 2 death).
- `placard_text(4)` — Kevin intro.
- `placard_text(5)` — all-dead final card.

**Win sequence / game-over** (`fmain2.c:1607`):
- `placard_text(6)`, `name()`, `placard_text(7)`, `placard()` — victory ending.

### Source-level typo note: msg7 "villanous"

`msg7` at `narr.asm:294` spells the word "villanous" (one `l`). This is a separate source-level typo that this spec preserves verbatim in the rendered text (unlike the msg8a comma, "villanous" is the word the author typed and there is no parallel message suggesting a different intent). Do not "correct" it to "villainous" in this spec or in ports that aim for author intent.

## placard

Source: `fsubs.asm:387-475`
Called by: `revive` (`fmain.c:2870`), `rescue` (`fmain2.c:1589`), `win_colors` (`fmain2.c:1607`)
Calls: `SetAPen`, `Move`, `Draw`, `TABLE:xmod`, `TABLE:ymod`, `rp_map`

Draws an animated Greek-key (meander) decorative border directly onto the live display via `rp_map`. The routine executes three nested loops. The outermost iterates `i` from 16 down through 0 (17 passes). For each pass, the middle loop iterates `j_idx` through all 16 entries of the `xmod`/`ymod` delta tables, computing the next endpoint `(dx, dy) = (xorg + xmod[j_idx], yorg + ymod[j_idx])`. After each j-step the origin is advanced: `(xorg, yorg) = (dx, dy)`, chaining the steps into a continuous polyline. After one complete 16-step j-pass the net delta is `(0, +16)`: `xorg` returns to its starting value each time and `yorg` advances by 16 pixels. One j-pass traces one repeat unit of the meander motif; 17 outer iterations lay down 17 repeat units along the long axis.

The innermost k-loop draws each segment **five times**: k = 4, 3, 2, 1 all set color 1 (white); k = 0 sets color 24 (brown-red). Because `rp_map` shares the currently-displayed bitmap — no double-buffer flip occurs inside this function — every individual `Draw` call is immediately visible on screen. The result is a visible white flash on each stroke that resolves to brown-red as each meander unit is laid down. The first `k=4` pass (white) and the final `k=0` pass (brown-red) of each stroke are both observable.

Four line pairs are drawn per k-iteration. Pairs **A** and **B** are 180° rotations of each other about center (142, 62); pairs **C** and **D** are likewise 180° symmetric about the same center. C and D are a 90° axis swap of A/B, so A/B tile vertically down the left/right edges of the frame while C/D tile horizontally along the top/bottom edges. All four sides are therefore drawn simultaneously on every stroke. Pairs A and B are suppressed when `i ≤ 9`: the left/right edges are only ~112 pixels tall and only need 7 meander units (i = 16–10) to fill, while the top/bottom edges are wider and continue to accumulate units through all 17 iterations via pairs C/D.

| Pair | Move to | Draw to | Condition |
|------|---------|---------|-----------|
| A | `(xorg, yorg)` | `(dx, dy)` | `i > 9` only |
| B | `(284 − xorg, 124 − yorg)` | `(284 − dx, 124 − dy)` | `i > 9` only |
| C | `(16 + yorg, 12 − xorg)` | `(16 + dy, 12 − dx)` | always |
| D | `(268 − yorg, 112 + xorg)` | `(268 − dy, 112 + dx)` | always |

The coordinate constants 284 and 124 define the width and height of the border rectangle; 268 and 112 are their complements minus the 16/12-pixel axis-swap offsets. The inner text rectangle cleared by callers (`RectFill(rp, 13, 13, 271, 107)`) sits entirely inside this frame. The `xmod`/`ymod` delta tables (`fsubs.asm:384-385`) each have 16 entries of ±4 or 0:

```
xmod: [-4, -4, -4,  0,  0,  0, +4, +4,  0, -4,  0, +4, +4,  0,  0,  0]
ymod: [ 0,  0,  0, +4, +4, +4,  0,  0, -4,  0, -4,  0,  0, +4, +4, +4]
```

```pseudo
def placard() -> None:
    """Draw the animated 4-fold symmetric Greek-key (meander) border onto the live playfield via rp_map."""
    xorg = 12                                            # fsubs.asm:393 — initial polyline x origin
    yorg = 0                                             # fsubs.asm:394 — initial polyline y origin
    i = 16                                               # fsubs.asm:396 — outer counter: 17 iterations (16 down to 0)
    while i >= 0:                                        # fsubs.asm:396 — iiloop
        j_idx = 0
        while j_idx < 16:                                # fsubs.asm:399 — jloop: 16 delta steps
            dx = xorg + xmod[j_idx]                      # fsubs.asm:408-411
            dy = yorg + ymod[j_idx]                      # fsubs.asm:401-406
            k = 4                                        # fsubs.asm:413 — 5 draw passes per segment
            while k >= 0:                                # fsubs.asm:413 — kloop
                if k == 0:
                    SetAPen(rp_map, 24)                  # fsubs.asm:417-418 — 24 = brown-red: final settled color
                else:
                    SetAPen(rp_map, 1)                   # fsubs.asm:415 — 1 = white: visible trace flash
                if i > 9:                                # fsubs.asm:421-422 — pairs A+B suppressed for i ≤ 9
                    Move(rp_map, xorg, yorg)             # fsubs.asm:424-426 — pair A: direct
                    Draw(rp_map, dx, dy)                 # fsubs.asm:427-429 — pair A: endpoint
                    Move(rp_map, 284 - xorg, 124 - yorg) # fsubs.asm:431-435 — pair B: 180° rotation about (142, 62)
                    Draw(rp_map, 284 - dx, 124 - dy)     # fsubs.asm:436-440 — pair B: endpoint
                Move(rp_map, 16 + yorg, 12 - xorg)       # fsubs.asm:442-446 — pair C: axis-swapped; fills top/bottom
                Draw(rp_map, 16 + dy, 12 - dx)           # fsubs.asm:447-451 — pair C: endpoint
                Move(rp_map, 268 - yorg, 112 + xorg)     # fsubs.asm:453-457 — pair D: 180° of C
                Draw(rp_map, 268 - dy, 112 + dx)         # fsubs.asm:458-462 — pair D: endpoint
                k = k - 1
            xorg = dx                                    # fsubs.asm:466 — advance polyline origin to next point
            yorg = dy                                    # fsubs.asm:467
            j_idx = j_idx + 1
        i = i - 1
```

## Notes

### Border motif — ASCII art

The motif is the classic Greek key (meander): each j-pass lays down one rectangular-spiral hook that interlocks with the next. Each grid cell below represents one 4×4-pixel unit (the step size of `xmod`/`ymod`). Box-drawing characters show the polyline drawn by one complete 16-step j-pass. Pair A/B tiles **vertically** along the left/right edges; pair C/D tiles **horizontally** along the top/bottom edges.

**Vertical tile** — pair A, left edge (pair B is the 180° rotation on the right edge). One hook occupies a 13 px wide × 16 px tall region (cols 0–3, rows 0–4 where row 4 is the top of the next tile). The j-pass starts at (col 3, row 0), traces leftward across the top (row 0, cols 3→0), down the outer edge (col 0, rows 0→3), partway along row 3 (cols 0→2), then curls inward — up col 2, left along row 2, up col 1, right along row 1 all the way to col 3 — and continues down col 3 past the tile into row 4 of the next. Two consecutive hooks shown:

```
col→    0     1     2     3
row 0   ┌─────────────────┐       ← tile 1 top (row 0, full width)
        │                         ← col 0 (outer) descends
row 1   │     ┌───────────┐       ← inner curl top; col 3 (inner) begins
        │     │           │
row 2   │     └─────┐     │       ← inner fold
        │           │     │
row 3   └───────────┘     │       ← tile 1 row 3 (partial, cols 0→2)
                          │       ← notch at col 0 between tiles
row 4   ┌─────────────────┘       ← tile 2 top joins tile 1 col 3 seamlessly
        │                 
row 5   │     ┌───────────┐
        │     │           │
row 6   │     └─────┐     │
        │           │     │
row 7   └───────────┘     │
                          ⋮
```

Column 0 is the **outer** edge of the border (closest to the screen edge); column 3 is the **inner** edge (closest to the text). Col 3 joins seamlessly across the tile boundary — each hook exits at (col 3, row 4) exactly where the next hook's top line begins. The outer edge (col 0) has a 4 px notch between consecutive hooks (rows 3→4) because each hook's row-3 horizontal stops at col 2. Pair B draws the same motif rotated 180° about (142, 62), placing mirror-image hooks on the right edge of the border with their inner column facing the text.

**Horizontal tile** — pair C, top edge (pair D is the 180° rotation on the bottom edge). Pair C is the 90° axis swap of pair A, so the same motif lies on its side: each hook is 16 px wide × 13 px tall with its inner edge on top (toward the text) and outer edge on the bottom (toward the screen edge). Two consecutive hooks shown side by side (cols 0–4 = hook 1, cols 4–8 = hook 2):

```
col→     0     1     2     3     4     5     6     7     8
row 0    │     ┌─────────────────┐     ┌─────────────────┐
         │     │                 │     │                 │
row 1    │     │     ┌─────┐     │     │     ┌─────┐     │
         │     │     │     │     │     │     │     │     │
row 2    │     └─────┘     │     │     └─────┘     │     │
         │                 │     │                 │     │
row 3    └─────────────────┘     └─────────────────┘     ⋯
         └──── hook 1 ────┘      └──── hook 2 ────┘
```

Row 3 is the **outer** edge (screen-side); row 0 is the **inner** edge (text-side, where col 0 of each hook continues as col 4 of the previous hook). The outer edge has a 4 px notch between hooks (row 3, col 3→4) because each hook's row-3 horizontal ends at its own col 3 before the next hook's col 4 continues the bottom. Pair D places the same motif rotated 180°, producing hooks along the bottom edge with their inner edge on top.

The four edges of the complete border are drawn independently by pairs A–D — the pair-A/B vertical hooks and the pair-C/D horizontal hooks are not specifically joined at the corners, so each corner of the border has a small notch between the last vertical hook and the last horizontal hook.

---

### Proposed SYMBOLS.md additions

These globals are required for the pseudo-code above to resolve under `check_symbol_resolution` and should be appended to SYMBOLS.md §3 (Global variables):

```text
mst: list                           # narr.asm:237 — TABLE:mst; 20-entry placard message dispatch table
xmod: list                          # fsubs.asm:384 — TABLE:xmod; 16-entry x-delta table for placard() spiral
ymod: list                          # fsubs.asm:385 — TABLE:ymod; 16-entry y-delta table for placard() spiral
rp_map: object                      # fsubs.asm:380 — full-screen placard RastPort (BitMap redirected by map_message)
```

And to SYMBOLS.md §4 (Table registry):

| `TABLE:mst`  | `narr.asm:237` | 20-entry pc-relative placard message offset table |
| `TABLE:xmod` | `fsubs.asm:384` | 16-entry signed x-delta table: `[-4,-4,-4,0,0,0,4,4,0,-4,0,4,4,0,0,0]` |
| `TABLE:ymod` | `fsubs.asm:385` | 16-entry signed y-delta table: `[0,0,0,4,4,4,0,0,-4,0,-4,0,0,4,4,4]` |
