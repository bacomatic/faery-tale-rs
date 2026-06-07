# Data Tables

This directory contains source-extracted tabular data for Faery Tale Adventure.

## What belongs here

Dense per-entry reference tables where the primary value is the data itself rather
than behavioral logic or trace narrative:

- Sprite sheet frame registries (atlases)
- Future candidates: terrain-type table, encounter probability tables, item/weapon
  stat tables

## Trust level

Same as `logic/` — normative, source-verified. Entries are derived from source code
(`fmain.c`, `fmain2.c`, `ftale.h`) with source line references. Unknown entries are
explicitly marked `*(unknown)*` rather than omitted.

## What is NOT here

- Behavioral pseudo-code → see `logic/`
- Raw trace notes → see `_discovery/`
- Narrative/quest data → see `STORYLINE.md` and sub-documents

## Contents

| File | Purpose |
|------|---------|
| [sprites/objects.md](sprites/objects.md) | OBJECTS sheet — 116 frames (items, overlays, effects) |
| [sprites/actors.md](sprites/actors.md) | PHIL + ENEMY sheets — actor animation body frames |
| [sprites/carriers.md](sprites/carriers.md) | RAFT, CARRIER, DRAGON, SETFIG sheets |
