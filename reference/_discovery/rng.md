# Discovery: Random Number Generation

**Status**: complete
**Investigated**: 2026-04-05
**Requested by**: orchestrator
**Prompt summary**: Trace the complete RNG implementation — algorithm, function family, seeding, state storage, and every callsite across all source files.

## Algorithm

The RNG is a **Linear Congruential Generator (LCG)** implemented in 68000 assembly at `fsubs.asm:299-306`.

Instruction-by-instruction breakdown of `_rand`:

```asm
_rand
    move.l  _seed1,d0       ; fsubs.asm:300 — load 32-bit state into d0
    mulu.w  #45821,d0       ; fsubs.asm:301 — unsigned 16×16→32 multiply: d0 = d0.low16 * 45821
    addq.l  #1,d0           ; fsubs.asm:302 — increment by 1
    move.l  d0,_seed1       ; fsubs.asm:303 — store updated state
    ror.l   #6,d0           ; fsubs.asm:304 — rotate right 6 bits (mixes high/low bits)
    and.l   #$7fffffff,d0   ; fsubs.asm:305 — clear sign bit (force non-negative)
    rts                     ; fsubs.asm:306 — return result in d0
```

### Key algorithmic notes

1. **`mulu.w` operates on the low 16 bits of d0 only.** The 68000 `mulu.w` instruction multiplies two unsigned 16-bit values, producing a 32-bit result. This means the multiplication only uses `seed1 & 0xFFFF` as input, discarding the upper 16 bits of the state for the multiply step. The full 32-bit result is then stored back.

2. **The recurrence is**: `seed1 = (seed1.low16 * 45821 + 1)` where the multiplication produces a 32-bit result. The constant 45821 (decimal) = 0xB2FD.

3. **Post-processing**: The state is rotated right by 6 bits and the sign bit is cleared. This scrambles bit positions so the low bits of the state contribute to high bits of the output, and vice versa. The caller gets a 31-bit non-negative value.

4. **Period**: Because `mulu.w` only reads the low 16 bits of the state, the effective state space for the recurrence is determined by the low 16 bits. The upper 16 bits of `seed1` are always the result of the multiplication and carry no independent state. The effective period is at most 65536 (2^16), not 2^32.

## Function Family

All defined at `fsubs.asm:296-297` (`public` declarations) and implemented at `fsubs.asm:299-340`.

### `_rand` — Base generator
- **Returns**: 31-bit non-negative integer in d0 (range 0 to 0x7FFFFFFF)
- **Side effect**: Advances `_seed1`
- **Location**: `fsubs.asm:299-306`

### `_bitrand(mask)` — Masked random
- **Implementation** (`fsubs.asm:308-310`):
  ```asm
  _bitrand    bsr.s   _rand       ; call _rand
              and.l   4(sp),d0    ; AND result with argument from stack
              rts
  ```
- **Signature**: `bitrand(x)` → `rand() & x`
- **Returns**: Random value masked to the bits of x. If x is a power-of-2 minus 1, this gives a uniform range [0, x].

### `_rand2` — Random bit (0 or 1)
- **Implementation** (`fsubs.asm:312-314`): `rand() & 1`
- **Returns**: 0 or 1

### `_rand4` — Random 2 bits (0–3)
- **Implementation** (`fsubs.asm:316-318`): `rand() & 3`
- **Returns**: 0, 1, 2, or 3

### `_rand8` — Random 3 bits (0–7)
- **Implementation** (`fsubs.asm:320-322`): `rand() & 7`
- **Returns**: 0 through 7

### `_rand64` — Random 6 bits (0–63)
- **Implementation** (`fsubs.asm:324-326`): `rand() & 63`
- **Returns**: 0 through 63

### `_rand256` — Random 8 bits (0–255)
- **Implementation** (`fsubs.asm:328-330`): `rand() & 255`
- **Returns**: 0 through 255

### `_rnd(n)` — Random modulo n
- **Implementation** (`fsubs.asm:332-338`):
  ```asm
  _rnd        bsr.s   _rand       ; call _rand
              move.l  4(sp),d1    ; load argument n from stack
              and.l   #$0000ffff,d0  ; mask result to 16 bits
              divu.w  d1,d0       ; unsigned divide: d0.low16 = quotient, d0.high16 = remainder
              clr.w   d0          ; clear quotient (low word)
              swap    d0          ; swap: remainder now in low word
              rts                 ; return remainder
  ```
- **Signature**: `rnd(n)` → `(rand() & 0xFFFF) % n`
- **Returns**: Value in range [0, n-1]. This is a true modulo operation, unlike the bitwise AND variants.

## Seeding

### Initial seed value
- **Declaration**: `fmain.c:682` — `long seed1 = 19837325, seed2 = 23098324;`
- `seed1` is initialized to the decimal value **19837325** (hex 0x012ED98D).
- `seed2` is initialized to **23098324** but is **never referenced** anywhere in the codebase outside this declaration. It appears to be dead code — possibly a remnant of a planned but unused second generator.

### Developer note about initialization
- `notes:1` — `"Need to initialize random number generator."`
- This is the **complete contents** of the `notes` file. It suggests Talin was aware that the RNG needed proper seeding (e.g., from a timer or clock) but the TODO was apparently never addressed.

### No runtime reseeding
- There is **no code anywhere** that writes to `seed1` other than the `_rand` function itself (`fsubs.asm:303`).
- The seed is not derived from system time, VBlank counter, user input timing, or any other entropy source.
- **Consequence**: Every game session starts with the same RNG sequence. The only source of variation is that `rand()` is called during the copy-protection input loop (`fmain2.c:1327`), so different typing speeds during copy protection change how far the sequence has advanced when gameplay begins.

### Copy-protection entropy
- `fmain2.c:1327` — `rand();` is called inside the keyboard input loop of the copy-protection check (`while (TRUE) { key = getkey(); ... rand(); }`). Each keypress iteration advances the RNG state by one step. Since different players type at different speeds, this provides incidental (but not intentional) seed variation before gameplay begins.

## State Storage

- **Variable**: `_seed1` (assembly label) / `seed1` (C name)
- **Declared**: `fmain.c:682` as `long seed1 = 19837325`
- **Exported**: `fsubs.asm:296` via `public _rand,_seed1,_bitrand,_rnd`
- **Size**: 32-bit long (but only low 16 bits are meaningful for the recurrence, see Algorithm notes)
- **Location**: BSS/data segment (global variable)

## Usage Map

### Combat System

| File:Line | Call | Context |
|-----------|------|---------|
| `fmain.c:2132` | `!bitrand(15)` | AI decision: whether enemy re-evaluates tactics (1/16 chance if bits 0-3 all zero) |
| `fmain.c:2142` | `rand4()+2` | Choose tactic for ranged enemy (tactics 2–5) |
| `fmain.c:2143` | `rand2()+3` | Choose tactic for melee enemy (tactics 3–4) |
| `fmain.c:2148` | `!rand4()` | Whether non-archer AI acts this tick (1/4 chance) |
| `fmain.c:2153` | `rand2()` | Low-vitality enemy: 50% chance to evade |
| `fmain.c:2246` | `bitrand(2)` | Extend weapon reach randomly (0–2 extra) |
| `fmain.c:2247` | `rand8()-3` | Sword hit X scatter (range -3 to +4) |
| `fmain.c:2248` | `rand8()-3` | Sword hit Y scatter (range -3 to +4) |
| `fmain.c:2249` | `rand4()` | Enemy bravery: `2 + rand4()` (range 2–5) |
| `fmain.c:2260` | `rand256()` | Enemy hit check: `rand256() > brave` — brave heroes dodge more |
| `fmain.c:2262` | `rand256()` | Near-miss sword clash sound effect pitch |
| `fmain.c:2291` | `bitrand(512)` | Missile hit check for player: `bitrand(512) > bv` |
| `fmain.c:2292` | `rand8()+4` | Fire-type missile damage (range 4–11) |
| `fmain.c:2293` | `rand8()+4` | Arrow missile damage (range 4–11) |
| `fmain.c:2375` | `rand2()+1` | Witch lightning damage (range 1–2) |
| `fmain.c:2369` | `rand4()` | Witch FX: 1/4 chance to change witch oscillation direction |
| `fmain2.c:1666` | `!(rand()&7)` | `do_tactic`: 1/8 chance AI acts on tactic this tick |
| `fmain2.c:1669` | `!(rand()&3)` | `do_tactic`: 1/4 chance for ATTACK2-level AI |
| `fmain2.c:1677` | `rand()&1` | `do_tactic` SHOOT: 50% chance to fire vs. pursue |
| `fmain2.c:1685` | `rand()&7` | `do_tactic` RANDOM: choose random facing direction (0–7) |

### Movement / Navigation

| File:Line | Call | Context |
|-----------|------|---------|
| `fmain.c:1442` | `rand4()` | Hunger stumble: 1/4 chance direction is deflected |
| `fmain.c:1443` | `rand()&1` | Hunger stumble direction: 50% clockwise vs. counterclockwise |
| `fmain.c:1556` | `rand2()` | SETFIG talking animation: random frame offset |
| `fmain.c:1712` | `rand4()` | State transition: random index into `trans_list[s].newstate[]` |
| `fmain2.c:202-206` | `_rand` (inline asm) | `do_walking`: 50% chance of adding vs. subtracting deviation from facing |

### Encounter Generation

| File:Line | Call | Context |
|-----------|------|---------|
| `fmain.c:2018` | `rand64() == 0` | Wake from sleep during battle: 1/64 chance per tick |
| `fmain.c:2059` | `rand()` | `mixflag`: random bits controlling encounter race/weapon mixing |
| `fmain.c:2059` | `rand4()` | `wt`: weapon type index (0–3) for encounter |
| `fmain.c:2085` | `rand64()` | Encounter spawn check: `rand64() <= danger_level` |
| `fmain.c:2086` | `rand4()` | `encounter_type`: random monster type (0–3) |
| `fmain.c:2723` | `rnd(extn->v2)` | `load_actors`: encounter count = base + rnd(range) |
| `fmain.c:2743` | `bitrand(spread)` | Encounter X placement scatter within spread |
| `fmain.c:2744` | `bitrand(spread)` | Encounter Y placement scatter within spread |
| `fmain.c:2753` | `rand2()` | Mix encounter race: base + 0 or 1 |
| `fmain.c:2756` | `rand4()` | Mix weapon type when `mixflag & 4` |

### Dragon Combat

| File:Line | Call | Context |
|-----------|------|---------|
| `fmain.c:1485` | `rand4()==0` | Dragon fire: 1/4 chance per tick |
| `fmain.c:1487` | `rand2()+1` | Dragon animation frame (1 or 2) |
| `fmain.c:1488` | `rand256()` | Dragon fire sound pitch offset |

### Sound Effects

| File:Line | Call | Context |
|-----------|------|---------|
| `fmain.c:1680` | `rand256()` | Arrow shot sound pitch offset |
| `fmain.c:1690` | `rand256()` | Magic weapon shot sound pitch offset |
| `fmain2.c:239` | `bitrand(511)` | Fire missile hit sound pitch offset |
| `fmain2.c:240` | `bitrand(511)` | Player hit sound pitch offset |

### Treasure / Loot

| File:Line | Call | Context |
|-----------|------|---------|
| `fmain.c:3201` | `rand4()` | Chest loot tier: 0=nothing, 1=single, 2=double, 3=triple |
| `fmain.c:3205` | `rand8()+8` | Random item from indices 8–15 (single loot) |
| `fmain.c:3212` | `rand8()+8` | Random item — first of double loot |
| `fmain.c:3217` | `rand8()+8` | Random item — second of double loot (reroll if same) |
| `fmain.c:3224` | `rand8()+8` | Random item — triple loot item check |
| `fmain.c:3228` | `rand8()+KEYBASE` | Random key from key range |
| `fmain.c:3261` | `rand8()+2` | Arrow loot count from body search (2–9) |
| `fmain.c:3272` | `rand8()` | Body search treasure: `encounter_chart[j].treasure * 8 + rand8()` indexes treasure_probs |
| `fmain.c:3349` | `rand8()+4` | Healing potion: restore 4–11 vitality |

### Stat Checks

| File:Line | Call | Context |
|-----------|------|---------|
| `fmain.c:3402` | `rand64()` | Sorceress luck boost: `luck < rand64()` → gain 5 luck |
| `fmain.c:3496` | `rand64()` | Give gold to beggar: `rand64() > kind` → gain 1 kindness |

### Region / Object Distribution

| File:Line | Call | Context |
|-----------|------|---------|
| `fmain2.c:1230` | `bitrand(0x3fff)` | Random X position for treasure distribution in region |
| `fmain2.c:1231` | `bitrand(0x1fff)` | Random Y position for treasure distribution in region |
| `fmain2.c:1236` | `bitrand(15)` | Index into `rand_treasure[]` table (16 entries) |

### Visual Effects

| File:Line | Call | Context |
|-----------|------|---------|
| `fmain2.c:428` | `bitrand(0xfff)` | `colorplay()` (C version): random 12-bit RGB color for teleport flash |
| `fsupp.asm:9` | `jsr _rand` then `and.l #$0fff,d0` | `_colorplay` (asm version — **NOT LINKED**, fsupp.asm not in makefile): same teleport flash effect, 32 colors × 32 frames |

### Cheat / Debug

| File:Line | Call | Context |
|-----------|------|---------|
| `fmain.c:1299` | `rnd(GOLDBASE)` | Cheat code ('.' key): give 3 of random item (index 0–30) |

### Copy Protection / Entropy

| File:Line | Call | Context |
|-----------|------|---------|
| `fmain2.c:1327` | `rand()` | Called inside copy-protection input loop; advances RNG per keystroke with result discarded |

## Cross-Cutting Findings

1. **`fmain2.c:1327`** — `rand()` called in copy-protection input loop. The result is discarded — this is not intentional seeding but an incidental source of sequence variation. The number of RNG advances depends on how many keystrokes and loop iterations occur during protection entry.

2. **`seed2` declared but never used** (`fmain.c:682`). Declared alongside `seed1` but referenced nowhere in assembly or C code. Possible vestige of a dual-generator design that was abandoned.

3. **`fsupp.asm:1-23`** contains an assembly implementation of `colorplay()` that also exists as a C function in `fmain2.c:427-432`. However, `fsupp.asm` is **not assembled or linked** by the makefile — only the C version in `fmain2.c` is compiled into the game executable.

4. **The `mulu.w` 16-bit limitation** means the RNG has an effective period of at most 65536, not 2^32. This is a significant quality limitation — sequences within a single game session may repeat noticeably. The `ror.l #6` rotation partially compensates by spreading bits, but doesn't increase the period.

## Unresolved

1. **Was `seed2` ever used in a prior version?** There's no usage anywhere, but it's initialized with a specific constant, suggesting a planned or removed purpose. Cannot determine from the surviving source alone.

2. ~~**Which `colorplay` is linked?**~~ **RESOLVED**: `fsupp.asm` is not assembled or linked by the makefile. Only the C version in `fmain2.c:425-432` is compiled into the game.

3. **Exact period of the LCG.** Mathematically, `mulu.w` using only the low 16 bits means the period divides 65536. Whether it is exactly 65536 (full period) depends on whether 45821 is a primitive root modulo 65536. This could be verified with an experiment.

4. **Does the `ror.l #6` affect uniformity?** The rotation produces a 31-bit output from essentially a 16-bit state, which means many 31-bit output values can never appear. The distribution is not uniform over [0, 0x7FFFFFFF].

## Refinement Log
- 2026-04-05: Initial complete discovery pass. All source files searched. All RNG callsites catalogued.
