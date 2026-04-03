# Combat Loop Rewrite

**Date:** 2026-04-02  
**Status:** Approved  
**Scope:** Port fmain.c combat loop; fix input routing; correct bow fire rate

## Problem

Three bugs in the combat system:

1. **Controller/HI-bar attack doesn't stop movement or show animation.**  
   `do_option(GameAction::Fight)` calls `apply_melee_combat()` as a one-shot without setting `input.fight`. Next frame, the flag is false and movement resumes.

2. **Numpad attack works but combat feels slow/turn-based.**  
   `fight_cooldown = 10` gates melee hits to ~3/sec. The original runs the combat loop every frame (30 Hz).

3. **Enemies only swing when the hero does.**  
   Enemy counterattack code is inside `apply_melee_combat()`, coupled to the hero's cooldown gate.

## Design

### 1. Unified combat tick (`run_combat_tick`)

A single `run_combat_tick()` method on `GameplayScene`, called every frame from the main game tick after `apply_player_input()` and `update_actors()`. Replaces both the combat branch in `apply_player_input()` and the counterattack block in `apply_melee_combat()`.

Mirrors fmain.c lines 2680–2730:

```
for each combatant i in 0..actor_count:
    skip if i == 1 (raft slot)
    skip if state >= WALKING (not fighting/shooting)
    skip if i > 0 and freeze_timer active

    wt = combatant weapon index
    if wt & 4: skip melee (bow/wand handled by shoot state machine)
    if wt >= 8: wt = 5 (cap touch attack)
    wt += bitrand(2)

    compute weapon tip: newx/newy(abs_x, facing, wt*2) + rand8() - 3
    compute reach: hero → (brave/20)+5, NPC → 2+rand4(), cap 15

    for each target j in 0..actor_count:
        skip if j == 1, j == i, or target dead/carrier
        distance = Chebyshev(target_pos - weapon_tip)
        if (i == 0 or rand256() > brave) and distance < reach:
            apply_hit(i, j, facing, wt)
            break  // one hit per swing
        else if distance < reach+2 and wt != 5:
            // near-miss sound (stub until audio effects wired)
```

`apply_melee_combat()` is deleted. `fight_cooldown` field is removed.

### 2. Input unification

All attack triggers set/clear the persistent `input.fight` flag:

- **Keyboard (Kp0):** Already works — `KeyDown` sets true, `KeyUp` clears.
- **Controller A button:** `ControllerButtonDown` sets `self.input.fight = true`. `ControllerButtonUp` clears it. No longer calls `do_option(GameAction::Fight)`.
- **HI bar click:** Sets `self.input.fight = true` on `MouseButtonDown`. Clears on `MouseButtonUp`.
- **`do_option(GameAction::Fight)`:** Removed as a combat entry point. If other code paths still dispatch this action, they set `input.fight = true` instead of calling combat directly.

The `apply_player_input()` fight branch remains responsible for:
- Suppressing movement when `input.fight` is true
- Updating hero facing from directional input
- Setting `ActorState::Fighting(_)` or `ActorState::Shooting(_)` on the hero actor
- Advancing fight/shoot animation via `advance_fight_state()`

It no longer calls any combat resolution — that's `run_combat_tick()`'s job.

### 3. Bow state machine (press-to-aim, release-to-fire)

Original behavior: pressing attack with a bow enters SHOOT1 (aiming). Releasing transitions to SHOOT3 (fire). One arrow per press-release cycle. Maximum rate: 15/sec (2 frames at 30 Hz).

Implementation:
- **Press attack with bow+arrows:** `apply_player_input()` sets `ActorState::Shooting(0)` (SHOOT1). Movement suppressed. Hero can change facing.
- **While held:** State stays Shooting. No missile fires. The `run_combat_tick()` `wt & 4` guard skips melee for this combatant.
- **Release (input.fight goes false):** If hero state is `Shooting`, transition fires: spawn missile via `fire_missile()`, decrement `stuff[ARROWS]`, set state to `Still`. The arrow spawns on the release frame.
- **Out of arrows:** Don't enter Shooting state. Stay in current state.

This is distinct from melee where holding attack means swinging every frame.

### 4. Hit application (`apply_hit`)

New helper method on `GameplayScene`. Ports `dohit(i, j, fc, wt)` from fmain2.c:317–356.

**Damage:** `target.vitality -= wt` (wt already includes bitrand bonus from combat tick).

**Special guards:** Necromancer (race 9) and Witch (race 0x89) immune unless attacker weapon >= 4. Hit registers but deals 0 damage.

**Pushback:** Target pushed 2px in attacker's facing direction. If attacker is hero and hit connects, hero also pushed 2px forward (recoil). Currently not implemented; this design adds it.

**Sound effects:** Stubbed with `// TODO: effect()` calls matching original parameters:
- Hero hit by melee: `effect(0, 800 + bitrand(511))`
- Near-miss: `effect(1, 150 + rand256())`
- Arrow-to-player: `effect(2, 500 + rand64())`
- Monster hit: `effect(3, 400 + rand256())`
- Arrow-to-target: `effect(4, 400 + rand256())`
- Magic hit: `effect(5, 3200 + bitrand(511))`

**Death (`checkdead`):** Existing logic (deactivate NPC, brave++, loot roll) plus:
- Set `ActorState::Dying` on the target actor.
- Hero death: vitality < 1 triggers death event, `luck -= 5`.

### 5. Code organization

| Change | Location |
|--------|----------|
| New `run_combat_tick()` | `GameplayScene` method, called from main tick |
| New `apply_hit()` | `GameplayScene` helper method |
| Delete `apply_melee_combat()` | `gameplay_scene.rs` |
| Remove `fight_cooldown` field | `GameplayScene` struct |
| Input routing changes | Event handling in `gameplay_scene.rs` |
| Bow release-to-fire | `apply_player_input()` fight branch |

Unchanged:
- `advance_fight_state()` / `FIGHT_TRANS_LIST` (animation transitions)
- `in_melee_range()`, `melee_reach()`, `bitrand_damage()` (reused in new loop)
- `fire_missile()` and missile tick logic
- NPC AI (`select_tactic` / `do_tactic`)
- `award_loot()`
- `archer_cooldown` (separate NPC ranged concern)

### 6. Testing

- Existing `resolve_combat()` tests stay (deprecated but test damage math).
- New: `test_run_combat_tick_hero_hits_enemy` — hero Fighting, enemy in range → damage applied.
- New: `test_run_combat_tick_npc_hits_hero` — NPC Fighting near hero → brave dodge roll verified.
- New: `test_run_combat_tick_bow_fires_missile` — hero Shooting, release triggers missile, arrows decremented.
- New: `test_pushback_on_hit` — verify 2px displacement on both target and attacker.
- Manual: `cargo run -- --debug`, verify dlog shows per-frame combat messages.

## Files modified

- `src/game/gameplay_scene.rs` — bulk of changes
- `src/game/combat.rs` — new helper functions if needed, cleanup deprecated code

## Reference

- Original combat loop: `original/fmain.c:2680–2730`
- Original dohit: `original/fmain2.c:317–356`
- Original shoot states: `original/fmain.c:1584–1622, 1907–1930`
- RESEARCH.md: Combat System section (lines 736–850)
