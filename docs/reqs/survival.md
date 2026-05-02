## 15. Survival (Hunger, Fatigue, Health)

### Requirements

| ID | Requirement |
|----|-------------|
| R-SURV-001 | Hunger shall increment by 1 every 128 game ticks (`(daynight & 127) == 0`) while alive and not sleeping. |
| R-SURV-002 | Hunger warnings: event(0) "getting rather hungry" at hunger == 35, event(1) "getting very hungry" at hunger == 60. Starvation warning event(2) "starving!" when hunger > 90 and `(hunger & 7) == 0`. |
| R-SURV-003 | Vitality −2 when `(hunger & 7) == 0` and (`hunger > 100` OR `fatigue > 160`), only when `vitality > 5`. |
| R-SURV-004 | Collapse at hunger > 140: event(24) "passed out!", hunger reset to 130, forced `state = SLEEP`. |
| R-SURV-005 | Fatigue shall increment by 1 on the same 128-tick timer as hunger, same conditions. |
| R-SURV-006 | Fatigue warnings: event(3) "getting tired" at fatigue == 70, event(4) "getting sleepy" at fatigue == 90. |
| R-SURV-007 | Forced fatigue sleep: event(12) when fatigue > 170, only when `vitality ≤ 5`. |
| R-SURV-008 | Health regeneration: +1 vitality every 1024 ticks (`(daynight & 0x3FF) == 0`), up to max vitality = `15 + brave / 4`. During sleep, `daynight` advances 64× faster, so healing occurs ≈63× faster. |
| R-SURV-009 | Voluntary sleep: stand on bed tile (IDs 161, 52, 162, 53) in region 8. `sleepwait` increments each tick; after 30 ticks: fatigue < 50 → event(25) "not sleepy"; fatigue ≥ 50 → event(26) "decided to lie down", `state = SLEEP`. |
| R-SURV-010 | Sleep processing: `daynight += 63` per frame, `fatigue--` per frame (if > 0). Wake conditions (any): fatigue == 0, OR (fatigue < 30 AND daynight ∈ [9000, 10000)), OR (`battleflag` AND `rand64() == 0`). On wake: `state = STILL`, Y-position snapped to grid. |
| R-SURV-011 | Safe zone detection: updated every 128 ticks when no enemies visible/loading, no witch encounter, `environ == 0`, no danger flag, hero alive. |
| R-SURV-012 | Auto-eat: in safe zone when `(daynight & 127) == 0`, if hunger > 30 and `stuff[24] > 0` (Fruit), consume one Fruit: `stuff[24]--; hunger -= 30; event(37)`. Direct subtraction, not via `eat()`. |
| R-SURV-013 | Hunger > 120 movement wobble: direction shifted ±1 with 75% probability (`rand4() != 0` selects wobble; `rand2()` selects ±1 direction). |
| R-SURV-014 | `eat(amt)` function: `hunger -= amt`; if hunger < 0, set to 0 and event(13) "full"; otherwise print "Yum!". Pickup fruit (hungry): `eat(30)`. Buy food from shop: `eat(50)`. |
| R-SURV-015 | Fruit pickup when hunger < 15: fruit stored in inventory (`stuff[24]++; event(36)`) rather than eaten. |
| R-SURV-016 | Drowning damage (`environ == 30`): −1 vitality every 8 ticks. Wraiths (race 2) and skeletons (race 3) shall be immune to drowning damage (the damage check at `fmain.c:1849-1851` is race-gated). Wraiths and snakes (race 4) additionally have terrain forced to 0, so they never enter drowning environ in the first place. |
| R-SURV-017 | Lava damage zone (`8802 < map_x < 13562`, `24744 < map_y < 29544`): `environ > 2` → −1 vitality per tick; `environ > 15` → instant death. Rose (`stuff[23]`) prevents lava damage by forcing `environ = 0`. |
| R-SURV-018 | Heal vial: `vitality += rand8() + 4` (4–11 HP), capped at `15 + brave / 4`. |
| R-SURV-019 | Priest healing: full heal to `15 + brave / 4`. Requires `kind >= 10`; below 10, priest gives dismissive dialogue. |
| R-SURV-020 | Bravery: +1 per enemy kill, −1 per Jade Skull kill. Affects combat: melee hit range = `brave / 20 + 5`, missile bravery = full `brave`, enemy dodge check = `rand256() > brave`, max HP = `15 + brave / 4`. Initial: Julian = 35, Phillip = 20, Kevin = 15. |
| R-SURV-021 | Luck: −5 on hero death, −2 on pit fall. Probabilistic +5 from sorceress: `if (luck < rand64()) luck += 5`. Clamped ≥ 0 on HUD redraw. Luck < 1 after death triggers brother succession instead of fairy rescue. Initial: Julian = 20, Phillip = 35, Kevin = 20. |
| R-SURV-022 | Kindness: −3 for killing non-witch SETFIGs, clamped ≥ 0. Probabilistic +1 from giving gold: `if (rand64() > kind) kind++`. Below 10, wizards and priests give dismissive dialogue. Initial: Julian = 15, Phillip = 15, Kevin = 35. |
| R-SURV-023 | Wealth: +50 from gold bags, +100 from containers, +100 from princess rescue, +variable from corpse loot. −price for shop purchases, −2 for giving gold. Initial: Julian = 20, Phillip = 15, Kevin = 10. |
| R-SURV-024 | HUD shall display Brv, Lck, Knd, Wlth (via prq(7)) and Vit (via prq(4)). Hunger and fatigue are NOT displayed on the HUD — communicated only through event messages. |
| R-SURV-025 | All random values shall be produced by a single 32-bit LCG: `seed = low16(seed) × 45821 + 1; output = ror32(seed, 6) & 0x7FFFFFFF`. The 68000 `mulu.w` operates on the low 16 bits only, giving an effective state space of 2¹⁶ and a maximum period of 65536. |
| R-SURV-026 | The LCG shall be seeded once at startup with `0x012ED98D` (19837325). There shall be no runtime reseeding from wall-clock, VBlank counter, or input. Sequence variation between sessions comes solely from per-keystroke discarded `rand()` calls during the copy-protection loop. |
| R-SURV-027 | Helper RNG functions shall match the originals exactly: `rand2() = rand() & 1`, `rand4() = rand() & 3`, `rand8() = rand() & 7`, `rand64() = rand() & 63`, `rand256() = rand() & 255`, `bitrand(x) = rand() & x`, `rnd(n) = (rand() & 0xFFFF) % n` (true modulo via 16-bit division). Results are uniform only for power-of-two-minus-one masks. |

### User Stories

- As a player, I must manage hunger and fatigue to avoid collapsing.
- As a player, I can sleep in beds to recover fatigue.
- As a player, I am warned progressively as hunger and fatigue increase.
- As a player, my health regenerates slowly over time, faster while sleeping.
- As a player, extreme hunger causes my character to stumble while walking.
- As a player, I can eat fruit to reduce hunger, and my character auto-eats in safe zones.
- As a player, my stats (bravery, luck, kindness, wealth) change through gameplay actions.
- As a player, hunger and fatigue are communicated through text messages, not HUD numbers.
- As a player, walking into the lava around the Citadel of Doom burns me — deeper exposure kills outright, but the Rose grants full immunity.

---


