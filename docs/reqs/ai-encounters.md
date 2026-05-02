## 9. AI & Encounters

### Requirements

| ID | Requirement |
|----|-------------|
| R-AI-001 | 11 goal modes shall control high-level NPC behavior: USER (0), ATTACK1 (1), ATTACK2 (2), ARCHER1 (3), ARCHER2 (4), FLEE (5), STAND (6), DEATH (7), WAIT (8), FOLLOWER (9), CONFUSED (10). |
| R-AI-002 | 13 tactical modes shall control per-tick NPC actions: FRUST (0), PURSUE (1), FOLLOW (2), BUMBLE_SEEK (3), RANDOM (4), BACKUP (5), EVADE (6), HIDE (7, planned but unimplemented), SHOOT (8), SHOOTFRUST (9), EGG_SEEK (10), DOOR_SEEK (11, vestigial), DOOR_LET (12, vestigial). |
| R-AI-003 | AI processing shall be suppressed when `goodfairy > 0 && goodfairy < 120` (fairy animation active). |
| R-AI-004 | The `set_course(object, target_x, target_y, mode)` algorithm shall compute movement direction using 7 modes: 0 (toward with snapping), 1 (toward + deviation at distance < 40), 2 (toward + deviation at distance < 30), 3 (away/reverse), 4 (toward without snapping), 5 (toward with snapping, no walk state), 6 (raw direction vector). |
| R-AI-005 | Directional snapping (modes ≠ 4): if one axis dominates (`(|major| >> 1) > |minor|`), the minor axis is zeroed. Mode 4 skips snapping, always allowing diagonal. |
| R-AI-006 | Random deviation in `set_course`: when deviation > 0, `rand() & 2` (bit 1) determines ±deviation, result wrapped with `& 7`. |
| R-AI-007 | Goal mode assignment at spawn: ranged weapon (code & 4) → ARCHER1/ARCHER2; melee → ATTACK1/ATTACK2. Cleverness field selects +0 or +1. |
| R-AI-008 | Runtime goal transitions: hero dead/falling with no leader → FLEE; with leader → FOLLOWER. Vitality < 2 → FLEE. Special extent mismatch (xtype > 59, race ≠ v3) → FLEE. Unarmed (weapon < 1) → CONFUSED. |
| R-AI-009 | All tactical movement shall be rate-limited: base 12.5% (`!(rand() & 7)`), upgraded to 25% (`!(rand() & 3)`) for ATTACK2 goal. When the gate fails, the actor continues its previous trajectory. |
| R-AI-010 | `do_tactic()` dispatch: PURSUE → `set_course` mode 0 to hero; FOLLOW → mode 0 to leader (+20 Y); BUMBLE_SEEK → mode 4 to hero; BACKUP → mode 3 from hero; EVADE → mode 2 to neighboring actor (+20 Y); SHOOT → mode 0 or 5 to hero (every tick, no rate limit); RANDOM → `facing = rand() & 7` directly; EGG_SEEK → mode 0 to fixed coords (23087, 5667). |
| R-AI-011 | AI loop processes actors 2 through `anix−1`. CARRIER type: face player every 16 ticks via `set_course(i, hero_x, hero_y, 5)`. SETFIG type: skipped entirely. |
| R-AI-012 | Battle detection: actors within 300×300 pixels of hero set `actors_on_screen = TRUE` and `battleflag = TRUE`. |
| R-AI-013 | Random reconsider probability: base `!bitrand(15)` = 1/16 (6.25%). For goals where `(mode & 2) == 0` (ATTACK1, ARCHER2): overridden to `!rand4()` = 25%. |
| R-AI-014 | Hostile AI tactic selection on reconsider: snake + turtle_eggs → EGG_SEEK; unarmed → RANDOM (mode→CONFUSED); vitality < 6 && rand2() → EVADE; archer near (xd<40, yd<30) → BACKUP; archer mid (xd<70, yd<70) → SHOOT; archer far → PURSUE; melee default → PURSUE. |
| R-AI-015 | Melee engagement threshold: `thresh = 14 − mode`. DKnight (race 7) overrides to 16. Within threshold (xd < thresh AND yd < thresh), enemy enters FIGHTING state. |
| R-AI-016 | Frustration cycle (NPC-only): when an NPC's movement is blocked in all three probe directions, `an->tactic` shall be set to FRUST. On the next AI tick, the frustration handler shall pick a random escape tactic keyed on weapon type: ranged (`weapon & 4` set) → `rand4() + 2` ∈ {FOLLOW, BUMBLE_SEEK, RANDOM, BACKUP}; melee (`weapon & 4` clear) → `rand2() + 3` ∈ {BUMBLE_SEEK, RANDOM}. The handler fires before goal-mode dispatch and can override FLEE, FOLLOWER, and CONFUSED (original quirk — preserve). SHOOTFRUST (tactic 9) is dead code in the original (never assigned); the tactic constant and the handler branch shall remain for fidelity. `set_encounter()` does not initialize `an->tactic`, a minor cosmetic bug that MAY be fixed by initializing to PURSUE on spawn. NPCs shall NOT use the player's `frustflag` animation system (R-INPUT-015). |
| R-AI-017 | Cleverness 0 (stupid): ATTACK1/ARCHER1 goal, 12.5% tactic rate (ATTACK1 reconsiders at 25%). Cleverness 1 (clever): ATTACK2/ARCHER2 goal, 25% tactic rate for ATTACK2 (reconsiders at 6.25%). ATTACK2 creates persistent, aggressive behavior — commits to tactics and executes energetically. |
| R-AI-018 | CONFUSED mode: assigned when hostile actor loses weapon. First tick runs `do_tactic(i, RANDOM)`. Subsequent ticks: no AI processing occurs — actor walks in last random direction until blocked. |
| R-AI-019 | DKnight special behavior: out of melee range → `state = STILL`, `facing = 5` (south), does not pursue or call `do_tactic()`. In range → FIGHTING state. Never flees (exempt from flee when race matches extent v3 for xtype > 59 zones). Respawns every time player re-enters hidden valley. Fixed position at (21635, 25762). |
| R-AI-020 | `leader` shall be set to the first living active enemy at end of AI loop. FOLLOWER goal uses this to follow the pack leader. |
| R-AI-021 | 23 extent zones (22 + sentinel) shall define encounter regions. `find_place()` performs linear scan of entries 0–21 each frame (first match wins). Entry 22 (whole world, etype=3) is the sentinel fallback. |
| R-AI-022 | Extent type categories: 0–49 = random encounter zone (etype used as `xtype`); 50–59 = forced group encounter (monsters spawn immediately, v1=count, v3=type); 52 = astral plane (forces encounter_type=8, Loraii); 60–61 = special figure (unique NPC at extent center); 70 = carrier (load bird/turtle/dragon via `load_carrier(v3)`); 80 = peace zone; 81 = king peace (weapon draw → `event(15)`); 82 = sorceress peace (weapon draw → `event(16)`); 83 = princess rescue. |
| R-AI-023 | Encounter placement shall occur every 16 frames: up to 10 random locations tried via `set_loc()` (150–213 pixels from hero), each validated for terrain type 0 (walkable). Dead enemy slots recycled when all 4 slots full. |
| R-AI-024 | Danger check every 32 frames. Conditions: no actors on screen, no pending load, no active carrier, `xtype < 50`. Danger level: indoor = `5 + xtype`, outdoor = `2 + xtype`. Spawn probability = `rand64() <= danger_level`, i.e., `(danger_level + 1) / 64`. |
| R-AI-025 | Monster type selection: base = `rand4()` (0–3 → ogre/orc/wraith/skeleton). Overrides: swamp (xtype=7) remaps wraith roll to snake (4); spider region (xtype=8) forces spider (6); xtype=49 forces wraith (2). |
| R-AI-026 | Monster count per encounter: `encounter_number = v1 + rnd(v2)`. Only 4 enemy actor slots (indices 3–6) exist; excess resolves over time as dead slots are recycled. |
| R-AI-027 | Race mixing: when `mixflag & 2` (and encounter_type ≠ snake), `race = (encounter_type & 0xFFFE) + rand2()`, allowing adjacent types to mix (ogre↔orc, wraith↔skeleton). `mixflag` disabled for xtype > 49 or xtype divisible by 4. |
| R-AI-028 | Weapon selection at spawn: `weapon_probs[arms * 4 + wt]` where `wt` is re-randomized per enemy if `mixflag & 4`, otherwise shared within batch. |
| R-AI-029 | Peace zones (etype 80–83) set `xtype ≥ 50`, failing the `xtype < 50` guard on danger checks, completely suppressing random encounters. The `aggressive` field in `encounter_chart[]` is never read at runtime. |
| R-AI-030 | Only extents 0 (bird) and 1 (turtle) are persisted in savegames. Turtle extent starts at (0,0,0,0) and must be repositioned via `move_extent()`. |
| R-AI-031 | Spirit Plane (astral) hazards shall activate automatically while the hero is inside an extent with `etype == 52`, independent of any door/portal entry logic. Required behaviors: (a) music override to astral tracks 16–19 via `setmood`; (b) forced `encounter_type = 8` (Loraii) with synchronous actor-shape load; (c) pit-fall trap — terrain probe returning `j == 9 && i == 0` with `xtype == 52` sets `STATE_FALL`, `luck -= 2`, and routes recovery through `goodfairy` → `revive(FALSE)` back to `(safe_x, safe_y)`; (d) quicksand drain at `hero_sector == 181` teleports the hero to `(0x1080, 34950)` in region 9 via `xfer` instead of killing (NPCs die outright); (e) velocity-ice (terrain 7) accepts monster placement even when `proxcheck` fails; (f) backwards-walk lava (terrain 8) environ `k = −3` reverses hero motion. Exiting the extent box fires `find_place` with a new `xtype` and `carrier_extent_update` zeroes `active_carrier`. |

### User Stories

- As a player, I encounter enemies randomly as I explore the world, with difficulty varying by region and danger level.
- As a player, I experience enemies that pursue, attack, flee, evade, and wander with distinct behaviors driven by their cleverness and goal mode.
- As a player, I find safe areas (towns, temples, king's domain) where no random enemies spawn and weapon draw may be blocked.
- As a player, I face the Dark Knight as a fixed-position guardian that blocks passage, fights only at close range, and respawns on re-entry.
- As a player, I encounter carriers (swan, turtle, dragon) in specific world zones that I can ride.
- As a player, enemy groups may contain mixed types (ogre/orc, wraith/skeleton) for variety.

---


