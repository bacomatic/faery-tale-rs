# Plan: Actor Death Mechanic

## Context

NPCs set to `NpcState::Dying` have no countdown or frame animation — they stay in Dying
forever. The hero death sequence skips the 7-tick dying animation entirely and always
routes to brother succession (never fairy rescue). `EnemyDiedEvent` is emitted but never
consumed, leaving special drops (Necromancer → Woodcutter + talisman, Witch → lasso)
unimplemented.

Reference: `docs/spec/death-revival.md §20`, `reference/logic/combat.md §checkdead`,
`reference/_discovery/combat.md §Dying Animation`.

---

## Step 1 — NPC dying-animation countdown

**Spec:** `an.tactic = 7` on death; `death_step` counts down to 0; DYING→DEAD at 0.
Port uses a dedicated `dying_countdown: u8` field (separate from the AI `Tactic` enum).

### `src/game/ecs/components.rs`
Add field to `AiState`:
```rust
pub dying_countdown: u8,  // 7→0 death-animation counter; 0 = not dying
```
Update `AiState::default()`: `dying_countdown: 0`.

### `src/game/ecs/spawn.rs` + `src/game/debug_tui/bridge.rs`
Add `dying_countdown: 0` to every `AiState { .. }` literal.

### `src/game/ecs/systems/damage.rs` — `trigger_death()`
After `ai.state = NpcState::Dying`, add:
```rust
ai.dying_countdown = 7;
```

### `src/game/ecs/systems/npc_ai.rs`
Replace the single `Dying | Dead | Sinking` skip with:
```rust
if matches!(state, NpcState::Dead | NpcState::Sinking) { continue; }
if matches!(state, NpcState::Dying) {
    // Tick the death-animation countdown (fmain.c:1719-1726).
    if let Ok(mut ai) = world.get::<&mut AiState>(entity) {
        if ai.dying_countdown > 0 { ai.dying_countdown -= 1; }
        else { ai.state = NpcState::Dead; }
    }
    continue;
}
```

### `src/game/ecs/scene.rs` — `enemy_frame()`
In the generic `else` branch (and LORAII's Dying arm), use `dying_countdown` for
frame selection when `NpcState::Dying`:
```
dying_countdown > 4 → frame 80 (north facing) or 81 (south facing)
dying_countdown > 0 → frame 81 / 80 (swapped)
dying_countdown == 0 → frame 82 (corpse; will be Dead by next tick)
```
This replaces the existing literal `0x3f` for LORAII's Dying frame — that only applies
to `update_actor_index`; `death_step`'s countdown frames apply to all races.

---

## Step 2 — Hero dying animation + safe position

Hero and NPC share the same 7-tick dying animation before the goodfairy sequence begins.

### `src/game/ecs/resources.rs` — `EncounterContext`
Add fields:
```rust
pub hero_dying_countdown: u8,   // 7→0 hero death animation; mirrors NPC dying_countdown
pub safe_pos: (f32, f32),       // last safe position; fairy rescue destination
pub safe_r:   u8,               // region at last safe position
```
Defaults: `safe_pos = (19036.0, 15755.0)`, `safe_r = 3` (Tambry).

### `src/game/ecs/systems/damage.rs` — `apply_damage()` hero branch
When hero vitality reaches ≤ 0, start the dying animation (guard: only once):
```rust
if new_vitality <= 0
    && res.encounter.hero_dying_countdown == 0
    && !res.encounter.dying
{
    res.encounter.hero_dying_countdown = 7;
}
```

### `src/game/ecs/systems/death.rs` — `run()`
Insert Phase 1 before the existing goodfairy logic:
```rust
if vitality <= 0 && res.encounter.hero_dying_countdown > 0 {
    res.encounter.hero_dying_countdown -= 1;
    return; // dying animation playing; goodfairy not yet started
}
```
(The hero sprite for the dying frames is a rendering concern — see Note below.)

### Safe-pos update (`src/game/ecs/systems/zone.rs` or `clock.rs`)

Reference: `fmain.c:2216-2224` (Phase 14n in the game loop). Fires every 128 `daynight`
ticks when all conditions hold — applies to **both indoor and outdoor regions**.

Update logic (run once per tick after collision system sets battleflag):
```rust
let safe_outdoors =
    !res.region.battleflag           // no enemies on screen
    && !res.brother.witchflag        // not a witch encounter
    && hero_environ == 0             // dry land (environ byte under hero)
    && !res.brother.safe_flag        // safe_flag terrain-code check (bool false = safe)
    && !res.encounter.dying          // hero not dead
    && vitality > 0;
if (res.clock.daynight & 127) == 0 && safe_outdoors {
    res.encounter.safe_r   = res.region.region_num;
    res.encounter.safe_pos = (hero_pos.x, hero_pos.y);
}
```

`hero_environ` is the terrain-code byte under the hero — read `ActorMotion.environ` on
the hero entity (already set by `npc_movement.rs`'s `apply_update_environ` logic).

On fairy rescue, also restore `res.region.new_region = res.encounter.safe_r` so the
region system loads the correct map.

---

## Step 3 — Fairy rescue vs. brother succession

Current `death.rs` always emits `BrotherDiedEvent` (succession). Split on luck.

### `src/game/ecs/systems/death.rs` — goodfairy-expired branch
Replace the unconditional `BrotherDiedEvent` emit with:
```rust
let luck  = world.get::<&HeroStats>(res.hero_entity).map(|s| s.luck).unwrap_or(0);
let brave = world.get::<&HeroStats>(res.hero_entity).map(|s| s.brave).unwrap_or(0);

if luck >= 1 {
    // Fairy rescue (revive(FALSE)): restore hero at safe position.
    let (sx, sy) = res.encounter.safe_pos;
    if let Ok(mut pos) = world.get::<&mut Position>(res.hero_entity) {
        pos.x = sx; pos.y = sy;
    }
    if let Ok(mut stats) = world.get::<&mut HeroStats>(res.hero_entity) {
        stats.vitality = 15 + stats.brave / 4;
        stats.hunger   = 0;
        stats.fatigue  = 0;
    }
    res.region.new_region        = res.encounter.safe_r;
    res.clock.daynight           = 8000;
    res.encounter.dying          = false;
    res.encounter.luck_gate_fired = false;
} else {
    // Brother succession (revive(TRUE)): emit event; drain_brother_deaths() handles it.
    // (existing BrotherDiedEvent push — pos/inv/bid already gathered above)
}
```

Also remove the existing incorrect "luck > 0 → restore 1 vitality" early-return (lines 30–39).

---

## Step 4 — Aftermath system (NPC special drops)

`EnemyDiedEvent` is emitted by `damage.rs` but never consumed. This system drains it
and handles race-specific death effects.

### New file: `src/game/ecs/systems/aftermath.rs`
```rust
pub fn run(world: &mut World, res: &mut Resources) {
    let events: Vec<EnemyDiedEvent> = res.events.died.drain(..).collect();
    for ev in events {
        handle_died(world, res, ev);
    }
}
```

Per event:
- **Necromancer (race 9)** — `fmain.c:1751`: transform the entity in place
  (mutate `EnemyKind.race = 10`, `Health.vitality = 10`, `AiState.state = NpcState::Still`,
  clear weapon via `Loot.weapon = 0`); spawn talisman ground item at `(ev.x, ev.y)`.
  Talisman `ob_id` — verify against `faery.toml` objects table (reference uses ob_id 22
  or item 139 depending on list; search `game_lib` for "talisman").
- **Witch (race 0x89)** — `fmain.c:1756`: spawn lasso ground item (`ob_id` 27) at
  `(ev.x, ev.y)`.
- All other races: no automatic drop. Weapon/gold loot stays on the body and is
  retrieved via `ItemEvent::SearchBody` (already implemented in `item.rs`).

Use existing `spawn_ground_item(world, ev.x, ev.y, WorldObj { ob_id, ob_stat: 1, region: res.region.region_num, visible: true, goal: 0 })`.

### `src/game/ecs/systems/mod.rs`
Add `pub mod aftermath;`.

### `src/game/ecs/scene.rs` — `run_tick()`
Insert after `systems::damage::run(...)`:
```rust
systems::aftermath::run(&mut self.world, &mut self.res);
```

---

## Notes

- **Hero dying sprite frames**: the hero renderer in `scene.rs` will need to check
  `res.encounter.hero_dying_countdown > 0` and render the appropriate dying frame
  (80/81 per facing). This is a rendering follow-up; the countdown mechanics are the
  priority here.
- **Goodfairy countdown length**: corrected to 255 ticks (u8) per reference.
  `luck -= 5` applied in `damage.rs` at death time (Gap 1 fix).
  Fade-to-black at goodfairy==1 implemented via `FadeController` in scene.rs (Gap 2 fix).
- **LORAII `update_actor_index`**: the existing `dex = 0x3f` for LORAII Dying in
  `scene.rs:1808` is in `update_actor_index`, which overrides the frame after
  `death_step`. Both coexist — countdown drives the transition timing;
  `update_actor_index` overrides the visual index for LORAII specifically.

---

## Verification

```
cargo test              # all tests pass; add tests for:
                        #   - npc dying_countdown decrements 7→0, state→Dead
                        #   - hero dying_countdown fires before goodfairy
                        #   - luck >= 1 → fairy rescue at safe_pos (not BrotherDiedEvent)
                        #   - luck < 1  → BrotherDiedEvent emitted
                        #   - Necromancer death → EnemyKind.race==10, talisman spawned
                        #   - Witch death → lasso spawned
```

Manual checks (in-game with `--debug`):
- Kill an enemy: see 7-frame flicker between frames 80/81, then corpse (frame 82).
- Hero takes lethal damage: 7-tick dying animation, then goodfairy 60-tick wait.
  - With luck ≥ 1: respawn at last safe position (indoor or outdoor).
  - With luck < 1: brother succession + bones at death location.
- Kill the Necromancer: entity becomes a friendly Woodcutter; talisman appears nearby.
