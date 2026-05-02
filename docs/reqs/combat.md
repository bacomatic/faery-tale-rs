## 8. Combat

### Requirements

| ID | Requirement |
|----|-------------|
| R-COMBAT-001 | Melee hit detection shall compute a strike point extending `weapon_code * 2` pixels in the attacker's facing direction, with ±3 to ±4 pixel random jitter per axis (`rand8() − 3`). |
| R-COMBAT-002 | Player melee reach (`bv`) shall be `(brave / 20) + 5`, capped at 15. Monster melee reach shall be `2 + rand4()` (2–5), re-rolled each frame. |
| R-COMBAT-003 | Target matching shall use Chebyshev distance (max of `|dx|`, `|dy|`) from strike point to target. A hit requires: distance < `bv`, `freeze_timer == 0`, and for monster attackers only, `rand256() > brave` must pass. Player attacks always hit if in range. |
| R-COMBAT-004 | Melee damage formula: `wt + bitrand(2)` where `wt` is the weapon code. Touch attack (code 8) clamps `wt` to 5 before damage. Vitality floors at 0. |
| R-COMBAT-005 | Necromancer (race 9) shall be immune to weapons with code < 4 (melee only); message `speak(58)` on blocked hit. Witch (race 0x89) shall be immune to weapons < 4 unless Sun Stone (`stuff[7]`) is held. Spectre (race 0x8a) and Ghost (race 0x8b) shall be completely immune to all damage with no feedback. |
| R-COMBAT-006 | Knockback: defender pushed 2 pixels in attacker's facing direction via `move_figure(j, fc, 2)`. If knockback succeeds and attacker is melee (`i >= 0`), attacker slides 2 pixels forward (follow-through). DRAGON and SETFIG types are immune to knockback. |
| R-COMBAT-007 | 6 missile slots shall support arrows and fireballs. Arrow hit radius = 6 pixels; fireball hit radius = 9 pixels. Missile damage = `rand8() + 4` (4–11) for both types. |
| R-COMBAT-008 | Missile dodge: for player target, `bv = brave`; for monsters, `bv = 20`. Only missile slot 0 applies the dodge check `bitrand(512) > bv`; slots 1–5 always hit if in range. `dohit` attacker code = −1 for arrows, −2 for fireballs. |
| R-COMBAT-009 | Bow attacks require SHOOT1 (aiming) → SHOOT3 (release) animation states and arrow inventory. |
| R-COMBAT-010 | Dragon shall have 25% chance per frame (`rand4() == 0`) of launching a fireball at the hero. Witch shall deal `rand2() + 1` (1–2) damage when `witchflag` is set and distance < 100. |
| R-COMBAT-011 | The 9-state `trans_list[]` fight animation shall cycle through states 0→1→2→3→4→5→6→8→0 via `newstate[0]`. Each tick selects a random transition: `trans_list[state].newstate[rand4()]`. Monsters at states 6 or 7 are forced to state 8. |
| R-COMBAT-012 | Weapon types: 0=none, 1=Dirk, 2=mace, 3=sword, 4=bow, 5=wand, 8=touch (monster-only). Damage equals weapon code; touch clamps to 5. Strike range = `weapon_code * 2` pixels. |
| R-COMBAT-013 | Near-miss sound shall play when Chebyshev distance < `bv + 2` and weapon ≠ wand: `effect(1, 150 + rand256())`. |
| R-COMBAT-014 | `checkdead(i, dtype)` shall trigger when vitality < 1 and state is not DYING or DEAD. Sets `goal=DEATH`, `state=DYING`, `tactic=7`. DKnight death triggers `speak(42)`. SETFIG (non-witch) death causes `kind −= 3`. Enemy death (i > 0) grants `brave++`. Player death (i == 0) triggers `event(dtype)`, `luck −= 5`, `setmood(TRUE)`. |
| R-COMBAT-015 | Death animation: `tactic` counts down from 7 to 0 (7 frames), sprites 80/81 alternating. At tactic 0, state transitions to DEAD with sprite index 82. |
| R-COMBAT-016 | Body search ("Get" action near dead body): weapon drop = monster's weapon code (1–5); if better than current, auto-equips. Bow drops also give `rand8() + 2` (2–9) arrows. Treasure from `treasure_probs[encounter_chart[race].treasure * 8 + rand8()]`. SetFig races (`race & 0x80`) yield no treasure. |
| R-COMBAT-017 | `aftermath()` fires when `battleflag` transitions from TRUE to FALSE. It counts dead and fleeing enemies for status messages but does not directly grant experience or loot. |
| R-COMBAT-018 | Bravery serves as both passive experience and active combat stat: melee reach = `(brave/20)+5` (max 15), monster dodge = `rand256() > brave`, missile dodge (slot 0) = `bitrand(512) > brave`, starting vitality = `15 + brave/4`, growth = +1 per kill. |
| R-COMBAT-019 | Luck decreases by 5 per player death and by 2 per ledge fall. When luck < 1 at the death countdown gate, the next death is permanent (no fairy rescue). |
| R-COMBAT-020 | Goodfairy countdown: 255→200 death sequence; 200→120 luck gate (luck < 1 → `revive(TRUE)` brother succession, FALL → `revive(FALSE)` non-lethal); 120→20 fairy sprite flies toward hero; 20→2 resurrection glow; 1 → `revive(FALSE)` fairy rescue. |
| R-COMBAT-021 | `revive(TRUE)` (new brother): brother increments (1→Julian, 2→Phillip, 3→Kevin, 4+→game over). Stats reset from `blist[]`. Inventory wiped for indices 0 to GOLDBASE−1. Starting weapon = Dirk. Vitality = `15 + brave/4`. Dead brother's body and ghost placed in world. |
| R-COMBAT-022 | `revive(FALSE)` (fairy rescue): no stat changes. Returns to last safe position (`safe_x`, `safe_y`). Vitality restored to `15 + brave/4`. |
| R-COMBAT-023 | Necromancer on death: transforms to Woodcutter (race 10, vitality 10) and drops Talisman (object 139). Witch on death: drops Golden Lasso (object 27). |

### User Stories

- As a player, I can fight enemies in melee and see damage applied based on my weapon code and random variation.
- As a player, I can use a bow to shoot arrows and a wand to shoot fireballs at enemies from a distance.
- As a player, I find weapons and treasure dropped by defeated enemies via body search.
- As a player, my bravery grows with each kill, making me progressively stronger in combat.
- As a player, if I die with sufficient luck, a fairy revives me; otherwise the next brother takes over.
- As a player, I must use ranged weapons or the Sun Stone to damage the Necromancer and Witch respectively.

---


