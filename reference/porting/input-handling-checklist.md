# Input Handling Checklist

Source scope:
- `fmain.c:3008-3046` (`add_device()`, `wrap_device()` — handler install/remove)
- `ftale.h:110-119` (`struct in_work` — handler's data area)
- `fsubs.asm:63-218` (`_HandlerInterface` — interrupt handler)
- `fsubs.asm:220-280` (`keytrans` table)
- `fmain.c:1300-1370` (main loop key processing)

Purpose:
- Ensure ports replicate the circular key buffer, event filtering, joystick-to-key synthesis, mouse-to-menu encoding, and heartbeat timer.

## A. Handler Data Structure (`ftale.h:110-119`)

- [ ] Implement `struct in_work` with fields at exact byte offsets:

| Offset | Field | Type | Purpose |
|--------|-------|------|---------|
| 0 | `xsprite` | short | Mouse pointer X, clamped 5–315 |
| 2 | `ysprite` | short | Mouse pointer Y, clamped 147–195 |
| 4 | `qualifier` | short | Last event qualifier (button/modifier state) |
| 6 | `laydown` | UBYTE | Keyboard buffer write pointer (0–127) |
| 7 | `pickup` | UBYTE | Keyboard buffer read pointer (0–127) |
| 8 | `newdisk` | char | Set to 1 on DISKIN event |
| 9 | `lastmenu` | char | Last mouse-click menu character; cleared on button-up |
| 10 | `gbase` | ptr | GfxBase pointer for sprite movement |
| 14 | `pbase` | ptr | SimpleSprite pointer; NULL disables sprite update |
| 18 | `vbase` | ptr | ViewPort for MoveSprite |
| 22 | `keybuf[128]` | UBYTE[] | Circular keyboard buffer |
| 150 | `ticker` | short | Heartbeat counter (0–16) |

## B. Keyboard Buffer Protocol

- [ ] Buffer is circular, 128 bytes, accessed via `laydown` (write) and `pickup` (read) indices.
- [ ] Both indices wrap with `& 0x7f` — `fsubs.asm:109-110`.
- [ ] Overflow detection: if `laydown == pickup` after increment, the key is dropped — `fsubs.asm:111-112`.
- [ ] Reading: `getkey()` returns `keybuf[pickup]` and increments `pickup`; returns 0 if buffer empty (`laydown == pickup`).
- [ ] All writes to `keybuf` go through `laydown`; reads go through `pickup`.

## C. RAWKEY Event Processing (`fsubs.asm:82-110`)

- [ ] Read raw keycode from event offset 6; save bit 7 (up/down flag) separately.
- [ ] Mask to 7-bit scancode: `scancode = keycode & 0x7f`.
- [ ] Ignore repeat events (qualifier bit 9 set) — `fsubs.asm:91-92`.
- [ ] Ignore scancodes > `$5A` (undefined keys) — `fsubs.asm:97-98`.
- [ ] **Nullify the event**: set event type to `$0000` so Intuition never sees keyboard input — `fsubs.asm:100`.
- [ ] Translate scancode via `keytrans[scancode]` table — `fsubs.asm:102`.
- [ ] Combine translated value with up/down bit: `d0 = keytrans[scancode] | (original_bit7)` — `fsubs.asm:103-104`.
- [ ] Queue the combined value into `keybuf`.

## D. RAWMOUSE Event Processing (`fsubs.asm:114-159`)

- [ ] Detect button state change: XOR current qualifier with stored qualifier.
- [ ] Left button **released** (bit 14 change, now up):
  - If `lastmenu != 0`: queue `lastmenu | $80` (key-up flag) into `keybuf`.
  - Clear `lastmenu` — `fsubs.asm:127`.
- [ ] Left button **pressed** (bit 14 change, now down):
  - Check if click in menu area: X range 215–265 — `fsubs.asm:134-137`.
  - If outside menu: encode joystick direction from mouse position.
  - If in menu area: compute row = `(ysprite - 144) / 9 + 'a'`; store in `lastmenu` and queue as key-down.
- [ ] Mouse movement: update `xsprite`, `ysprite`, clamp to valid ranges.
- [ ] Save new qualifier as `handler_data.qualifier`.

## E. TIMER Event Processing (Heartbeat, `fsubs.asm:72-80`)

- [ ] On each TIMER event: increment `ticker`; if `ticker >= 16`, reset to 0 and synthesize a null key-up event.
- [ ] Synthesized null event uses keycode `$80 + $60` (key-up for scancode $60) — translated value is 0.
- [ ] Purpose: prevents game loop from stalling when no real input arrives.
- [ ] This means `keybuf` can contain 0-value key-up entries; game loop must handle them without side effects.

## F. DISKIN Event Processing (`fsubs.asm:160-170` approx)

- [ ] On DISKIN event: set `handler_data.newdisk = 1`.
- [ ] `waitnewdisk()` polls `handler_data.newdisk` waiting for this flag — `fmain2.c:1442-1450`.

## G. Mouse Pointer Update (`fsubs.asm:185-200`)

- [ ] On RAWMOUSE: update `xsprite`/`ysprite` from event delta.
- [ ] Clamp X to 5–315, Y to 147–195 — `fsubs.asm:150-158`.
- [ ] If `pbase != NULL`: call `MoveSprite(vbase, pbase, xsprite >> 1, ysprite)` — Amiga sprite positioning.

## H. Menu Character Encoding

- [ ] Menu rows mapped to characters `'a'` through `'z'` (or subset) by `(y - 144) / 9 + 'a'`.
- [ ] Button-down: queue character as key-down (no high bit).
- [ ] Button-up: queue `character | 0x80` as key-up.
- [ ] Game loop processes key-down and key-up pairs for menu navigation — verify exact processing in `fmain.c:1300-1370`.

## I. `keytrans` Table (`fsubs.asm:220-280`)

- [ ] 91-entry translation table (`$5B` entries) mapping Amiga raw scancodes to game key codes.
- [ ] Scancodes > `$5A` map to 0 (ignored before table lookup).
- [ ] Joystick events are synthesized as specific key codes; verify exact mappings against `keytrans`.
- [ ] Key codes are game-defined values — not ASCII; verify usage sites in main loop.

## J. Handler Priority

- [ ] Handler registered at priority 51 (higher than Intuition's 50) — sees all events first — `fmain.c:3027`.
- [ ] Events are nullified (`type = $0000`) so Intuition cannot process keyboard input independently.
- [ ] Mouse events are NOT nullified — Intuition still processes RAWMOUSE for pointer positioning.

## K. Known Quirks To Preserve (or Deliberately Normalize)

- [ ] Heartbeat null key-ups in buffer: game loop must skip 0-valued key-up codes without side effects.
- [ ] `lastmenu` holds the last mouse-down menu character until release — only one pending click at a time.
- [ ] Buffer overflow silently drops keys (no error signaled).
- [ ] Repeat keys are silently dropped at interrupt level — no auto-repeat in the game.

## L. Minimum Parity Test Matrix

- [ ] Keyboard keypress: correct translated value appears in `keybuf` as down+up pair.
- [ ] Keyboard repeat: second event silently dropped; only one entry per physical press.
- [ ] Left-click in menu area: character queued on down, `char|$80` queued on release.
- [ ] Left-click outside menu: joystick direction synthesized as key code.
- [ ] No input for 16 TIMER events: null key-up queued; game loop processes it without visible effect.
- [ ] Buffer full: oldest key-down dropped; no crash.
