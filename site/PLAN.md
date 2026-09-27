# Site plan — reference documentation & asset browser

A static MkDocs (Material) site that browses the whole research corpus. Designed to be built
in **small bursts** — each phase below is independent, ~15–45 min, and leaves the site working.

## Decisions (locked with the user, 2026-08-30)
- **Engine:** MkDocs + Material theme (already installed in `.toolenv`; dep recorded in
  `tools/requirements.txt`).
- **v1 scope:** docs + search + clickable source citations + asset gallery.
- **Built site is committed** (browsable straight from a checkout). Config + generator scripts
  also committed. The staging/content dir is gitignored.
- **Location:** this top-level `site/` dir.

## Layout (target)
```
site/
  PLAN.md          # this file
  README.md        # how to build/serve (write in Phase 1)
  mkdocs.yml       # MkDocs config
  gen.py           # single generator script: stage content → render src → linkify → galleries
  content/         # STAGING (gitignored) — assembled docs tree mkdocs builds from
  build/           # COMMITTED built site (mkdocs output)
```

Build/serve:
```
.toolenv/bin/python site/gen.py           # stage + generate + mkdocs build → site/build/
.toolenv/bin/python -m http.server -d site/build 8000
```

## Phases (do in order; each ends with a working site)

### Phase 1 — skeleton: docs + search  (~30 min)
- [ ] `site/mkdocs.yml`: Material theme, `docs_dir: content`, `site_dir: build`, search plugin
      (default), repo-agnostic (no edit links).
- [ ] `site/gen.py` v1: wipe/rebuild `content/`; copy `reference/**/*.md` → `content/reference/`,
      `docs/**/*.md` → `content/docs/`; write a small `content/index.md` landing page linking the
      major entry points (`reference/README.md`, ARCHITECTURE, RESEARCH index, logic/, _discovery/,
      `docs/README.md`). Then run `mkdocs build`.
- [ ] `.gitignore`: add `site/content/`.
- [ ] `site/README.md`: the two commands above + one paragraph on what the site is.
- Verify: `python -m http.server`, open, click through 5 docs, search for "encounter_chart".
- Known wrinkle: `world_db.json`/`quest_db.json` are large — do NOT copy into content in v1.

### Phase 2 — rendered source with line anchors  (~30 min)
- [ ] Extend `gen.py`: render every `src/*.{c,h,asm,i}` (read-only inputs!) to
      `content/src/<name>.html` via pygments: `HtmlFormatter(linenos='inline', lineanchors='L',
      anchorlinenos=True, full=True)` → line N gets anchor `#L-N`.
- [ ] Add a `content/src/index.md` listing the rendered files.
- Verify: open `build/src/fmain.c.html#L-1222` — lands on `load_track_range(896,...)`.

### Phase 3 — clickable citations  (~30–45 min)
- [ ] Extend `gen.py`: after copying md files, rewrite citations to links into the Phase-2 pages.
      Pattern (inside or outside backticks): `(fmain|fmain2|mtrack|gdriver|narr|ftale|...)\.(c|h|asm|i):(\d+)(-\d+)?`
      → `[\`file:lines\`](<rel>/src/<file>.html#L-<start>)`. Compute `<rel>` from each md file's
      depth. Skip fenced code blocks (citations inside ``` blocks stay plain — simplest rule; refine
      later only if it hurts).
- Verify: open `reference/logic/day-night.md` in the site; click 3 citations; each lands on the
  right line. Spot-check a doc deep in `_discovery/`.

### Phase 4 — asset gallery v1 (what exists today)  (~45 min)
- [ ] Extend `gen.py`: generate `content/gallery/*.md` pages (markdown with inline HTML):
  - **Palettes** — from `assets/palettes/*.json`: swatch grid per palette (colored `<span>`s from
    `rgba8`), index + `rgb4` labels.
  - **Sprites (pre-bundle)** — thumbnail grid of `sprite_output/*.png` grouped by actor prefix;
    CSS `image-rendering: pixelated`, click = open full size.
  - **Day/night experiment** — grid of `experiment/shaders/` baked frames if present.
- [ ] Add gallery to the nav.
- Verify: palette swatches match known colors (e.g. pagecolors[0] black); sprites render crisp.

### Phase 5 — polish & wire-in  (~20 min, optional)
- [ ] Nav ordering/titles in `mkdocs.yml` (reference first, gallery second, src last).
- [ ] Add a one-line pointer to `site/` in `docs/README.md` (task-doc index).
- [ ] Decide whether `build/` needs a `.nojekyll`/anything for your hosting (if ever hosted).

### Later (post-Wave-2, not now)
- Gallery pages for each landed asset task (T1.4 text, T2.x) — extend `gen.py` per resource;
  audio pages with `<audio>` players for SFX/music previews.
- Light-level reference-render browser page (T3.1 `assets/shaders/previews/`).
- Consider `mkdocs serve` watch mode for authoring sessions.

## Conventions
- Generator is **one file** (`gen.py`), stdlib + pygments + mkdocs API only. No framework, no
  plugins beyond Material defaults. Keep it under ~300 lines; split only when that breaks.
- `content/` is disposable; never edit it by hand. `reference/`/`docs/` md files are the source.
- Source files under `src/` are read-only — rendered, never modified.
- Rebuild is one command; commit `site/build/` after each phase (with user consent, as always).
