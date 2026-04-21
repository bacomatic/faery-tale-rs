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

*(Rows are appended as new logic docs are authored. Orphan entries and orphan function definitions both fail `lint_logic.py`.)*
