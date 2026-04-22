# Discovery: Princess Rescue & Win Condition

**Status**: complete
**Investigated**: 2026-04-06
**Requested by**: orchestrator
**Prompt summary**: Trace the complete win condition, princess rescue sequence, necromancer death → talisman → victory flow, win_colors() display, and all related placard/speech texts.

## References Found

### rescue() — fmain2.c:1584-1603
- fmain2.c:1580 — read — `extern short princess;` — princess counter declaration
- fmain2.c:1584-1603 — write — full `rescue()` function body
- fmain.c:2685 — call — `rescue(); flag = 0; goto findagain;` — triggered from extent system
- fmain.c:1333 — call — `else if (key == 'R' && cheat1) rescue();` — cheat key

### princess counter
- fmain.c:568 — write — `short princess;` — declaration in save block
- fmain2.c:1587 — read — `i = princess*3;` — indexes placard text
- fmain2.c:1594 — write — `princess++;` — incremented after rescue display
- fmain.c:2843 — (no reset) — `ob_list8[9].ob_stat = 3` but princess counter not reset in revive

### win_colors() — fmain2.c:1605-1636
- fmain2.c:1605-1636 — write — full `win_colors()` function body
- fmain.c:3246 — call — `map_message(); SetFont(rp,afont); win_colors();` — called after talisman pickup

### quitflag
- fmain.c:590 — write — `char quitflag;` — declaration
- fmain.c:1269 — write — `cheat1 = quitflag = FALSE;` — reset at game start
- fmain.c:1270 — read — `while (!quitflag)` — main game loop condition
- fmain.c:2872 — write — `if (brother > 3) { quitflag = TRUE; Delay(500); }` — all brothers dead = loss
- fmain.c:3245 — write — `quitflag = TRUE; viewstatus = 2;` — win condition (talisman picked up)
- fmain.c:3466 — write — `if (hit == 6) quitflag = TRUE;` — SAVEX menu → Exit option

### Necromancer → Talisman
- fmain.c:1750-1755 — write — necromancer death transforms to woodcutter, drops talisman
- fmain.c:62 — read — `{ 50, TRUE,5,0,0, 9 }` — necromancer encounter stats (50 HP)
- fmain.c:343 — read — `{ 9563,33883, 10144,34462, 60, 1, 1, 9 }` — necromancer extent

### ob_list8[9].ob_stat — princess capture flag
- fmain.c:2843 — write — `ob_list8[9].ob_stat = 3;` — set during revive (brother succession)
- fmain.c:2684 — read — `if (xtype == 83 && ob_list8[9].ob_stat)` — extent trigger check
- fmain2.c:1601 — write — `ob_list8[9].ob_stat = 0;` — cleared after rescue
- fmain.c:2099 — read — `case 0x84: if (ob_list8[9].ob_stat) speak(16)` — princess proximity speech
- fmain.c:3397 — read — `case 4: if (ob_list8[9].ob_stat) speak(16)` — talk to princess
- fmain.c:3398 — read — `case 5: if (ob_list8[9].ob_stat) speak(17)` — talk to king

### Talisman item (stuff[22])
- fmain.c:407 — read — `{ 11, 0, 80, 0, 8,8, 1, "Talisman" }` — inv_list entry
- fmain2.c:983 — read — `139,22` — itrans mapping: world object 139 → stuff[22]
- fmain.c:3244 — read — `if (stuff[22])` — win condition check after every pickup
- fmain.c:1299 — write — `stuff[22]=0;` — cheat key '.' resets talisman (prevents accidental win)

---

## rescue() Function — Complete Trace

**Definition**: `fmain2.c:1584-1603`

```c
rescue()
{   register long i;
    map_message(); SetFont(rp,afont);       // fade down, set up drawing rp
    i = princess*3;                          // index for princess-specific text
    placard_text(8+i); name(); placard_text(9+i); name(); placard_text(10+i);
    placard(); Delay(380);                   // display rescue story, wait ~7.6 sec
    SetAPen(rp,0); RectFill(rp,13,13,271,107); Delay(10); SetAPen(rp,24);
    placard_text(17); name(); placard_text(18); Delay(380);  // "After seeing the princess..."
    message_off();

    princess++;                              // advance princess counter
    xfer(5511,33780,0);                      // teleport to castle area
    move_extent(0,22205,21231);              // reposition bird extent
    ob_list8[2].ob_id = 4;                   // change noble slot to princess
    stuff[28] = 1;                           // give Writ item
    speak(18);                               // king says "Here is a writ..."
    wealth += 100;                           // +100 gold
    ob_list8[9].ob_stat = 0;                 // clear princess-captured flag
    for (i=16; i<22; i++) stuff[i] += 3;     // +3 of each key type (6 types)
}
```

### Trigger Conditions

**Normal trigger** — `fmain.c:2684-2685`:
```c
if (xtype == 83 && ob_list8[9].ob_stat)
{   rescue(); flag = 0; goto findagain;     }
```
The hero enters extent index 6 (princess extent at coordinates 10820,35646 → 10877,35670 — `fmain.c:346`) AND `ob_list8[9].ob_stat` is nonzero (princess is captured). The extent type is 83 — `fmain.c:346`.

**Cheat trigger** — `fmain.c:1333`:
```c
else if (key == 'R' && cheat1) rescue();
```

### Step-by-step Flow

1. `map_message()` — fade screen to black, set up rp for drawing on map bitmap
2. Set Amber font (`SetFont(rp,afont)`)
3. Compute princess text offset: `i = princess*3` (0, 3, or 6)
4. Display princess-specific rescue text via 3 placard_text calls with `name()` interpolation
5. `placard()` — render ornamental border, `Delay(380)` — wait ~7.6 seconds
6. Clear inner rectangle, then display post-rescue text (placard_text 17 + name + 18)
7. Wait another 380 ticks (~7.6 seconds)
8. `message_off()` — restore normal display
9. `princess++` — advance counter (0→1, 1→2, 2→3)
10. `xfer(5511,33780,0)` — teleport hero near king's castle. Flag=0 means don't recalculate region.
11. `move_extent(0,22205,21231)` — move bird extent (index 0) to new location, creating 500×400 box centered on (22205,21231). This repositions the bird for the player's next quest phase.
12. `ob_list8[2].ob_id = 4` — change ob_list8 slot 2 from noble (ob_id=6) to princess (ob_id=4), placing a rescued princess in the castle interior.
13. `stuff[28] = 1` — give the Writ item.
14. `speak(18)` — king says: "Here is a writ designating you as my official agent. Be sure and show this to the Priest before you leave Marheim."
15. `wealth += 100` — reward 100 gold.
16. `ob_list8[9].ob_stat = 0` — clear the princess-captured flag so rescue can't trigger again until next brother.
17. `for (i=16; i<22; i++) stuff[i] += 3` — give +3 of each key type (Gold, Green, Blue, Red, Grey, White).

### Rewards Summary (per rescue)
- +100 gold (`wealth += 100` — `fmain2.c:1600`)
- +3 of each key type (stuff[16]–stuff[21]) — `fmain2.c:1602`
- Writ (stuff[28] = 1) — `fmain2.c:1598`
- Bird extent repositioned — `fmain2.c:1596`
- Princess NPC placed in castle — `fmain2.c:1597`

---

## Princess Counter

**Declaration**: `fmain.c:568` — `short princess;`

**Values and Meaning**:
| Value | State | Placard Texts Used (indices 8+i*3) |
|-------|-------|-------------------------------------|
| 0 | No princesses rescued yet | 8, 9, 10 — Katra (first princess) |
| 1 | Katra rescued | 11, 12, 13 — Karla (second princess) |
| 2 | Karla rescued | 14, 15, 16 — Kandy (third princess) |
| 3+ | All three rescued | (no more rescue possible — ob_list8[9].ob_stat stays 0) |

**Key behaviors**:
- Incremented by `rescue()` after displaying text — `fmain2.c:1594`
- NOT reset during brother succession (`revive()` at `fmain.c:2830-2850` does NOT touch `princess`)
- Persists across brother deaths — part of the save block (offset within 80-byte block starting at `map_x`)
- `ob_list8[9].ob_stat` IS reset to 3 during `revive()` (`fmain.c:2843`), meaning each brother can trigger a rescue. But the `princess` counter is global, so each rescue shows a different princess's text.

**Princess Names** (from `narr.asm:302-336`):
1. **Katra** — Princess of Marheim (princess=0)
2. **Karla** — Katra's sister, Princess of Marheim (princess=1)
3. **Kandy** — Katra's and Karla's sister, Princess of Marheim (princess=2)

**How many princesses**: There are exactly 3 princesses. There is no cap check on `princess` before computing text offset `8+i*3` — if `princess >= 3`, the `rescue()` function would read placard_text entries beyond msg10b (indices 17+), which are the post-rescue texts. However, in practice this cannot happen because `ob_list8[9].ob_stat` is cleared after each rescue and `revive()` only fires on brother death — so at most 3 rescues occur (one per brother lifetime, three brothers total).

---

## Win Path — Complete Quest Chain

### End-to-End Win Sequence

1. **Rescue princesses** (optional but provides resources):
   - Enter princess extent (xtype=83) at (10820-10877, 35646-35670) — `fmain.c:346`
   - Requires `ob_list8[9].ob_stat` nonzero (set to 3 by `revive()` — `fmain.c:2843`)
   - Calls `rescue()` → rewards: writ, gold, keys, bird relocation

2. **Get Writ from King** (via princess rescue):
   - `stuff[28] = 1` — `fmain2.c:1598`
   - King speaks: "Here is a writ designating you as my official agent..." — speak(18)

3. **Show Writ to Priest**:
   - Talk to priest (setfig index 1) with stuff[28] set — `fmain.c:3383-3386`
   - Priest speaks(39): "Ah! You have a writ from the king. Here is one of the golden statues..."
   - `ob_listg[10].ob_stat = 1` — reveals a golden statue

4. **Collect all 5 golden figurines** (stuff[25]):
   - Sorceress gives one: speak(45) — `fmain.c:3404-3406`
   - Priest gives one (with writ): speak(39)
   - Other sources: various locations (world objects)

5. **Find hidden city / reach Necromancer**:
   - Five figurines reveal the vanishing city
   - Navigate to Necromancer extent at (9563-10144, 33883-34462) — `fmain.c:343`

6. **Defeat the Necromancer**:
   - Race 9, 50 HP, arms=5, immune to weapons < 4 — `fmain.c:62`
   - Only bow (weapon 4) or wand (weapon 5) can damage — combat immunity check
   - On death (vitality → 0, state → DYING, tactic countdown finishes):
     - `an->race = 10` (transforms to Woodcutter) — `fmain.c:1751`
     - `an->vitality = 10; an->state = STILL; an->weapon = 0` — `fmain.c:1752-1753`
     - `leave_item(i,139)` — drops Talisman (world object 139) — `fmain.c:1754`

7. **Pick up the Talisman**:
   - World object 139 maps to stuff[22] via itrans — `fmain2.c:983`
   - After pickup, the win check fires — `fmain.c:3244-3247`:
     ```c
     if (stuff[22])
     {   quitflag = TRUE; viewstatus = 2;
         map_message(); SetFont(rp,afont); win_colors();
     }
     ```

8. **Win sequence plays** (`win_colors()`) — victory display with "winpic"

9. **Game loop exits** — `while (!quitflag)` at `fmain.c:1270` terminates
   - Falls through to `stopscore()` at `fmain.c:2616`
   - Then `quit_all:` label → `close_all()` at `fmain.c:2619-2620`

---

## Win Sequence (win_colors) — Complete Trace

**Definition**: `fmain2.c:1605-1636`

```c
win_colors()
{   register long i, j;
    placard_text(6); name(); placard_text(7); placard(); Delay(80);
    bm_draw = fp_drawing->ri_page->BitMap;
    unpackbrush("winpic",bm_draw,0,0);
    LoadRGB4(&vp_page,(void *)blackcolors,32);
    LoadRGB4(&vp_text,(void *)blackcolors,32);
    vp_text.Modes = HIRES | SPRITES | VP_HIDE;
    screen_size(156);
    for (i=25; i> -30; i--)
    {   fader[0] = fader[31] = 0;
        fader[1] = fader[28] = 0xfff;
        for (j=2; j<28; j++)
        {   if (i+j > 0) fader[j] = sun_colors[i+j];
            else fader[j] = 0;
        }
        if (i > -14)
        {   fader[29] = 0x800;
            fader[30] = 0x400;
        }
        else
        {   j = (i+30)/2;
            fader[29] = 0x100 * j;
            fader[30] = 0x100 * (j/2);
        }
        LoadRGB4(&vp_page,fader,32);
        if (i==25) Delay(60);
        Delay(9);
    }
    Delay(30);
    LoadRGB4(&vp_page,(void *)blackcolors,32);
}
```

### Step-by-step

1. **Victory placard text**: `placard_text(6)` + `name()` + `placard_text(7)`
   - msg7: "Having defeated the villanous Necromancer and recovered the Talisman, " [name]
   - msg7a: [name] "returned to Marheim where he wed the princess..."
   - `placard()` renders ornamental border
   - `Delay(80)` — pause ~1.6 seconds

2. **Load win picture**: `unpackbrush("winpic",bm_draw,0,0)` — loads IFF brush from `game/winpic` file into the drawing page bitmap at position (0,0).

3. **Black out both viewports**: Load black colors into page and text viewports.

4. **Hide text viewport**: `vp_text.Modes = HIRES | SPRITES | VP_HIDE` — hides the HUD.

5. **Set screen to full size**: `screen_size(156)` — expands the playfield viewport.

6. **Sunrise color animation**: A loop from i=25 down to i=-29 (55 iterations):
   - Colors 0 and 31 stay black
   - Colors 1 and 28 stay white (0xfff)
   - Colors 2-27 fade in from `sun_colors[]` table — a gradient from dark to warm sunset/sunrise colors
   - Colors 29-30 transition: start at 0x800/0x400 (reds), then fade down as i drops below -14
   - Each frame loaded via `LoadRGB4(&vp_page,fader,32)`
   - First frame holds extra 60 ticks (~1.2 sec), then 9 ticks per frame (~0.18 sec)
   - Total animation: ~60 + 55*9 = 555 ticks ≈ 11.1 seconds

7. **Final pause**: `Delay(30)` — 0.6 seconds

8. **Fade to black**: `LoadRGB4(&vp_page,(void *)blackcolors,32)` — final blackout

### sun_colors[] Table — fmain2.c:1569-1578

53 entries of Amiga 12-bit RGB values forming a sunrise/sunset gradient:
- Starts at 0x000 (black)
- Builds through dark blues (0x001, 0x002) → blue-greens → warm oranges (0xFAA)
- Peaks around index 40: warm sunset colors (0xFAA, 0xF99, 0xF98)
- Ends at 0x76F — a blue-purple tone

The index `i+j` slides a window across this gradient, creating a progressive color reveal from dark to bright, simulating a sunrise behind the win picture.

---

## Necromancer Death → Talisman

### Necromancer Stats — fmain.c:62
```c
{ 50, TRUE,5,0,0, 9 },  /* 9 - Necromancer - final arena */
```
- 50 HP, aggressive, arms=5 (wand), cleverness=0, treasure group=0 (no normal drops), file_id=9

### Necromancer Extent — fmain.c:343
```c
{  9563,33883, 10144,34462, 60, 1, 1, 9 },  /* necromancer */
```
- Extent type 60 = carrier/special encounter. v3=9 means encounter race 9 (Necromancer).

### Proximity Speech — fmain.c:2099-2100
```c
case 0x89: ...
case    9: speak(43); break; /* necromancer */
```
When player enters necromancer extent and encounter loads, necromancer speaks:
- speak(43): '"So this is the so-called Hero who has been sent to hinder my plans. Simply Pathetic. Well, try this, young Fool!"' — `narr.asm:470-472`

### Death Transition — fmain.c:1749-1755
```c
if (s==DYING && !(--(an->tactic)))
{   an->state = DEAD;
    if (an->race == 0x09)
    {   an->race = 10;           // transform to Woodcutter
        an->vitality = 10;       // set new HP
        an->state = STILL;       // alive again
        an->weapon = 0;          // disarmed
        leave_item(i,139);       // drop the talisman
    }
```

The necromancer does NOT actually die. When the DYING state countdown (tactic field) reaches 0:
1. Race changes from 9 (Necromancer) to 10 (Woodcutter) — `fmain.c:1751`
2. Vitality set to 10 — alive with 10 HP — `fmain.c:1752`
3. State set to STILL (no longer dead/dying) — `fmain.c:1752`
4. Weapon set to 0 (unarmed) — `fmain.c:1753`
5. `leave_item(i,139)` drops the Talisman as world object 139 at the necromancer's position — `fmain.c:1754`

### leave_item() — fmain2.c:1191-1195
```c
leave_item(i,object) short i,object;
{   ob_listg[0].xc = anim_list[i].abs_x;
    ob_listg[0].yc = anim_list[i].abs_y + 10;
    ob_listg[0].ob_id = object;
    ob_listg[0].ob_stat = 1;
}
```
Places world object at the actor's position (+10 on Y), using global object slot 0. Sets ob_stat=1 (ground state, visible/pickup-able).

### Speak(44) — Unused Necromancer Transformation Text
`narr.asm:474-475`: "% gasped. The Necromancer had been transformed into a normal man. All of his evil was gone."
- No `speak(44)` call found anywhere in the codebase. This text appears to be **unused** — the transformation happens silently in code. The woodcutter (race 10) then uses its own speech entries (speak 9, 10, 11) if talked to.

---

## Placard Text — All Victory/Rescue Related

### Index Table — narr.asm:245-249
```
mst: dc.l msg1-mst, msg2-mst, msg3-mst, msg4-mst, msg5-mst, msg6-mst
     dc.l msg7-mst, msg7a-mst
     dc.l msg8-mst, msg8a-mst, msg8b-mst
     dc.l msg9-mst, msg9a-mst, msg9b-mst
     dc.l msg10-mst, msg10a-mst, msg10b-mst
     dc.l msg11-mst, msg11a-mst, msg12-mst
```

Indices 0-based:

| Index | Label | Context | Text |
|-------|-------|---------|------|
| 0 | msg1 | Brother succession: Julian starts | '"Rescue the Talisman!" was the Mayor\'s plea. "Only the Talisman can protect our village from the evil forces of the night." And so Julian set out on his quest to recover it.' |
| 1 | msg2 | Julian dies | "Unfortunately for Julian, his luck had run out. Many months passed and Julian did not return..." |
| 2 | msg3 | Phillip starts | "So Phillip set out, determined to find his brother and complete the quest." |
| 3 | msg4 | Phillip dies | "But sadly, Phillip's cleverness could not save him from the same fate as his older brother." |
| 4 | msg5 | Kevin starts | "So Kevin took up the quest, risking all, for the village had grown desperate. Young and inexperienced, his chances did not look good." |
| 5 | msg6 | All brothers dead (LOSS) | "And so ends our sad tale. The Lesson of the Story: Stay at Home!" |
| **6** | **msg7** | **Win sequence (part 1)** | **"Having defeated the villanous Necromancer and recovered the Talisman, "** [name inserted] |
| **7** | **msg7a** | **Win sequence (part 2)** | [name] **"returned to Marheim where he wed the princess..."** |
| **8** | **msg8** | **Rescue princess 0 (Katra) pt 1** | [name inserted] |
| **9** | **msg8a** | **Rescue Katra pt 2** | [name] **" had rescued Katra, Princess of Marheim. Though they had pledged their love for each other, "** [name] |
| **10** | **msg8b** | **Rescue Katra pt 3** | [name] **" knew that his quest could not be forsaken."** |
| **11** | **msg9** | **Rescue princess 1 (Karla) pt 1** | [name inserted] |
| **12** | **msg9a** | **Rescue Karla pt 2** | [name] **" had rescued Karla (Katra's sister), Princess of Marheim. Though they had pledged their love for each other, "** |
| **13** | **msg9b** | **Rescue Karla pt 3** | [name] **"knew that his quest could not be forsaken."** |
| **14** | **msg10** | **Rescue princess 2 (Kandy) pt 1** | [name inserted] |
| **15** | **msg10a** | **Rescue Kandy pt 2** | [name] **" had rescued Kandy (Katra's and Karla's sister), Princess of Marheim. Though they had pledged their love for each other, "** |
| **16** | **msg10b** | **Rescue Kandy pt 3** | [name] **" knew that his quest could not be forsaken."** |
| **17** | **msg11** | **Post-rescue (all princesses)** | **"After seeing the princess safely to her home city, and with a king's gift in gold, "** [name] |
| **18** | **msg11a** | **Post-rescue pt 2** | [name] **" once more set out on his quest."** |
| 19 | msg12 | Copy protection (intro) | "So... You, game seeker, would guide the brothers to their destiny?..." |

### Speech Texts Related to Win Path

| speak() | Source | Text |
|----------|--------|------|
| 16 | narr.asm (princess) | "Please, sir, rescue me from this horrible prison!" pleaded the princess. |
| 17 | narr.asm (king) | "I cannot help you, young man." said the king. "My armies are decimated, and I fear that with the loss of my children, I have lost all hope." |
| 18 | narr.asm (king post-rescue) | "Here is a writ designating you as my official agent. Be sure and show this to the Priest before you leave Marheim." |
| 39 | narr.asm (priest with writ) | "Ah! You have a writ from the king. Here is one of the golden statues of Azal-Car-Ithil. Find all five and you'll find the vanishing city." |
| 43 | narr.asm (necromancer) | "So this is the so-called Hero who has been sent to hinder my plans. Simply Pathetic. Well, try this, young Fool!" |
| 44 | narr.asm (UNUSED) | "% gasped. The Necromancer had been transformed into a normal man. All of his evil was gone." |

---

## quitflag — Game Termination

**Declaration**: `fmain.c:590` — `char quitflag;`

Three paths set `quitflag = TRUE`:

1. **Win condition** — `fmain.c:3245`: After picking up the Talisman (`stuff[22]` nonzero). Also sets `viewstatus = 2`, calls `map_message()`, `SetFont(rp,afont)`, `win_colors()`. The victory animation plays BEFORE the loop exits.

2. **All brothers dead** — `fmain.c:2872`: `if (brother > 3) { quitflag = TRUE; Delay(500); }`. After the loss placard (msg6: "And so ends our sad tale...Stay at Home!") displays at `fmain.c:2869-2870`.

3. **Exit from menu** — `fmain.c:3466`: `if (hit == 6) quitflag = TRUE;` — SAVEX menu, "Exit" option.

**After loop exits** (`fmain.c:2616-2620`):
```c
    stopscore();
quit_all:
    rp_text.BitMap = wb_bmap;
    SetRast(&rp_text,0);
    close_all();
```
The game stops music, clears the text screen, then `close_all()` frees all resources and calls `exit(0)` — `fmain.c:1005`.

---

## Win Picture ("winpic")

**Display location**: `fmain2.c:1609` — `unpackbrush("winpic",bm_draw,0,0);`

- File: `game/winpic` — IFF ILBM brush image
- Loaded into the drawing page bitmap at position (0,0)
- The image is loaded AFTER the victory placard displays and BEFORE the sunrise color animation
- The text viewport is hidden (`VP_HIDE`) and the page is expanded to full height via `screen_size(156)`
- Colors fade in via the sun_colors animation, revealing the image as a sunrise

---

## Cross-Cutting Findings

- **fmain.c:1299** — `stuff[22]=0` in cheat key handler: The '.' cheat key gives 3 random gold items AND explicitly zeroes stuff[22] (Talisman). This prevents the cheat from accidentally triggering the win condition — a deliberate safeguard.

- **fmain2.c:1597** — `ob_list8[2].ob_id = 4` in rescue(): Changes an NPC slot from noble to princess in the castle interior. This means after rescue, if the player returns to the castle, they see a princess NPC instead of a noble — narrative consistency.

- **Princess counter not reset in revive()**: `fmain.c:2843` resets `ob_list8[9].ob_stat = 3` (re-enabling rescue trigger) but does NOT reset `princess`. This means a second brother rescuing = second princess (Karla), not a repeat of Katra. The game tracks total rescues across brothers.

- **Speak(44) is orphaned**: The necromancer transformation text ("The Necromancer had been transformed into a normal man...") has no `speak(44)` call anywhere. This appears to be cut content or a never-implemented narrative beat.

- **ob_list8[9].ob_stat initial value**: Set to 3 in `revive()` which is called before the first brother starts (via `revive(TRUE)` at `fmain.c:1246`). So the princess is always captive at game start.

- **Woodcutter speaks different lines**: After necromancer transforms to race 10, talking to it yields generic woodcutter speeches (speak 9-11) from the enemy speech table. The woodcutter has 4 HP, 0 arms, 0 cleverness — harmless — `fmain.c:63`.

---

## Unresolved

- **Princess extent location**: The extent at (10820-10877, 35646-35670) — `fmain.c:346` — is a very small box (57×24 units). What in-game location does this correspond to? Without a decoded map overlay, the exact world location (which building/area) cannot be confirmed from source code alone.

- **Bird extent relocation purpose**: `move_extent(0,22205,21231)` moves the bird extent after rescue. Coordinates (22205,21231) — what region is this? The bird is a transport carrier. Moving it post-rescue presumably places it somewhere useful for the next quest phase, but the exact game geography significance is unclear from code alone.

- **Can princess > 2 actually occur?**: With 3 brothers and the `ob_list8[9].ob_stat` reset in revive, theoretically all 3 brothers could each rescue a princess (princess goes 0→1→2→3). If `princess == 3` and another rescue somehow triggered, `placard_text(8+9)` = `placard_text(17)` which is msg11 (the post-rescue text) — the display would be garbled but not crash. In practice this shouldn't happen because `ob_list8[9].ob_stat = 0` after rescue and only resets on brother death.

- **Full quest chain intermediaries**: The complete sequence of 5 golden figurines, spectre bone quest, sorceress interaction, dream knight, etc. — these are part of the broader NPC quest chain documented in `docs/_discovery/npc-quests.md`, not fully re-traced here as they are outside the direct win-condition scope.

## Refinement Log
- 2026-04-06: Initial discovery pass — complete trace of rescue(), win_colors(), quitflag, placard texts, necromancer death, and full win path.
