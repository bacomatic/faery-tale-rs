# Logic Documentation — Index

This directory contains strict, linter-backed pseudo-code specifications for every non-trivial branching function in *The Faery Tale Adventure*. Combined with [ARCHITECTURE.md](../ARCHITECTURE.md), [RESEARCH.md](../RESEARCH.md), [STORYLINE.md](../STORYLINE.md), and the spatial/quest JSON databases, these docs are sufficient to reproduce the game's behavior without reading the 1987 source.

**Fidelity target:** behavioral. Same inputs produce the same observable gameplay. Implementation primitives (RNG algorithm, integer widths when not observable, fixed-point layout) are left to the porter. See the [design spec](../superpowers/specs/2026-04-20-logic-docs-design.md) for the full rationale.

**Normative references:**
- [STYLE.md](STYLE.md) — pseudo-code grammar.
- [SYMBOLS.md](SYMBOLS.md) — global symbol registry.

**Lint:**
```bash
tools/run.sh lint_logic.py
```

---

## Reading Order (for porters)

1. [STYLE.md](STYLE.md) — learn the grammar.
2. [SYMBOLS.md](SYMBOLS.md) — skim the registry.
3. `game-loop.md` *(Wave 2)* — the canonical per-frame sequence.
4. Subsystem docs in order of gameplay centrality (Wave 3+): combat → movement → encounters → quests → npc-dialogue → save-load → shops → brother-succession → visual-effects.

---

## Function Index

Every documented function appears here with a link to its canonical definition. The linter verifies completeness in both directions.

| Function | File | Purpose |
|---|---|---|
| `option_handler` | [menu-system.md#option_handler](menu-system.md#option_handler) | Dispatch one menu-slot mouse event on `enabled[hit]` action type |
| `key_dispatch` | [menu-system.md#key_dispatch](menu-system.md#key_dispatch) | Route one code from the input ring buffer to the appropriate handler |
| `advance_goal` | [ai-system.md#advance_goal](ai-system.md#advance_goal) | Per-tick NPC goal FSM |
| `game_tick` | [game-loop.md#game_tick](game-loop.md#game_tick) | Canonical 24-phase per-frame sequence |
| `process_input_key` | [game-loop.md#process_input_key](game-loop.md#process_input_key) | Phase 2: key dispatch + viewstatus 1/2/4 handling |
| `resolve_player_state` | [game-loop.md#resolve_player_state](game-loop.md#resolve_player_state) | Phase 7: player motion state from input |
| `actor_tick` | [game-loop.md#actor_tick](game-loop.md#actor_tick) | Phase 9: per-actor type/state dispatch |
| `check_door` | [game-loop.md#check_door](game-loop.md#check_door) | Phase 12: door straddle + transfer |
| `redraw_or_scroll` | [game-loop.md#redraw_or_scroll](game-loop.md#redraw_or_scroll) | Phase 13: full redraw vs scroll vs no-motion |
| `no_motion_tick` | [game-loop.md#no_motion_tick](game-loop.md#no_motion_tick) | Phase 14: periodic game logic when map is stationary |
| `melee_hit_detection` | [game-loop.md#melee_hit_detection](game-loop.md#melee_hit_detection) | Phase 15: weapon-reach proximity → dohit |
| `missile_tick` | [game-loop.md#missile_tick](game-loop.md#missile_tick) | Phase 16: age + advance in-flight missiles |
| `sort_sprites` | [game-loop.md#sort_sprites](game-loop.md#sort_sprites) | Phase 19: bubble-sort anim_index by Y + nearest-person |
| `render_sprites` | [game-loop.md#render_sprites](game-loop.md#render_sprites) | Phase 22: body + weapon blits with terrain mask |
| `dohit` | [combat.md#dohit](combat.md#dohit) | Apply damage with immunity / SFX / knockback / death roll |
| `melee_swing` | [combat.md#melee_swing](combat.md#melee_swing) | Per-attacker body: reach, Chebyshev target match, near-miss |
| `missile_step` | [combat.md#missile_step](combat.md#missile_step) | Per-missile body: age, terrain, victim scan, advance |
| `checkdead` | [combat.md#checkdead](combat.md#checkdead) | Vitality → STATE_DYING transition + stat bookkeeping |
| `aftermath` | [combat.md#aftermath](combat.md#aftermath) | Post-battle recap: dead vs fled tally, turtle-egg trigger |
| `move_figure` | [combat.md#move_figure](combat.md#move_figure) | Displace actor by (dir, dist) if proxcheck clears |
| `newx` | [movement.md#newx](movement.md#newx) | Compute new X from (x, dir, dist) via `com2` vector table |
| `newy` | [movement.md#newy](movement.md#newy) | Compute new Y from (y, dir, dist) via `com2` vector table |
| `walk_step` | [movement.md#walk_step](movement.md#walk_step) | Per-tick WALKING body: deviations, proxcheck, door collision |
| `still_step` | [movement.md#still_step](movement.md#still_step) | Per-tick STILL body: terrain update only |
| `proxcheck` | [movement.md#proxcheck](movement.md#proxcheck) | Collision query: terrain + actor blocking |
| `update_environ` | [movement.md#update_environ](movement.md#update_environ) | Terrain → environ code (sinking/drowning/teleport) |
| `set_course` | [movement.md#set_course](movement.md#set_course) | Face actor toward a target; used by witch + AI |
| `place_extent_encounters` | [encounters.md#place_extent_encounters](encounters.md#place_extent_encounters) | Phase 14i: drain `encounter_number` into dead/empty slots around the hero |
| `roll_wilderness_encounter` | [encounters.md#roll_wilderness_encounter](encounters.md#roll_wilderness_encounter) | Phase 14j: danger_level vs rand64 roll + biome overrides + `load_actors` |
| `set_loc` | [encounters.md#set_loc](encounters.md#set_loc) | Pick (encounter_x, encounter_y) on a 150–213 px ring around hero |
| `set_encounter` | [encounters.md#set_encounter](encounters.md#set_encounter) | Place one ENEMY in anim_list[i]: collision-free coord, race, weapon, goal, HP |
| `prep` | [encounters.md#prep](encounters.md#prep) | Wait on actor-shape disk I/O channel 8, then build the sprite mask table |
| `load_actors` | [encounters.md#load_actors](encounters.md#load_actors) | Set `encounter_number` and async-read the new race's shape file if needed |
| `roll_treasure` | [encounters.md#roll_treasure](encounters.md#roll_treasure) | Corpse drop lookup into `treasure_probs[tier*8 + rand8]` (0 for setfigs) |
| `roll_weapon` | [encounters.md#roll_weapon](encounters.md#roll_weapon) | Enemy weapon lookup into `weapon_probs[arms*4 + col]` |
| `give_item_to_npc` | [quests.md#give_item_to_npc](quests.md#give_item_to_npc) | GIVE submenu dispatch on (slot hit, target race): gold / bone / no-op |
| `check_quest_flag` | [quests.md#check_quest_flag](quests.md#check_quest_flag) | Read a `stuff[]` slot and return a menu-enable byte (8 or 10) |
| `necromancer_death_drop` | [quests.md#necromancer_death_drop](quests.md#necromancer_death_drop) | STATE_DYING transition: race 9 → woodcutter+Talisman; race 0x89 drops Lasso |
| `leave_item` | [quests.md#leave_item](quests.md#leave_item) | Drop a ground-state world object at actor i's feet via `ob_listg[0]` |
| `rescue` | [quests.md#rescue](quests.md#rescue) | Princess-rescue cinematic + Marheim teleport + Writ/gold/keys grant |
| `get_turtle` | [quests.md#get_turtle](quests.md#get_turtle) | USE Shell: spawn turtle carrier on a nearby deep-water tile |
| `try_win_condition` | [quests.md#try_win_condition](quests.md#try_win_condition) | Post-pickup Talisman check: set quitflag + run the end-game sequence |
| `end_game_sequence` | [quests.md#end_game_sequence](quests.md#end_game_sequence) | Win placard + winpic + 55-frame sunrise fade + blackout |
| `talk_dispatch` | [npc-dialogue.md#talk_dispatch](npc-dialogue.md#talk_dispatch) | CMODE_TALK dispatch: Yell/Say/Ask range + per-setfig/carrier/enemy speech |
| `wizard_hint` | [npc-dialogue.md#wizard_hint](npc-dialogue.md#wizard_hint) | Wizard TALK speech: kind gate + goal-indexed quest hints |
| `priest_speech` | [npc-dialogue.md#priest_speech](npc-dialogue.md#priest_speech) | Priest TALK speech: writ→statue, kind rebuke, or daily-rotating heal + hint |
| `bartender_speech` | [npc-dialogue.md#bartender_speech](npc-dialogue.md#bartender_speech) | Bartender TALK speech: fatigue + dayperiod tri-branch |
| `ranger_hint` | [npc-dialogue.md#ranger_hint](npc-dialogue.md#ranger_hint) | Ranger TALK speech: region-2 override or goal-indexed cave hint |
| `proximity_auto_speak` | [npc-dialogue.md#proximity_auto_speak](npc-dialogue.md#proximity_auto_speak) | Phase 14: one-shot greeting for 5 named NPC races when nearest_person changes |
| `savegame` | [save-load.md#savegame](save-load.md#savegame) | Top-level save/load dispatcher: disk detection, slot file open, record stream, post-load fixup |
| `serialize_save_record` | [save-load.md#serialize_save_record](save-load.md#serialize_save_record) | Write the thirteen blocks of the save record in order |
| `deserialize_save_record` | [save-load.md#deserialize_save_record](save-load.md#deserialize_save_record) | Read the thirteen blocks of the save record in order |
| `saveload_block` | [save-load.md#saveload_block](save-load.md#saveload_block) | Low-level Read/Write primitive directed by `svflag` |
| `mod1save` | [save-load.md#mod1save](save-load.md#mod1save) | Blocks 5-8: brother inventories + reseat `stuff` + missiles |
| `locktest` | [save-load.md#locktest](save-load.md#locktest) | Non-destructive AmigaDOS path-presence probe |
| `waitnewdisk` | [save-load.md#waitnewdisk](save-load.md#waitnewdisk) | Poll the input handler for a disk-insert event up to ~30s |
| `buy_dispatch` | [shops.md#buy_dispatch](shops.md#buy_dispatch) | CMODE_BUY case: bartender-gated `jtrans` row lookup, gold check, food/arrow/stuff side effect |
| `revive` | [brother-succession.md#revive](brother-succession.md#revive) | Start / restart active brother: place bones + ghost, load next `blist[]` stats, teleport to Tambry, placards, or fairy-rescue the current brother |
| `pickup_brother_bones` | [brother-succession.md#pickup_brother_bones](brother-succession.md#pickup_brother_bones) | Merge a dead brother's item inventory into the current brother's on bones pickup; retire both ghost set-figures |
| `fade_page` | [visual-effects.md#fade_page](visual-effects.md#fade_page) | Scale 32-entry palette by (r,g,b) weights with night floors, torch tint, sky/water blue boost |
| `colorplay` | [visual-effects.md#colorplay](visual-effects.md#colorplay) | 32-frame random-palette strobe used at teleport events |
| `fade_down` | [visual-effects.md#fade_down](visual-effects.md#fade_down) | Ramp pagecolors from 100% to 0% brightness over 21 frames |
| `fade_normal` | [visual-effects.md#fade_normal](visual-effects.md#fade_normal) | Ramp pagecolors from 0% to 100% brightness over 21 frames |
| `map_message` | [visual-effects.md#map_message](visual-effects.md#map_message) | Enter full-screen placard mode: fade out, retarget rp to map RastPort, hide text VP |
| `message_off` | [visual-effects.md#message_off](visual-effects.md#message_off) | Leave full-screen placard mode: fade out, reattach rp to text VP, queue a fade-in |
| `copypage` | [visual-effects.md#copypage](visual-effects.md#copypage) | Intro page: hold, blit pageb→pagea, unpack next brush pair into pageb, flipscan |
| `flipscan` | [visual-effects.md#flipscan](visual-effects.md#flipscan) | 22-frame columnar wipe between pagea and pageb using flip1/flip2/flip3 tables |
| `screen_size` | [visual-effects.md#screen_size](visual-effects.md#screen_size) | Resize playfield viewport to 2x×2y; sync palette brightness via fade_page |
| `win_colors` | [visual-effects.md#win_colors](visual-effects.md#win_colors) | Victory placard + winpic + 55-frame sunrise walk of sun_colors[] |
| `xfer` | [doors.md#xfer](doors.md#xfer) | fmain.c:2625-2645 — teleport hero + re-derive region + reload + nudge-out-of-wall |
| `doorfind` | [doors.md#doorfind](doors.md#doorfind) | fmain.c:1081-1128 — resolve door tile, match open_list by key, rewrite tiles |
| `use_key_on_door` | [doors.md#use_key_on_door](doors.md#use_key_on_door) | fmain.c:3472-3488 — CMODE_KEYS case: sweep 9 directions, consume key on success |
| `tick_daynight` | [day-night.md#tick_daynight](day-night.md#tick_daynight) | fmain.c:2014-2045 — advance daynight, refresh lightlevel, classify dayperiod, regen vitality |
| `sleep_tick` | [day-night.md#sleep_tick](day-night.md#sleep_tick) | fmain.c:2014-2021 — sleep fast-forward, fatigue drain, wake conditions |
| `day_fade` | [day-night.md#day_fade](day-night.md#day_fade) | fmain2.c:1653-1660 — outdoor/indoor palette refresh driver (calls fade_page) |
| `setmood` | [day-night.md#setmood](day-night.md#setmood) | fmain.c:2936-2957 — select music track for mood (death/astral/battle/indoor/day/night) |
| `hunger_fatigue_tick` | [day-night.md#hunger_fatigue_tick](day-night.md#hunger_fatigue_tick) | fmain.c:2188-2220 — safe-zone + auto-eat + hunger/fatigue advance + warnings |

*(Rows are appended as new logic docs are authored. Orphan entries and orphan function definitions both fail `lint_logic.py`.)*
