# Spec & Requirements Split Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Split `docs/SPECIFICATION.md` (198 KB, 4139 lines) and `docs/REQUIREMENTS.md` (120 KB, 903 lines) into per-subsystem files so agents read only the relevant 5–20 KB per task instead of the full documents.

**Architecture:** Create `docs/spec/` and `docs/reqs/` directories. Each gets a `README.md` index and one file per subsystem. Original files are replaced by redirects pointing to the new structure. All internal cross-references are updated to use the new paths.

**Tech Stack:** Shell (sed/awk for extraction), Rust doc-comment grepping to update src/ references.

---

## Section line map (SPECIFICATION.md → docs/spec/)

Source section ranges (1-based, inclusive):

| Output file | Source sections | Lines |
|-------------|----------------|-------|
| `display-rendering.md` | §1 (43-139), §3 (223-254), §4 (255-302), §5 (303-473), §6 (474-554), §27 (3981-4096) | ~545 |
| `world-structure.md` | §2 (140-222) | 83 |
| `palettes-daynight-visuals.md` | §7 (555-648) | 94 |
| `characters-animation.md` | §8 (649-845) | 197 |
| `movement-input.md` | §9 (846-1065) | 220 |
| `combat.md` | §10 (1066-1297) | 232 |
| `ai-encounters.md` | §11 (1298-1494), §12 (1495-1684) | 387 |
| `npcs-dialogue.md` | §13 (1685-1923) | 239 |
| `inventory-items.md` | §14 (1924-2290) | 367 |
| `quests.md` | §15 (2291-2522) | 232 |
| `doors-buildings.md` | §16 (2523-2693) | 171 |
| `daynight-cycle.md` | §17 (2694-2841) | 148 |
| `survival.md` | §18 (2842-3054) | 213 |
| `magic.md` | §19 (3055-3090) | 36 |
| `death-revival.md` | §20 (3091-3234) | 144 |
| `carriers.md` | §21 (3235-3340) | 106 |
| `audio.md` | §22 (3341-3437) | 97 |
| `intro-narrative.md` | §23 (3438-3589) | 152 |
| `save-load.md` | §24 (3590-3653) | 64 |
| `ui-menus.md` | §25 (3654-3889) | 236 |
| `asset-formats.md` | §26 (3890-3980) | 91 |
| `appendices.md` | Appendices (4097-4139) | 43 |

TOC: lines 11-42 → goes into `README.md` (rewritten as file map).

## Section line map (REQUIREMENTS.md → docs/reqs/)

| Output file | Source sections | Lines |
|-------------|----------------|-------|
| `display-rendering.md` | §1 (40-65), §3 (95-116), §4 (117-149), §5 (150-174) | ~109 |
| `world-map.md` | §2 (66-94) | 29 |
| `daynight-visuals.md` | §6 (175-202) | 28 |
| `movement-input.md` | §7 (203-248) | 46 |
| `combat.md` | §8 (249-290) | 42 |
| `ai-encounters.md` | §9 (291-340) | 50 |
| `npcs-dialogue.md` | §10 (341-383) | 43 |
| `inventory-items.md` | §11 (384-435) | 52 |
| `quests.md` | §12 (436-483) | 48 |
| `doors-buildings.md` | §13 (484-521) | 38 |
| `daynight-cycle.md` | §14 (522-554) | 33 |
| `survival.md` | §15 (555-603) | 49 |
| `magic.md` | §16 (604-630) | 27 |
| `death-revival.md` | §17 (631-667) | 37 |
| `carriers.md` | §18 (668-700) | 33 |
| `audio.md` | §19 (701-732) | 32 |
| `intro-narrative.md` | §20 (733-764) | 32 |
| `save-load.md` | §21 (765-788) | 24 |
| `ui-menus.md` | §22 (789-822) | 34 |
| `asset-loading.md` | §23 (823-846) | 24 |
| `special-effects.md` | §24 (847-873) | 27 |
| `traceability.md` | Traceability Matrix (874-903) | 30 |

TOC: lines 11-39 → goes into `README.md`.

---

## Task 1: Extract SPECIFICATION.md into docs/spec/

**Files:**
- Create: `docs/spec/README.md`
- Create: `docs/spec/*.md` (22 subsystem files)

- [ ] **Step 1: Create docs/spec/ directory**

```bash
mkdir -p docs/spec
```

- [ ] **Step 2: Extract each subsystem file using sed**

Run this script from the repo root:

```bash
#!/usr/bin/env bash
set -e
SRC="docs/SPECIFICATION.md"
OUT="docs/spec"

# Helper: extract lines START..END from SRC into OUT/FILE
# Usage: extract FILE START END
extract() {
  local file="$1" start="$2" end="$3"
  sed -n "${start},${end}p" "$SRC" > "$OUT/$file"
  echo "  wrote $OUT/$file ($(wc -l < "$OUT/$file") lines)"
}

echo "Extracting docs/spec/ files..."

extract "display-rendering.md"      43  139   # §1 Display
# Append §3 Tiles, §4 Scrolling, §5 Sprites, §6 Terrain Masking, §27 Special Effects
sed -n '223,554p' "$SRC" >> "$OUT/display-rendering.md"
sed -n '474,554p' "$SRC" >> "$OUT/display-rendering.md"
# NOTE: §6 already covered in 474-554 above; §27 is separate
sed -n '3981,4096p' "$SRC" >> "$OUT/display-rendering.md"

# CORRECTION: extract non-overlapping ranges cleanly
# Redo display-rendering with exact non-overlapping sections
{
  sed -n '43,139p'    "$SRC"   # §1 Display
  echo ""
  sed -n '223,254p'   "$SRC"   # §3 Tiles
  echo ""
  sed -n '255,302p'   "$SRC"   # §4 Scrolling
  echo ""
  sed -n '303,473p'   "$SRC"   # §5 Sprites
  echo ""
  sed -n '474,554p'   "$SRC"   # §6 Terrain Masking
  echo ""
  sed -n '3981,4096p' "$SRC"   # §27 Special Effects
} > "$OUT/display-rendering.md"
echo "  wrote $OUT/display-rendering.md ($(wc -l < "$OUT/display-rendering.md") lines)"

extract "world-structure.md"        140  222
extract "palettes-daynight-visuals.md" 555 648
extract "characters-animation.md"   649  845
extract "movement-input.md"         846  1065
extract "combat.md"                 1066 1297
{ sed -n '1298,1494p' "$SRC"; echo ""; sed -n '1495,1684p' "$SRC"; } > "$OUT/ai-encounters.md"
echo "  wrote $OUT/ai-encounters.md ($(wc -l < "$OUT/ai-encounters.md") lines)"
extract "npcs-dialogue.md"          1685 1923
extract "inventory-items.md"        1924 2290
extract "quests.md"                 2291 2522
extract "doors-buildings.md"        2523 2693
extract "daynight-cycle.md"         2694 2841
extract "survival.md"               2842 3054
extract "magic.md"                  3055 3090
extract "death-revival.md"          3091 3234
extract "carriers.md"               3235 3340
extract "audio.md"                  3341 3437
extract "intro-narrative.md"        3438 3589
extract "save-load.md"              3590 3653
extract "ui-menus.md"               3654 3889
extract "asset-formats.md"          3890 3980
extract "appendices.md"             4097 4139

echo "Done. Files in $OUT:"
ls -la "$OUT/"
```

- [ ] **Step 3: Verify total line count matches source**

```bash
wc -l docs/spec/*.md | tail -1
wc -l docs/SPECIFICATION.md
```

Expected: spec/ total should be within ~50 lines of SPECIFICATION.md total (small difference due to added blank separators).

- [ ] **Step 4: Write docs/spec/README.md**

Create `docs/spec/README.md` with this content:

```markdown
# SPECIFICATION — Index

This directory replaces the monolithic `docs/SPECIFICATION.md`.
Read only the file relevant to your current task.

## Subsystem map

| Topic | File | ~Size |
|-------|------|-------|
| Display, tiles, scrolling, sprites, terrain masking, special effects | `display-rendering.md` | 545 lines |
| World regions & map structure | `world-structure.md` | 83 lines |
| Color palettes & day/night visuals | `palettes-daynight-visuals.md` | 94 lines |
| Characters & animation | `characters-animation.md` | 197 lines |
| Player movement & input | `movement-input.md` | 220 lines |
| Combat system | `combat.md` | 232 lines |
| AI, behavior & encounter generation | `ai-encounters.md` | 387 lines |
| NPCs & dialogue | `npcs-dialogue.md` | 239 lines |
| Inventory & items | `inventory-items.md` | 367 lines |
| Quest system | `quests.md` | 232 lines |
| Doors & buildings | `doors-buildings.md` | 171 lines |
| Day/night cycle & clock | `daynight-cycle.md` | 148 lines |
| Survival (hunger, fatigue, health) | `survival.md` | 213 lines |
| Magic system | `magic.md` | 36 lines |
| Death & revival | `death-revival.md` | 144 lines |
| Carriers (raft, turtle, bird) | `carriers.md` | 106 lines |
| Audio system | `audio.md` | 97 lines |
| Intro & narrative | `intro-narrative.md` | 152 lines |
| Save/load system | `save-load.md` | 64 lines |
| UI & menus | `ui-menus.md` | 236 lines |
| Asset formats & data loading | `asset-formats.md` | 91 lines |
| Appendices | `appendices.md` | 43 lines |

## Original section numbering

Original section numbers (§1–§27) are preserved inside each file.
Cross-references in source code (`reference/...` doc comments) refer to
the research branch, not these files.
```

- [ ] **Step 5: Replace docs/SPECIFICATION.md with a redirect stub**

```bash
cat > docs/SPECIFICATION.md << 'EOF'
# SPECIFICATION (split)

This file has been split into per-subsystem files in `docs/spec/`.
See `docs/spec/README.md` for the full index.

**Do not read this file** — read only the relevant subsystem file.
EOF
```

- [ ] **Step 6: Commit**

```bash
git add docs/spec/ docs/SPECIFICATION.md
git commit -m "docs: split SPECIFICATION.md into per-subsystem files in docs/spec/

Splits 198 KB monolith into 22 focused files (~5-25 KB each).
Agents should read only the relevant subsystem file, not the full spec.
See docs/spec/README.md for the subsystem index."
```

---

## Task 2: Extract REQUIREMENTS.md into docs/reqs/

**Files:**
- Create: `docs/reqs/README.md`
- Create: `docs/reqs/*.md` (22 subsystem files)

- [ ] **Step 1: Create docs/reqs/ directory**

```bash
mkdir -p docs/reqs
```

- [ ] **Step 2: Extract each subsystem file**

```bash
#!/usr/bin/env bash
set -e
SRC="docs/REQUIREMENTS.md"
OUT="docs/reqs"

extract() {
  local file="$1" start="$2" end="$3"
  sed -n "${start},${end}p" "$SRC" > "$OUT/$file"
  echo "  wrote $OUT/$file ($(wc -l < "$OUT/$file") lines)"
}

echo "Extracting docs/reqs/ files..."

# display-rendering: §1 + §3 Scrolling + §4 Sprites + §5 Terrain
{
  sed -n '40,65p'  "$SRC"
  echo ""
  sed -n '95,116p' "$SRC"
  echo ""
  sed -n '117,149p' "$SRC"
  echo ""
  sed -n '150,174p' "$SRC"
} > "$OUT/display-rendering.md"
echo "  wrote $OUT/display-rendering.md ($(wc -l < "$OUT/display-rendering.md") lines)"

extract "world-map.md"          66   94
extract "daynight-visuals.md"   175  202
extract "movement-input.md"     203  248
extract "combat.md"             249  290
extract "ai-encounters.md"      291  340
extract "npcs-dialogue.md"      341  383
extract "inventory-items.md"    384  435
extract "quests.md"             436  483
extract "doors-buildings.md"    484  521
extract "daynight-cycle.md"     522  554
extract "survival.md"           555  603
extract "magic.md"              604  630
extract "death-revival.md"      631  667
extract "carriers.md"           668  700
extract "audio.md"              701  732
extract "intro-narrative.md"    733  764
extract "save-load.md"          765  788
extract "ui-menus.md"           789  822
extract "asset-loading.md"      823  846
extract "special-effects.md"    847  873
extract "traceability.md"       874  903

echo "Done. Files in $OUT:"
ls -la "$OUT/"
```

- [ ] **Step 3: Verify total line count**

```bash
wc -l docs/reqs/*.md | tail -1
wc -l docs/REQUIREMENTS.md
```

- [ ] **Step 4: Write docs/reqs/README.md**

```markdown
# REQUIREMENTS — Index

This directory replaces the monolithic `docs/REQUIREMENTS.md`.
Read only the file relevant to your current task.

## Subsystem map

| Topic | Spec file (paired) | Requirements file | ~Lines |
|-------|-------------------|-------------------|--------|
| Display, tiles, scrolling, sprites, terrain | `docs/spec/display-rendering.md` | `display-rendering.md` | ~109 |
| World regions & map structure | `docs/spec/world-structure.md` | `world-map.md` | 29 |
| Day/night visuals | `docs/spec/palettes-daynight-visuals.md` | `daynight-visuals.md` | 28 |
| Player movement & input | `docs/spec/movement-input.md` | `movement-input.md` | 46 |
| Combat | `docs/spec/combat.md` | `combat.md` | 42 |
| AI & encounters | `docs/spec/ai-encounters.md` | `ai-encounters.md` | 50 |
| NPCs & dialogue | `docs/spec/npcs-dialogue.md` | `npcs-dialogue.md` | 43 |
| Inventory & items | `docs/spec/inventory-items.md` | `inventory-items.md` | 52 |
| Quests | `docs/spec/quests.md` | `quests.md` | 48 |
| Doors & buildings | `docs/spec/doors-buildings.md` | `doors-buildings.md` | 38 |
| Day/night cycle & clock | `docs/spec/daynight-cycle.md` | `daynight-cycle.md` | 33 |
| Survival | `docs/spec/survival.md` | `survival.md` | 49 |
| Magic | `docs/spec/magic.md` | `magic.md` | 27 |
| Death & revival | `docs/spec/death-revival.md` | `death-revival.md` | 37 |
| Carriers | `docs/spec/carriers.md` | `carriers.md` | 33 |
| Audio | `docs/spec/audio.md` | `audio.md` | 32 |
| Intro & narrative | `docs/spec/intro-narrative.md` | `intro-narrative.md` | 32 |
| Save/load | `docs/spec/save-load.md` | `save-load.md` | 24 |
| UI & menus | `docs/spec/ui-menus.md` | `ui-menus.md` | 34 |
| Asset loading | `docs/spec/asset-formats.md` | `asset-loading.md` | 24 |
| Special effects | `docs/spec/display-rendering.md` | `special-effects.md` | 27 |
| Traceability matrix | — | `traceability.md` | 30 |
```

- [ ] **Step 5: Replace docs/REQUIREMENTS.md with redirect stub**

```bash
cat > docs/REQUIREMENTS.md << 'EOF'
# REQUIREMENTS (split)

This file has been split into per-subsystem files in `docs/reqs/`.
See `docs/reqs/README.md` for the full index.

**Do not read this file** — read only the relevant subsystem file.
EOF
```

- [ ] **Step 6: Commit**

```bash
git add docs/reqs/ docs/REQUIREMENTS.md
git commit -m "docs: split REQUIREMENTS.md into per-subsystem files in docs/reqs/

Splits 120 KB monolith into 22 focused files (~1-3 KB each).
Agents should read only the relevant subsystem file, not the full reqs.
See docs/reqs/README.md for the subsystem index."
```

---

## Task 3: Update AGENTS.md with subsystem lookup table and context-mode enforcement

**Files:**
- Modify: `AGENTS.md`

- [ ] **Step 1: Read current AGENTS.md**

```bash
cat AGENTS.md
```

- [ ] **Step 2: Add spec/reqs file map section**

Add the following section after `## Agent working rules` in `AGENTS.md`:

```markdown
## Spec & requirements file map

**Never read `docs/SPECIFICATION.md` or `docs/REQUIREMENTS.md` directly** — they are redirect stubs.
Read only the relevant subsystem file. Use `ctx_execute_file` for analysis (keeps content out of context window); use `read` only when editing.

| Topic | Spec file | Requirements file |
|-------|-----------|-------------------|
| Display, tiles, scrolling, sprites, terrain masking, special effects | `docs/spec/display-rendering.md` | `docs/reqs/display-rendering.md` |
| World regions & map structure | `docs/spec/world-structure.md` | `docs/reqs/world-map.md` |
| Color palettes & day/night visuals | `docs/spec/palettes-daynight-visuals.md` | `docs/reqs/daynight-visuals.md` |
| Characters & animation | `docs/spec/characters-animation.md` | — |
| Player movement & input | `docs/spec/movement-input.md` | `docs/reqs/movement-input.md` |
| Combat | `docs/spec/combat.md` | `docs/reqs/combat.md` |
| AI, behavior & encounter generation | `docs/spec/ai-encounters.md` | `docs/reqs/ai-encounters.md` |
| NPCs & dialogue | `docs/spec/npcs-dialogue.md` | `docs/reqs/npcs-dialogue.md` |
| Inventory & items | `docs/spec/inventory-items.md` | `docs/reqs/inventory-items.md` |
| Quests | `docs/spec/quests.md` | `docs/reqs/quests.md` |
| Doors & buildings | `docs/spec/doors-buildings.md` | `docs/reqs/doors-buildings.md` |
| Day/night cycle & clock | `docs/spec/daynight-cycle.md` | `docs/reqs/daynight-cycle.md` |
| Survival (hunger, fatigue, health) | `docs/spec/survival.md` | `docs/reqs/survival.md` |
| Magic | `docs/spec/magic.md` | `docs/reqs/magic.md` |
| Death & revival | `docs/spec/death-revival.md` | `docs/reqs/death-revival.md` |
| Carriers (raft, turtle, bird) | `docs/spec/carriers.md` | `docs/reqs/carriers.md` |
| Audio | `docs/spec/audio.md` | `docs/reqs/audio.md` |
| Intro & narrative | `docs/spec/intro-narrative.md` | `docs/reqs/intro-narrative.md` |
| Save/load | `docs/spec/save-load.md` | `docs/reqs/save-load.md` |
| UI & menus | `docs/spec/ui-menus.md` | `docs/reqs/ui-menus.md` |
| Asset formats & data loading | `docs/spec/asset-formats.md` | `docs/reqs/asset-loading.md` |
```

- [ ] **Step 3: Commit AGENTS.md update**

```bash
git add AGENTS.md
git commit -m "docs: add spec/reqs subsystem file map to AGENTS.md

Agents now have a direct lookup table from topic to file.
Prevents full-file reads of 198 KB spec and 120 KB reqs."
```

---

## Task 4: Verify — spot-check section content in extracted files

- [ ] **Step 1: Spot-check combat spec**

```bash
head -5 docs/spec/combat.md
grep -c '##' docs/spec/combat.md
```

Expected: starts with `## 10. Combat System`, contains several `###` subsections.

- [ ] **Step 2: Spot-check combat requirements**

```bash
head -5 docs/reqs/combat.md
grep '^- R-' docs/reqs/combat.md | head -5
```

Expected: starts with `## 8. Combat`, contains `R-COMBAT-*` requirement IDs.

- [ ] **Step 3: Verify redirect stubs**

```bash
cat docs/SPECIFICATION.md
cat docs/REQUIREMENTS.md
```

Expected: each is a short stub pointing to the split directory.

- [ ] **Step 4: Check no src/ references break**

```bash
grep -r 'docs/SPECIFICATION\.md\|docs/REQUIREMENTS\.md' src/ --include='*.rs' | head -20
```

If any results: update those doc comments to point to the relevant `docs/spec/` or `docs/reqs/` file instead.

- [ ] **Step 5: Final commit if src/ refs were updated**

```bash
git add src/
git commit -m "docs: update src/ doc-comment references to split spec/reqs paths"
```
