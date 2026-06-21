# Faery Tale Adventure — ECS Port: Remaining Work

> Generated after Plans A–F landed (commit `787d5db`).
> Each plan below is **independent**: it can be executed in a fresh context
> without reading the other plans.  Plans are sequenced so earlier ones unblock
> later ones where a dependency exists; otherwise order is arbitrary.
>
> **Naming convention:** plans are lettered G, H, I, … continuing from F.

---

## Plans

| Plan | File | Status |
|------|------|--------|
| G | `2025-01-01-ecs-plan-g-region-transitions.md` | ✅ Done |
| H | `2025-01-01-ecs-plan-h-door-system.md` | ✅ Done |
| I | `2025-01-01-ecs-plan-i-menu-system.md` | ✅ Done |
| J | `2025-01-01-ecs-plan-j-inventory-screen.md` | ✅ Done |
| J2 | `2025-01-01-ecs-plan-j2-item-text-and-food-fix.md` | ✅ Done |
| K | `2025-01-01-ecs-plan-k-magic-system.md` | ✅ Done |
| L | `2025-01-01-ecs-plan-l-narrative-queue.md` | ✅ Done |
| M | `2025-01-01-ecs-plan-m-shop-system.md` | ✅ Done |
| N | `2025-01-01-ecs-plan-n-combat-melee.md` | ✅ Written |
| O | `2025-01-01-ecs-plan-o-encounter-system.md` | ✅ Done |
| P | `2025-01-01-ecs-plan-p-carrier-system.md` | ✅ Written |
| Q | `2025-01-01-ecs-plan-q-sleep-system.md` | ✅ Written |
| R | `2025-01-01-ecs-plan-r-brother-succession.md` | ✅ Done |
| S | `2025-01-01-ecs-plan-s-setfig-sprites.md` | ✅ Done |
| T | `2025-01-01-ecs-plan-t-weapon-overlays.md` | ✅ Done |
| U | `2025-01-01-ecs-plan-u-save-load-keys.md` | ✅ Written |
| V | `2025-01-01-ecs-plan-v-quest-state.md` | ✅ Done |
| W | `2025-01-01-ecs-plan-w-debug-tui-extras.md` | ✅ Done |
| X | `2025-01-01-ecs-plan-x-parity-and-cleanup.md` | ✅ Written |
| Y | `2025-01-01-ecs-plan-y-debug-tui-ecs-integration.md` | ✅ Written |
| Z | `2025-01-01-ecs-plan-z-setfig-spawn-cfile.md` | ✅ Written |

---

## Plan G — Region Transitions

**Why first:** nearly every plan below touches NPCs, items, or encounters that
only make sense once the player can move between regions.  Region reload is the
single most impactful stub.

**What to build:**

1. `src/game/ecs/systems/region.rs` — implement `run()`:
   - Drain `res.events.region` (already emitted by door/zone systems).
   - Reload `WorldData` + `MapRenderer` from ADF for the new region (refactor
     `EcsScene::load_world()` into a reusable `reload_region(&mut res, region,
     adf)` helper).
   - Despawn all `Enemy` + `SetFig` entities for the old region.
   - Respawn NPCs from the new region's NPC table (`npc.rs` / `WorldData`).
   - Recompute the region palette (call `region_palette` + mark `dirty`).
   - Reposition the hero camera at the transition destination.

2. `src/game/ecs/systems/zone.rs` — populate the zone list:
   - Currently `res.map.world` holds the `WorldData`; zones live in
     `WorldData::zones`.  Move them into `Resources.map` on region load so
     `zone::run()` can iterate them without re-reading `WorldData` each tick.

**Files touched:** `systems/region.rs`, `systems/zone.rs`, `ecs/scene.rs`
(load_world refactor), `ecs/resources.rs` (MapData.zones field).

**Independence:** standalone; no dependency on later plans.

**Verify:** write a test that constructs a minimal `EcsScene`, sets
`res.events.region` to a transition, calls `region::run()`, and asserts
`region_num` changed and the framebuf is marked dirty.

---

## Plan H — Door System

**Dependency:** Plan G (zones must be loaded before doors make sense).

**What to build:**

1. `src/game/ecs/systems/door.rs` — implement `run()`:
   - Door table currently lives in `WorldData`; add `MapData.doors: Vec<Door>`
     populated on region load.
   - Detect hero position overlapping a door rect (same proximity logic as the
     old `gameplay_scene/doors.rs`).
   - On hit: emit `RegionTransitionEvent` with destination region + coords.
   - Track opened doors in `MapData.opened_doors: HashSet<u16>` (door id).
   - Emit `SfxEvent` for door open sound.

2. `src/game/ecs/resources.rs` — add `MapData.doors` and `MapData.opened_doors`.

**Files touched:** `systems/door.rs`, `ecs/resources.rs`.

**Verify:** unit test — hero at door position triggers `RegionTransitionEvent`.

---

## Plan I — Menu System + HI Bar Buttons

**Why now:** the menu system is the gateway to every player-facing action
(inventory, magic, shop, talk, save/load).  Nothing interactive works until
menus are wired.

**What to build:**

1. `src/game/ecs/scene.rs` — `handle_event()`:
   - Add key handling for menu trigger keys from `src/game/menu.rs::LETTER_LIST`
     (I/M/U/G/K/T/B/S) plus Escape to close.
   - Gamepad shoulder / face button → menu open/navigate.
   - Delegate navigation (up/down/select within menu) to `MenuState`.

2. `src/game/ecs/scene.rs` — add `menu: MenuState` field to `EcsScene`.

3. `src/game/ecs/scene.rs` — `render_hibar()`:
   - Port the button rendering block from `gameplay_scene/rendering.rs`
     (`topaz_font.render_string_with_bg` / `render_string` per `btn`).
   - The `MenuState::print_options()` call already returns the button list;
     the rendering code is verbatim from the old scene.

4. Menu action dispatch — new helper `EcsScene::dispatch_menu_action()`:
   - `Action::Game` → open game sub-menu (Save / Load / Quit).
   - `Action::Items` / `Action::Use` / `Action::Give` / `Action::Keys` →
     inventory (Plan J).
   - `Action::Magic` → magic use (Plan K).
   - `Action::Talk` → proximity dialogue (Plan L).
   - `Action::Buy` → shop (Plan M).

**Files touched:** `ecs/scene.rs`, `ecs/resources.rs` (add MenuState),
`game/menu.rs` (read-only).

**Verify:** compile + manual smoke: pressing I opens Items menu; buttons appear
in HI bar; Escape closes.

---

## Plan J — Inventory Screen + Item Use

**Dependency:** Plan I (menu must be wired to reach inventory).

**What to build:**

1. `src/game/ecs/systems/item.rs` — implement `run()` fully:
   - Item pickup: add item to `Inventory.stuff`, despawn ground entity, emit
     `SfxEvent` + `MessageEvent` (use `game_lib.narr` via
     `crate::game::events::event_msg`).
   - Body search: transfer gold + weapon from `Bones` component into hero
     inventory; despawn `Bones` entity; emit message.

2. Inventory overlay rendering — new `EcsScene::render_inventory()` method:
   - Port the inventory grid drawing from `gameplay_scene/rendering.rs`
     (item icons drawn from sprite sheet, slot highlight, count labels using
     topaz font).
   - Only render when `menu.mode == MenuMode::Items` (or Use/Give/Keys).

3. Item use dispatch in `dispatch_menu_action()`:
   - Food (stuff[0]) → restore hunger; emit message from `narr.event_msg`.
   - Arrows (stuff[1..2]) → refill quiver (stuff[35]).
   - Use key / staff / ring → delegate to magic dispatch (Plan K).
   - Give → transfer item to nearest eligible NPC (future plan).

**Files touched:** `systems/item.rs`, `ecs/scene.rs` (render_inventory,
dispatch_menu_action).

**Verify:** unit test item pickup; manual test inventory screen opens with I.

---

## Plan K — Magic System (ECS)

**Dependency:** Plan I (menu) + Plan J (inventory for items).

**What to build:**

Migrate `src/game/magic.rs` to accept `(&mut World, &mut Resources)` instead of
`&mut GameState`.  The business logic is fully correct — only the data
extraction needs updating.

1. Add `pub fn magic_dispatch_ecs(item_slot: usize, world: &mut World, res:
   &mut Resources) -> MagicResult` to `src/game/magic.rs` (or a new
   `src/game/ecs/systems/magic.rs`):
   - Read `Inventory` from hero entity.
   - Read spell timers from `res.clock` (light_timer, secret_timer,
     freeze_timer).
   - Apply result: update timers in `res.clock`, emit `SfxEvent`,
     `MessageEvent`.

2. Wire into `dispatch_menu_action()` for Magic menu selection.

3. VFX placeholders in `resources.rs` — replace `WitchEffectPlaceholder` and
   `TeleportEffectPlaceholder` with real types from `src/game/gfx_effects.rs`.

**Files touched:** `game/magic.rs`, `ecs/scene.rs`, `ecs/resources.rs`.

**Verify:** unit test — using Green Jewel (stuff[10]) sets `light_timer > 0`.

---

## Plan L — NPC Dialogue + Narrative Queue

**Dependency:** Plan I (menu for Talk action).

**What to build:**

1. `src/game/ecs/resources.rs` — add `NarrativeQueue`:
   ```rust
   pub struct NarrativeQueue {
       pub pending: VecDeque<NarrEvent>,
       pub active:  Option<NarrEvent>,
       pub timer:   u32,
   }
   pub enum NarrEvent { Placard(String), Speech(String) }
   ```

2. `src/game/ecs/systems/narrative.rs` — implement `run()`:
   - Pop from `NarrativeQueue.pending` when `active` is `None`.
   - Decrement `timer`; on expiry advance to next item.
   - Set a `res.view.viewstatus` flag so the render pass knows to draw the
     placard overlay.

3. `src/game/ecs/scene.rs` — `render_placard()`:
   - Port placard drawing from `gameplay_scene/rendering.rs`
     (dark overlay + topaz font centered text).
   - Only render when `res.narr_queue.active.is_some()`.

4. Proximity speech and Talk menu — feed `SpeechEvent` resolution into
   `NarrativeQueue` rather than directly into the scroll messages.

**Files touched:** `ecs/resources.rs`, `systems/narrative.rs`, `ecs/scene.rs`.

**Verify:** unit test — pushing a `NarrEvent::Placard` into the queue renders
it on the next frame tick.

---

## Plan M — Shop System (ECS)

**Dependency:** Plan I (Buy menu), Plan J (inventory for item delivery).

**What to build:**

Migrate `src/game/shop.rs` to accept `(&mut World, &mut Resources)`.

1. Add `pub fn buy_slot_ecs(slot: u8, world: &mut World, res: &mut Resources)
   -> BuyResult` to `src/game/shop.rs`:
   - Read `HeroStats.wealth` from hero entity.
   - Deduct price; add item to `Inventory.stuff`.
   - Emit `SfxEvent` + `MessageEvent`.

2. Wire into `dispatch_menu_action()` for Buy menu selection.

3. Proximity guard — Buy menu only available when hero is within range of a
   bartender NPC (same logic as old scene: check for NPC race == BARTENDER in
   proximity results).

**Files touched:** `game/shop.rs`, `ecs/scene.rs`.

**Verify:** unit test — `buy_slot_ecs(0, ...)` with sufficient wealth adds food
to inventory and decrements wealth.

---

## Plan N — Combat System (Melee)

**Dependency:** none (can be built independently; will integrate once Plan I
wires the attack action).

**What to build:**

`src/game/ecs/systems/combat.rs` — implement `run()`:
- Detect hero in `ActorState::Fighting(frame)` (emitted by input when attack
  key is pressed).
- For each `Enemy` entity within weapon range (weapon-dependent):
  - Compute hit probability from `brave + weapon bonus - enemy defense`.
  - Roll; on hit emit `DamageEvent`.
- Process `DamageEvent` queue:
  - Reduce target's `Health.vitality`.
  - Set target `CombatState.state = ActorState::Dying` if vitality ≤ 0.
  - Emit `SfxEvent` for hit sound.
- Enemy counter-attack: if enemy is `ActorState::Fighting` and hero is in range,
  apply reverse damage roll against hero `HeroStats.vitality`.

Reference: `src/game/combat.rs` (legacy) for formulas; spec
`docs/spec/combat.md`.

**Files touched:** `systems/combat.rs`, `ecs/components.rs` (ensure `Health`
component exists on enemies), `ecs/events.rs`.

**Verify:** unit tests for hit/miss probability, damage application, death
transition.

---

## Plan O — Encounter System

**Dependency:** Plan G (region loaded) + Plan N (combat) for meaningful
encounters.

**What to build:**

`src/game/ecs/systems/encounter.rs` — implement `run()`:
- Expose `src/game/encounter.rs` tables through `Resources` (add
  `res.encounter.table: &'static EncounterTable` populated on region load).
- Random encounter trigger: check terrain type under hero + daynight bucket →
  probability roll → spawn enemy group.
- Spawn logic: call `spawn::spawn_enemy()` for each enemy in the group with
  position scattered around the hero.
- Set `res.region.encounter_type` and `res.encounter.in_encounter_zone = true`.

Reference: `src/game/encounter.rs` (legacy) + `docs/spec/ai-encounters.md`.

**Files touched:** `systems/encounter.rs`, `ecs/resources.rs`, `ecs/spawn.rs`.

**Verify:** unit test — encounter trigger on a known terrain type spawns expected
enemy count.

---

## Plan P — Carrier / Transport System

**Dependency:** none; standalone.

**What to build:**

`src/game/ecs/systems/carrier.rs` — implement `run()`:
- Detect hero `CarrierMount.active_carrier > 0`.
- Raft (carrier 1): move with current; terrain collision on water/land boundary;
  mount/dismount on action key.
- Swan (carrier 3): flying movement; `CarrierMount.flying` flag; auto-dismount
  on non-flyable terrain.
- Carrier sprite rendering: `blit_actors_inner` already loads sprite sheets 4–5
  (raft/turtle); add `Carrier` entity blit pass.

Reference: `src/game/gameplay_scene/carriers.rs` (legacy) +
`docs/spec/carriers.md`.

**Files touched:** `systems/carrier.rs`, `ecs/scene.rs` (carrier blit).

**Verify:** unit tests for mount/dismount state transitions.

---

## Plan Q — Sleep System

**Dependency:** none; standalone.

**What to build:**

New `src/game/ecs/systems/sleep.rs`:
- `SleepState` component (ticks remaining, recovery rate).
- `run()`: if `res.encounter.sleeping`:
  - Advance `clock.daynight` by 64× per tick (time compression).
  - Recover `HeroStats.hunger` and `HeroStats.fatigue` at accelerated rate.
  - Wake-up triggers: enemy proximity (from `res.region.battleflag`),
    `clock.daynight` crossing a period boundary, explicit wake action.
  - On wake: clear `res.encounter.sleeping`.

Wire into `run_tick()` — the `if res.encounter.sleeping { sleep::run(); return; }` skeleton already exists in `scene.rs`.

Reference: `docs/spec/survival.md` + legacy `gameplay_scene/sleep.rs`.

**Files touched:** new `systems/sleep.rs`, `systems/mod.rs`, `ecs/scene.rs` (remove skip comment).

**Verify:** unit test — 64 sleep ticks advance `daynight` by 64 × 64 = 4096
ticks; hunger decreases.

---

## Plan R — Brother Succession

**Dependency:** Plan G (region loaded for successor spawn position).

**What to build:**

`src/game/ecs/scene.rs` — drain `BrotherDiedEvent` in `update()` (after ticks):
- Spawn `Bones` entity at death location with dead brother's `Inventory`.
- Determine next living brother (scan `inactive_inventories` + existing `Bones`
  entities).
- If a successor exists:
  - Despawn current hero entity.
  - `spawn_hero()` for the successor at their safe point or starting coords.
  - Update `res.hero_entity`, `res.brother.active_brother`, `res.brother.brother`.
  - Load successor's inventory from `res.brother.inactive_inventories`.
- If no successor: set a game-over flag → scene returns `SceneResult::GameOver`.

Wire `SceneResult::GameOver` handling in `main.rs` → return to title/intro.

Reference: `docs/spec/death-revival.md` + `reference/logic/brother-succession.md` on research branch.

**Files touched:** `ecs/scene.rs`, `ecs/spawn.rs` (spawn_bones), `game/scene.rs`
(SceneResult enum), `main.rs`.

**Verify:** unit test — hero at vitality 0, goodfairy countdown expires →
`BrotherDiedEvent` processed → new hero entity spawned with correct inventory.

---

## Plan S — SetFig Sprite Rendering

**Dependency:** Plan G (setfigs are loaded per-region).

**What to build:**

`src/game/ecs/scene.rs` — `blit_actors_inner()`:
- Add a third query pass for `(&SetFig, &Position, &SpriteRef)` entities.
- `SpriteRef.cfile` indexes into `res.sprites.sheets` (13–17 for setfigs).
- Setfigs are stationary; frame = 0 (single idle frame) unless the setfig type
  has a walk cycle (check original sprite data).
- `npc_type_to_cfile()` already returns `None` for `NPC_TYPE_HUMAN` setfigs;
  change it to return `Some(13 + setfig_variant)` based on `SpriteRef`.

Reference: old `gameplay_scene/rendering.rs` setfig blit block;
`docs/spec/characters-animation.md`.

**Files touched:** `ecs/scene.rs` (blit_actors_inner, npc_type_to_cfile),
`ecs/components.rs` (SpriteRef — verify field layout).

**Verify:** setfig entities blitted at correct world position.

---

## Plan T — Weapon Overlays

**Dependency:** Plan N (combat, so weapons are meaningful).

**What to build:**

`src/game/ecs/scene.rs` — `blit_actors_inner()`, after the body blit:
- Read `CombatState.weapon` from the hero entity.
- Look up `STATELIST[body_frame]` for `wpn_x`, `wpn_y`, `figure` offsets.
- For weapon > 0: blit weapon sprite at `(rel_x + wpn_x, rel_y + wpn_y)` using
  weapon sprite sheet (sprite cfile for weapons is cfile 3).
- Bow walk-cycle: use `BOW_X[cycle % 32]` / `BOW_Y[cycle % 32]` offset tables
  from `sprites.rs` instead of `wpn_x`/`wpn_y` when weapon == ITEM_BOW.

Reference: `src/game/sprites.rs` STATELIST, BOW_X/BOW_Y; old
`gameplay_scene/rendering.rs` weapon overlay block.

**Files touched:** `ecs/scene.rs` (blit_actors_inner).

**Verify:** hero with sword equipped shows sword sprite offset correctly.

---

## Plan U — Save/Load Key Binding + Game Menu

**Dependency:** Plan I (Game sub-menu must be open for Save/Load to be
accessible); `ecs_save_game`/`ecs_load_game` already exist.

**What to build:**

1. `src/main.rs` — add `F5` / `F9` key handling in the event loop:
   ```rust
   Keycode::F5 => { /* save slot 0 */ }
   Keycode::F9 => { /* load slot 0 */ }
   ```
   Call `persist::ecs_save_game(ecs, 0)` / `persist::ecs_load_game(0, ecs)`.

2. `dispatch_menu_action()` — wire `Action::Game` sub-menu:
   - "Save" → `persist::ecs_save_game(self, slot)`.
   - "Load" → `persist::ecs_load_game(slot, self)`.
   - "Quit" → set a quit flag consumed by `update()` returning
     `SceneResult::Exit`.

3. `main.rs` — fix the two `TODO(Plan D)` stubs:
   - Victory detection: check `res.region.princess` counter in `EcsScene`.
   - Hero name: read `BrotherKind.id` → index into `game_lib.brothers[id].name`.

**Files touched:** `main.rs`, `ecs/scene.rs`.

**Verify:** F5 creates a save file; F9 restores position.

---

## Plan V — Quest State + Tracking

**Dependency:** Plan G (region) + Plan L (narrative events announce quest
progress).

**What to build:**

1. `src/game/ecs/resources.rs` — add `QuestState`:
   ```rust
   pub struct QuestState {
       pub princess_captive:   bool,
       pub princess_rescues:   u16,
       pub statues_collected:  u8,
       pub has_writ:           bool,
       pub has_talisman:       bool,
   }
   ```

2. Quest event hooks in existing systems:
   - `item::run()` — on picking up statue/writ/talisman: update `QuestState`.
   - `zone::run()` — on entering rescue zone: set `princess_rescues += 1`.
   - `death::run()` — on game-over with all rescues complete: victory flag.

3. `persist.rs` — add `QuestState` fields to `ecs_to_proto` / `proto_to_ecs`
   (reuse existing `princess` proto field; add `statues`, `has_writ`,
   `has_talisman` proto fields).

4. Feed `QuestState` into `DebugSnapshot` in `main.rs`.

**Files touched:** `ecs/resources.rs`, `systems/item.rs`, `systems/zone.rs`,
`persist.rs`, `main.rs`, `proto/faery_save.proto`.

**Verify:** unit test — picking up a statue item increments
`QuestState.statues_collected`.

---

## Plan W — Debug TUI: Eliminate DebugSnapshot + Actor Watch

**Dependency:** none; standalone.

### Why does the debug TUI still use a snapshot?

One of the goals of moving to ECS was to eliminate the `DebugSnapshot` push
model — instead of serializing game state into a bag-of-scalars every frame,
the TUI could read ECS data directly.  The snapshot exists today because
`DebugConsole` (crossterm TUI) and `EcsScene` are both owned by `main.rs` and
both need to be mutably borrowed in the same frame: `dc.drain_commands()` takes
`&mut DebugConsole` while the ECS tick takes `&mut EcsScene`.  Rust cannot hold
both borrows simultaneously, so `main.rs` copies data out of the ECS into a
`DebugSnapshot` and hands that to the console.

**Is there a better architecture?**  Yes — pass a read-only view of the ECS
*into* `DebugConsole::render()` and `DebugConsole::update_status()` at call
sites, instead of a pre-built snapshot.  This requires refactoring
`DebugConsole` to hold `&EcsScene` or equivalent for the duration of `render()`,
which is possible because `render()` only reads.  However:

- `DebugConsole::render()` is called after the ECS tick (no overlap).
- The existing `DebugSnapshot` struct is only ~130 lines; the copy cost is
  negligible.
- The real win from ECS was eliminating `GameState` (a 400-field God Object),
  not eliminating the TUI snapshot.

**Decision for this plan:** keep `DebugSnapshot` as the bridge type — it is
small, cheap to copy, and avoids complex lifetime threading through the TUI
crate.  The snapshot fields that are currently missing (actors, quest state,
hero extras) will be populated from the ECS world before `update_status()` is
called.  If a future plan wants to refactor to a direct reference, that is a
separate architectural decision.

### What to build

`src/main.rs` — in the ECS `DebugSnapshot` construction block (the block that
calls `dc.update_status(status)`), populate the currently-empty fields:

**Actor watch (`actors: Vec<ActorSnapshot>`):**
- Query `(&Enemy, &Position, &Facing, &EnemyKind, &CombatState, &Health)` —
  build one `ActorSnapshot` per entity (up to 19 slots; slot 0 is hero).
- Query hero entity for slot 0 (Position, Facing, HeroStats, CombatState).
- Cap at 20 total entries to match the spec limit.
- Populate all `ActorSnapshot` fields using the existing bridge helpers in
  `src/game/debug_tui/bridge.rs`: `actor_state_u8`, `actor_state_name`,
  `facing_name`, `weapon_short_name`, `race_label`, `goal_name`, `tactic_name`.

**Hero top-row extras (currently zeroed):**
- `max_vitality`: `15 + brave/4` (from `HeroStats`).
- `hero_weapon` / `hero_weapon_name`: from `CombatState.weapon` +
  `weapon_short_name()`.
- `hero_state_u8` / `hero_state_name`: from `CombatState.state`.
- `hero_facing`: from `Facing` component.
- `hero_environ`: from `ActorMotion.environ`.
- `active_carrier` / `active_carrier_name`: from `CarrierMount.active_carrier`
  + `carrier_name()`.
- `jewel_timer` (Green Jewel): from `res.clock.light_timer`.
- `orb_timer` (Crystal Orb): from `res.clock.secret_timer`.
- `freeze_timer` (Gold Ring): from `res.clock.freeze_timer`.

**Quest state (currently zeroed):** wire from `QuestState` resource once Plan V
lands; leave zeroed until then.

Reference: `docs/DEBUG_SPECIFICATION.md` §DebugSnapshot Data Model.

**Files touched:** `src/main.rs` only.

**Verify:** open debug console in-game, run `/actors` — hero and any spawned
enemies appear with correct coordinates, facing, and weapon.

---

## Plan X — Legacy Code Cleanup

**Dependency:** all plans A–W complete (or at least G–U so every legacy caller
has an ECS replacement).

**What to remove / migrate:**

1. `src/game/game_state.rs` — deprecate after verifying no live callers remain.
   Callers to replace:
   - `src/game/combat.rs` → Plan N provides ECS combat
   - `src/game/magic.rs` → Plan K provides ECS magic
   - `src/game/shop.rs` → Plan M provides ECS shop
   - `src/game/loot.rs` → Plan J/O provide ECS loot
   - `src/game/hiscreen.rs` → already replaced by `render_hibar` in `scene.rs`
   - `src/game/npc_ai.rs` test helpers → migrate to ECS test helpers

2. `src/game/persist.rs` — remove `state_to_proto`, `load_from_path` (GameState
   versions) and their tests once all save/load goes through ECS path.

3. `src/game/ecs/systems/input.rs` — the stub module; the real input is handled
   directly in `EcsScene::handle_event()`. Either implement it or delete the
   module and remove it from `mod.rs`.

4. Render system stubs (`render/palette.rs`, `render/sprite.rs`,
   `render/hibar.rs`) — either implement them or remove them; current rendering
   lives directly in `scene.rs`.

**Files touched:** many; only safe after all gameplay paths are on ECS.

---

## Execution order

```
G (region) → H (doors) → I (menus) → J (inventory) → J2 (item text + food fix) → K (magic)
                                   → L (narrative)
                                   → M (shop, after J2 frees BuyItem)
           → Z (setfig spawn cfile fix)
           → S (setfig sprites)
           → R (succession)

N (combat, independent) → O (encounters)
                        → T (weapon overlays)

P (carriers, independent)
Q (sleep, independent)
U (save/load keys, after I)
V (quests, after G + L)
W (debug actor watch, independent)

X (cleanup, after all above)
```

---

## What makes each plan independently executable

Each plan above:
- Names the **exact files to modify** and the **exact functions to add/change**.
- Names the **reference document** (spec file or legacy source file) for
  behavior details.
- Defines a **verify step** (unit test or manual smoke test).
- Lists its **dependencies** explicitly so context can be reconstructed without
  reading other plan files.

When starting a new plan, the agent needs only:
1. This TODO file.
2. The spec files referenced in that plan's section.
3. The source files listed under "Files touched."
