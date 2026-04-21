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

*(Rows are appended as new logic docs are authored. Orphan entries and orphan function definitions both fail `lint_logic.py`.)*
