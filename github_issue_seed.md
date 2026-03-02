# GitHub Issue Seed (from PLAN rollups)

This file bootstraps GitHub Issues for active roadmap rollups in `PLAN.md` / `plan_status.toml`.

## Prerequisites

```bash
gh auth status
```

## Conventions

- Prefix issue titles with the rollup ID (for example: `[world-001] ...`).
- Add the rollup ID to the body so plan ↔ issue mapping is searchable.
- For tasks already completed before 2026-03-01, use `issue = "pre-issues"` in `plan_status.toml`.
- Do **not** create retroactive placeholder issues for pre-issues work.

## Labels to create once (optional)

```bash
gh label create roadmap --color 1d76db --description "Roadmap rollup task" || true
gh label create area-audio --color 0052cc --description "Audio system work" || true
gh label create area-graphics --color 5319e7 --description "Graphics/effects work" || true
gh label create area-world --color 0e8a16 --description "World/map systems" || true
gh label create area-gameplay --color fbca04 --description "Gameplay/player systems" || true
gh label create area-npc --color d93f0b --description "NPC systems" || true
gh label create area-input --color b60205 --description "Input/key/controller systems" || true
gh label create area-save --color cfd3d7 --description "Persistence/save systems" || true
```

## Seed issues

### 1) [audio-001] Audio system

```bash
gh issue create \
  --title "[audio-001] Audio system" \
  --label "roadmap" --label "area-audio" \
  --body "Rollup ID: audio-001

Source of truth:
- PLAN.md (Status Index + Audio plan)
- plan_status.toml task id=audio-001

Current status:
- in_progress (core playback complete; deferred gameplay audio items remain)

Acceptance:
- [ ] audio-103 sound effects channel implemented
- [ ] audio-105 gameplay setmood switching implemented
"
```

### 2) [gfx-001] Graphics effects

```bash
gh issue create \
  --title "[gfx-001] Graphics effects" \
  --label "roadmap" --label "area-graphics" \
  --body "Rollup ID: gfx-001

Source of truth:
- PLAN.md (Status Index + Graphics plan)
- plan_status.toml task id=gfx-001

Current status:
- in_progress (palette infrastructure in; gameplay wiring pending)

Acceptance:
- [ ] gfx-101 day/night cycle gameplay wiring complete
- [ ] gfx-102 copper list parsing complete
- [ ] gfx-103 witch effect complete
- [ ] gfx-104 teleport effect complete
"
```

### 3) [world-001] Game world & map system

```bash
gh issue create \
  --title "[world-001] Game world & map system" \
  --label "roadmap" --label "area-world" \
  --body "Rollup ID: world-001

Source of truth:
- PLAN.md (Status Index + World plan)
- plan_status.toml task id=world-001

Current status:
- todo

Acceptance:
- [ ] world-101..world-109 complete
"
```

### 4) [player-001] Player & movement

```bash
gh issue create \
  --title "[player-001] Player & movement" \
  --label "roadmap" --label "area-gameplay" \
  --body "Rollup ID: player-001

Source of truth:
- PLAN.md (Status Index + Player plan)
- plan_status.toml task id=player-001

Current status:
- todo

Acceptance:
- [ ] player-101..player-105 complete
"
```

### 5) [npc-001] NPC system

```bash
gh issue create \
  --title "[npc-001] NPC system" \
  --label "roadmap" --label "area-npc" \
  --body "Rollup ID: npc-001

Source of truth:
- PLAN.md (Status Index + NPC plan)
- plan_status.toml task id=npc-001

Current status:
- todo

Acceptance:
- [ ] npc-101..npc-103 complete
"
```

### 6) [keys-001] Key bindings

```bash
gh issue create \
  --title "[keys-001] Key bindings" \
  --label "roadmap" --label "area-input" \
  --body "Rollup ID: keys-001

Source of truth:
- PLAN.md (Status Index + Key bindings plan)
- plan_status.toml task id=keys-001

Current status:
- todo

Acceptance:
- [ ] keys-101..keys-107 complete
"
```

### 7) [persist-001] Persistence

```bash
gh issue create \
  --title "[persist-001] Persistence" \
  --label "roadmap" --label "area-save" \
  --body "Rollup ID: persist-001

Source of truth:
- PLAN.md (Status Index + Persistence plan)
- plan_status.toml task id=persist-001

Current status:
- todo

Acceptance:
- [ ] persist-101..persist-103 complete
"
```

## After creating issues

Update each matching rollup in `plan_status.toml` with `issue = "#<number>"` and keep
`PLAN.md` status index text in sync.
