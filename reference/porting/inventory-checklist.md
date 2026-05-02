# Inventory System Checklist

Source scope:
- `fmain.c:380-434` (`inv_list[36]`, `struct inv_item`, group constants, `stuff[]` declaration)
- `fmain.c:425-432` (MAGICBASE, KEYBASE, STATBASE, GOLDBASE, ARROWBASE constants)
- `fmain.c:3114-3145` (inventory display, `ITEMS` case)
- `fmain.c:3472-3488` (key use, `KEYS` case)
- `fmain.c:3244-3247` (win condition on Talisman acquisition)
- `fmain2.c:1190-1230` (item pickup `ppick`, `take_command`)

Purpose:
- Ensure ports implement the inventory array layout, per-item semantics, and all item effect hooks correctly.

## A. `stuff[]` Array Layout

- [ ] Three static arrays: `julstuff[35]`, `philstuff[35]`, `kevstuff[35]` (35 = ARROWBASE) — `fmain.c:432`.
- [ ] `stuff` is a pointer to the current brother's array; reassigned on brother switch and on load.
- [ ] Implement all group constant offsets:
  - `MAGICBASE = 9` — first magic consumable (Blue Stone)
  - `KEYBASE = 16` — first key (Gold Key)
  - `STATBASE = 25` — Gold Statue
  - `GOLDBASE = 31` — first gold denomination
  - `ARROWBASE = 35` — quiver alias (not a real inventory slot)

## B. Item Index to Semantic Mapping (0–34)

- [ ] Indices 0–4: weapons (Dirk, Mace, Sword, Bow, Magic Wand) — binary present/absent.
- [ ] Index 5: Golden Lasso — binary flag.
- [ ] Index 6: Sea Shell — binary flag.
- [ ] Index 7: Sun Stone — binary flag.
- [ ] Index 8: Arrows — integer count (max display 45).
- [ ] Indices 9–15: magic consumables (Blue Stone through Jade Skull) — integer counts, decremented on use.
- [ ] Indices 16–21: keys (Gold through White) — integer counts, decremented on door unlock.
- [ ] Index 22: Talisman — binary flag; collecting it triggers the win condition (`fmain.c:3244-3247`).
- [ ] Index 23: Rose — binary flag; grants fire immunity via `environ=0` override (`fmain.c:1844`).
- [ ] Index 24: Fruit — integer count; auto-consumed at safe checkpoints when `hunger > 30` (`fmain.c:2195-2196`).
- [ ] Index 25: Gold Statue — integer count (max 5); 5 required to enter desert gate (`fmain.c:1919`, `fmain.c:3594`).
- [ ] Indices 31–34: gold denominations (2, 5, 10, 100 Gold Pieces) — added to `wealth` on pickup.

## C. Inventory Display (`fmain.c:3114-3145`)

- [ ] Iterate indices 0 to GOLDBASE-1 (0–30) only — gold pieces (31–34) are never drawn.
- [ ] Draw `min(stuff[j], inv_list[j].maxshown)` copies of each item.
- [ ] Per-copy position: start at `(xoff+20, yoff)`, increment Y by `ydelta` for each copy.
- [ ] Image source from `inv_list`: row = `image_number * 80 + img_off`, height = `img_height`, width = 16px.

## D. Item Pickup (`ppick`, `take_command`)

- [ ] `ppick` increments `stuff[item_index]` by item count when player walks over an object.
- [ ] Objects with `type == OBJECT_TYPE_ITEM` are eligible for pickup; check `ob->type` field.
- [ ] Talisman pickup (`item_index == 22`): fire win condition immediately after `stuff[22]++` — `fmain.c:3244-3247`.
- [ ] Gold pickup (indices 31–34): add gold value to `wealth` stat directly; `stuff[31..34]` not incremented for wealth tracking.

## E. Weapon System (`stuff[0..4]`)

- [ ] `an->weapon` field for actor encodes weapon type as a bitmask — separate from `stuff[]`.
- [ ] Player weapon slot (`stuff[0..4]`) tracks possession; equipping sets `an->weapon` at `fmain.c:2848`.
- [ ] `weapon & 4` is the ranged-weapon flag (bit 2 = bow or wand) used by AI tactic selection — `fmain.c:2148`.
- [ ] `weapon < 1` means unarmed — sets CONFUSED mode in AI; affects player combat too.

## F. Magic Item Effects (`stuff[9..15]`, `MAGICBASE=9`)

- [ ] Magic items are consumed on use (decrement `stuff[MAGICBASE+n]`).
- [ ] Each magic item type has a specific dispatch effect; verify all 7 items against `fmain.c` magic dispatch.
- [ ] Magic use is blocked when current extent has `extn->v3 == 9` — `fmain.c:3304-3305`.
- [ ] Wand (index 4, weapon slot): ranged weapon; fires projectile but is not a magic consumable.

## G. Key System (`stuff[16..21]`, `KEYBASE=16`)

- [ ] Key count decremented after successful `doorfind()` only (not on failed attempts) — `fmain.c:3486`.
- [ ] Key type maps to `open_list` `keytype` field (GOLD=1, GREEN=2, KBLUE=3, RED=4, GREY=5, WHITE=6) — `fmain.c:1049`.
- [ ] See [doors-xfer-checklist.md](doors-xfer-checklist.md) Section F for complete key usage algorithm.

## H. Stat Items and Quest Items

- [ ] Fruit (`stuff[24]`): auto-consumed when `hunger > 30` at safe checkpoint — `fmain.c:2195-2196`.
- [ ] Gold Statue (`stuff[25]`): count check `>= 5` for desert gate passage — `fmain.c:1919`.
- [ ] Sun Stone (`stuff[7]`): quest item, given to NPC; check specific dialogue trigger in `fmain2.c`.
- [ ] Sea Shell (`stuff[6]`): quest item, triggers NPC dialogue; verify `fmain2.c` quest path.
- [ ] Golden Lasso (`stuff[5]`): quest item; verify use effect vs. NPC lasso mechanic.

## I. Brother-Switching Semantics

- [ ] On brother switch: `stuff = blist[new_brother-1].stuff` — inventory pointer updated immediately — `fmain.c:2848`.
- [ ] Stats (brave, luck, kind, wealth) are shared globals — NOT per-brother.
- [ ] On brother death/succession: `blist[brother-1].stuff` retains the dead brother's inventory; items not transferred automatically.

## J. Known Quirks To Preserve (or Deliberately Normalize)

- [ ] `stuff[8]` (Arrows) can exceed `maxshown=45` in actual count — display is capped but storage is not.
- [ ] Win condition fires in the middle of the pickup handler — game exits the normal flow immediately.
- [ ] `stuff` pointer aliasing: if `brother` changes without updating `stuff`, the wrong inventory is read. Both must be synchronized.

## K. Minimum Parity Test Matrix

- [ ] Pickup Talisman: win condition fires, game transitions to end sequence.
- [ ] Use key on correct door: key count decrements, door opens (tile changes in `sector_mem`).
- [ ] Use key on wrong door: no key consumed, "It's locked." message (once, suppressed by `bumped`).
- [ ] Eat Fruit when `hunger > 30` at safe checkpoint: `stuff[24]--`, `hunger` decremented.
- [ ] Enter desert gate with < 5 Gold Statues: blocked; with exactly 5: permitted.
- [ ] Magic use in `v3==9` extent: blocked by guard; magic count unchanged.
- [ ] Switch brother: inventory display reflects new brother's `stuff[]` array.
