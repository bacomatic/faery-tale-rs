# Reference Material

Reference material for the original Amiga game lives on the `research` branch of this repo. It is indexed in the graphify global graph under the tag `faery-tale-research` and queryable without checking out that branch.

## Querying reference documentation

Always query graphify first. The global graph contains the full research branch: original C source (`fmain.c`, `fmain2.c`, `ftale.h`, etc.), all `reference/logic/` docs, `reference/_discovery/` notes, porting checklists, and sprite data.

**Research-only query (reference docs and original C):**
```bash
graphify query "<topic>" --graph ~/.graphify/global-graph.json --filter repo=faery-tale-research
```

**Cross-codebase query (original C + Rust port together):**
```bash
graphify query "<topic>" --graph ~/.graphify/global-graph.json
```

**Trace a path between two concepts:**
```bash
graphify path "<C symbol>" "<Rust symbol>" --graph ~/.graphify/global-graph.json
```

**Explain a specific node:**
```bash
graphify explain "<symbol or concept>" --graph ~/.graphify/global-graph.json
```

## Reference doc inventory

The research graph includes:
- Original C source: `fmain.c`, `fmain2.c`, `ftale.h`, `terrain.c`, `text.c`, `mtrack.c`, etc.
- `reference/logic/` — verified pseudo-code for every subsystem (combat, AI, doors, magic, inventory, etc.)
- `reference/_discovery/` — per-subsystem reverse-engineering notes
- `reference/ARCHITECTURE.md`, `reference/STORYLINE.md` — high-level structure and quest flow
- `reference/porting/` — per-subsystem porting checklists
- `reference/quest_db.json`, `reference/world_db.json` — structured game data

## Fallback: fetch from GitHub

If the graphify graph is missing content (e.g. a newly added doc not yet re-indexed by the research agent), fetch directly:

- Raw content: `https://raw.githubusercontent.com/bacomatic/faery-tale-rs/research/reference/<path>`
- Browse: `https://github.com/bacomatic/faery-tale-rs/blob/research/reference/<path>`

Binary assets (PNG region maps, `overworld.png`) cannot be graph-queried — fetch with `web_fetch` (raw URL) or `gh api` only when the image is genuinely needed.

## Keeping the graph current

The research agent manages `faery-tale-research` in the global graph independently. If you need to refresh it manually:
```bash
graphify global add <path-to-research-graph.json> --as faery-tale-research
```
