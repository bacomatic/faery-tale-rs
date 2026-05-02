## 22. UI & Menus

### Requirements

| ID | Requirement |
|----|-------------|
| R-UI-001 | 10 menu modes shall be supported: ITEMS(0), MAGIC(1), TALK(2), BUY(3), GAME(4), SAVEX(5), KEYS(6), GIVE(7), USE(8), FILE(9). |
| R-UI-002 | Modes 0–4 (ITEMS through GAME) share a top bar of 5 entries from `label1` ("Items Magic Talk Buy  Game"). Entries 5+ come from each menu's own `label_list`. USE and FILE skip the top bar. |
| R-UI-003 | `enabled[i]` byte encoding: bit 0 = selected/highlighted, bit 1 = visible, bits 2–7 = action type (atype). atype values: 0=nav, 4=toggle, 8=immediate, 12=one-shot highlight. Common encoded: 2=visible nav, 3=visible+highlighted, 6=visible toggle off, 7=visible toggle on, 8=hidden action, 10=visible action. |
| R-UI-004 | `print_options()` renders on `rp_text2`: 2-column layout (x=430, x=482), 6 rows at 9px spacing starting at y=8. `real_options[12]` indirection array maps screen positions to actual enabled[] indices. |
| R-UI-005 | Background pen varies by mode: USE=14, FILE=13, top bar (k<5)=4, KEYS=`keycolors[k-5]` where `keycolors={8,6,4,2,14,1}`, SAVEX=entry index, others=`menus[cmode].color`. |
| R-UI-006 | `set_options()` shall dynamically update menu enabled states after every `do_option()` call based on inventory: MAGIC indices 5–11 from `stuff[9..15]`, USE indices 0–6 from `stuff[0..6]`, KEYS indices 5–10 from `stuff[16..21]`, USE Sun from `stuff[7]`, GIVE Gold if wealth>2. |
| R-UI-007 | `do_option()` dispatch shall handle all 10 modes with correct sub-actions: ITEMS (List/Take/Look/Use/Give), MAGIC (7 spells with guards), TALK (Yell/Say/Ask with NPC response dispatch), BUY (7 purchasable items with costs), GAME (Pause/Music/Sound/Quit/Load), SAVEX (Save/Exit), KEYS (6 key types with `doorfind`), GIVE (Gold/Book/Writ/Bone), USE (equip weapons/items), FILE (8 save slots). |
| R-UI-008 | `gomenu(mode)` shall be blocked if game is paused (checks `menus[GAME].enabled[5] & 1`). |
| R-UI-009 | 38 keyboard shortcuts via `letter_list[38]`: F1–F7 for magic spells, 1–7 for weapons, letters for actions. SAVEX guard: V and X blocked unless `cmode==SAVEX`. KEYS special: if `cmode==KEYS` and key '1'–'6', dispatch directly. |
| R-UI-010 | 8-direction compass at (567,15) on HUD: base compass (`hinor`, 48×24px) with highlighted direction overlay (`hivar`). Only bitplane 2 differs. Direction regions from `comptable[10]` (8 cardinal/ordinal rectangles + 2 null). |
| R-UI-011 | Stats display via print queue: `prq(7)` full stats at y=52 (Brv x=14, Lck x=90, Knd x=168, Wlth x=321), `prq(4)` vitality at (245,52). |
| R-UI-012 | Print queue: 32-entry circular buffer. `prq(n)` enqueues, `ppick()` dequeues one per call from Phase 14a. Commands: 2=debug coords, 3=debug position, 4=vitality, 5=refresh menu, 7=full stats, 10="Take What?". Empty queue yields to OS via `Delay(1)`. |
| R-UI-013 | Two fonts: Topaz 8 (ROM, for status/menu labels), Amber 9 (custom disk font from `fonts/Amber/9`, for scrolling messages and placard text). |
| R-UI-014 | Text rendering: `print(str)` scrolls up 10px then renders at (TXMIN,42). `print_cont(str)` appends without scroll. Bounds: TXMIN=16, TYMIN=5, TXMAX=400, TYMAX=44. Colors: pen 10 fg, pen 11 bg, JAM2 mode. |
| R-UI-015 | `extract()` template engine: word-wrap at 37 chars, `%` substitutes `datanames[brother-1]` (Julian/Phillip/Kevin), CR(13) forces line break, uses `mesbuf[200]` buffer. |
| R-UI-016 | `cheat1` debug flag: in the original, persisted in save files (offset 18 of 80-byte block), only enabled via hex-editing. Gates debug keys: B=summon swan (+Golden Lasso if already on Swan), '.'=random item, R=rescue, '='=prq(2), F9=advance daynight 1000, F10=prq(3), ↑/↓=teleport ±150 Y, ←/→=teleport ±280 X, and the map spell region restriction. The port enables it via a debug-console toggle (see DEBUG_SPECIFICATION) rather than save-file editing. |

### User Stories

- As a player, I can navigate menus to manage inventory, use magic, talk to NPCs, buy items, and save/load.
- As a player, I can use keyboard shortcuts for quick access to common actions.
- As a player, I see my stats and compass direction on the HUD at all times.
- As a player, I see scrolling text messages for events and dialogue.
- As a player, I see location names when entering new areas.

---


