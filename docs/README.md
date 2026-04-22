# Faery Tale Adventure Documentation Index

This file is the entry point for all project documentation. Copy this file to any branch or repository, link to it, and agents can navigate the full doc set without extra context.

## How To Use This Index

1. Start with the canonical docs in this order:
   1. ARCHITECTURE.md
   2. RESEARCH.md
   3. STORYLINE.md
   4. PROBLEMS.md
2. Use docs/logic/ for normative pseudo-code behavior.
3. Use docs/world_db.json and docs/quest_db.json for machine-readable data lookup.
4. Use docs/_discovery/ as supporting trace notes, not final truth.

## Canonical Documentation (Primary Sources)

| Document | Purpose |
|---|---|
| [ARCHITECTURE.md](ARCHITECTURE.md) | High-level subsystem architecture, data flow, game-loop structure, display and memory model. |
| [RESEARCH.md](RESEARCH.md) | Ground-truth mechanics reference with citations to original 1987 source lines. |
| [STORYLINE.md](STORYLINE.md) | Narrative progression, quest sequencing, NPC interactions, and story flow diagrams. |
| [PROBLEMS.md](PROBLEMS.md) | Open and resolved research questions that cannot be settled by straightforward source tracing. |
| [world_db.json](world_db.json) | Spatial database of objects, doors, encounter extents, terrain summaries, and region grids. |
| [quest_db.json](quest_db.json) | Machine-readable quest and progression data extracted from source analysis. |

## Logic Specifications (Normative Behavior)

These files define strict pseudo-code for branching logic and runtime behavior.

| Document | Purpose |
|---|---|
| [logic/README.md](logic/README.md) | Logic docs overview, reading order, and function index. |
| [logic/STYLE.md](logic/STYLE.md) | Grammar and writing rules for logic pseudo-code. |
| [logic/SYMBOLS.md](logic/SYMBOLS.md) | Shared symbol registry (enums, globals, constants, references). |
| [logic/game-loop.md](logic/game-loop.md) | Canonical per-frame sequence and tick phases. |
| [logic/input-handling.md](logic/input-handling.md) | Input pipeline, ring buffer handling, and direction decoding. |
| [logic/movement.md](logic/movement.md) | Movement vectors, course-setting, walk/still updates, collision entry points. |
| [logic/terrain-collision.md](logic/terrain-collision.md) | Terrain sampling and collision/proximity logic. |
| [logic/encounters.md](logic/encounters.md) | Encounter spawning, placement, and wilderness roll flow. |
| [logic/ai-system.md](logic/ai-system.md) | AI goals/tactics and per-tick AI state progression. |
| [logic/combat.md](logic/combat.md) | Damage, hits, missiles, death checks, and battle aftermath. |
| [logic/inventory.md](logic/inventory.md) | Take/use/magic/look command behavior and item-state updates. |
| [logic/menu-system.md](logic/menu-system.md) | Menu dispatch and option/key routing behavior. |
| [logic/doors.md](logic/doors.md) | Door detection, transfer, and key-use interaction logic. |
| [logic/day-night.md](logic/day-night.md) | Time-of-day progression, light/fade transitions, and hunger/fatigue ticks. |
| [logic/carrier-transport.md](logic/carrier-transport.md) | Turtle/swan/raft transport behavior and mount/dismount rules. |
| [logic/astral-plane.md](logic/astral-plane.md) | Astral/extent place resolution and related state transitions. |
| [logic/quests.md](logic/quests.md) | Quest-state transitions, rescue flow, drops, and win condition hooks. |
| [logic/npc-dialogue.md](logic/npc-dialogue.md) | NPC talk dispatch and conditional speech logic. |
| [logic/shops.md](logic/shops.md) | Shop purchase dispatch, pricing, and inventory/gold effects. |
| [logic/save-load.md](logic/save-load.md) | Save/load serialization blocks and disk flow behavior. |
| [logic/brother-succession.md](logic/brother-succession.md) | Brother death/succession, revive, and bones pickup logic. |
| [logic/frustration.md](logic/frustration.md) | Frustration-state animation and AI frustration transitions. |
| [logic/visual-effects.md](logic/visual-effects.md) | Palette fades, flips, map messaging, and win-color sequence behavior. |

## Discovery Notes (Raw Trace Artifacts)

These are working research notes produced by exploration waves. They are useful context but are not the final authority over canonical docs.

| Document | Focus |
|---|---|
| [_discovery/high_level_scan.md](_discovery/high_level_scan.md) | Broad inventory of source files, subsystems, and candidate research topics. |
| [_discovery/game-loop.md](_discovery/game-loop.md) | Main loop flow and tick-order observations. |
| [_discovery/actor-state.md](_discovery/actor-state.md) | Actor structs, state fields, and animation-state usage notes. |
| [_discovery/movement.md](_discovery/movement.md) | Direction, movement, and related state-transition tracing. |
| [_discovery/terrain-collision.md](_discovery/terrain-collision.md) | Terrain lookup and collision/proximity behavior findings. |
| [_discovery/encounters.md](_discovery/encounters.md) | Encounter generation and spawning evidence notes. |
| [_discovery/ai-system.md](_discovery/ai-system.md) | AI goal/tactic behavior traces across files. |
| [_discovery/combat.md](_discovery/combat.md) | Damage, hit resolution, death, and weapon behavior notes. |
| [_discovery/inventory.md](_discovery/inventory.md) | Inventory command paths and item interaction tracing. |
| [_discovery/menu-system.md](_discovery/menu-system.md) | Menu/input action dispatch notes. |
| [_discovery/input-handling.md](_discovery/input-handling.md) | Raw input pipeline and key/mouse processing traces. |
| [_discovery/doors.md](_discovery/doors.md) | Door transfer and key/door interaction findings. |
| [_discovery/day-night.md](_discovery/day-night.md) | Day/night progression, light, and periodic effects notes. |
| [_discovery/stats-hunger.md](_discovery/stats-hunger.md) | Hunger, fatigue, stat drift, and related periodic mechanics. |
| [_discovery/carriers.md](_discovery/carriers.md) | Carrier entities (bird/turtle/dragon) behavior notes. |
| [_discovery/carrier-transport-system.md](_discovery/carrier-transport-system.md) | Transport/mounting system mechanics and movement exceptions. |
| [_discovery/astral-plane.md](_discovery/astral-plane.md) | Astral-plane and zone-specific transition findings. |
| [_discovery/world-objects.md](_discovery/world-objects.md) | World object lists, placement, and object-state notes. |
| [_discovery/npc-quests.md](_discovery/npc-quests.md) | NPC quest triggers and progression conditions. |
| [_discovery/brother-succession.md](_discovery/brother-succession.md) | Brother lifecycle and succession behavior traces. |
| [_discovery/win-condition.md](_discovery/win-condition.md) | Endgame trigger and completion-state findings. |
| [_discovery/visual-effects.md](_discovery/visual-effects.md) | Fade, palette, and visual-transition mechanics notes. |
| [_discovery/display-system.md](_discovery/display-system.md) | Display/page/bitplane rendering architecture notes. |
| [_discovery/map-rendering.md](_discovery/map-rendering.md) | Map draw and scrolling pipeline findings. |
| [_discovery/sprite-compositing.md](_discovery/sprite-compositing.md) | Sprite mask/blit/composition behavior notes. |
| [_discovery/text-display.md](_discovery/text-display.md) | Text rendering, placards, and dialogue output behavior. |
| [_discovery/audio.md](_discovery/audio.md) | Music/SFX driver behavior and playback paths. |
| [_discovery/save-load.md](_discovery/save-load.md) | Save/load implementation traces and format observations. |
| [_discovery/disk-io.md](_discovery/disk-io.md) | Disk device/file I/O routing and read/write behavior. |
| [_discovery/disk-layout.md](_discovery/disk-layout.md) | Track/block layout and asset placement findings. |
| [_discovery/iff-loading.md](_discovery/iff-loading.md) | IFF/ILBM parse and unpack behavior notes. |
| [_discovery/rng.md](_discovery/rng.md) | Random generator behavior and period/usage traces. |
| [_discovery/copy-protection.md](_discovery/copy-protection.md) | Copy-protection questions/prompts and check-path notes. |
| [_discovery/frustration-mechanics.md](_discovery/frustration-mechanics.md) | Frustration-state triggers and behavior observations. |
| [_discovery/npc-terrain-speed.md](_discovery/npc-terrain-speed.md) | NPC speed interactions with terrain/environment states. |
| [_discovery/turtle-terrain.md](_discovery/turtle-terrain.md) | Turtle movement/terrain interaction specifics. |
| [_discovery/quest-stat-items.md](_discovery/quest-stat-items.md) | Quest-critical item and stat interaction notes. |
| [_discovery/npc-item-location-map.md](_discovery/npc-item-location-map.md) | Cross-map of NPCs, items, and location relationships. |
| [_discovery/narr-asm-complete-message-database.md](_discovery/narr-asm-complete-message-database.md) | Catalog of narrative/message table content from narr.asm. |
| [_discovery/screen-size.md](_discovery/screen-size.md) | Screen-size/viewport handling behavior notes. |
| [_discovery/dark-knight.md](_discovery/dark-knight.md) | Dark Knight-specific encounter/logic trace notes. |

## Visual and Map Assets

| Document | Purpose |
|---|---|
| [overworld.png](overworld.png) | Full overworld reference image used for geography orientation. |
| [region_0.png](region_0.png) | Region 0 reference map image. |
| [region_1.png](region_1.png) | Region 1 reference map image. |
| [region_2.png](region_2.png) | Region 2 reference map image. |
| [region_3.png](region_3.png) | Region 3 reference map image. |
| [region_4.png](region_4.png) | Region 4 reference map image. |
| [region_5.png](region_5.png) | Region 5 reference map image. |
| [region_6.png](region_6.png) | Region 6 reference map image. |
| [region_7.png](region_7.png) | Region 7 reference map image. |
| [region_8.png](region_8.png) | Region 8 reference map image. |
| [region_9.png](region_9.png) | Region 9 reference map image. |

## Agent Navigation Quick Paths

- Mechanics truth: start at [RESEARCH.md](RESEARCH.md), jump to matching [logic/](logic/) file for exact branching behavior.
- Narrative and quest flow: start at [STORYLINE.md](STORYLINE.md), then verify mechanics in [RESEARCH.md](RESEARCH.md).
- Architectural orientation: start at [ARCHITECTURE.md](ARCHITECTURE.md), then drill into specific subsystem docs.
- Uncertain or disputed claim: check [PROBLEMS.md](PROBLEMS.md), then inspect matching discovery artifact in [_discovery/](_discovery/).
- Location-dependent questions: use [world_db.json](world_db.json) first, then corroborate in RESEARCH and logic docs.

## Trust Model

- Final authority for mechanics: RESEARCH.md + docs/logic/*.md
- Final authority for architecture: ARCHITECTURE.md
- Final authority for narrative progression: STORYLINE.md
- Open uncertainties and unresolved edge cases: PROBLEMS.md
- Supporting trace context only: docs/_discovery/*