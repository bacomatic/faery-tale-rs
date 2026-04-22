# Discovery: Copy Protection System

**Status**: complete
**Investigated**: 2026-04-06
**Requested by**: orchestrator
**Prompt summary**: Trace the full copy protection system — riddle-based protection, disk timestamp validation, locktest, trigger points, failure consequences, cheat flag interaction, and seekn.

## Overview

The game has two independent copy protection mechanisms:
1. **Riddle-based quiz** (`copy_protect_junk()`) — asks 3 fill-in-the-blank questions from the game manual
2. **Disk timestamp validation** (`cpytest()`) — checks a magic value on the game disk

Both are disabled in the preserved source via `#define NO_PROTECT` at fmain2.c:14.

## 1. copy_protect_junk() — Riddle Quiz

**Location**: fmain2.c:1309-1334

### Answer Table

The 8 answers are stored in a C string array at fmain2.c:1306-1308:

```c
char *answers[] = {
    "LIGHT","HEED","DEED","SIGHT","FLIGHT","CREED","BLIGHT","NIGHT" };
```

### Riddle Questions (narr.asm:63-85)

The `_question` function at narr.asm:63 uses a jump table (`qq`) to index 8 question strings:

| Index | Question (narr.asm) | Answer (fmain2.c:1307) |
|-------|---------------------|------------------------|
| 0 | "To Quest for the...?" | LIGHT |
| 1 | "Make haste, but take...?" | HEED |
| 2 | "Scorn murderous...?" | DEED |
| 3 | "Summon the...?" | SIGHT |
| 4 | "Wing forth in...?" | FLIGHT |
| 5 | "Hold fast to your...?" | CREED |
| 6 | "Defy Ye that...?" | BLIGHT |
| 7 | "In black darker than...?" | NIGHT |

### Algorithm

1. Loop 3 times (`h=0; h<3`) — fmain2.c:1314
2. Each iteration:
   a. Pick a random index 0-7 via `rand8()` (which is `rand() & 7` — fsubs.asm:320-321). Retry until `answers[j] != NULL` — fmain2.c:1316
   b. Display the question via `question(j)` (calls `_question` in narr.asm:63, which indexes the `qq` table and jumps to `_print_cont`) — fmain2.c:1317
   c. Read user input character-by-character via `getkey()`, supporting backspace (`\b`) and enter (`\r`), max 9 characters stored in `answr[]` — fmain2.c:1321-1328
   d. Compare answer: walk through `*a` (correct answer) and `*b` (user answer) byte-by-byte. If any mismatch, return FALSE — fmain2.c:1330-1331
   e. Mark used question as NULL (`answers[j] = NULL`) so it won't repeat — fmain2.c:1333
3. If all 3 answers match, return TRUE — fmain2.c:1335

### Answer Comparison Logic (fmain2.c:1329-1331)

```c
b = answr;
#ifndef NO_PROTECT
while (*a) if (*a++ != *b++) return FALSE;
#endif
```

The comparison walks the correct answer string until NUL terminator. It checks if the typed answer starts with the correct answer but does NOT verify the typed answer is the same length. So typing "LIGHTXYZ" would pass for "LIGHT". The comparison is case-sensitive (uppercase required).

### NO_PROTECT Guard

The answer comparison is wrapped in `#ifndef NO_PROTECT` (fmain2.c:1330). When `NO_PROTECT` is defined (as it is at fmain2.c:14), the comparison is skipped entirely — any answer is accepted.

## 2. cpytest() — Disk Timestamp Validation

**Location**: fmain2.c:1409-1434

### Floppy Disk Path (IsHardDrive() == FALSE)

1. Lock "df0:" for reading — fmain2.c:1417
2. Convert BPTR to C pointer via `ADDR()` macro (`(void *)((int)ptr << 2)`) — fmain2.c:1408, 1419-1420
3. Get the `FileLock` struct, then navigate to `fl_Volume` → `DeviceList` — fmain2.c:1419-1420
4. Check `fdev->dl_VolumeDate.ds_Tick` against magic value **230** — fmain2.c:1421
5. If not 230, call `cold()` — a crash/reset (jumps to address -4 = 0xFFFFFFFC) — fmain2.c:1421, 1437-1438
6. UnLock and return — fmain2.c:1423-1425

This check is guarded by `#ifndef NO_PROTECT` (fmain2.c:1420).

### Hard Drive Path (IsHardDrive() == TRUE)

1. Allocate a 512-byte buffer on the stack — fmain2.c:1428
2. Read disk block 880 (1 block) into buffer via `load_track_range(880,1,buffer,0)` — fmain2.c:1430
3. Check `buffer[123]` (a ULONG at byte offset 492) against magic value **230** — fmain2.c:1431
4. If not 230, call `close_all()` — graceful shutdown — fmain2.c:1431

**Important**: The hard drive path check is NOT gated by `#ifndef NO_PROTECT`. It always executes even when NO_PROTECT is defined.

### IsHardDrive() — hdrive.c:152-155

Simply returns the value of the `hdrive` global variable:
```c
BOOL IsHardDrive(void) { return hdrive; }
```

### cold() — fmain2.c:1437-1438

```asm
_cold   jmp     -4
```

Jumps to address 0xFFFFFFFC — an illegal address that causes an immediate system crash/guru meditation on the Amiga. This is the harsh punishment for floppy-based copy protection failure.

## 3. locktest() — File/Device Lock Test

**Location**: fmain2.c:1401-1405

```c
locktest(name,access) char *name; long access;
{   flock = Lock(name,access);
    if (flock) UnLock(flock);
    return (int)flock;
}
```

A utility function that tests whether a file or device can be locked (i.e., exists and is accessible). It:
1. Attempts to `Lock()` the given path with the given access mode
2. If successful, immediately `UnLock()`s it
3. Returns the lock value (non-zero = success, zero = failure)

Sets the global `flock` (BPTR, fmain2.c:1399) as a side effect.

### Usage in Copy Protection Context

`locktest()` is not directly part of copy protection logic, but is used in `savegame()` to detect available drives:
- fmain2.c:1486 — `locktest("image",ACCESS_READ)` — test for hard drive install
- fmain2.c:1490 — `locktest("df1:",ACCESS_WRITE)` — test for writable df1:
- fmain2.c:1493-1494 — `locktest("df0:",ACCESS_WRITE)` and `!locktest("df0:winpic",ACCESS_READ)` — test df0: is writable and not the game disk

## 4. seekn() — Copy Protection Trigger Point

**Location**: fmain2.c:730-744

```c
seekn()
{   cpytest();
}
```

The function body is minimal — it calls `cpytest()` and nothing else. Below the call is a large block of commented-out code (fmain2.c:733-743) that would have:
- Read disk block 0 into `shape_mem` using IO slot 9
- Called `prot2()` (a function not defined in the current source)
- Called `motor_off()`

This commented-out code suggests an earlier version had additional disk-based protection (`prot2()`), but it was stripped from the preserved source.

## 5. When Copy Protection Triggers

### Startup Sequence

Both protection mechanisms trigger during game startup, after the intro sequence but before gameplay begins:

1. **seekn() / cpytest()** — called at fmain.c:1212, immediately after the intro pages finish (`end_intro:`/`no_intro:` label). This is the silent disk timestamp check.

2. **copy_protect_junk()** — called at fmain.c:1238, after loading shadows and the hi-res screen, displaying placard_text(19). The sequence is:
   - fmain.c:1233 — `stillscreen()` — display the current page
   - fmain.c:1234 — `SetAPen(rp,1)` — set pen color
   - fmain.c:1235 — `placard_text(19)` — display the riddle intro text (msg12 in narr.asm)
   - fmain.c:1237 — `k = TRUE`
   - fmain.c:1238 — `if (copy_protect_junk()==0) goto quit_all;`

### Intro Text (msg12 in narr.asm, index 19)

The placard displayed before the riddles reads:
```
"So...
You, game seeker, would guide the brothers to their destiny? You would
aid them and give directions? Answer, then, these three questions and prove
your fitness to be their advisor:"
```

## 6. What Happens on Failure

### Riddle Failure (copy_protect_junk returns FALSE)

- fmain.c:1238 — `if (copy_protect_junk()==0) goto quit_all;`
- fmain.c:2617-2620 — `quit_all:` label leads to `rp_text.BitMap = wb_bmap; SetRast(&rp_text,0); close_all();`
- This performs a **graceful shutdown** — clears the screen, frees all allocated resources, and exits to the Amiga Workbench.

### Disk Timestamp Failure — Floppy Path

- fmain2.c:1421 — calls `cold()` which executes `jmp -4` (address 0xFFFFFFFC)
- This causes an **immediate system crash** (guru meditation). No cleanup, no graceful exit.

### Disk Timestamp Failure — Hard Drive Path

- fmain2.c:1431 — calls `close_all()` (fmain.c:950)
- This performs a **graceful shutdown** — frees memory, restores the display, and exits.

### No Delayed Punishment

There is no evidence of delayed punishment in the source. Both checks are pass/fail at startup. If either fails, the game does not start. There is no mechanism to degrade gameplay quality over time as a consequence of failed protection.

## 7. cheat1 Flag Interaction with Copy Protection

The `cheat1` flag (fmain.c:562, declared as `short`) does NOT directly interact with the copy protection system. There is no code path that checks `cheat1` before or during either `copy_protect_junk()` or `cpytest()`.

### cheat1 Details

- **Declaration**: fmain.c:562 — `short cheat1;`
- **Initialization**: fmain.c:1269 — `cheat1 = quitflag = FALSE;` (set to 0 at main loop entry)
- **No code sets it to TRUE**: There is no assignment `cheat1 = TRUE` or `cheat1 = 1` anywhere in the source.
- **Persists in saves**: `cheat1` is at byte offset 18 in the 80-byte save data block (fmain.c:557-562), so it survives save/load cycles. A hex-edited save could enable it.
- **Effect when enabled**: Guards debug/cheat keys in the main loop:
  - 'B' — spawn boat/carrier (fmain.c:1293)
  - '.' — add 3 items to random gold slot (fmain.c:1298)
  - 'R' — call `rescue()` (fmain.c:1333)
  - '=' — call `prq(2)` (fmain.c:1334)
  - Key 19 — call `prq(3)` (fmain.c:1335)
  - Key 18 — advance daynight by 1000 (fmain.c:1336)
  - Keys 1-4 — teleport hero (fmain.c:1337-1340)
- **Region restriction**: fmain.c:3310 — `if (cheat1==0 && region_num > 7) return;` — when cheat1 is 0, the map display spell is restricted to regions 0-7.

## 8. NO_PROTECT Compile Flag

**Location**: fmain2.c:14 — `#define NO_PROTECT`

This flag is defined in the preserved source, disabling two protection checks:
1. fmain2.c:1330-1331 — The riddle answer comparison is skipped (`copy_protect_junk` always returns TRUE)
2. fmain2.c:1420-1421 — The floppy disk timestamp check is skipped

**Not disabled**: The hard drive path check (`buffer[123] != 230` at fmain2.c:1431) is NOT wrapped in `#ifndef NO_PROTECT` and will still execute if running from hard drive.

## References Found

- fmain2.c:14 — write — `#define NO_PROTECT` — compile flag disabling protection
- fmain2.c:1306-1308 — read — `char *answers[]` — riddle answer table
- fmain2.c:1309-1335 — read — `copy_protect_junk()` — riddle-based quiz function
- fmain2.c:1329-1331 — read — answer comparison with `#ifndef NO_PROTECT` guard
- fmain2.c:1333 — write — `answers[j] = NULL` — mark used question
- fmain2.c:1401-1405 — read — `locktest()` — file/device existence test
- fmain2.c:1408 — read — `#define ADDR(ptr)` — BPTR to C pointer conversion macro
- fmain2.c:1409-1434 — read — `cpytest()` — disk timestamp validation
- fmain2.c:1420-1421 — read — floppy timestamp check with `#ifndef NO_PROTECT`
- fmain2.c:1428-1431 — read — hard drive block check (NOT gated by NO_PROTECT)
- fmain2.c:1437-1438 — read — `_cold` — crash via `jmp -4`
- fmain2.c:730-744 — read — `seekn()` — wrapper calling cpytest() with commented-out code
- narr.asm:63-85 — read — `_question` function and 8 question strings
- fmain.c:1212 — call — `seekn()` — disk check at startup
- fmain.c:1235 — call — `placard_text(19)` — display riddle intro text
- fmain.c:1238 — call — `if (copy_protect_junk()==0) goto quit_all;` — riddle check at startup
- fmain.c:2617-2620 — read — `quit_all:` label — graceful shutdown on riddle failure
- fmain.c:950 — read — `close_all()` — resource cleanup and exit
- fmain.c:562 — read — `short cheat1;` — declaration
- fmain.c:1269 — write — `cheat1 = quitflag = FALSE;` — initialization
- fmain.c:1293-1340 — read — cheat key handlers gated by `cheat1`
- fmain.c:3310 — read — map spell region restriction gated by `cheat1`
- fsubs.asm:320-321 — read — `_rand8` returns `rand() & 7` (values 0-7)
- hdrive.c:152-155 — read — `IsHardDrive()` returns `hdrive` global
- narr.asm:349-355 — read — msg12 (placard_text index 19) — riddle intro text

## Code Path

### Path 1: Disk Timestamp Check (startup)

1. Entry: fmain.c:1212 — `seekn()` called after intro
2. Calls: fmain2.c:730-731 — `seekn()` calls `cpytest()`
3. Branch: fmain2.c:1412 — `IsHardDrive()` (hdrive.c:152) check
4. Floppy path: fmain2.c:1414-1425 — Lock df0:, check `dl_VolumeDate.ds_Tick == 230`
   - Fail: fmain2.c:1421 → `cold()` → `jmp -4` (crash)
   - Pass: fmain2.c:1423 — UnLock, return
5. Hard drive path: fmain2.c:1428-1432 — Read block 880, check `buffer[123] == 230`
   - Fail: fmain2.c:1431 → `close_all()` (graceful exit)
   - Pass: return normally

### Path 2: Riddle Quiz (startup)

1. Entry: fmain.c:1233-1235 — display still screen, show placard_text(19)
2. Calls: fmain.c:1238 — `copy_protect_junk()`
3. Loop: fmain2.c:1314 — 3 iterations
4. Each: pick random question (rand8), display via `question(j)`, read input, compare
5. Pass: fmain2.c:1335 — returns TRUE → game continues to `revive(TRUE)`
6. Fail: fmain.c:1238 — returns FALSE → `goto quit_all` → `close_all()` (graceful exit)

## Cross-Cutting Findings

- **Hard drive path not protected by NO_PROTECT**: The `buffer[123] != 230` check at fmain2.c:1431 always executes on hard drive installs, even with NO_PROTECT defined. This is likely a bug or oversight in the source — the floppy path is gated but the HD path is not.
- **cheat1 in save data**: `cheat1` is persisted in saves (offset 18 in the 80-byte block starting at `&map_x`), meaning a hex editor could enable cheat mode by setting this short to non-zero in a save file. No code in the source ever sets it to TRUE.
- **prot2() reference**: The commented-out code in `seekn()` at fmain2.c:739 references `prot2()`, which is not defined anywhere in the preserved source. This suggests an additional protection routine existed in earlier versions but was removed.
- **Answer comparison prefix-only**: The comparison at fmain2.c:1330-1331 checks until the correct answer's NUL terminator but doesn't verify the strings are the same length. Typing "LIGHTABC" would pass for "LIGHT".

## Unresolved

None — all 8 questions answered with source citations.

## Refinement Log

- 2026-04-06: Initial discovery pass. Traced both copy protection mechanisms, all 8 riddles, trigger points, failure consequences, cheat flag, and NO_PROTECT flag.
