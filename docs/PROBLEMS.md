# Open Problems

Unresolvable questions requiring expert input, runtime experiments, or information not present in the surviving source code. Problems marked **RESOLVED** have been answered through code tracing or experimentation and are retained here for reference.

---

## Open Problems

### P21. `saveload(&map_x, 80)` saves wrong memory region — BSS layout mismatch

**Source**: `fmain2.c:1507` — `saveload((void *)&map_x,80);`

**Description**: The 80-byte "misc variables" save block is critically broken. The developer assumed that the variables declared together in `fmain.c:557-581` would be contiguous in memory starting at `&map_x`, but the Aztec C compiler scattered them across BSS:

| Variable | Declared offset (source comment) | Actual A4-relative | Offset from &map_x |
|----------|-----|-----------|---------|
| map_x | 0 | -16472 | 0 ✓ |
| map_y | 2 | -16474 | **-2** (below block) |
| hero_x | 4 | -16318 | **+154** (above block) |
| hero_y | 6 | -16316 | **+156** (above block) |
| cheat1 | 18 | -16946 | **-474** (below block) |

`saveload(&map_x, 80)` writes 80 bytes starting at A4-16472 going upward (to A4-16393). Of the intended save variables, only `map_x` itself is at offset 0 — the rest are scattered hundreds of bytes away and are **not included** in the save block.

**Evidence**:
- `map_x` confirmed at A4-16472 via `pea (-16472,a4)` at code+0xCED8 (the saveload call) and `addi.w #280,(-16472,a4)` at code+0x1B1E (cheat key handler)
- `map_y` confirmed at A4-16474 via `subi.w #150,(-16474,a4)` at code+0x1ADA (cheat key handler)
- `hero_x` confirmed at A4-16318 via `move.w #19036,(-16318,a4)` at code+0x6AB0 (revive function)
- `hero_y` confirmed at A4-16316 via `move.w #15755,(-16316,a4)` at code+0x6AB6 (revive function)
- `cheat1` confirmed at A4-16946 via 11 `tst.w (-16946,a4)` instructions matching 11 source-code tests
- `sizeof(struct shape)` confirmed as 22 bytes via the computed `anix * sizeof(struct shape)` expression at code+0xCEFE–0xCF10

**Impact**: The save file's 80-byte misc block captures `map_x` at offset 0 followed by 78 bytes of **unrelated BSS variables** (from earlier declarations that the linker placed above `map_x`). Variables essential for correct game state — `map_y`, `hero_x`, `hero_y`, `safe_x`, `safe_y`, `cheat1`, `riding`, `flying`, `wcarry`, `daynight` — are **never saved or restored**.

**Open questions**:
1. **Does the game compensate?** Some variables may be derivable from other saved data (e.g., `anim_list[0].abs_x/abs_y` captures the hero's screen position, and `region_num` is saved separately). The game may partially reconstruct state on load.
2. **What 40 shorts does the block actually contain?** Identifying the variables at A4-relative offsets -16472 through -16394 requires systematic disassembly of all code accessing those addresses.
3. **Did this affect the shipped game?** The save/load bug may explain reported issues with save corruption or unexpected behavior after loading a saved game.
4. **Is this an Aztec C quirk or a linker ordering issue?** Within a single declaration statement (`unsigned short map_x, map_y, ...`), the compiler reversed variable order (last declared at lowest address). Between statements, allocation order appears unrelated to source order.

Reproduction: `python tools/decode_savegame.py game/A.faery` shows `map_x`=18892 (correct for post-revive) but `map_y`=0 at offset 2 (should be 15665).

---

## Resolved Problems

These problems have been conclusively answered from the source code.

---

### P1. `environ` field values in struct shape — RESOLVED

**Source**: `ftale.h:62` — `char environ`
**Resolution**: Complete mapping traced across 43 references. The `environ` field is a terrain-state variable assigned in the sinker block at `fmain.c:1760-1800` based on terrain type:

| Terrain Type | Environ Value | Speed | Effect |
|---|---|---|---|
| 0 (normal) | 0 | 2 | Safe ground |
| 2 (shallow water) | 2 | 1 | Wading, sprite raised 10px |
| 3 (medium water) | 5 | 1 | Deeper wading |
| 4 (deep water) | increments to 10→15 | 1 | SINK state at environ>15 |
| 5 (very deep) | increments to 30 | 1 | Death at 30; quicksand teleport in sector 181 |
| 6 (slippery) | −1 | 4 | Fast walking |
| 7 (ice) | −2 | 40–42 | Velocity-based momentum physics |
| 8 (lava) | −3 | −2 | Walk backwards |

Additional behaviors gated by environ: submersion sprite offset (`fmain.c:2491-2506`), depth-sort adjustment (`fmain.c:2352-2353`), safe zone qualification (`fmain.c:2191`), and weapon hiding at environ>29 (`fmain.c:2503`). Raft proximity (`raftprox==1||2`) overrides environ to 0. The Rose item (`stuff[23]`) clears environ to 0 in the volcanic zone (`fmain.c:1844`).

---

### P4. High weapon overlay indices (80+) in statelist — RESOLVED

**Source**: `fmain.c:164-203` — statelist entries with `wpn_no` values 80–87
**Resolution**: Values 80+ are encoded indices where bit 7 (0x80) serves as a control flag. The rendering code at `fmain.c:2444` computes `inum = statelist[inum].wpn_no + k` where `k` is a weapon-type offset (0=bow, 32=mace, 48=sword, 64=dagger). The check at `fmain.c:2524` tests `inum & 128`: when the high bit is set, inum is masked to 7 bits and the sprite gets a +8 pixel y-offset with reduced height (8px instead of 16px). This allows the same wpn_no values to encode different geometric offsets for ranged vs. melee weapon overlays depending on weapon type.

---

### P7. Exact period of the LCG — RESOLVED

**Source**: `fsubs.asm:299-306`
**Resolution**: The period is **exactly 65536** (full 16-bit period). Verified both mathematically (Hull-Dobell theorem — all three conditions satisfied: gcd(1,65536)=1, 2|45820, 4|45820) and empirically via brute-force simulation from the actual seed 19837325. The 32-bit seed value does not fully repeat (high bits differ), but only the low 16 bits participate in `mulu.w` state progression.

**Verification** (`python tools/verify_lcg_period.py`):
- Hull-Dobell theorem confirms full period of 65536 for a=45821, c=1, m=2^16
- Python model matches 68k Musashi emulator for 100 steps (cross-validation PASS)
- Empirical brute-force period measurement: 65536 (from initial seed 19837325)
- Period confirmed identical from seeds 0 and 1 (65536 each)

---

### P8. `_ion` symbol purpose — RESOLVED

**Source**: `fsubs.asm:50` (declared `public`), defined at `fsubs.asm:357-378`
**Resolution**: `_ion` is a **decimal number-to-ASCII converter**, not interrupt-related. It converts a 32-bit value to up to 10 right-aligned ASCII digit characters in `_numbuf[0..9]`, with leading positions space-filled (`$20`). The implementation uses repeated `divu.w #10` to extract digits. The symbol is exported as a C-callable public interface but is **never called** directly — only the shared inner label `ion6` is used by `_prdec` (`fsubs.asm:341-355`), which renders numbers via `GfxBase->Text()`. The name likely abbreviates "integer on" (integer to string).

---

### P17. `fiery_death` flag — gating mechanism — RESOLVED

**Source**: `fmain.c:1843` — `if (fiery_death)` guards environ-based damage
**Resolution**: `fiery_death` is **actively computed every frame** at `fmain.c:1384-1385` as a volcanic zone bounding box:

$$\text{fiery\_death} = (8802 < \text{map\_x} < 13562) \wedge (24744 < \text{map\_y} < 29544)$$

When TRUE: environ-based water/fire damage applies (`fmain.c:1843-1847`), swan dismount is blocked with event 32 ("Ground is too hot") at `fmain.c:1418`. When FALSE: water/fire terrain is visual-only. The flag is reset to 0 in the end-game function (`fmain.c:2911`).

---

### P21. Sorceress/Priest gold statue inventory — RESOLVED

**Source**: `fmain.c:3400-3403` (sorceress talk), `fmain.c:3384-3385` (priest talk)
**Resolution**: Dialogue does NOT directly increment `stuff[25]`. Instead, setting `ob_listg[].ob_stat = 1` makes the statue object **visible and pickable** by the standard Take handler. The flow: (1) Dialogue sets `ob_stat = 1` → (2) `set_objects()` at `fmain2.c:1262` renders the object at its world coordinates → (3) Player walks to it and uses Take → (4) Normal `itrans` lookup at `fmain.c:3187-3190` maps `STATUE → stuff[25]++`. The desert gate at `fmain.c:1919` checks `stuff[STATBASE] < 5` (the inventory count). All 5 statues (3 ground-placed + 2 dialogue-revealed) use the same itrans pickup path.

---

### P22. Sound toggle handler — RESOLVED

**Source**: `fmain.c:3443-3447` (GAME case in `do_option`)
**Resolution**: The Sound toggle needs no `do_option` handler because it is gated **at the playback call site**. The `effect()` function at `fmain.c:3616-3620` checks `menus[GAME].enabled[7] & 1` before every `playsample()` call. When the bit is clear, all sound effects are silently suppressed. All 8 sound effect call sites in the game (combat hits, bow shots, fireball impacts, carrier sounds) flow through `effect()`, making this a single-point mute gate. This differs from the Music toggle (hit==6), which requires an active `setmood(TRUE)` call to restart playback.

---

### P24. `fade_page` palette mutation — RESOLVED (no glitch)

**Source**: `fmain2.c:381-386`
**Resolution**: No visible palette glitch occurs during region transitions. `fade_page()` **self-corrects** `pagecolors[31]` based on the current `region_num` **before** reading the array for fader computation (lines 381–386 execute before the fading loop at lines 403–416). Since `region_num` is updated at `fmain.c:3609` and `fade_page()` is called via `day_fade()` in the same tick, the corrected value is always used. No `LoadRGB4` call with raw `pagecolors` occurs between region changes and the correction. There is no pristine copy — the mutation is harmless because it is always overwritten before display.

---

### P6. Which `colorplay` is linked — RESOLVED

**Source**: `fmain2.c:425-432` (C version) and `fsupp.asm:1-23` (assembly `_colorplay`)
**Resolution**: The **C version is linked**. `fsupp.asm` is never assembled or linked by the makefile — it does not appear in any build target. Only the C version in `fmain2.c:425-432` is compiled and linked into the game executable. The C version starts its loop at `i=1`, preserving `fader[0]` (background color). `fsupp.asm` is unused dead code, not part of the build.

---

### P13. Backward walking in terrain type 8 — RESOLVED (intentional direction reversal)

**Source**: `fmain.c:1600` — `e = -2` when `environ == -3` (terrain type 8)
**Resolution**: Terrain type 8 is a direction-reversal zone near the Necromancer's area, not lava. The mechanic simply reverses player input — negative speed (`e = -2`) combined with facing flip (`dex ^= 7` at `fmain.c:1654`) causes the actor to walk opposite to the commanded direction. There is no damage-over-time effect from this terrain; it is purely a navigation obstacle. The mechanic applies to both player and NPCs equally (no actor-type check). The terrain assignment at `fmain.c:1765` sets `k = -3` for terrain type 8, and the inline comment reads "walk backwards".

---

### P3. trans_list newstate[4] indexing semantics — RESOLVED

**Source**: `fmain.c:139-149`
**Resolution**: The **only** access point is `fmain.c:1712`: `s = an->state = trans_list[s].newstate[rand4()]`, where `rand4()` returns `rand() & 3`. All four indices are selected **purely randomly** with equal probability each animation tick. Each of the 9 states (0–8) is documented by inline comments describing the animation pose:

| State | Comment | Animation |
|---|---|---|
| 0 | arm down, weapon low | Rest position |
| 1 | arm down, weapon diagonal down | Low guard |
| 2 | arm swing1, weapon horizontal | Swing begins |
| 3 | arm swing2, weapon raised | Mid-swing |
| 4 | arm swing2, weapon diag up | Upswing |
| 5 | arm swing2, weapon high | High swing |
| 6 | arm high, weapon up | Overhead |
| 7 | arm high, weapon horizontal | Overhead strike |
| 8 | arm middle, weapon raise fwd | Forward thrust |

The four `newstate[]` columns are transition targets selected at random each tick, producing stochastic combat animation. Column patterns (analyzed from the table data): [0] generally advances the swing, [1] generally reverses, [2] holds or gently advances, [3] jumps backward. State 7 (overhead strike) is only reachable via column [3] from state 3.

---

### P11. Unused tactics: HIDE (7), DOOR_SEEK (11), DOOR_LET (12) — RESOLVED

**Source**: `ftale.h:49-54`
**Resolution**: HIDE was a planned enemy tactic that was never implemented. DOOR_SEEK and DOOR_LET were planned for the DKnight's door-blocking behavior but were replaced by hardcoded logic: the DKnight stands STILL facing south outside melee range and attacks within 16 pixels (`fmain.c:2162-2169`). All three tactic values have no case in `do_tactic()` (`fmain2.c:1664-1700`) and fall through as silent no-ops.

---

### P18. Spectre/Ghost absolute immunity — RESOLVED

**Source**: `fmain2.c:234` — silent return for race 0x8a/0x8b
**Resolution**: Intentional design. Spectre and Ghost (dead brothers) are non-combatants immune to all damage. The silent return is correct — there is no feedback because the player should not be attacking them.

---

### P16. Missile slot 0 dodge bias — RESOLVED

**Source**: `fmain.c:2289` — `(i != 0 || bitrand(512)>bv)`
**Resolution**: The dodge check is a projectile-vs-target interaction, so using the archer identity would be incorrect — the target dodges the projectile, not the archer. The `archer` field is used at `fmain.c:2284` for its correct purpose: preventing self-hits. Only slot 0 projectiles can be dodged; with 6 missile slots assigned round-robin (`mdex` at `fmain.c:1479`), ~17% of projectiles are dodge-eligible. This was likely intentional to limit dodge frequency, since projectiles are already harder to aim than melee.

---

### P20. Four identical doorlist entries 0–3 — RESOLVED

**Source**: `fmain.c:240-243`
**Resolution**: Likely an editing artifact — possibly placeholder entries that were pasted in but never removed. The duplicates are functionally harmless since all four are byte-identical and the door index is not used after the search.



