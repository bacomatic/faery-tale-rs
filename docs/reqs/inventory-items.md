## 11. Inventory & Items

### Requirements

| ID | Requirement |
|----|-------------|
| R-INV-001 | Each brother shall have a `stuff[]` array of 36 elements in memory (indices 0–34 active inventory, index 35 = `ARROWBASE` transient accumulator): weapons (0–4), special items (5–8), magic consumables (9–15), keys (16–21), quest/stat items (22–30), gold pickups (31–34). Index 35 (`ARROWBASE`) shall serve as a temporary accumulator for quiver pickups, with `stuff[8] += stuff[ARROWBASE] * 10` applied after Take. The save-file payload serializes the 35 active slots only (see R-SAVE-003). |
| R-INV-002 | Item pickup shall use the `itrans` translation table (31 ob_id→stuff-index pairs, 0-terminated) to map ground object types to inventory slots. Lookup shall be a linear scan of pairs until the terminator. On match, `stuff[index]` shall be incremented. |
| R-INV-003 | Body search on dead enemies shall roll weapon drop then treasure drop from probability tables. |
| R-INV-004 | Weapon damage ranges: Dirk 1–3, Mace 2–4, Sword 3–5, Bow 4–11 (missile, consumes arrows), Wand 4–11 (missile, no ammo cost). Equipping a weapon via USE shall set `weapon = hit + 1`. |
| R-INV-005 | Inventory state shall be preserved per-brother (separate static arrays for Julian, Phillip, Kevin). The `stuff` pointer shall be bound to the current brother via `blist[brother-1].stuff`. All three inventories shall be saved and loaded. |
| R-INV-006 | Key items (gold, green, blue, red, grey, white at stuff[16–21]) shall be consumed when used to open locked doors. The key handler shall try 9 directions from the hero's position at 16-pixel distance via `doorfind(x, y, keytype)`. |
| R-INV-007 | Bow shall consume one arrow per shot (`stuff[8]--`). When arrows are depleted mid-combat, the system shall auto-switch to the next best weapon. |
| R-INV-008 | All magic consumables (stuff[9–15]) shall be consumed on use (`--stuff[4+hit]`). Magic shall be blocked when `extn->v3 == 9` (restricted areas). |
| R-INV-009 | Magic item effects: Blue Stone teleports via stone circle (sector 144 only). Green Jewel adds 760 to `light_timer`. Glass Vial heals `rand8() + 4` (4–11) vitality, capped at `15 + brave/4`. Crystal Orb adds 360 to `secret_timer`. Bird Totem renders the overhead map with player position. Gold Ring adds 100 to `freeze_timer` (disabled while riding). Jade Skull kills all visible enemies with `vitality > 0`, `type == ENEMY`, `race < 7`, and shall decrement `brave` per kill. |
| R-INV-010 | The shop system (`jtrans`) shall offer 7 items: Food (3g, calls `eat(50)`), Arrows (10g, `stuff[8] += 10`), Glass Vial (15g), Mace (30g), Sword (45g), Bow (75g), Bird Totem (20g). Purchase shall require proximity to a shopkeeper (race `0x88`) and `wealth > price`. Food shall call `eat(50)` rather than adding to any stuff[] slot. |
| R-INV-011 | Container loot (chest, urn, sacks) shall use `rand4()` for tier: 0 = nothing, 1 = one random item (`rand8() + 8` → indices 8–15, where index 8 means quiver), 2 = two different random items from same range (index 8 → 100 gold instead), 3 = three copies of same item (index 8 → 3 random keys from `KEYBASE` to `KEYBASE+5`). |
| R-INV-012 | Gold pickup items (stuff[31–34]) shall add their `maxshown` value (2, 5, 10, 100) to the `wealth` variable instead of being stored in `stuff[]`. |
| R-INV-013 | GIVE mode: giving gold costs 2 gold (`wealth -= 2`), and if `rand64() > kind` then `kind++`. Beggars (race `0x8d`) give a goal speech. Giving bone to Spectre (race `0x8a`) shall produce `speak(48)` and drop a crystal shard. Non-spectre NPCs shall reject bone with `speak(21)`. |
| R-INV-014 | Fruit (stuff[24]) shall auto-consume when `hunger > 30` at safe points, reducing hunger by 30. On pickup when `hunger >= 15`, fruit shall be eaten immediately via `eat(30)` instead of stored. When `hunger < 15`, fruit shall be stored in inventory. |
| R-INV-015 | Rose (stuff[23]) shall grant lava immunity: force `environ = 0` in the fiery_death zone (`map_x` 8802–13562, `map_y` 24744–29544). Without it, `environ > 15` kills instantly and `environ > 2` drains vitality per tick. Only protects the player (actor 0), not carriers or NPCs. |
| R-INV-016 | Sun Stone (stuff[7]) shall make the Witch (race `0x89`) vulnerable to melee weapons. Without it, attacks on the Witch shall produce `speak(58)`: "Stupid fool, you can't hurt me with that!" |
| R-INV-017 | Golden Lasso (stuff[5]) shall enable mounting the swan carrier. The Witch shall drop the lasso on death. |
| R-INV-018 | Sea Shell (stuff[6]) USE shall call `get_turtle()` to summon the sea turtle carrier near water. Summoning shall be blocked inside the rectangle (11194–21373, 10205–16208). |
| R-INV-019 | Crystal Shard (stuff[30]) shall override terrain type 12 collision blocking. Terrain type 12 tiles exist only in terra set 8 (Region 8 building interiors) — specifically the maze-layout sectors (2, 3, 5–9, 11–12, 35) and Doom Tower sectors 137–138. They are **not** present in Region 9 (dungeons/caves). The shard shall never be consumed. |
| R-INV-020 | On brother succession (`revive(TRUE)`), all inventory shall be wiped. Starting loadout for the new brother shall be one Dirk only (`stuff[0] = 1`). |
| R-INV-021 | Special-cased pickups bypassing `itrans`: gold bag (ob_id 13) adds 50 to wealth; scrap (ob_id 20) triggers `event(17)` plus region-specific event; dead brother bones (ob_id 28) recovers dead brother's full inventory; containers (ob_id 14 urn, 15 chest, 16 sacks) use the container loot system; turtle eggs (ob_id 102) and footstool (ob_id 31) cannot be taken. |
| R-INV-022 | World objects shall use a 6-byte record: world X (u16), world Y (u16), ob_id (i8), ob_stat (i8). ob_stat values: 0 = disabled/skipped, 1 = on ground (pickable), 2 = in inventory/taken (skipped), 3 = setfig NPC, 4 = dead setfig, 5 = hidden (revealed by Look), 6 = cabinet item. |
| R-INV-023 | Objects shall be organized into 1 global list (11 entries, processed every tick) and 10 regional lists (regions 0–9, only current region processed per tick). |
| R-INV-024 | Random treasure (10 items) shall be distributed on first visit to each outdoor region (0–7). Regions 8 (building interiors) and 9 (underground) shall be excluded. Distribution shall be weighted: 4/16 sacks, 3/16 grey key, 2/16 chest, 1/16 each for money, gold key, quiver, red key, bird totem, vial, white key. Positions shall be randomized within the region, rejecting non-traversable terrain. |
| R-INV-025 | Only one dynamically dropped item (`ob_listg[0]`) may exist at a time; each `leave_item()` call overwrites the previous drop slot contents. |
| R-INV-026 | Object state shall be fully persisted in save/load: global list (66 bytes), mapobs counts (20 bytes), dstobs distribution flags (20 bytes), and all 10 regional lists (variable size). |
| R-INV-027 | Menu item availability shall be driven by `stuff_flag(index)`: return 10 (enabled) if `stuff[index] > 0`, else 8 (disabled). The Book entry in the GIVE menu shall be hardcoded disabled. |
| R-INV-028 | Opened chests shall change `ob_id` from CHEST (15) to empty chest (0x1d/29). Other taken objects shall set `ob_stat = 2`. Look-revealed hidden objects (ob_stat 5) shall change to `ob_stat = 1` (pickable). |
| R-INV-029 | Per-tick object processing shall handle global objects first (with flag `0x80`), then regional objects. The setfig/enemy boundary (`anix`) shall be updated when the setfig count exceeds 3. No more than 20 object entries shall be rendered per tick. |
| R-INV-030 | Writ (stuff[28]) obtained from princess rescue shall also grant 100 gold and 3 of each key type (`stuff[16..21] += 3`). Showing Writ to Priest shall trigger `speak(39)` and reveal gold statue at `ob_listg[10]`. |

### User Stories

- As a player, I can pick up items from the ground and see them added to my inventory.
- As a player, I can equip different weapons that affect my combat damage range.
- As a player, my inventory is separate for each brother and is wiped on death, with only a Dirk as starting equipment for the next brother.
- As a player, I can purchase items from shopkeepers when I have enough gold.
- As a player, I can use magic consumable items from my inventory with various tactical effects.
- As a player, I can open containers (chests, urns, sacks) to receive random loot of varying quality.
- As a player, I can give items to NPCs for quest progression (bone to spectre for crystal shard, gold for kindness).
- As a player, I can use LOOK to discover hidden items in the environment.
- As a player, I find random treasure scattered across outdoor regions I visit for the first time.

---


