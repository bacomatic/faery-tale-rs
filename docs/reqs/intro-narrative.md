## 20. Intro & Narrative

### Requirements

| ID | Requirement |
|----|-------------|
| R-INTRO-001 | Intro sequence shall follow this order: legal text (title text on dark blue in Cinematic config) → 1s pause → load audio (music + samples) → start intro music (tracks 12–15) → load title image (`page0`) → **zoom-in**: iris opens from center 0→160 step +4 with synchronized red→green→blue palette fade-in on `introcolors` (see R-FX-007) → 3 storybook pages at 320×200 with columnar-reveal animation (`copypage` + `flipscan`) → final pause (3.8s) → **zoom-out**: 156→0 step −4 with synchronized reverse palette fade (starts at 156 to skip the redundant no-op at 160) → copy protection. |
| R-INTRO-002 | Copy protection: 3 random questions from 8 rhyming-word pairs. Input is restricted to uppercase A–Z keystrokes (the Amiga input loop converted every typed character to uppercase ASCII before storing), so the answer comparison is effectively a byte-for-byte `==` against the stored uppercase answer. Comparison is prefix-only: the loop walks the correct answer until NUL terminator and does not verify the typed length, so any uppercase prefix of the correct answer is accepted. After a correct answer, the entry is nulled to prevent repeats within the session. First question is deterministic from the initial RNG seed. |
| R-INTRO-003 | Complete answer table: 0="LIGHT", 1="HEED", 2="DEED", 3="SIGHT", 4="FLIGHT", 5="CREED", 6="BLIGHT", 7="NIGHT". |
| R-INTRO-004 | Disk timestamp check (`cpytest`): validates magic value 230. Floppy path via `FileLock→DeviceList→dl_VolumeDate.ds_Tick`. Hard drive path reads block 880, checks `buffer[123]`. |
| R-INTRO-005 | `NO_PROTECT` compile flag disables riddle comparison and floppy timestamp check. Hard drive block-880 check always executes. |
| R-INTRO-006 | 39 event messages shall display during gameplay via `event(n)` with `%` substitution for the current brother name. |
| R-INTRO-007 | 29 outdoor place names and 31 indoor place names shall trigger on sector entry via `find_place()` (first-match linear scan of sector-range tables). Mountain messages (index 4) vary by region. |
| R-INTRO-008 | 20 placard/story messages via `placard_text(n)` using `ssp()` renderer with embedded XY positioning (byte 128 + x_half + y, X doubled during rendering). |
| R-INTRO-009 | Line width constraints: max 36 chars for scroll text, 29 for placard text. |
| R-INTRO-010 | Player may skip the intro at multiple checkpoints. |
| R-INTRO-011 | `placard()` visual effect: recursive fractal line pattern on `rp_map` using `xmod`/`ymod` offset tables (±4 pixel deltas), mirror-symmetric with center at (284,124), 16×15 outer iterations with 5 inner passes, color 1 for most lines, color 24 for first inner pass. |
| R-INTRO-012 | **Scroll-area message provenance.** Every string rendered to the HI scroll area (via `print`, `print_cont`, `extract`, `prdec`, or the `ppick` print-queue dispatcher) shall originate from exactly one of two authoritative sources: (a) the `[narr]` tables in `faery.toml` (`event_msg`, `place_msg`, `inside_msg`, `speeches`) dispatched through `event(n)`, `speak(n)`, or `msg(table, n)`; or (b) the hardcoded string literals enumerated in [`reference/logic/dialog_system.md`](https://github.com/bacomatic/faery-tale-rs/blob/research/reference/logic/dialog_system.md) under "Hardcoded scroll messages — complete reference" (door/key feedback, bow/arrow prompts, TAKE treasure composition, TAKE body-search composition, USE-menu responses, battle aftermath, eating, and floppy-only save/load prompts). No other source of scroll-area text is permitted; implementations shall not invent new player-facing prose in Rust code. |
| R-INTRO-013 | **Hardcoded literal fidelity.** Every row in the `dialog_system.md` "Hardcoded scroll messages — complete reference" tables shall be rendered under its documented trigger condition with its documented composition order (including `extract()` word-wrap at 37 chars, `%` → brother-name substitution, and `prdec` zero-padded decimal insertion for numeric fragments such as arrow counts and foe counts). The floppy-only save/load prompts (`fmain2.c:1498`, `:1499`, `:1532`, `:1533`, `:1538`) may be omitted on hard-drive-equivalent configurations, matching the original's conditional-compilation behavior. |
| R-INTRO-014 | **Print primitives and queue.** The four text primitives (`print`, `print_cont`, `prdec`, `extract`) and the 32-entry circular print queue dispatched by `ppick` shall behave as specified in [`reference/logic/dialog_system.md`](https://github.com/bacomatic/faery-tale-rs/blob/research/reference/logic/dialog_system.md): `print()` scrolls the HI area up 10 pixels and redraws at y=42 within the clip rect (TXMIN=16, TYMIN=5, TXMAX=400, TYMAX=44); `print_cont()` appends in place without scrolling; `extract()` word-wraps at 37 chars using a 200-byte row buffer with `%` → `datanames[brother-1]` substitution and CR (13) as a forced break; `prdec()` appends zero-padded decimals via `print_cont`; the queue supports codes 2, 3, 4, 5, 7, and 10 (drops silently when full). |

### User Stories

- As a player, I see the original intro sequence with title zoom, story pages, and music.
- As a player, I can skip the intro to get into the game quickly.
- As a player, I see location names when entering named areas.
- As a player, I see decorative placard borders during story sequences.
- As a player, every message that appears in the scroll area is one the original 1987 game would have shown in the same situation — no modern additions, reworded prose, or developer debug text leaks into my playthrough.

---


