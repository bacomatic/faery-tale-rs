# Logic Documentation — Index

This directory contains strict, linter-backed pseudo-code specifications for every non-trivial branching function in *The Faery Tale Adventure*. Combined with [ARCHITECTURE.md](../ARCHITECTURE.md), [RESEARCH.md](../RESEARCH.md), [STORYLINE.md](../STORYLINE.md), and the spatial/quest JSON databases, these reference docs are sufficient to reproduce the game's behavior without reading the 1987 source.

**Fidelity target:** behavioral. Same inputs produce the same observable gameplay. Implementation primitives (RNG algorithm, integer widths when not observable, fixed-point layout) are left to the porter.

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
4. Subsystem reference docs in order of gameplay centrality (Wave 3+): combat → movement → encounters → quests → npc-dialogue → save-load → shops → brother-succession → visual-effects.

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
| `compute_raftprox` | [carrier-transport.md#compute_raftprox](carrier-transport.md#compute_raftprox) | fmain.c:1455-1464 — derive wcarry / raftprox and latch swan ice-physics environ |
| `load_carrier` | [carrier-transport.md#load_carrier](carrier-transport.md#load_carrier) | fmain.c:2784-2802 — load swan/turtle/dragon shapes into anim_list[3] at extent center |
| `carrier_extent_update` | [carrier-transport.md#carrier_extent_update](carrier-transport.md#carrier_extent_update) | fmain.c:2716-2719 — auto-spawn / despawn the slot-3 carrier on xtype==70 transitions |
| `carrier_tick` | [carrier-transport.md#carrier_tick](carrier-transport.md#carrier_tick) | fmain.c:1494-1547 — CARRIER body: swan mount, turtle mount, turtle water-swim AI, move_extent drag |
| `raft_tick` | [carrier-transport.md#raft_tick](carrier-transport.md#raft_tick) | fmain.c:1562-1573 — RAFT body: snap anim_list[1] onto hero on terrain 3-5 with raftprox==2 |
| `swan_dismount` | [carrier-transport.md#swan_dismount](carrier-transport.md#swan_dismount) | fmain.c:1417-1428 — fire-button dismount: lava veto, ±15 velocity gate, two-probe landing clear |
| `use_sea_shell` | [carrier-transport.md#use_sea_shell](carrier-transport.md#use_sea_shell) | fmain.c:3457-3461 — USE Shell: swamp-rectangle veto then delegate to get_turtle |
| `find_place` | [astral-plane.md#find_place](astral-plane.md#find_place) | fmain.c:2647-2720 — Place-name lookup + extent matcher; on xtype change: rescue / astral Loraii preload / forced spawn / carrier update |
| `take_command` | [inventory.md#take_command](inventory.md#take_command) | fmain.c:3149-3248 — ITEMS→Take dispatch: gold/scrap/fruit/bones/container/itrans pickup + Talisman win latch |
| `search_body` | [inventory.md#search_body](inventory.md#search_body) | fmain.c:3249-3282 — Take against an actor: loot weapon + arrows (Bow), treasure_probs drop |
| `use_dispatch` | [inventory.md#use_dispatch](inventory.md#use_dispatch) | fmain.c:3444-3467 — USE submenu: weapon equip, Keys submenu, Sea Shell turtle, Sun Stone witch unmask |
| `magic_dispatch` | [inventory.md#magic_dispatch](inventory.md#magic_dispatch) | fmain.c:3300-3365 — MAGIC submenu: 7 magic-item effects with precondition guards + charge decrement |
| `look_command` | [inventory.md#look_command](inventory.md#look_command) | fmain.c:3286-3295 — ITEMS→Look: reveal hidden objects (race==0) within 40 px |
| `select_frust_anim` | [frustration.md#select_frust_anim](frustration.md#select_frust_anim) | fmain.c:1655-1659 — Hero sprite override from frustflag thresholds (20 / 40) |
| `trigger_frust` | [frustration.md#trigger_frust](frustration.md#trigger_frust) | fmain.c:1654-1661 — Blocked dispatch: hero frustflag++ vs NPC `tactic = TACTIC_FRUST` |
| `resolve_frust_tactic` | [frustration.md#resolve_frust_tactic](frustration.md#resolve_frust_tactic) | fmain.c:2141-2144 — AI-tick reroll of a latched TACTIC_FRUST / TACTIC_SHOOTFRUST |
| `add_device` | [input-handling.md#add_device](input-handling.md#add_device) | fmain.c:3017-3036 — install priority-51 input.device handler and reset the ring buffer |
| `wrap_device` | [input-handling.md#wrap_device](input-handling.md#wrap_device) | fmain.c:3038-3046 — detach the handler and free its device resources |
| `handler_interface` | [input-handling.md#handler_interface](input-handling.md#handler_interface) | fsubs.asm:63-218 — event-chain callback: timer heartbeat, rawkey translate, mouse-strip synth, pointer clamp |
| `handle_rawkey` | [input-handling.md#handle_rawkey](input-handling.md#handle_rawkey) | fsubs.asm:84-111 — translate one RAWKEY via keytrans and enqueue the 128-byte ring buffer |
| `handle_rawmouse` | [input-handling.md#handle_rawmouse](input-handling.md#handle_rawmouse) | fsubs.asm:112-157 — left-button transitions → synthesize menu-slot codes in status-bar strip |
| `keybuf_push` | [input-handling.md#keybuf_push](input-handling.md#keybuf_push) | fsubs.asm:105-113 — 128-slot wrap; drop on overflow |
| `update_pointer` | [input-handling.md#update_pointer](input-handling.md#update_pointer) | fsubs.asm:163-200 — accumulate mouse deltas, clamp to status-bar strip, MoveSprite |
| `getkey` | [input-handling.md#getkey](input-handling.md#getkey) | fsubs.asm:281-295 — pop one translated code from the 128-byte ring buffer (0 if empty) |
| `decode_mouse` | [input-handling.md#decode_mouse](input-handling.md#decode_mouse) | fsubs.asm:1488-1590 — per-frame direction fuser (mouse > joystick > keydir) + compass refresh |
| `decode_mouse_strip` | [input-handling.md#decode_mouse_strip](input-handling.md#decode_mouse_strip) | fsubs.asm:1497-1530 — cursor → 3×3 compass grid in the status-bar strip |
| `decode_joystick` | [input-handling.md#decode_joystick](input-handling.md#decode_joystick) | fsubs.asm:1533-1562 — read JOY1DAT, derive (xjoy, yjoy), look up direction via com2[] |
| `decode_keydir` | [input-handling.md#decode_keydir](input-handling.md#decode_keydir) | fsubs.asm:1567-1580 — latched keypad 20..29 → direction 0..9 |
| `px_to_im` | [terrain-collision.md#px_to_im](terrain-collision.md#px_to_im) | fsubs.asm:542-620 — pixel → terrain type via sub-tile mask + image/sector/terra chain |
| `prox` | [terrain-collision.md#prox](terrain-collision.md#prox) | fsubs.asm:1590-1614 — two-probe collision: right ≥10 / left ≥8 thresholds |
| `mapxy` | [terrain-collision.md#mapxy](terrain-collision.md#mapxy) | fsubs.asm:1085-1130 — image-tile coord → sector_mem byte offset (doorfind uses for tile rewrite) |

*(Rows are appended as new logic reference docs are authored. Orphan entries and orphan function definitions both fail `lint_logic.py`.)*
