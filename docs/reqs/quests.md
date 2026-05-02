## 12. Quest Progression

### Requirements

| ID | Requirement |
|----|-------------|
| R-QUEST-001 | The main quest shall follow this critical path: rescue princess → obtain Writ from King → show Writ to Priest for gold statue → collect 5 golden statues total → enter hidden city of Azal → obtain Rose for lava immunity → cross lava to Citadel of Doom → traverse Spirit Plane (Crystal Shard required for terrain-12 barriers) → defeat Necromancer (Bow/Wand required) → pick up Talisman → victory. |
| R-QUEST-002 | Up to 3 princesses (Katra, Karla, Kandy) shall be rescuable, tracked by the `princess` counter (0, 1, 2). The counter persists across brother succession; each rescue shows a different princess's narrative text. After `princess >= 3`, no further rescues occur. |
| R-QUEST-003 | The rescue sequence shall be triggered by entering the princess extent zone (xtype 83, coordinates 10820–10877, 35646–35670) with `ob_list8[9].ob_stat != 0`. The `rescue()` function shall: (1) display rescue narrative via `placard_text(8 + princess*3)` through three princess-specific texts with `name()` interpolation; (2) display shared post-rescue text via `placard_text(17)` and `placard_text(18)` with 7.6-second pauses; (3) increment `princess`; (4) teleport hero to King's castle at (5511, 33780); (5) reposition bird extent via `move_extent(0, 22205, 21231)`; (6) place rescued princess NPC in castle (`ob_list8[2].ob_id = 4`); (7) grant Writ (`stuff[28] = 1`); (8) King speaks `speak(18)`; (9) reward 100 gold; (10) clear princess captive flag (`ob_list8[9].ob_stat = 0`); (11) grant +3 of each key type (`stuff[16..21] += 3`). |
| R-QUEST-004 | Quest state shall be tracked via object list `ob_stat` fields: `ob_list8[9].ob_stat` (princess captive, set to 3 by `revive(TRUE)`, cleared to 0 by `rescue()`), `ob_listg[9].ob_stat` (Sorceress statue given), `ob_listg[10].ob_stat` (Priest statue given), `ob_listg[5].ob_stat` (Spectre visibility: 3 if `lightlevel < 40`, else 2), `ob_listg[1-2].ob_stat` (dead brother bones, set to 1 on death), `ob_listg[3-4].ob_stat` (ghost brothers, set to 3 on death, cleared to 0 on bone pickup). Additional quest-relevant state: `stuff[22]` (Talisman), `stuff[25]` (gold statue count), `stuff[28]` (Writ), `stuff[29]` (Bone), `stuff[30]` (Crystal Shard), `princess` counter. |
| R-QUEST-005 | Hidden city access in region 4 (desert) shall be blocked when `stuff[25] < 5`: four map tiles at offset `(11×128)+26` shall be overwritten to impassable tile 254 on every region load. With `stuff[25] >= 5`, tiles remain passable. All 5 DESERT-type oasis doors shall also require `stuff[25] >= 5` to enter. |
| R-QUEST-006 | Win condition: when `stuff[22]` (Talisman) becomes nonzero after item pickup, set `quitflag = TRUE` and `viewstatus = 2`, then launch the victory sequence. |
| R-QUEST-007 | Victory sequence (`win_colors()`): (1) display `placard_text(6)` + `name()` + `placard_text(7)` with placard and 80-tick pause; (2) load victory image (`winpic`); (3) black out both viewports and hide HUD; (4) switch to Cinematic config (312×194, HUD hidden) — this triggers a one-frame `introcolors` fade per R-FX-007 which step 5 overrides; (5) 55-frame sunrise animation (i=25 to −29) using `sun_colors[53]` gradient — first frame pauses 60 ticks, subsequent frames 9 ticks each (total ≈11.1 seconds); (6) final 30-tick pause then fade to black. |
| R-QUEST-008 | 11 stone ring locations shall form a teleportation network; destination = `(current_stone + facing + 1) % 11`. |
| R-QUEST-009 | Stone ring activation requires: standing on sector 144, center-of-tile position check, match against `stone_list[]`. Visual effect: 32 frames of random palette cycling (`colorplay()`). |
| R-QUEST-010 | The princess captive flag (`ob_list8[9].ob_stat`) shall be reset to 3 during `revive(TRUE)` (brother succession), enabling each new brother to trigger one rescue with a different princess. `revive(FALSE)` (fairy rescue of same brother) shall NOT reset this flag. |
| R-QUEST-011 | Five golden figurines of Azal-Car-Ithil are required to access the desert. Sources: (1) Sorceress — first talk sets `ob_listg[9].ob_stat = 1`; (2) Priest — talk with Writ sets `ob_listg[10].ob_stat = 1`; (3) Seahold ground pickup at `ob_listg[6]`, (11092, 38526); (4) Ogre Den ground pickup at `ob_listg[7]`, (25737, 10662); (5) Octal Room ground pickup at `ob_listg[8]`, (2910, 39023). Dialogue-revealed statues work through standard Take: setting `ob_stat = 1` makes the object visible for `itrans` pickup. |
| R-QUEST-012 | The Necromancer (race 9, 50 HP, wand weapon) is the final boss. Invulnerable to weapons with `weapon < 4` (only Bow or Wand deal damage). Magic is blocked in the arena (`extn->v3 == 9`). On death: transforms to Woodcutter (race 10, vitality 10, weapon 0), drops Talisman (object 139 → `stuff[22]`). |
| R-QUEST-013 | The Rose (`stuff[23]`, `ob_list8[51]`) shall grant lava immunity: when hero is in the `fiery_death` area, `stuff[23]` resets environmental damage to 0 (`environ = 0`). Required for reaching the Citadel of Doom which sits inside the volcanic lava zone. |
| R-QUEST-014 | The Crystal Shard (`stuff[30]`) shall enable the hero to walk through terrain type 12 barriers (`stuff[30] && j==12` bypasses terrain collision). Required for navigating the Spirit Plane maze to reach the Necromancer's arena. |
| R-QUEST-015 | The Sun Stone (`stuff[7]`, `ob_list8[18]`, located in the Elf Glade behind door 48) shall make the Witch vulnerable to all weapons. Without it, only Bow/Wand can damage the Witch. Defeating the Witch drops the Golden Lasso. |
| R-QUEST-016 | The Golden Lasso (`stuff[5]`) shall be required to mount the Swan carrier (bird, `actor_file == 11`). Without the lasso, the bird cannot be ridden. The swan enables unrestricted flight over all terrain types. |
| R-QUEST-017 | The Sea Shell (`stuff[6]`) shall be obtained by talking to the Turtle carrier after finding turtle eggs at the extent zone (22945–23225, 5597–5747). USEing the shell anywhere summons the turtle for ocean travel. |
| R-QUEST-018 | Game over shall occur when all three brothers have permanently died (`brother > 3`): `placard_text(5)` ("Stay at Home!"), `Delay(500)` (10 seconds), `quitflag = TRUE`. |
| R-QUEST-019 | On brother succession (`revive(TRUE)`): the `princess` counter and all quest flags (`ob_listg`, `ob_list8` entries) persist. Stats are loaded fresh from `blist[]`, inventory is cleared (hero gets only a Dirk), position resets to Tambry (19036, 15755), and hunger/fatigue reset to 0. Dead brother's bones and ghost are placed in the world. The princess captive flag resets to 3. |
| R-QUEST-020 | When a living brother picks up a dead brother's bones (ob_id 28), both ghost setfigs shall be removed (`ob_listg[3].ob_stat = ob_listg[4].ob_stat = 0`) and the dead brother's entire 31-slot inventory shall be merged into the current brother's inventory. |
| R-QUEST-021 | The DreamKnight (race 7, 40 HP, sword) shall guard the Hidden Valley (extent index 15), blocking access to the Elf Glade (door 48) where the Sun Stone is located. It stands still facing south, fights when the player comes within 16px, never flees, and respawns on every extent entry. On death: `speak(42)` and `brave++`. |
| R-QUEST-022 | Pax zones shall enforce weapon restrictions: King's castle grounds (`xtype == 81`) trigger `event(15)` (weapon sheathed); Sorceress area (`xtype == 82`) trigger `event(16)` (calming influence). |
| R-QUEST-023 | The Spectre (`ob_listg[5]`) shall only be visible when `lightlevel < 40` (nighttime): `ob_stat` set to 3 at night, 2 during day. The hero must visit the Spectre at night to trade the King's Bone for the Crystal Shard. |
| R-QUEST-024 | The King's Bone (`stuff[29]`, `ob_list9[8]`) shall be found in the underground dungeon. It is required for the Spectre trade to obtain the Crystal Shard. |

### User Stories

- As a player, I can rescue captive princesses and receive rewards from the King including a Writ, gold, and keys.
- As a player, I need to collect 5 golden statues from various sources to unlock the hidden city of Azal.
- As a player, I can use stone rings to teleport across the world based on the direction I face.
- As a player, defeating the Necromancer and obtaining the Talisman wins the game with a victory cinematic.
- As a player, I must defeat the DreamKnight to access the Sun Stone, use the Sun Stone to kill the Witch, use the Lasso to ride the Swan, and collect the Rose to survive lava.
- As a player, I can recover my dead brother's inventory by finding his bones in the world.
- As a player, if all three brothers perish, the game ends with a game-over message.
- As a player, each new brother can trigger a new princess rescue, advancing through Katra, Karla, and Kandy.
- As a player, I must visit the Spectre at night and trade the King's Bone for the Crystal Shard to navigate the Spirit Plane.
- As a player, the Necromancer can only be damaged by a ranged weapon (Bow or Wand) — melee strikes are deflected, and magic is suppressed inside his arena.
- As a player, once I have the Rose I can cross the lava surrounding the Citadel of Doom; without it, standing in the fiery_death zone drains my vitality each tick and deep exposure kills me instantly.

---


