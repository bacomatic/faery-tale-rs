# Merge DECODE.md Into RESEARCH.md — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Merge all content from DECODE.md into RESEARCH.md, delete DECODE.md, and update all cross-references so RESEARCH.md becomes the single canonical reverse-engineering reference.

**Architecture:** DECODE.md (1,100 lines) is merged into RESEARCH.md (1,318 lines) using section-by-section deduplication per the merge map in the design spec (`docs/superpowers/specs/2026-03-27-merge-decode-into-research-design.md`). New sections are inserted at logical positions. Overlapping sections use the more detailed version. All references across 6 files are updated. No code changes — documentation only.

**Tech Stack:** Markdown, TOML (`research_index.toml`), Bash (`check_docs_links.sh`)

**Design spec:** `docs/superpowers/specs/2026-03-27-merge-decode-into-research-design.md`

---

## File Map

| File | Action | What changes |
|------|--------|-------------|
| `RESEARCH.md` | Modify | Preamble update + 5 merged sections + 5 new sections |
| `DECODE.md` | Delete | After all content migrated |
| `research_index.toml` | Modify | 2 entries change `doc` from `DECODE.md` to `RESEARCH.md`; add 5 new entries for new sections |
| `AGENTS.md` | Modify | "Canonical sources" bullet list |
| `CLAUDE.md` | Modify | "Canonical sources" bullet list |
| `README.md` | Modify | "Canonical Sources" bullet list |
| `PLAN.md` | Modify | Line 10 reference + line 401 reference |
| `scripts/check_docs_links.sh` | Modify | Remove `DECODE.md` from `required_files` array |

---

## Section Insertion Order in RESEARCH.md

After the merge, RESEARCH.md's `##` sections will appear in this order (★ = new or substantially changed):

```
## RESEARCH.md                         (preamble — ★ updated)
## Maintenance workflow
## Game World & Map System: Data format
## Game World & Map System: Constants, addresses, and implementation notes
## Key Bindings: Original game key map  (★ merged: augmented with DECODE manual details)
## Key Bindings: Design and compatibility notes
## Input Decoding                       (★ new from DECODE lines 697–809)
## Menu System                          (★ new from DECODE lines 810–1074)
## Player Character Stats
## Hunger & Fatigue System
## Day/Night Cycle
## Door / Portal System
## Terrain Collision System             (★ replaced with DECODE lines 425–595, keeping RESEARCH masking table)
## Combat System
## Enemy Types (Encounter Chart)
## Inventory System
## setmood() — Music State Machine
## game/songs — Music Score Data        (★ new from DECODE lines 129–280)
## game/v6 — Voice/Waveform Data        (★ new from DECODE lines 281–313)
## Save / Load Format
## Sound Effects (game/samples)         (★ merged: DECODE trigger/speed/jitter details added)
## Sprite / Shape File Layout (ADF)     (★ merged: DECODE bitplane layout + statelist[] + mask details added)
## NPC Behavior (Goal/Tactic System)
## Extents and Encounter Zones
## Compass Rose                         (★ new from DECODE lines 596–696)
## Screen Layout: Amiga Mixed-Resolution Viewports
## Known Original Exploits              (★ new from DECODE lines 1075–1100)
## World Map: Region Diagrams
```

---

### Task 1: Update RESEARCH.md Preamble

**Files:**
- Modify: `RESEARCH.md:1-14`

- [ ] **Step 1: Replace preamble**

Replace lines 1–5 of RESEARCH.md (the current preamble that references PLAN.md) with:

```markdown
## RESEARCH.md

Canonical reverse-engineering reference for The Faery Tale Adventure (1987
Amiga). Covers game systems, binary file formats (`songs`, `v6`, ADF layout),
original game mechanics, and implementation notes derived from the manual and
source code.

For build/run setup, see `README.md`. Stable agent lookup keys live in
`research_index.toml`.
```

Keep the "## Maintenance workflow" section and the `---` separator unchanged.

- [ ] **Step 2: Commit**

```bash
git add RESEARCH.md
git commit -m "docs: update RESEARCH.md preamble for unified reference role

Remove PLAN.md reference. Describe RESEARCH.md as the single canonical
reverse-engineering reference covering both game systems and binary formats.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 2: Merge Input & Command Reference Into Key Bindings

**Files:**
- Modify: `RESEARCH.md` — section "Key Bindings: Original game key map" (lines 79–115)
- Source: `DECODE.md` lines 12–128

**Context:** DECODE has a comprehensive manual-derived reference (Movement, Combat, Command Menu System with 5 sub-menus, Player Stats, Map Size). RESEARCH has a flat key table that has some inaccuracies (e.g., `M` listed as "Map view" but original is "Toggle music"; `F` listed as "Find (compass)" but original is "Toggle sound effects"). The DECODE version is authoritative.

- [ ] **Step 1: Replace the Key Bindings section content**

Keep the `## Key Bindings: Original game key map` heading. Replace the section body (everything from "From `fmain.c`..." through the table ending before the `---` separator) with the full DECODE Input & Command Reference content (lines 14–128: Movement, Combat, Command Menu System with all sub-menus, Player Stats summary, Map Size). Keep the DECODE subheadings as `###` under the existing `##`.

Do NOT include the DECODE heading "## Input & Command Reference (from original manual)" — use RESEARCH's existing heading.

- [ ] **Step 2: Commit**

```bash
git add RESEARCH.md
git commit -m "docs: merge DECODE Input & Command Reference into key bindings

Replace the abbreviated key table with the full manual-derived reference
including movement, combat, command menu system with all sub-menus,
player stats summary, and map size.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 3: Insert Input Decoding Section

**Files:**
- Modify: `RESEARCH.md` — insert after "Key Bindings: Design and compatibility notes" section
- Source: `DECODE.md` lines 697–809

- [ ] **Step 1: Insert new section**

After the "Key Bindings: Design and compatibility notes" section (which ends with a `---` separator), insert DECODE lines 697–809 verbatim as a new `##` section. Keep the heading `## Input Decoding — decode_mouse / decodekey (fsubs.asm:1490–1576)` and all subsections/content exactly as-is.

- [ ] **Step 2: Commit**

```bash
git add RESEARCH.md
git commit -m "docs: add Input Decoding section from DECODE.md

Adds decode_mouse/decodekey reference covering direction index convention,
keytrans table, joystick decoding, mouse compass click, and Rust port mapping.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 4: Insert Menu System Section

**Files:**
- Modify: `RESEARCH.md` — insert immediately after the new Input Decoding section
- Source: `DECODE.md` lines 810–1074

- [ ] **Step 1: Insert new section**

After the Input Decoding section (which ends with a `---` separator), insert DECODE lines 810–1074 verbatim as a new `##` section. Keep the heading `## Menu System (fmain.c:538–589, 3758–3820, 4409–4441; fmain2.c:613–675; fsubs.asm:120–165)` and all subsections/content exactly as-is. This is the largest new addition (~265 lines).

- [ ] **Step 2: Commit**

```bash
git add RESEARCH.md
git commit -m "docs: add Menu System section from DECODE.md

Adds comprehensive menu system reference: 10 menu modes, enabled[] bit flags,
label strings, settings toggles, gomenu(), print_options(), propt() rendering,
set_options() inventory-driven visibility, do_option() dispatch table,
letter_list[] keyboard shortcuts, keycolors[], prq() deferred actions,
and mouse click-to-button mapping.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 5: Replace Terrain Collision System

**Files:**
- Modify: `RESEARCH.md` — section "Terrain Collision System" (lines ~250–289)
- Source: `DECODE.md` lines 425–595

**Context:** DECODE's terrain collision section (172 lines) is far more detailed than RESEARCH's (42 lines). DECODE covers: Overview, Memory Buffers, Terrain Descriptor Layout, `px_to_im()`, Terrain Type Table, `proxcheck()`, Special Terrain Behaviors, and Terrain Source Files. However, RESEARCH has a unique "Sprite depth/masking block type" table (the `k` values 0–7 table for `maskit()`) that DECODE does not include. This masking table must be preserved.

- [ ] **Step 1: Replace section with DECODE version**

Replace the entire "## Terrain Collision System" section in RESEARCH.md (from the `## Terrain Collision System` heading through the `---` separator before "## Combat System") with DECODE's version (lines 425–595, from `## Terrain Collision` through the final `---`).

Use the heading `## Terrain Collision System` (RESEARCH's original heading, which matches the research_index.toml anchor).

- [ ] **Step 2: Merge RESEARCH's masking table into the replaced section**

After the "Terrain Descriptor Layout" subsection (which describes the 4-byte terra_mem entry including byte +1's lower nibble), insert RESEARCH's sprite depth/masking block type table. Place it right after the DECODE `px_to_im` ASM access pattern code block and before the `---` that precedes "Coordinate-to-Terrain Lookup". The content to insert (from RESEARCH lines ~275–289):

```markdown
**Sprite depth/masking block type** (lower nibble of `terra_mem[cm+1]`, i.e. `& 0x0f`), used by `maskit`:

| k | Name | Masking condition (skip masking if…) |
|---|---|---|
| 0 | Transparent | Always skip (fully passable) |
| 1 | Right-half | `xm == 0` (left column only) |
| 2 | Ground-level | `ystop > 35` (above ground line) |
| 3 | Bridge | `hero_sector != 48 || i != 1` (bridge sector special) |
| 4 | Right+Ground | `xm == 0 OR ystop > 35` |
| 5 | Right OR Ground | `xm == 0 AND ystop > 35` |
| 6 | Full-if-above | If `ym != 0`: substitute tile 64 as solid mask |
| 7 | Near-top | `ystop > 20` |

This table controls sprite-depth overlap (whether a sprite is drawn in front of or behind terrain tiles), not walking passability. Walking passability is handled separately by `proxcheck()`, which tests for hard collisions with tile geometry via `prox()`.
```

- [ ] **Step 3: Commit**

```bash
git add RESEARCH.md
git commit -m "docs: replace Terrain Collision with comprehensive DECODE version

Replace 42-line summary with DECODE's 172-line version covering memory buffers,
terrain descriptor layout, px_to_im() algorithm, terrain type table (0-15),
proxcheck() with asymmetric thresholds, special terrain behaviors, and terrain
source files. Preserves RESEARCH's sprite depth/masking block type table.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 6: Insert Songs Section

**Files:**
- Modify: `RESEARCH.md` — insert after "setmood() — Music State Machine" section
- Source: `DECODE.md` lines 129–280

- [ ] **Step 1: Insert new section**

After the "## `setmood()` — Music State Machine" section (which ends with a `---` separator before "## Save / Load Format"), insert DECODE lines 129–280 as a new `##` section. Use the exact heading from DECODE: `` ## `game/songs` — Music Score Data (5,984 bytes) ``. Keep the backtick formatting around the filename and the file-size annotation. Keep all subsections (File Layout, Event Encoding, Timing, Song Groups, Parsed Track Statistics) and content exactly as-is.

- [ ] **Step 2: Commit**

```bash
git add RESEARCH.md
git commit -m "docs: add game/songs music score data section from DECODE.md

Adds file layout, event encoding (note/rest/instrument/tempo/end), timing at
60 Hz NTSC, 7 song groups with context mapping, PTABLE pitch layout, and
parsed track statistics for all 28 tracks.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 7: Insert v6 Section

**Files:**
- Modify: `RESEARCH.md` — insert immediately after the new Songs section
- Source: `DECODE.md` lines 281–313

- [ ] **Step 1: Insert new section**

After the Songs section, insert DECODE lines 281–313 as a new `##` section. Use the exact heading from DECODE: `` ## `game/v6` — Music Voice/Waveform Data (4,628 bytes) ``. Keep the backtick formatting and "Music" in the title (it matches the existing research_index.toml entry). Keep all content (Layout table, Role in the engine) exactly as-is.

- [ ] **Step 2: Commit**

```bash
git add RESEARCH.md
git commit -m "docs: add game/v6 voice/waveform data section from DECODE.md

Adds wavmem/volmem buffer layout, envelope structure, and role in the
VBlank music engine.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 8: Merge Sound Effects

**Files:**
- Modify: `RESEARCH.md` — section "Sound Effects (`game/samples`)" (lines ~672–691)
- Source: `DECODE.md` lines 314–341

**Context:** Both versions cover the same format. DECODE adds the IFF-style `for each of 6 samples` loop pseudocode, the `playsample()` call signature detail (`sample_size[num] / 2` is word count for Paula DMA), and the `rand` jitter column. RESEARCH has the same trigger/speed data but organized slightly differently.

- [ ] **Step 1: Replace section body with DECODE's version**

Keep the RESEARCH heading `## Sound Effects (game/samples)`. Replace the section body with DECODE lines 316–340 (from "Loaded from **ADF block 920**..." through the table and final `---`). This gives us the more complete version with the explicit loop pseudocode and jitter column.

Note: RESEARCH's heading uses backtick formatting: `` ## Sound Effects (`game/samples`) ``. Keep this formatting — it matches the research_index.toml anchor.

- [ ] **Step 2: Commit**

```bash
git add RESEARCH.md
git commit -m "docs: merge DECODE sound effects details into existing section

Adds IFF-style loop pseudocode, Paula DMA word count detail, and per-sample
speed base/jitter values from DECODE.md.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 9: Merge Sprite / Shape File Layout

**Files:**
- Modify: `RESEARCH.md` — section "Sprite / Shape File Layout (ADF)" (lines ~693–749)
- Source: `DECODE.md` lines 342–424

**Context:** Both versions have the `cfiles[]` struct and table. DECODE adds: bitplane layout details, `statelist[]` animation frame index, `trans_list[9]`, and the critical mask computation detail (`make_mask()` — mask is NOT stored on disk, computed at runtime by ORing all planes and inverting; color 31 = transparent). RESEARCH has `setfig_table[]` that DECODE lacks, and an incorrect note about mask being stored in the file ("`Total size = size * 6`" implies 6 planes including mask on disk).

- [ ] **Step 1: Replace section body with merged content**

Keep RESEARCH's heading `## Sprite / Shape File Layout (ADF)`. Build the merged section as follows:

1. **Use DECODE's opening paragraph** (lines 342–343): "All animated character sprites..."
2. **Use DECODE's `cfiles[]` struct** (lines 348–357) — identical to RESEARCH's
3. **Use DECODE's frame/ADF size formulas** (lines 359–363): These are more accurate — they note that mask is NOT stored on disk and is computed at runtime by `make_mask()`, and that `nextshape` advances by `frame_bytes × count × 5` (5 planes only).
4. **Include DECODE's mask computation note** (lines 364–367): The `make_mask()` description with color 31 = transparent.
5. **Use DECODE's cfiles table** (lines 369–389) — same data, use DECODE's version.
6. **Add DECODE's "Bitplane layout" subsection** (lines 392–403): plane-major format description and offset formula. This is new content not in RESEARCH.
7. **Add DECODE's "`statelist[]` — Animation Frame Index" subsection** (lines 406–421): animation state mapping. This is new content not in RESEARCH.
8. **Keep RESEARCH's "`setfig_table[]`" subsection** (RESEARCH lines ~731–749): This maps setfig_type to cfile entries and is not in DECODE.

- [ ] **Step 2: Commit**

```bash
git add RESEARCH.md
git commit -m "docs: merge DECODE sprite details into shape file layout

Adds bitplane layout format, statelist[] animation frame index, trans_list[]
combat transitions, and make_mask() runtime computation detail. Fixes
incorrect note about mask being stored on disk. Keeps setfig_table[].

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 10: Insert Compass Rose Section

**Files:**
- Modify: `RESEARCH.md` — insert before "Screen Layout: Amiga Mixed-Resolution Viewports"
- Source: `DECODE.md` lines 596–696

**Context:** The spec says "Insert near Screen Layout section." The compass is a HI-bar UI element closely related to screen layout. Insert it immediately before Screen Layout, after "Extents and Encounter Zones."

- [ ] **Step 1: Insert new section**

After the "## Extents and Encounter Zones" section (which ends with a `---` separator before "## Screen Layout"), insert DECODE lines 596–696 as a new `##` section. Use heading: `## Compass Rose — Direction Indicator Bitmaps`. Keep all subsections (Source data, `drawcompass(dir)` algorithm, `comptable[10]`, How plane 2 produces colour, Rust port notes) and content exactly as-is.

- [ ] **Step 2: Commit**

```bash
git add RESEARCH.md
git commit -m "docs: add Compass Rose section from DECODE.md

Adds drawcompass() algorithm, comptable[10] direction sub-regions, _hinor/_hivar
bitmap source data, BltBitMap parameters, plane 2 colour mechanics, and
Rust port notes.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 11: Insert Known Original Exploits Section

**Files:**
- Modify: `RESEARCH.md` — insert before "World Map: Region Diagrams" (the last section)
- Source: `DECODE.md` lines 1075–1100

- [ ] **Step 1: Insert new section**

Before the "## World Map: Region Diagrams" section, insert DECODE lines 1075–1100 as a new `##` section. Use heading: `## Known Original Exploits`. Keep all content (Pause-Take duplication, Key replenishment after save/reload) exactly as-is.

- [ ] **Step 2: Commit**

```bash
git add RESEARCH.md
git commit -m "docs: add Known Original Exploits section from DECODE.md

Documents Pause-Take item duplication bug and key replenishment after
save/reload exploit from the original 1987 release.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 12: Update research_index.toml

**Files:**
- Modify: `research_index.toml`

- [ ] **Step 1: Update existing `decode.songs` entry**

Change entry with `id = "decode.songs"`:
- `doc` from `"DECODE.md"` to `"RESEARCH.md"`
- `anchor` from `"#gamesongs--music-score-data-5984-bytes"` to `"#gamesongs--music-score-data-5984-bytes"`

Note: the anchor slug depends on the exact heading used. If the heading is `## game/songs — Music Score Data (5,984 bytes)`, the slug is `#gamesongs--music-score-data-5984-bytes`. Verify the actual heading inserted in Task 6 and compute the correct slug.

- [ ] **Step 2: Update existing `decode.v6` entry**

Change entry with `id = "decode.v6"`:
- `doc` from `"DECODE.md"` to `"RESEARCH.md"`
- `anchor` from `"#gamev6--music-voicewaveform-data-4628-bytes"` to `"#gamev6--voicewaveform-data-4628-bytes"`

Same note: verify the actual heading slug matches.

- [ ] **Step 3: Add new entries for new sections**

Add these new entries to `research_index.toml`:

```toml
[[entry]]
id = "input.decoding"
title = "Input Decoding"
doc = "RESEARCH.md"
anchor = "#input-decoding--decode_mouse--decodekey-fsubsasm14901576"
tags = ["input", "mouse", "joystick", "keyboard", "direction", "decoding"]

[[entry]]
id = "menu.system"
title = "Menu System"
doc = "RESEARCH.md"
anchor = "#menu-system-fmainc538589-37583820-44094441-fmain2c613675-fsubsasm120165"
tags = ["menu", "ui", "hibar", "buttons", "options", "gomenu", "propt"]

[[entry]]
id = "gfx.compass"
title = "Compass Rose — Direction Indicator Bitmaps"
doc = "RESEARCH.md"
anchor = "#compass-rose--direction-indicator-bitmaps"
tags = ["graphics", "compass", "hibar", "direction", "drawcompass", "bitmaps"]

[[entry]]
id = "game.exploits"
title = "Known Original Exploits"
doc = "RESEARCH.md"
anchor = "#known-original-exploits"
tags = ["bugs", "exploits", "pause", "save", "original"]
```

**Important:** The anchor slugs above are best-guess computations from the heading text. After inserting them, verify each anchor resolves to an actual heading in RESEARCH.md. GitHub-flavored markdown slugification rules: lowercase, replace spaces with `-`, strip punctuation except `-`, collapse consecutive `-`.

- [ ] **Step 4: Bump `last_updated` date**

Change `last_updated = "2026-03-02"` to `last_updated = "2026-03-27"` at the top of the file.

- [ ] **Step 5: Commit**

```bash
git add research_index.toml
git commit -m "docs: update research_index.toml for DECODE.md merge

Point decode.songs and decode.v6 entries to RESEARCH.md. Add new entries
for Input Decoding, Menu System, Compass Rose, and Known Original Exploits.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 13: Update Cross-Reference Files

**Files:**
- Modify: `AGENTS.md:29-34`
- Modify: `CLAUDE.md:30-35`
- Modify: `README.md:11-16`
- Modify: `PLAN.md:10` and `PLAN.md:401`

- [ ] **Step 1: Update AGENTS.md**

In the "Canonical sources by topic" section, replace:
```
- Reverse-engineering/file formats (`songs`, `v6`, etc.): `DECODE.md`
```
with:
```
- Reverse-engineering, file formats, and game mechanics: `RESEARCH.md` (check `research_index.toml` for stable lookup keys)
```

- [ ] **Step 2: Update CLAUDE.md**

In the "Canonical sources by topic" section, replace:
```
- Reverse-engineering and file formats (`songs`, `v6`, ADF layout, etc.): **always check `DECODE.md` before guessing at binary format details**
- Architecture deep-dives (screen layout, Amiga rendering pipeline, palette handling, etc.): **always check `RESEARCH.md` before re-deriving implementation decisions**
```
with:
```
- Reverse-engineering, file formats, and game mechanics: **always check `RESEARCH.md` before guessing at binary format details or re-deriving implementation decisions** (stable lookup keys in `research_index.toml`)
```

- [ ] **Step 3: Update README.md**

In the "Canonical Sources" section, replace:
```
- Reverse-engineering and asset format notes: `DECODE.md`
```
with:
```
- Reverse-engineering, file formats, and game mechanics: `RESEARCH.md`
```

- [ ] **Step 4: Update PLAN.md line 10**

Replace:
```
- Reverse-engineering and file format details: `DECODE.md`
```
with:
```
- Reverse-engineering, file formats, and game mechanics: `RESEARCH.md`
```

- [ ] **Step 5: Update PLAN.md line ~401**

In the terrain collision mask research task description, replace:
```
   - Document findings in `DECODE.md` (or the relevant world-data section) before finalizing movement collision implementation
```
with:
```
   - Document findings in `RESEARCH.md` (Terrain Collision System section) before finalizing movement collision implementation
```

- [ ] **Step 6: Commit**

```bash
git add AGENTS.md CLAUDE.md README.md PLAN.md
git commit -m "docs: update all DECODE.md references to point to RESEARCH.md

Update canonical source pointers in AGENTS.md, CLAUDE.md, README.md,
and PLAN.md.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 14: Update check_docs_links.sh and Delete DECODE.md

**Files:**
- Modify: `scripts/check_docs_links.sh:7-16`
- Delete: `DECODE.md`

- [ ] **Step 1: Remove DECODE.md from required_files**

In `scripts/check_docs_links.sh`, remove the `"DECODE.md"` line from the `required_files` array (line ~12).

- [ ] **Step 2: Delete DECODE.md**

```bash
git rm DECODE.md
```

- [ ] **Step 3: Commit**

```bash
git add scripts/check_docs_links.sh
git commit -m "docs: delete DECODE.md after merge into RESEARCH.md

All content has been merged into RESEARCH.md. Remove DECODE.md from the
required_files check in check_docs_links.sh.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```

---

### Task 15: Validation

- [ ] **Step 1: Run check_docs_links.sh**

```bash
bash scripts/check_docs_links.sh
```

Expected: passes with no errors.

- [ ] **Step 2: Verify no remaining DECODE.md references**

```bash
git grep -l 'DECODE\.md' -- ':!docs/superpowers/'
```

Expected: no output (no tracked files reference DECODE.md, excluding the design spec in docs/superpowers/).

- [ ] **Step 3: Verify all research_index.toml entries point to RESEARCH.md**

```bash
grep 'doc = ' research_index.toml | grep -v 'RESEARCH.md'
```

Expected: no output.

- [ ] **Step 4: Verify all research_index.toml anchors resolve**

For each `anchor` in research_index.toml, verify the corresponding heading exists in RESEARCH.md. Extract all anchors and check each one:

```bash
# Extract anchors, strip leading #, check each exists as a heading slug in RESEARCH.md
python3 -c "
import re, sys

with open('research_index.toml') as f:
    anchors = re.findall(r'anchor\s*=\s*\"#([^\"]+)\"', f.read())

with open('RESEARCH.md') as f:
    content = f.read()

# Generate slugs from all ## headings
headings = re.findall(r'^##+ (.+)$', content, re.MULTILINE)
slugs = set()
for h in headings:
    slug = h.lower().strip()
    slug = re.sub(r'[^\w\s-]', '', slug)
    slug = re.sub(r'[\s]+', '-', slug)
    slug = re.sub(r'-+', '-', slug)
    slug = slug.strip('-')
    slugs.add(slug)

missing = [a for a in anchors if a not in slugs]
if missing:
    print('MISSING ANCHORS:')
    for m in missing:
        print(f'  #{m}')
    sys.exit(1)
else:
    print(f'All {len(anchors)} anchors resolve OK')
"
```

Expected: "All N anchors resolve OK"

If any anchors fail to resolve, fix them in research_index.toml. The most common issue is punctuation in heading slugs — GitHub-flavored markdown strips most punctuation. Debug by adding `print(slugs)` to see actual generated slugs.

- [ ] **Step 5: Spot-check no duplicate sections**

Verify RESEARCH.md doesn't have duplicate sections covering the same topic:

```bash
grep '^## ' RESEARCH.md | sort | uniq -d
```

Expected: no output (no duplicate `##` headings).

- [ ] **Step 6: Amend or create final commit if any fixes were needed**

If validation found issues that required fixes, stage and commit them:

```bash
git add -A
git commit -m "docs: fix validation issues from DECODE.md merge

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>"
```
