## 9. Player Movement & Input

### 9.1 Input Sources (Priority Order)

1. **Mouse/compass click** (highest): when either button held (`qualifier & 0x6000`), cursor X > 265 maps position to 3×3 compass grid. X ≤ 265 → direction 9 (menu area, no movement).
2. **Joystick**: JOY1DAT register at `$dff00c` decoded via XOR of adjacent bits per axis, then `com2[4 + yjoy*3 + xjoy]`.
3. **Keyboard** (lowest): stored `keydir` value from numpad keys (codes 20–29). Direction = `keydir − 20`.

Direction lookup table `com2[9]`:

```
com2 = {0, 1, 2, 7, 9, 3, 6, 5, 4}
```

| yjoy\\xjoy | −1 | 0 | +1 |
|---|---|---|---|
| −1 | 0 (NW) | 1 (N) | 2 (NE) |
| 0 | 7 (W) | 9 (stop) | 3 (E) |
| +1 | 6 (SW) | 5 (S) | 4 (SE) |

### 9.2 Input Handler Architecture

The input handler installs at priority 51 (above Intuition's 50). Handler data in `struct in_work` (150+ bytes):

- `xsprite`/`ysprite`: mouse position, clamped X: 5–315, Y: 147–195 (confines pointer to 48-pixel status bar)
- `keybuf[128]`: circular keyboard FIFO with `laydown`/`pickup` pointers (`& 0x7F` wrap)
- `ticker`: heartbeat counter 0–16; at 16, synthesizes fake key event `$E0` to prevent stalls
- `qualifier`: button/modifier state word

Event processing:
- **TIMER** (type 6): increments ticker; at 16, generates synthetic RAWKEY
- **RAWKEY** (type 1): ignores repeats (qualifier bit 9); ignores scancodes > `$5A`; nullifies event (type=0); translates via `keytrans[]`; queues in circular buffer
- **RAWMOUSE** (type 2): XOR detects button transitions; left press in menu area (X: 215–265) computes character code; left press outside menu → direction 9
- **DISKIN** (type $10): sets `newdisk = 1`
- **All events**: apply mouse delta, clamp position, call MoveSprite if `pbase ≠ NULL`

### 9.3 Keyboard Translation

`keytrans[91]` maps Amiga raw scancodes to game-internal codes:

**Numpad direction codes (20–29)**:

```
7=NW(20)   8=N(21)    9=NE(22)
4=W(27)    5=stop(29)  6=E(23)
1=SW(26)   2=S(25)    3=SE(24)
```

Cursor keys ($4C–$4F) → values 1–4 (cheat movement only, NOT direction codes).
Function keys F1–F10 ($50–$59) → values 10–19.

### 9.4 Fight Detection

Combat stance activates when any of:
- Right mouse button held: `qualifier & 0x2000`
- Keyboard numpad-0 held: `keyfight` flag (set on key-down, cleared on key-up)
- Joystick fire button: CIA-A PRA register `$bfe001` bit 7 == 0 (active low, bypasses input.device)

Melee weapon → `state = FIGHTING`; ranged weapon (bow/wand) → `state = SHOOT1`.

Walk trigger: `qualifier & 0x4000` (left mouse) OR `keydir != 0`.

### 9.5 Movement Speed by Terrain

Speed value `e` passed to `newx`/`newy` during WALKING. The if/else chain applies to **all actors** — hero and NPC share the same code path:

| Condition | Speed | Scope | Effect |
|-----------|-------|-------|--------|
| `i == 0 && riding == 5` | 3 | Hero only | Turtle mount (fast overland) |
| `environ == −3` (terrain 8) | −2 | All actors | Direction reversal. NPCs are blocked from terrain 8 by `proxcheck` (`fmain2.c:282`), so in practice this is hero-only. |
| `environ == −2` (terrain 7, ice) | velocity | All actors | Velocity-based physics — no `i == 0` guard |
| `environ == −1` (terrain 6) | 4 | All actors | Fast/slippery, 2× normal |
| `environ == 2` or `> 6` | 1 | All actors | Wading / deep water, half speed |
| Default | 2 | All actors | Normal walking |

Per-speed pixel displacement per frame:

| Speed `e` | Cardinal | Diagonal |
|-----------|----------|----------|
| −2 (reversed) | 3 | 2 |
| 1 | 1 | 1 |
| 2 | 3 | 2 |
| 3 | 4 | 3 |
| 4 | 6 | 4 |

Negative speed (−2) causes backward movement — the signed multiply in `newx`/`newy` handles inversion.

**Hero-only movement rules:**
- Turtle mount (`riding == 5`) forces speed 3 regardless of terrain.
- Crystal shard (`stuff[30]`) passes through terrain type 12 barriers (`fmain.c:1611`).
- Terrain types 8 and 9 are passable for the hero but cause effects (reversal / pit fall).

**NPC-only movement rules:**
- `freeze_timer > 0` → all non-hero actors skip movement entirely (`fmain.c:1473`, `goto statc`).
- Terrain ≥ 10 always blocks (no crystal-shard exception).
- Terrain 8 and 9 are blocked by `proxcheck`'s second probe (threshold ≥ 8).

**Race-based terrain immunity (NPCs):**
- Wraiths (`race == 2`) skip terrain collision entirely (`fmain2.c:279-280`) and have terrain forced to 0 at `fmain.c:1639` (normal speed everywhere).
- Snakes (`race == 4`) have terrain forced to 0 (same mechanism), normal speed everywhere.

**Wading speed gap:** The `environ == 2 || environ > 6` condition creates a brief normal-speed window at environ 3–6 during water-depth ramping. Affects hero and NPCs identically.

### 9.6 Position Update — `newx` / `newy`

```
newx(x, dir, speed):
    if dir > 7: return x
    return (x + (xdir[dir] * speed) >> 1) & 0x7FFF

newy(x, dir, speed):
    same as newx using ydir[], plus preserves bit 15 of original y
```

The `>> 1` is a logical right shift. The `& 0x7FFF` clamps to 15-bit range [0, 32767], providing implicit world wrapping.

### 9.7 Velocity System

#### Ice Physics (environ == −2)

```
vel_x += xdir[dir]
vel_y += ydir[dir]
clamp |vel| to 42 (40 on swan)
position += vel / 4
facing derived from velocity: set_course(0, −vel_x, −vel_y, 6)
```

#### Normal Walking — Velocity Recording

After each non-ice movement: `vel = (new_pos − old_pos) * 4`. Feeds swan dismount check: dismount only when `|vel_x| < 15 && |vel_y| < 15`.

#### FALL State Friction

```
vel_x = (vel_x * 3) / 4
vel_y = (vel_y * 3) / 4
```

Velocity halves approximately every 3 ticks. Position continues updating by `vel / 4`.

### 9.8 Collision Deviation

When **any** actor's movement is terrain-blocked, the game auto-deviates before declaring a block (`fmain.c:1612-1626`):
1. Try `dir + 1` (clockwise) — if clear, commit
2. Try `dir − 2` (counterclockwise from original) — if clear, commit
3. All three blocked → fall through to the shared `blocked:` label (`fmain.c:1654`)

At `blocked:`, behavior diverges by actor index:

- **Player** (`i == 0`): `frustflag++` with escalating visual feedback. This gives the player a clear cue that he has walked into impassable terrain — the character shakes his head, then turns south.
  - **`frustflag` 0–20**: normal standing sprite (no visible effect)
  - **`frustflag` 21–40**: head-shaking animation — oscillation sprites 84/85 (figures 64/65), alternating every 2 game cycles via `dex = 84 + ((cycle >> 1) & 1)` (`fmain.c:1658`)
  - **`frustflag` 41+**: fixed `dex = 40` (statelist[40] = figure 35, south-facing pose). The player sprite **snaps to face south** regardless of input facing (`fmain.c:1657`)
- **NPC** (`i != 0`): `an->tactic = FRUST` for AI resolution next tick — see §11.10.

`frustflag` is a **global `char`** (`fmain.c:589`), not a per-actor field. Only the player path increments it, but **any** actor's successful action inside the shared animation loop resets it to 0: successful walk (`fmain.c:1650`), sink (`fmain.c:1577`), shooting (`fmain.c:1707`), melee hit (`fmain.c:1715`), dying (`fmain.c:1725`). This shared-reset semantics is intentional, not a bug: it suppresses the head-shake / south-snap animation during combat, since active NPCs reset the global on every tick they act. The escalating feedback is reserved for solo exploration moments when the player walks into impassable terrain.

**Port simplification (observable-equivalent).** Rather than threading resets through every actor's success path to emulate the shared-global side effect, the port MAY gate the player increment on "no active enemy NPCs in the current encounter/region." If any enemy NPC is active, hold `frustflag` at 0; otherwise increment on full block and reset on the player's own successful walk. Result is observably identical to the original (no animation during combat; animation only during solo exploration when walking into a wall), with far less coupling. The user-facing behavior is what MUST match — the implementation structure is free.

### 9.9 World Wrapping

Outdoor regions (`region_num < 8`): hero coordinates wrap toroidally at 300 and 32565:

```
if abs_x < 300:      abs_x = 32565
else if abs_x > 32565: abs_x = 300
else if abs_y < 300:  abs_y = 32565
else if abs_y > 32565: abs_y = 300
```

Indoor regions do not wrap. NPCs are never wrapped.

### 9.10 Camera Tracking — `map_adjust`

Dead zone: ±20 pixels X, ±10 pixels Y. Outside dead zone: scroll 1 pixel per tick.

Large jump thresholds: > 70 pixels X, > 44 pixels Y (downward), > 24 pixels Y (upward) → snap immediately. The asymmetric Y thresholds account for the player sprite being offset from screen center.

### 9.11 Hunger Stumble

When `hunger > 120`: 1/4 chance per walking tick of deflecting direction by ±1 (50/50 via `rand() & 1`), wrapped with `& 7`.

### 9.12 Keyboard Shortcuts — `letter_list[38]`

| Key | Menu | Choice | Action |
|-----|------|--------|--------|
| I | ITEMS (0) | 5 | Items menu |
| T | ITEMS (0) | 6 | Take |
| ? | ITEMS (0) | 7 | Look |
| U | ITEMS (0) | 8 | Use |
| G | ITEMS (0) | 9 | Give |
| Y | TALK (2) | 5 | Yell |
| S | TALK (2) | 6 | Say |
| A | TALK (2) | 7 | Ask |
| Space | GAME (4) | 5 | Pause |
| M | GAME (4) | 6 | Music toggle |
| F | GAME (4) | 7 | Sound toggle |
| Q | GAME (4) | 8 | Quit |
| L | GAME (4) | 9 | Load |
| O | BUY (3) | 5 | Buy item |
| R | BUY (3) | 6 | Buy item |
| 8 | BUY (3) | 7 | Buy item |
| C | BUY (3) | 8 | Buy Mace |
| W | BUY (3) | 9 | Buy Sword |
| B | BUY (3) | 10 | Buy Bow |
| E | BUY (3) | 11 | Buy Totem |
| V | SAVEX (5) | 5 | Save |
| X | SAVEX (5) | 6 | Exit |
| F1–F7 | MAGIC (1) | 5–11 | Magic spells 1–7 |
| 1–7 | USE (8) | 0–6 | Use item slots 1–7 |
| K | USE (8) | 7 | Use key |

### 9.13 Crystal Shard Terrain Bypass

When the hero attempts to move and `proxcheck()` returns terrain type 12 (blocked), the movement is permitted if `stuff[30]` (crystal shard) is nonzero. Checked after the door check (terrain 15) and before deviation. Terrain type 12 tiles exist only in terra set 8 (Region 8 building interiors) — tile index 93 in 12 sectors containing small chambers, twisting tunnels, forked intersections, and doom tower. Terra set 10 maps the same tile 93 to type 1/impassable (not crystal wall).

---


