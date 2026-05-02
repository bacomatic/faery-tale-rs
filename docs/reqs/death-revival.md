## 17. Death & Revival

### Requirements

| ID | Requirement |
|----|-------------|
| R-DEATH-001 | When any actor's vitality < 1 and state is not DYING/DEAD: set vitality = 0, tactic = 7, goal = DEATH, state = DYING. Death types: 5 = combat, 6 = drowning, 27 = lava. |
| R-DEATH-002 | Hero death effects: display death event message (by death type), `luck -= 5`, `setmood(TRUE)` (death music). |
| R-DEATH-003 | NPC kill effects: `brave++` for the attacker. If killed NPC is a SETFIG and not witch (0x89): `kind -= 3` (clamped ≥ 0). If DreamKnight (race 7): `speak(42)`. |
| R-DEATH-004 | DYING → DEAD transition shall occur when `tactic` counts down to 0 during the death animation. |
| R-DEATH-005 | `goodfairy` shall be a u8 countdown from 255 after hero enters DEAD or FALL state. The death animation and death song always play fully (frames 2–57) before any rescue decision. |
| R-DEATH-006 | Fairy rescue luck gate at `goodfairy` range 199–120: if `luck < 1` → brother succession (`revive(true)`). If FALL state → fairy recovery (`revive(false)`) regardless of luck. If `luck >= 1` and DEAD → fairy rescue proceeds. This gate is fully deterministic with no random element. |
| R-DEATH-007 | Luck cannot change during DEAD state: `checkdead` is guarded against DYING/DEAD states, pit fall requires movement, sorceress requires TALK. If luck ≥ 1 when the gate fires, fairy rescue is guaranteed. |
| R-DEATH-008 | Fairy animation at `goodfairy` 119–20 (fairy sprite approaches hero, `battleflag = FALSE`, AI suspended), resurrection glow at 19–2, revival `revive(false)` at `goodfairy == 1`. |
| R-DEATH-009 | Brother succession (`revive(true)`): place ghost at death location (brothers 1–2 only), reset `ob_list8[9].ob_stat = 3` (princess captive), load next brother stats from `blist[]`, clear inventory (zero 31 slots, give single Dirk), reset all timers to 0, spawn at Tambry (19036, 15755) in region 3, display brother-specific placard, load brother sprites, display journey message. |
| R-DEATH-010 | Fairy revival (`revive(false)`): teleport to last safe zone (`safe_x, safe_y`), full HP (`15 + brave / 4`), clear hunger/fatigue to 0, set `daynight = 8000`, `lightlevel = 300`. Skips ghost placement, stat/inventory reset, and placard text. |
| R-DEATH-011 | Brother base stats: Julian (brave=35, luck=20, kind=15, wealth=20, HP=23), Phillip (brave=20, luck=35, kind=15, wealth=15, HP=20), Kevin (brave=15, luck=20, kind=35, wealth=10, HP=18). Each brother has an independent inventory array (35 active slots + 1 ARROWBASE accumulator = 36 bytes in memory; 35-byte serialized payload — see R-INV-001, R-SAVE-003). |
| R-DEATH-012 | Max fairy rescues per brother (from initial luck / 5): Julian = 3, Phillip = 6, Kevin = 3. |
| R-DEATH-013 | Succession placard text: Julian → placard(0); Phillip → placard(1) + placard(2) ("Julian's luck ran out…"); Kevin → placard(3) + placard(4) ("Phillip's cleverness could not save him…"). Journey start: event(9), plus event(10) for Phillip or event(11) for Kevin. |
| R-DEATH-014 | Dead brother ghost: bones at death location (`ob_listg[brother]`), ghost setfig activated (`ob_listg[brother + 2].ob_stat = 3`). Only for brothers 1 and 2 (Kevin has no successor). |
| R-DEATH-015 | Bones pickup (ob_id 28): clear both ghost setfigs (`ob_listg[3].ob_stat = ob_listg[4].ob_stat = 0`), merge dead brother's 31-slot inventory into current brother. Ghost dialogue before pickup: `speak(49)`. |
| R-DEATH-016 | Game over when `brother > 3`: placard(5) "And so ends our sad tale. The Lesson of the Story: Stay at Home!", 10-second pause, `quitflag = TRUE`. |
| R-DEATH-017 | Persistence across succession: princess counter, quest flags, world object state, `dstobs[]` persist. Stats, inventory, position, hunger/fatigue, timers, daynight all reset. Princess captive flag (`ob_list8[9].ob_stat`) resets to 3 enabling each brother to trigger a rescue. |

### User Stories

- As a player, when I die, a fairy may revive me depending on my luck stat.
- As a player, if too unlucky, my next brother takes over the quest from the village.
- As a player, if all three brothers die, the game ends with a "Stay at Home" message.
- As a player, killing innocent NPCs reduces my kindness stat.
- As a player, I can find a dead brother's bones and recover their inventory.
- As a player, each brother starts fresh with different strengths but the quest state is preserved.
- As a player, the fairy rescue is deterministic — if I have luck remaining, the fairy always saves me.

---


