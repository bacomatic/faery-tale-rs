## 23. Intro & Narrative

### 23.1 Intro Sequence

1. Legal text display (title text on dark blue background) via `ssp(titletext)`
2. 1-second pause
3. Load audio: music + samples from `v6` and `songs` files
4. Start intro music (tracks 12–15)
5. Load title image (`page0`), blit to both display pages
6. Zoom-in: iris opens from center (x=0 to x=160, step +4) with synchronized red→green→blue palette fade-in on `introcolors` (see §27.6).
7. Three story pages with columnar-reveal animation (`copypage` with `flipscan`)
8. Final pause (3.8 seconds)
9. Zoom-out: x=156 down to 0 step −4 with synchronized reverse palette fade. See §27.6.
10. Copy protection challenge

Player can skip at multiple checkpoints.

### 23.2 Copy Protection

#### Riddle System (`copy_protect_junk`)

Presents 3 random fill-in-the-blank questions from 8 question/answer pairs. **Input is restricted to uppercase A–Z keystrokes** — the Amiga input loop converted every typed character to uppercase ASCII before appending to the answer buffer, so lowercase input was not representable. Comparison is therefore a byte-for-byte `==` of the uppercase input against the uppercase stored answer; there is no separate case-folding step. Comparison is **prefix-only**: the loop walks the stored answer until its NUL terminator and does not verify the typed length, so any uppercase prefix of the correct answer is accepted. After each correct answer, the entry is nulled to prevent repeats within the session. Failure triggers `goto quit_all`.

First question effectively deterministic: RNG seed starts at 19837325, no `rand()` consumption before reaching `copy_protect_junk()`, first `rand8()` resolves to index 1 → "Make haste, but take...?"

| Index | Question | Answer |
|-------|----------|--------|
| 0 | "To Quest for the...?" | LIGHT |
| 1 | "Make haste, but take...?" | HEED |
| 2 | "Scorn murderous...?" | DEED |
| 3 | "Summon the...?" | SIGHT |
| 4 | "Wing forth in...?" | FLIGHT |
| 5 | "Hold fast to your...?" | CREED |
| 6 | "Defy Ye that...?" | BLIGHT |
| 7 | "In black darker than...?" | NIGHT |

#### Disk Timestamp Check (`cpytest`)

Validates magic value 230:
- Floppy: navigates `FileLock → fl_Volume → DeviceList`, checks `dl_VolumeDate.ds_Tick`. Failure: `cold()` → `jmp -4` (guru meditation crash).
- Hard drive: reads block 880, checks `buffer[123]`. Failure: `close_all()` (graceful shutdown).

`NO_PROTECT` compile flag disables riddle comparison and floppy timestamp check. Hard drive block-880 check always executes.

### 23.3 Event Messages

39 event messages (indices 0–38) via `event(n)` function, which indexes `_event_msg` table and calls `extract()`. The `%` character substitutes the current brother's name via `datanames[brother-1]`.

### 23.4 Place Names

`find_place()` called from Phase 14g. Determines `hero_sector`, selects message table:
- Outdoor (`region_num < 8`): `_place_tbl` / `_place_msg` — 29 entries
- Indoor (`region_num > 7`): `_inside_tbl` / `_inside_msg` — 31 entries

Each table entry: 3 bytes `{min_sector, max_sector, message_index}`. Linear scan, first match wins. Mountain messages (index 4) vary by region.

### 23.5 Text System

#### Fonts

- **Topaz 8** (`tfont`): ROM font via `OpenFont()`. Used for status bar labels, menu text, map-mode text.
- **Amber 9** (`afont`): Custom disk font from `fonts/Amber/9` via `LoadSeg`. Used for scrolling messages and placard text. Applied with pen 10 foreground, pen 11 background.

#### `ssp` — Scrolling String Print

Embedded positioning via escape code `XY` (byte 128/$80). Format: printable ASCII segments interspersed with `{XY, x_half, y}` triples. X coordinate stored at half value, doubled during rendering.

Algorithm: read byte → if 0: exit; if 128: read (x/2, y), Move(rp, x×2, y); else: scan printable bytes, Text(rp, buffer, count); loop.

Line width: max 36 chars for scroll text, 29 for placard text.

#### `placard` — Decorative Border

Fractal line pattern on `rp_map`: offset tables `xmod`/`ymod` (±4 pixel deltas), mirror-symmetric with center at (284,124) and two 90°/270° rotations, 16×15 outer iterations with 5 inner passes, color 1 for most lines, color 24 for first inner pass.

#### `print` / `print_cont`

- `print(str)`: Scroll `rp_text` up 10px via `ScrollRaster(rp, 0, 10, TXMIN, TYMIN, TXMAX, TYMAX)`, render at (TXMIN, 42). Bounds: TXMIN=16, TYMIN=5, TXMAX=400, TYMAX=44.
- `print_cont(str)`: Append on same line, no scroll.
- Both use global `rp` (set to `rp_text` during gameplay). Text colors: pen 10 fg, pen 11 bg, JAM2 mode.

#### `extract` — Template Engine

Word-wrap at 37 chars using `mesbuf[200]` buffer. `%` substitutes `datanames[brother-1]`. CR (13) forces line break.

#### `prdec` — Decimal Number Printing

Converts number to ASCII digits in `numbuf[11]`, divides by 10 repeatedly, space-fills leading positions.

#### Print Queue (`prq` / `ppick`)

32-entry circular buffer (`print_que[32]`, `prec`/`pplay` indices). `prq(n)` enqueues (drops silently if full). `ppick()` dequeues one per call from Phase 14a:

| Code | Action |
|------|--------|
| 2 | Debug: coords + available memory |
| 3 | Debug: position, sector, extent |
| 4 | Vitality at (245,52) |
| 5 | Refresh menu via `print_options()` |
| 7 | Full stats: Brv(14), Lck(90), Knd(168), Wlth(321) at y=52 |
| 10 | "Take What?" |

Empty queue: `Delay(1)` yields to OS.

### 23.6 Message Dispatch

Three functions indexing into null-terminated string tables and calling `extract()`:
- `event(n)` — `_event_msg` table: hunger, drowning, journey start, etc.
- `speak(n)` — `_speeches` table: NPC dialogue by speech number.
- `msg(table, n)` — generic: explicit table + index.

Common handler `msg1`: skips `n` null-terminated strings to find target, then calls `extract()`.

#### Scroll-area message provenance (invariant)

Every string that reaches the HI scroll area shall originate from exactly one of two authoritative sources — no other source is permitted:

1. **`narr.asm` tables**, shipped in this project as the `[narr]` section of `faery.toml` (`event_msg`, `place_msg`, `inside_msg`, `speeches`) and dispatched through `event(n)`, `speak(n)`, `msg(table, n)`. These are indexed null-terminated strings, printed via `extract()` with `%` → brother-name substitution.
2. **Hardcoded string literals from `fmain.c` / `fmain2.c`**, enumerated exhaustively in [`reference/logic/dialog_system.md`](https://github.com/bacomatic/faery-tale-rs/blob/research/reference/logic/dialog_system.md) under "Hardcoded scroll messages — complete reference". These are composed via `print`, `print_cont`, `prdec`, `extract` and cover door/key feedback, bow/arrow prompts, TAKE-treasure composition, TAKE-body-search composition, USE-menu responses, battle aftermath, eating, and floppy-only save/load prompts.

The complete set of valid scroll-area text is therefore exactly the union of these two inventories. Any string emitted to the scroll area that is not traceable to a `[narr]` index **or** to a specific row in `dialog_system.md`'s hardcoded-message tables is a fidelity violation. Implementors porting these literals should either keep them as the hardcoded composition strings documented in `dialog_system.md` (preserving `extract()` wrapping, `%` substitution, and `prdec` zero-padding semantics) or lift them verbatim into `faery.toml` under a non-`[narr]` table; either way, no new player-facing prose may be invented in Rust code.

Primitives (`print`, `print_cont`, `prdec`, `extract`) and the print-queue dispatcher (`ppick`) are specified behaviorally in [`reference/logic/dialog_system.md`](https://github.com/bacomatic/faery-tale-rs/blob/research/reference/logic/dialog_system.md) §§"print"–"ppick".

### 23.7 Placard Text Messages

20 story messages via `placard_text(n)`:

| Index | Message |
|-------|---------|
| 0 | Julian's quest intro |
| 1 | Julian's failure |
| 2 | Phillip sets out |
| 3 | Phillip's failure |
| 4 | Kevin sets out |
| 5 | Game over ("Stay at Home!") |
| 6–7 | Victory / Talisman recovered |
| 8–10 | Princess Katra rescue |
| 11–13 | Princess Karla rescue |
| 14–16 | Princess Kandy rescue |
| 17–18 | After seeing princess home |
| 19 | Copy protection intro |

### 23.8 Location Messages

`map_message()`: switch to fullscreen text overlay — fade down, clear playfield, hide status bar (VP_HIDE), set `rp = &rp_map`, `viewstatus = 2`.

`message_off()`: return to gameplay — fade down, restore `rp = &rp_text`, show status bar, `viewstatus = 3`.

---


