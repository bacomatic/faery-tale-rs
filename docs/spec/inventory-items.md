## 14. Inventory & Items

### 14.1 Inventory Structure

Three static arrays per brother, plus a pointer for the active brother. Each array is sized to include both the 35 active inventory slots **and** the 1-byte `ARROWBASE` accumulator, i.e. 36 bytes in memory. The save-file payload serializes only the first 35 bytes (see §24).

```
UBYTE julstuff[ARROWBASE + 1], philstuff[ARROWBASE + 1], kevstuff[ARROWBASE + 1]; // 36 bytes each
UBYTE *stuff; // bound to current brother via blist[brother-1].stuff
```

`stuff[]` layout (indices 0–34 active, index 35 as temporary accumulator):

| Index | Category | Item |
|-------|----------|------|
| 0 | Weapon | Dirk |
| 1 | Weapon | Mace |
| 2 | Weapon | Sword |
| 3 | Weapon | Bow |
| 4 | Weapon | Magic Wand |
| 5 | Special | Golden Lasso |
| 6 | Special | Sea Shell |
| 7 | Special | Sun Stone |
| 8 | Special | Arrows (integer count, max display 45) |
| 9 | Magic | Blue Stone |
| 10 | Magic | Green Jewel |
| 11 | Magic | Glass Vial |
| 12 | Magic | Crystal Orb |
| 13 | Magic | Bird Totem |
| 14 | Magic | Gold Ring |
| 15 | Magic | Jade Skull |
| 16 | Key | Gold Key |
| 17 | Key | Green Key |
| 18 | Key | Blue Key |
| 19 | Key | Red Key |
| 20 | Key | Grey Key |
| 21 | Key | White Key |
| 22 | Quest | Talisman (win condition) |
| 23 | Quest | Rose (lava immunity) |
| 24 | Quest | Fruit (portable food) |
| 25 | Quest | Gold Statue (5 needed for desert gate) |
| 26 | Quest | Book (vestigial — not obtainable) |
| 27 | Quest | Herb (vestigial — not obtainable) |
| 28 | Quest | Writ (royal commission) |
| 29 | Quest | Bone |
| 30 | Quest | Crystal Shard (terrain 12 bypass) |
| 31–34 | Gold pickup | Values 2, 5, 10, 100 → added to `wealth` |

Constants: `MAGICBASE = 9`, `KEYBASE = 16`, `STATBASE = 25`, `GOLDBASE = 31`, `ARROWBASE = 35`.

On brother succession (`revive(TRUE)`): all items wiped, starting loadout is one Dirk (`stuff[0] = 1`). The `stuff` pointer is rebound to the new brother.

### 14.2 Weapon Details

| Index | Item | Melee Damage | Notes |
|-------|------|-------------|-------|
| 0 | Dirk | 1–3 | Starting weapon for each brother |
| 1 | Mace | 2–4 | Purchasable (30 gold) |
| 2 | Sword | 3–5 | Purchasable (45 gold) |
| 3 | Bow | 4–11 (missile) | Purchasable (75 gold); consumes `stuff[8]` per shot; auto-switches to next best weapon on depletion |
| 4 | Magic Wand | 4–11 (missile) | Fires fireballs; no ammo cost |

Equip via USE menu: `weapon = hit + 1`.

### 14.3 Special Items

| Index | Item | Effect |
|-------|------|--------|
| 5 | Golden Lasso | Enables mounting the swan carrier. Dropped by Witch (race `0x89`) on death. Requires Sun Stone first (witch must be killable). |
| 6 | Sea Shell | USE calls `get_turtle()` to summon sea turtle carrier near water. Blocked inside rectangle `(11194–21373, 10205–16208)`. Obtained from turtle NPC dialogue or ground pickup at `(10344, 36171)` in ob_list2/ob_list8. |
| 7 | Sun Stone | Makes Witch (race `0x89`) vulnerable to melee. Without it, attacks produce `speak(58)`. Ground pickup at `(11410, 36169)` in ob_list8. |
| 8 | Arrows | Integer count (max display 45). Consumed by bow (`stuff[8]--` per shot). Purchased in batches of 10 for 10 gold. |

### 14.4 Magic Consumables (`MAGICBASE = 9`)

All consumed on use (`--stuff[4+hit]`). Guarded by `extn->v3 == 9` check — magic does not work in certain restricted areas.

| Index | Item | Effect |
|-------|------|--------|
| 9 | Blue Stone | Teleport via stone circle (only at sector 144) |
| 10 | Green Jewel | `light_timer += 760` — temporary light effect brightening dark outdoor areas |
| 11 | Glass Vial | Heal: `vitality += rand8() + 4` (4–11), capped at `15 + brave/4` |
| 12 | Crystal Orb | `secret_timer += 360` — reveals hidden passages |
| 13 | Bird Totem | Renders overhead map with player position |
| 14 | Gold Ring | `freeze_timer += 100` — freezes all enemies (disabled while riding) |
| 15 | Jade Skull | Kill spell: kills all visible enemies with `vitality > 0`, `type == ENEMY`, `race < 7`. Decrements `brave` per kill — the only item that reduces bravery. |

### 14.5 Quest & Stat Items

| Index | Item | Effect |
|-------|------|--------|
| 22 | Talisman | Win condition: collecting triggers end sequence. Dropped by Necromancer (race `0x09`) on death; Necromancer transforms to normal man (race 10) and speaks `speak(44)`. |
| 23 | Rose | Lava immunity: forces `environ = 0` in fiery_death zone (`map_x` 8802–13562, `map_y` 24744–29544). Without it, `environ > 15` kills instantly; `environ > 2` drains vitality per tick. Protects player only (actor 0), not carriers or NPCs. Ground pickup at `(5473, 38699)`. |
| 24 | Fruit | Auto-consumed when `hunger > 30` at safe points, reducing hunger by 30. On pickup: stored only when `hunger < 15`; otherwise eaten immediately via `eat(30)`. 10 fruits placed in ob_list8. |
| 25 | Gold Statue | Desert gate key: need 5 to access Azal. Dual-gated: DESERT door type blocked when `stuff[25] < 5`, AND region 4 map tiles overwritten to impassable sector 254 at load time. |
| 26 | Book | Vestigial — defined in inventory system but no world placement, no handler, not obtainable. |
| 27 | Herb | Vestigial — defined in inventory system but no world placement, no handler, not obtainable. |
| 28 | Writ | Royal commission: obtained from `rescue()` after saving princess. Grants `princess++`, 100 gold, and 3 of each key type (`stuff[16..21] += 3`). Shown to Priest triggers `speak(39)` and reveals gold statue (`ob_listg[10]` set to stat 1). GIVE menu entry exists but has no handler — the Writ functions only as a passive dialogue check. |
| 29 | Bone | Found underground at `(3723, 39340)` in ob_list9. Given to Spectre (race `0x8a`): `speak(48)`, drops crystal shard. Non-spectre NPCs reject with `speak(21)`. |
| 30 | Crystal Shard | Overrides terrain type 12 collision blocking. Type-12 walls appear only in terra set 8 (Region 8 building interiors) — specifically the maze-layout sectors 2, 3, 5–9, 11–12, 35 and Doom Tower sectors 137–138 adjacent to the Stargate portal. Terra set 10 (Region 9 dungeons) is unaffected — the same tile index 93 maps to terrain type 1 there. Never consumed. Obtained from Spectre trade. |

#### Gold Statue Locations

All 5 statues use object ID `STATUE` (149), mapped to `stuff[25]` via `itrans`:

| # | Source | Location | How Obtained |
|---|--------|----------|-------------|
| 1 | `ob_listg[6]` | Seahold `(11092, 38526)` | Ground pickup (ob_stat=1) |
| 2 | `ob_listg[7]` | Ogre Den `(25737, 10662)` | Ground pickup (ob_stat=1) |
| 3 | `ob_listg[8]` | Octagonal Room `(2910, 39023)` | Ground pickup (ob_stat=1) |
| 4 | `ob_listg[9]` | Sorceress `(12025, 37639)` | Talk to Sorceress — revealed on first visit (stat set to 1) |
| 5 | `ob_listg[10]` | Priest `(6700, 33766)` | Show Writ to Priest — `speak(39)`, requires `stuff[28]` |

### 14.6 Gold Handling

Gold pickup items (stuff[31–34]) have `maxshown` values (2, 5, 10, 100) added to the `wealth` variable instead of `stuff[]`. Gold bag world object (ob_id 13) adds 50 to wealth directly as a special-cased pickup.

### 14.7 `itrans` — World Object to Inventory Mapping

31 `(ob_id, stuff_index)` pairs, terminated by `(0, 0)`. Lookup is a linear scan.

| World Object ID | Name | → stuff[] Index | Inventory Item |
|-----------------|------|----------------|----------------|
| 11 (QUIVER) | Quiver | 35 | Arrows (×10 via ARROWBASE accumulator) |
| 18 (B_STONE) | Blue Stone | 9 | Blue Stone |
| 19 (G_JEWEL) | Green Jewel | 10 | Green Jewel |
| 22 (VIAL) | Glass Vial | 11 | Glass Vial |
| 21 (C_ORB) | Crystal Orb | 12 | Crystal Orb |
| 23 (B_TOTEM) | Bird Totem | 13 | Bird Totem |
| 17 (G_RING) | Gold Ring | 14 | Gold Ring |
| 24 (J_SKULL) | Jade Skull | 15 | Jade Skull |
| 145 (M_WAND) | Magic Wand | 4 | Magic Wand |
| 27 | — | 5 | Golden Lasso |
| 8 | — | 2 | Sword |
| 9 | — | 1 | Mace |
| 12 | — | 0 | Dirk |
| 10 | — | 3 | Bow |
| 147 (ROSE) | Rose | 23 | Rose |
| 148 (FRUIT) | Fruit | 24 | Fruit |
| 149 (STATUE) | Gold Statue | 25 | Gold Statue |
| 150 (BOOK) | Book | 26 | Book |
| 151 (SHELL) | Sea Shell | 6 | Sea Shell |
| 155 | — | 7 | Sun Stone |
| 136 | — | 27 | Herb |
| 137 | — | 28 | Writ |
| 138 | — | 29 | Bone |
| 139 | — | 22 | Talisman |
| 140 | — | 30 | Crystal Shard |
| 25 (GOLD_KEY) | Gold Key | 16 | Gold Key |
| 153 (GREEN_KEY) | Green Key | 17 | Green Key |
| 114 (BLUE_KEY) | Blue Key | 18 | Blue Key |
| 242 (RED_KEY) | Red Key | 19 | Red Key |
| 26 (GREY_KEY) | Grey Key | 20 | Grey Key |
| 154 (WHITE_KEY) | White Key | 21 | White Key |

### 14.8 Special-Cased Pickups

These bypass `itrans` in the Take handler:

| ob_id | Item | Special Handling |
|-------|------|-----------------|
| 13 (MONEY) | Gold bag | `wealth += 50` |
| 20 (SCRAP) | Scrap | `event(17)` + region-specific event |
| 28 | Dead brother bones | Recovers dead brother's full inventory |
| 15 (CHEST) | Chest | Container → random loot (see §14.10) |
| 14 (URN) | Brass urn | Container → random loot |
| 16 (SACKS) | Sacks | Container → random loot |
| 102 (TURTLE) | Turtle eggs | Cannot be taken |
| 31 (FOOTSTOOL) | Footstool | Cannot be taken |

### 14.9 Shop System (`jtrans`)

7 purchasable items defined as `(stuff_index, price)` pairs. Requires proximity to shopkeeper (race `0x88`) and `wealth > price`.

| Menu Label | Item | Price | Effect |
|------------|------|-------|--------|
| Food | (special) | 3 gold | Calls `eat(50)` — reduces hunger by 50, not stored in stuff[] |
| Arrow | Arrows | 10 gold | `stuff[8] += 10` |
| Vial | Glass Vial | 15 gold | `stuff[11]++` |
| Mace | Mace | 30 gold | `stuff[1]++` |
| Sword | Sword | 45 gold | `stuff[2]++` |
| Bow | Bow | 75 gold | `stuff[3]++` |
| Totem | Bird Totem | 20 gold | `stuff[13]++` |

Menu label string: `"Food ArrowVial Mace SwordBow  Totem"`.

### 14.10 Container Loot

When a container (chest, urn, sacks) is opened, `rand4()` determines the tier:

| Roll | Result | Details |
|------|--------|---------|
| 0 | Nothing | "nothing." |
| 1 | One item | `rand8() + 8` → indices 8–15 (arrows or magic items). Index 8 → quiver. |
| 2 | Two items | Two different random items from same range. Index 8 → 100 gold. |
| 3 | Three of same | Three copies of one item. Index 8 → 3 random keys (`KEYBASE` to `KEYBASE+5`). |

### 14.11 GIVE Mode

| Menu Hit | Action |
|----------|--------|
| Gold | Give 2 gold to NPC. `wealth -= 2`. If `rand64() > kind`, `kind++`. Beggars (`0x8d`) give goal speech. |
| Writ | Handled via TALK/Priest interaction, not the GIVE handler. |
| Bone | Give to Spectre (`0x8a`): `speak(48)`, drops crystal shard via `leave_item()`. Non-spectre NPCs: `speak(21)`. |

### 14.12 Menu System

#### Menu Modes

```
enum cmodes {ITEMS=0, MAGIC, TALK, BUY, GAME, SAVEX, KEYS, GIVE, USE, FILE};
```

Menu item availability managed by `set_options()`, which calls `stuff_flag(index)` — returns 10 (enabled) if `stuff[index] > 0`, else 8 (disabled). The Book in the GIVE menu is hardcoded disabled: `menus[GIVE].enabled[6] = 8`.

### 14.13 World Object Structure

Each world object is a 6-byte record (`struct object`):

| Field | Type | Purpose |
|-------|------|---------|
| `xc` | `u16` | World X coordinate (pixel-space, 0–65535) |
| `yc` | `u16` | World Y coordinate (pixel-space, 0–65535) |
| `ob_id` | `i8` | Object type identifier (see §14.15) |
| `ob_stat` | `i8` | Object status code |

#### ob_stat Values

| Value | Meaning | Rendering |
|-------|---------|-----------|
| 0 | Disabled | Skipped |
| 1 | On ground (pickable) | OBJECTS type, `race=1` |
| 2 | In inventory / taken | Skipped |
| 3 | Setfig (NPC character) | NPC with `state=STILL` |
| 4 | Dead setfig | NPC with `state=DEAD` |
| 5 | Hidden (revealed by Look) | OBJECTS type, `race=0` |
| 6 | Cabinet item | OBJECTS type, `race=2` |

The `race` values on OBJECTS entries encode interaction behavior: `race=0` = not directly pickable (revealed by Look), `race=1` = normal ground item, `race=2` = cabinet item.

### 14.14 Region Object Lists

Objects are organized into one global list (always processed) and 10 regional lists (only the current region's list is processed each tick).

#### ob_listg — Global Objects (11 entries)

| Index | ob_id | Initial ob_stat | Purpose |
|-------|-------|----------------|---------|
| 0 | 0 | 0 | Drop slot — overwritten by `leave_item()` for dynamically dropped items |
| 1 | 28 (bones) | 0 | Dead brother 1 (Julian) — coordinates filled at death |
| 2 | 28 (bones) | 0 | Dead brother 2 (Phillip) — coordinates filled at death |
| 3 | 11 (ghost) | 0 | Ghost brother 1 — activated during succession |
| 4 | 11 (ghost) | 0 | Ghost brother 2 — activated during succession |
| 5 | 10 (spectre) | 3 | Spectre NPC — toggles visibility: `lightlevel < 40` → stat 3, else stat 2 |
| 6 | STATUE (149) | 1 | Gold statue — Seahold |
| 7 | STATUE (149) | 1 | Gold statue — Ogre Den |
| 8 | STATUE (149) | 1 | Gold statue — Octagonal Room |
| 9 | STATUE (149) | 0 | Gold statue — Sorceress (hidden until first talk → stat 1) |
| 10 | STATUE (149) | 0 | Gold statue — Priest (hidden until writ presented → stat 1) |

#### Regional Lists (ob_list0–ob_list9)

Each outdoor region (0–7) has pre-placed objects plus 10 blank slots (`TENBLANKS`) reserved for random treasure. Regions 8 and 9 have larger fixed lists and no random scatter slots.

| Region | List | Initial Count | Description |
|--------|------|---------------|-------------|
| 0 | `ob_list0` | 3 | Snow Land — 3 rangers |
| 1 | `ob_list1` | 1 | Maze Forest North — turtle eggs |
| 2 | `ob_list2` | 5 | Swamp Land — 2 wizards, ranger, sacks, shell |
| 3 | `ob_list3` | 12 | Tambry / Manor area — mixed NPCs and items |
| 4 | `ob_list4` | 3 | Desert — 2 dummies + beggar |
| 5 | `ob_list5` | 5 | Farm & City — beggar, 2 wizards, ring, chest |
| 6 | `ob_list6` | 1 | Lava Plain — dummy object |
| 7 | `ob_list7` | 1 | Southern Mountains — dummy object |
| 8 | `ob_list8` | 77 | Building Interiors — indices 0–15 setfig NPCs, 16–60 ground items, 61–76 hidden Look items (ob_stat=5) |
| 9 | `ob_list9` | 9 | Underground — 4 wands, 2 chests, wizard, money, king's bone |

### 14.15 Object ID Registry

Named constants from the `enum obytes`:

| Value | Constant | Description |
|-------|----------|-------------|
| 0–10 | (setfig NPCs) | Wizard (0), Priest (1), Guard (2/3), Princess (4), King (5), Noble (6), Sorceress (7), Bartender (8), Witch (9), Spectre (10) |
| 11 | `QUIVER` | Quiver of arrows |
| 13 | `MONEY` | 50 gold pieces |
| 14 | `URN` | Brass urn (container) |
| 15 | `CHEST` | Chest (container) |
| 16 | `SACKS` | Sacks (container) |
| 17–24 | `G_RING`..`J_SKULL` | Ring, stone, jewel, scrap, orb, vial, totem, skull |
| 25–26 | `GOLD_KEY`, `GREY_KEY` | Keys |
| 28 | (dead brother) | Brother's bones |
| 29 | 0x1d | Opened/empty chest |
| 31 | `FOOTSTOOL` | Cannot be taken |
| 102 | `TURTLE` | Turtle eggs — cannot be taken |
| 114 | `BLUE_KEY` | Blue key |
| 139 | (talisman) | Dropped by Necromancer on death |
| 140 | (crystal shard) | Dropped when giving bone to Spectre |
| 145–151 | `M_WAND`..`SHELL` | Wand, meal, rose, fruit, statue, book, shell |
| 153–154 | `GREEN_KEY`, `WHITE_KEY` | Keys |
| 242 | `RED_KEY` | Red key |

### 14.16 Object Management Arrays

**`ob_table[10]`**: Maps region numbers to object list pointers (`ob_list0`..`ob_list9`).

**`mapobs[10]`**: Tracks current entry count per region. Initial values: `{ 3, 1, 5, 12, 3, 5, 1, 1, 77, 9 }`. Mutable — incremented as random treasure is distributed.

**`dstobs[10]`**: Tracks whether random treasure has been distributed per region. Initial values: `{ 0, 0, 0, 0, 0, 0, 0, 0, 1, 1 }`. Regions 8 and 9 start as 1 (excluded). Set to 1 after distribution.

### 14.17 Per-Tick Object Processing (`do_objects`)

1. Set `j1 = 2` — starting anim_list index for setfig NPCs (0 = hero, 1 = carrier are reserved)
2. Call `set_objects(ob_listg, glbobs, 0x80)` — process global objects with flag `0x80`
3. Call `set_objects(ob_table[region_num], mapobs[region_num], 0)` — process regional objects
4. If `j1 > 3`, update `anix = j1` — adjusts the setfig/enemy boundary in anim_list

### 14.18 `set_objects` — Region Load and Rendering

**Random treasure distribution**: On first region load (when `dstobs[region_num] == 0` and `new_region >= 10`), 10 random objects are scattered from `rand_treasure[]`:

| Distribution | Items |
|-------------|-------|
| 4/16 | SACKS |
| 3/16 | GREY_KEY |
| 2/16 | CHEST |
| 1/16 each | MONEY, GOLD_KEY, QUIVER, RED_KEY, B_TOTEM, VIAL, WHITE_KEY |

Positions randomized within the region's quadrant, rejecting non-traversable terrain via `px_to_im()`. After distribution, `dstobs[region_num]` is set to 1 and `mapobs[region_num]` incremented per new object.

**Per-object processing**: For each object, performs a screen-bounds check, then:
- **Setfigs** (ob_stat 3/4): Loads sprite via `setfig_table[id]`, creates anim_list entry with `type=SETFIG`, `race = id + 0x80`, `vitality = 2 + id*2`, `goal = i` (list index). Dead setfigs (stat 4) get `state=DEAD`. Witch presence sets `witchflag = TRUE`.
- **Items** (ob_stat 1/5/6): Creates anim_list entry with `type=OBJECTS`, `index=ob_id`, `vitality = i + f` (list index + global flag `0x80`).
- **Resource limit**: Returns early if `anix2 >= 20`.

### 14.19 Object State Mutation (`change_object`)

Decodes the anim_list entry's `vitality` field to locate the original `struct object`: bit 7 selects global vs. regional list (`vitality & 0x80`), bits 0–6 give the list index (`vitality & 0x7f`).

- Normal objects: sets `ob_stat = flag`
- Chests: changes `ob_id` from CHEST (15) to `0x1d` (29, empty chest) instead of modifying `ob_stat`

Callers:
- Take handler: `change_object(nearest, 2)` — marks object as taken
- Look handler: `change_object(i, 1)` — reveals hidden items (ob_stat 5 → 1)

### 14.20 `leave_item` — Drop Item in World

Always uses `ob_listg[0]` (the dedicated drop slot), setting coordinates to the actor's position (+10 Y offset) and `ob_stat = 1`. Only one dynamically dropped item can exist at a time — each call overwrites the previous.

Callers:
- Necromancer death (race `0x09`) → talisman (ob_id 139)
- Witch death (race `0x89`) → golden lasso (ob_id 27)
- Bone given to Spectre → crystal shard (ob_id 140)

### 14.21 Save/Load of Object State

Object state is fully persisted:

1. `ob_listg` — 66 bytes (11 × 6)
2. `mapobs` — 20 bytes (10 × 2, current counts including random additions)
3. `dstobs` — 20 bytes (10 × 2, distribution flags)
4. All 10 regional lists — variable size based on current `mapobs[i]`

---


