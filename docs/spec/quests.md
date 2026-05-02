## 15. Quest System

### 15.1 Main Quest Flow

1. **Rescue princess** (up to 3 times): Enter princess extent with `ob_list8[9].ob_stat` set. Rewards: Writ, 100 gold, +3 of each key type, bird extent repositioned.
2. **Show Writ to Priest**: Talk to priest (setfig index 1) with `stuff[28]` → `speak(39)`, reveals golden statue (`ob_listg[10].ob_stat = 1`).
3. **Collect 5 golden figurines** (`stuff[25]`): Sorceress gives one on first visit (`ob_listg[9].ob_stat = 1`); Priest gives one with Writ; three ground pickups at Seahold, Ogre Den, and Octal Room.
4. **Enter hidden city of Azal**: Desert/oasis doors require `stuff[25] >= 5`. Find the Rose (`stuff[23]`) inside Azal for lava immunity.
5. **Obtain Crystal Shard**: Find King's Bone (`stuff[29]`) in underground, give to Spectre (night only) → Crystal Shard (`stuff[30]`) for terrain-12 barrier bypass.
6. **Obtain Sun Stone**: Defeat DreamKnight (40 HP, race 7) at Hidden Valley → access Elf Glade (door 48) → pick up Sun Stone (`stuff[7]`).
7. **Defeat the Witch**: Sun Stone makes Witch (race 0x89) vulnerable to all weapons; Bow/Wand work regardless. Witch drops Golden Lasso (`stuff[5]`) → enables swan flight.
8. **Cross lava to Citadel of Doom** (door 16): Rose provides fire immunity. Enter Doom castle interior, then Stargate portal (door 15) to Spirit Plane.
9. **Navigate Spirit Plane**: Crystal Shard required to pass terrain-12 barriers. Reach Necromancer's arena (sector 46, extent index 4).
10. **Defeat the Necromancer** (race 9, 50 HP): Only Bow or Wand (`weapon >= 4`) can damage. Magic blocked in arena. On death → transforms to Woodcutter (race 10), drops Talisman (object 139).
11. **Pick up the Talisman**: `stuff[22]` set → `quitflag = TRUE` → `win_colors()` victory sequence.

Note: Steps are not strictly ordered — the world is nonlinear — but item gates create this natural progression.

### 15.2 Quest State Flags

| Flag | Meaning | Set By | Cleared By |
|------|---------|--------|------------|
| `ob_list8[9].ob_stat` | Princess captive (nonzero = captive) | `revive(TRUE)` → 3 (`fmain.c:2843`) | `rescue()` → 0 (`fmain2.c:1601`) |
| `ob_listg[9].ob_stat` | Sorceress statue given | First talk → 1 (`fmain.c:3403`) | Never cleared |
| `ob_listg[10].ob_stat` | Priest statue given | Writ presented → 1 (`fmain.c:3384-3385`) | Never cleared |
| `ob_listg[5].ob_stat` | Spectre visibility | `lightlevel < 40` → 3, else → 2 (`fmain.c:2027-2028`) | Dynamically toggled by light level |
| `ob_listg[1-2].ob_stat` | Dead brother bones | Brother death → 1 (`fmain.c:2839`) | Bones picked up → 0 (implicit) |
| `ob_listg[3-4].ob_stat` | Ghost brothers | Brother death → 3 (`fmain.c:2841`) | Bones picked up → 0 (`fmain.c:3174`) |
| `stuff[22]` | Talisman held | Necromancer death → pickup | Triggers win |
| `stuff[25]` | Gold statue count | Various (§15.3) | Never decremented |
| `stuff[28]` | Writ | `rescue()` → 1 (`fmain2.c:1598`) | Never cleared |
| `stuff[29]` | King's Bone | Ground pickup (`ob_list9[8]`) | Spectre trade → 0 (`fmain.c:3503`) |
| `stuff[30]` | Crystal Shard | Spectre drops object 140 | Never cleared |
| `princess` | Rescue counter (0–2) | `rescue()` → increment (`fmain2.c:1594`) | Never reset (persists across brothers) |

### 15.3 Gold Statue Sources

Five golden figurines required to access desert/Azal (`stuff[25] >= 5`):

| # | Source | Object | Location (x, y) | Mechanism |
|---|--------|--------|-----------------|-----------|
| 1 | Sorceress | ob_listg[9] | (12025, 37639) | Talk → `speak(45)`, sets `ob_listg[9].ob_stat = 1` |
| 2 | Priest | ob_listg[10] | (6700, 33766) | Talk with Writ → `speak(39)`, sets `ob_listg[10].ob_stat = 1` |
| 3 | Seahold | ob_listg[6] | (11092, 38526) | Ground pickup via `itrans` |
| 4 | Ogre Den | ob_listg[7] | (25737, 10662) | Ground pickup via `itrans` |
| 5 | Octal Room | ob_listg[8] | (2910, 39023) | Ground pickup via `itrans` |

Dialogue-revealed statues (Sorceress, Priest) work through standard Take: setting `ob_stat = 1` makes the world object visible, and the player picks it up via `itrans` like any ground object.

### 15.4 Key Quest Items

| Item | stuff[] | Obtained From | Purpose |
|------|---------|---------------|---------|
| Talisman | stuff[22] | Necromancer drops on death (obj 139) | Picking it up wins the game |
| Rose | stuff[23] | Ground pickup, `ob_list8[51]` | Lava immunity (`environ = 0` in `fiery_death` zone) |
| Gold Statues ×5 | stuff[25] | Various (§15.3) | Gate to desert/Azal |
| Writ | stuff[28] | Princess rescue → `fmain2.c:1598` | Show to Priest for Gold Statue |
| King's Bone | stuff[29] | Ground pickup, `ob_list9[8]` | Give to Spectre for Crystal Shard |
| Crystal Shard | stuff[30] | Give Bone to Spectre (obj 140) | Walk through terrain type 12 barriers |
| Sun Stone | stuff[7] | Ground pickup, `ob_list8[18]`, inside Elf Glade (door 48) | Makes Witch vulnerable to all weapons |
| Golden Lasso | stuff[5] | Witch drops on death (obj 27) | Enables riding the Swan |
| Sea Shell | stuff[6] | Talk to Turtle carrier | Summon Turtle for ocean travel |

### 15.5 Quest State Gates

| Gate | Condition | Effect | Citation |
|------|-----------|--------|----------|
| Desert/Azal entrance | `stuff[STATBASE] < 5` | DESERT-type doors blocked | `fmain.c:1919` |
| Azal city map | `stuff[25] < 5` | Tiles overwritten to impassable 254 | `fmain.c:3594-3596` |
| King's castle pax | `xtype == 81` | `event(15)` — weapon sheathed | `fmain.c:1413` |
| Sorceress pax | `xtype == 82` | `event(16)` — calming influence | `fmain.c:1414` |
| Witch invulnerability | `weapon < 4 && (race==9 \|\| (race==0x89 && stuff[7]==0))` | Damage blocked | `fmain2.c:231-233` |
| Necromancer invulnerability | `weapon < 4` | Damage blocked to race 9 | `fmain2.c:231-232` |
| Spectre/Ghost immunity | Absolute | `dohit()` returns early for 0x8a/0x8b | `fmain2.c:234` |
| Magic blocked in necro arena | `extn->v3 == 9` | `speak(59)` ("Your magic won't work here") | `fmain.c:3305` |
| Crystal shard passwall | `stuff[30] && j==12` | Bypass terrain type 12 collision | `fmain.c:1609` |
| Rose lava protection | `stuff[23]` | `environ = 0` (no lava damage) | `fmain.c:1844` |
| Golden lasso + bird | `stuff[5]` | Required to ride bird carrier | `fmain.c:1498` |

### 15.6 Princess Rescue Sequence

Triggered when the player enters the princess extent (`xtype == 83`, coordinates 10820–10877, 35646–35670) and `ob_list8[9].ob_stat` is set (`fmain.c:2684-2685`). Cheat shortcut: `'R' && cheat1` (`fmain.c:1333`).

`rescue()` function (`fmain2.c:1584-1603`):

1. `map_message()` + `SetFont(rp, afont)` — enter fullscreen text mode with Amber font.
2. Compute text offset `i = princess * 3` — indexes princess-specific placard text.
3. Display rescue story: `placard_text(8+i)`, `name()`, `placard_text(9+i)`, `name()`, `placard_text(10+i)`, then `placard()` + `Delay(380)` (~7.6 sec).
4. Clear inner rectangle, display post-rescue text: `placard_text(17)` + `name()` + `placard_text(18)`, `Delay(380)` (~7.6 sec).
5. `message_off()` — restore normal display.
6. `princess++` — advance counter.
7. `xfer(5511, 33780, 0)` — teleport hero near King's castle.
8. `move_extent(0, 22205, 21231)` — reposition bird extent from southern mountains to Marheim farmlands.
9. `ob_list8[2].ob_id = 4` — place rescued princess NPC in castle.
10. `stuff[28] = 1` — give Writ item.
11. `speak(18)` — King says "Here is a writ designating you as my official agent…"
12. `wealth += 100` — gold reward.
13. `ob_list8[9].ob_stat = 0` — clear princess captive flag.
14. `for (i=16; i<22; i++) stuff[i] += 3` — give +3 of each key type.

**Three princesses**:

| `princess` value | Name | Placard Text Indices |
|------------------|------|----------------------|
| 0 | Katra | 8, 9, 10 |
| 1 | Karla | 11, 12, 13 |
| 2 | Kandy | 14, 15, 16 |

Shared post-rescue texts: `placard_text(17)` and `placard_text(18)`, used for all three princesses.

The `princess` counter persists across brother succession — `revive(TRUE)` does NOT reset it. However, `ob_list8[9].ob_stat` IS reset to 3 during `revive(TRUE)`, enabling each new brother to trigger a rescue with different princess text. After `princess >= 3`, no further rescues can fire because the third `rescue()` call clears `ob_stat` to 0 and subsequent placard indices would overflow.

### 15.7 Necromancer and Talisman

**Necromancer stats**: Race 9, 50 HP, weapon 5 (wand), aggressive. Extent at coordinates 9563–10144, 33883–34462 (`fmain.c:343`).

**Combat**: Only Bow (`weapon == 4`) or Wand (`weapon == 5`) can damage. Magic is blocked in the arena (`extn->v3 == 9` → `speak(59)`). Proximity auto-speak: `speak(43)` ("So this is the so-called Hero… Simply Pathetic.").

**On death** (`fmain.c:1750-1755`):
- Transforms to Woodcutter: `an->race = 10`, `an->vitality = 10`, `an->state = STILL`, `an->weapon = 0`.
- Drops the Talisman: `leave_item(i, 139)`.

World object 139 maps to `stuff[22]` via the `itrans` lookup (`fmain2.c:983`).

### 15.8 Win Condition and Victory Sequence

**Win check** — after every item pickup (`fmain.c:3244-3247`):

```
if (stuff[22])
{   quitflag = TRUE; viewstatus = 2;
    map_message(); SetFont(rp,afont); win_colors();
}
```

**Victory sequence** (`win_colors()` — `fmain2.c:1605-1636`):

1. Display victory placard: `placard_text(6)` + `name()` + `placard_text(7)` — "Having defeated the villainous Necromancer and recovered the Talisman, [name] returned to Marheim where he wed the princess…". `placard()` + `Delay(80)`.
2. Load win picture: `unpackbrush("winpic", bm_draw, 0, 0)` — IFF image from `game/winpic`.
3. Black out both viewports and hide HUD.
4. Switch to Cinematic config (312×194, HUD hidden). This triggers a one-frame `introcolors` palette fade per §27.6 which step 5 overrides.
5. Sunrise animation — 55 frames (i=25 down to −29): slides a window across the `sun_colors[53]` gradient table (53 entries of 12-bit RGB values). Colors 2–27 fade in progressively, colors 29–30 transition through reds. First frame pauses 60 ticks; subsequent frames at 9 ticks each. Total: ~555 ticks ≈ 11.1 seconds.
6. Final pause `Delay(30)`, then blackout via `LoadRGB4(&vp_page, blackcolors, 32)`.

### 15.9 Game Termination

`quitflag` (`fmain.c:590`) controls the main loop `while (!quitflag)`:

| Trigger | Value | Meaning |
|---------|-------|---------|
| Game start | `FALSE` | Reset at `fmain.c:1269` |
| All brothers dead | `TRUE` | `fmain.c:2872` — game over after `placard_text(5)` + `Delay(500)` |
| Talisman picked up | `TRUE` | `fmain.c:3245` — victory |
| SAVEX → Exit | `TRUE` | `fmain.c:3466` — player quit |

After loop exits: `stopscore()` at `fmain.c:2616`, then `close_all()` at `fmain.c:2619-2620`.

### 15.10 Hidden City Reveal

When entering region 4 (desert) with fewer than 5 golden statues (`stuff[25] < 5`), four tiles at map offset `(11×128)+26` are overwritten with impassable tile 254 (`fmain.c:3594-3596`). With ≥ 5 statues, the tiles remain passable. Patch is applied on every region load (RAM-only modification).

Additionally, all 5 DESERT-type oasis doors (door indices 7–11) require `stuff[25] >= 5` to enter (`fmain.c:1919`). Without sufficient statues, door entry is blocked.

### 15.11 Stone Ring Teleportation Network

11 stone ring locations. Activation requires:
1. Standing on stone ring tile (sector 144)
2. Center-of-tile position (sub-tile check)
3. Match against `stone_list[]`

Destination = `(current_stone + facing + 1) % 11`. Direction determines which ring to teleport to. Visual effect: 32 frames of random palette cycling (`colorplay()`).

### 15.12 Brother Succession and Quest Continuity

On brother death with `luck < 1` (permanent), `revive(TRUE)` activates the next brother:

**Persists across brothers**:
- `princess` counter (advances through Katra → Karla → Kandy)
- All quest flags (`ob_listg`, `ob_list8` entries)
- Princess captive flag reset to 3 (`ob_list8[9].ob_stat = 3`) — enables next rescue
- Dead brother's bones (`ob_listg[1-2].ob_stat = 1`) and ghost (`ob_listg[3-4].ob_stat = 3`) placed in world

**Resets for new brother**:
- Stats loaded fresh from `blist[]` (Julian/Phillip/Kevin have different brave/luck/kind/wealth)
- Inventory cleared — new brother starts with only a Dirk (`stuff[0] = 1`)
- Position resets to Tambry (19036, 15755)
- Hunger and fatigue reset to 0

**Inventory recovery**: When a living brother picks up dead brother's bones (ob_id 28, `fmain.c:3173-3177`):
1. Both ghost setfigs removed: `ob_listg[3].ob_stat = ob_listg[4].ob_stat = 0`
2. Dead brother's entire 31-slot inventory merged into current brother's inventory

**Three brothers** (from `blist[]` — `fmain.c:2807-2812`):

| Brother | Brave | Luck | Kind | Wealth | Starting Vitality |
|---------|-------|------|------|--------|-------------------|
| Julian | 35 | 20 | 15 | 20 | 23 |
| Phillip | 20 | 35 | 15 | 15 | 20 |
| Kevin | 15 | 20 | 35 | 10 | 18 |

Vitality = `15 + brave/4`. Placard texts for succession:
- Julian starts: `placard_text(0)` + `event(9)`
- Phillip starts: `placard_text(1)`, `placard_text(2)` + `event(9)`, `event(10)`
- Kevin starts: `placard_text(3)`, `placard_text(4)` + `event(9)`, `event(11)`
- All dead: `placard_text(5)` ("Stay at Home!") → `quitflag = TRUE`

### 15.13 Stargate Portal

The Stargate is the narrative name for the bidirectional door pair at `doorlist[14..15]` (`fmain.c:254-255`). Mechanically it is a plain pair of `STAIR`-type doors (value 15, horizontal) routed through the generic `check_door` / [`xfer()`](#166-the-xfer-function) flow — there is **no stargate-specific code path**.

| Entry | Role | Outdoor Endpoint | Indoor Endpoint | Destination `secs` |
|-------|------|------------------|-----------------|--------------------|
| `doorlist[14]` | Citadel entry | Outdoor Citadel-of-Doom approach (inside the fiery_death zone) | Doom castle interior (region 8, sectors 135–138) | 1 |
| `doorlist[15]` | Spirit Plane portal | Doom castle interior (region 8, within sec 135–138) | Spirit Plane / astral maze (region 9, sectors 43–59, 100, 143–149) | 2 |

**Traversal requirements (door-level):** none. The Stargate doors themselves have no key, no statue count, and no item check — unlike `DESERT` doors (5-statue gate, §15.5) or locked doors (key-of-type, §16.3). The gameplay progression gates that guard the Stargate are upstream:

- **Rose** (`stuff[23]`) to survive the fiery_death zone surrounding the Citadel approach (§18.9, §14.5).
- **Crystal Shard** (`stuff[30]`) to traverse the terrain-12 walls *inside* the Spirit Plane once the Stargate has already been crossed (§9.13, §14.5, §15.5).
- Reaching `doorlist[14]` physically (crossing lava on foot) implicitly requires Rose; there is no code check at the door itself.

**Post-traversal behavior:** all Spirit Plane behaviors are extent-driven by `find_place` (xtype 52), not by the door. The `xfer()` call performs a standard region reload with fade on enter and instant exit (§16.5–§16.6). On the Spirit Plane, the following hazards activate automatically via the astral extent:

- Astral music override (tracks 16–19) via `setmood`.
- Loraii-type encounters forced via encounter_type 8 (§12.7).
- Pit-fall trap: `j == 9 && i == 0 && xtype == 52` → `STATE_FALL`, `luck -= 2`, goodfairy revive to `(safe_x, safe_y)`.
- Quicksand drain (sector 181): teleport to `(0x1080, 34950)` in region 9 rather than killing (§16.7).
- Velocity-ice (terrain 7) and backwards-walk lava (terrain 8) environ effects within the astral extent.

The Stargate is the only door pair whose outdoor endpoint sits inside the fiery_death rectangle, and the only door pair whose indoor destination is region 9 via the `secs == 2` flag (§16.1).


