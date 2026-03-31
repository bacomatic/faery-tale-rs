# Bug Tracker Skill Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `/bug` workflow that investigates bugs deeply, records each bug in a session growable spec, files one GitHub issue per bug, comments on issues when fix design is approved, and then transitions into writing-plans.

**Architecture:** A thin VS Code slash prompt in the project (`.github/prompts/bug.prompt.md`) delegates to a new reusable superpowers skill (`skills/bug-tracker/SKILL.md`) in the superpowers repository. The skill controls the state machine: investigate -> append spec section -> file issue -> design fix -> comment on issue -> invoke writing-plans once for the session.

**Tech Stack:** Markdown prompt files, superpowers SKILL.md conventions, VS Code custom prompts (`.github/prompts`), GitHub MCP issue/comment tools, shell-based skill trigger tests.

---

## File Structure

### Files to create

- `/home/ddehaven/projects/superpowers/skills/bug-tracker/SKILL.md`
- `/home/ddehaven/projects/faery-tale-rs/.github/prompts/bug.prompt.md`
- `/home/ddehaven/projects/superpowers/tests/explicit-skill-requests/prompts/use-bug-tracker.txt`

### Files to modify

- `/home/ddehaven/projects/superpowers/README.md` (add skill mention)
- `/home/ddehaven/projects/superpowers/tests/explicit-skill-requests/run-all.sh` (add explicit trigger test)

---

### Task 1: Create `bug-tracker` skill with full workflow contract

**Files:**
- Create: `/home/ddehaven/projects/superpowers/skills/bug-tracker/SKILL.md`

- [ ] **Step 1: Write the skill file with required frontmatter and hard gates**

Create `/home/ddehaven/projects/superpowers/skills/bug-tracker/SKILL.md` with this exact header and top-level sections:

```markdown
---
name: bug-tracker
description: Use when a user wants to capture and investigate a bug from a short description, track it in a session spec, and correlate it with GitHub issues before planning implementation
---

# Bug Tracker

## Overview

Capture bugs from short user descriptions, investigate deeply before proposing fixes, maintain a session-scoped bug spec, and keep GitHub issue correlation tight through issue creation and follow-up comments.

<HARD-GATE>
Do NOT write implementation code or apply code fixes while running this skill. This skill ends by invoking writing-plans.
</HARD-GATE>
```

- [ ] **Step 2: Add the process checklist in skill order**

Add this ordered checklist section to the skill:

```markdown
## Checklist

1. Explore relevant project context and affected code paths
2. Investigate deeply (systematic-debugging Phase 1 style)
3. Create or append session bug spec file in `docs/superpowers/specs/YYYY-MM-DD-bugs.md`
4. File one GitHub issue per bug using spec content as issue body
5. Back-fill issue ID/link in bug section header
6. Ask whether to add another bug or move to fix design
7. For each bug: propose 2-3 fix approaches and get approval
8. Post approved design summary as comment on corresponding GitHub issue
9. Invoke writing-plans for session-wide implementation plan with issue cross-references
```

- [ ] **Step 3: Add explicit session-spec rules and section template**

Add a section that defines session behavior and this bug template:

```markdown
## Session Spec Contract

- Session spec path: `docs/superpowers/specs/YYYY-MM-DD-bugs.md`
- First `/bug` in a session creates the file with session header.
- Subsequent `/bug` runs append new bug sections to the same file.
- Every bug section header must include issue ID once filed.

### Bug Section Template

## Bug #<issue_number>: <bug_title> — [#<issue_number>](https://github.com/<owner>/<repo>/issues/<issue_number>)

**Filed:** YYYY-MM-DD
**Status:** investigating | designed | planned

### Description
<expanded from user-provided short description>

### Investigation
- **Affected files:** <paths>
- **Root cause:** <confirmed or best current hypothesis>
- **Reproduction path:** <steps>
- **Evidence:** <code/trace/history evidence>

### Fix Design
<added after user approves approach>

### Plan Reference
<added after writing-plans runs>
```

- [ ] **Step 4: Add GitHub integration behavior and fallback text**

Add a section requiring:

```markdown
## GitHub Integration

- File one issue per bug after writing Description + Investigation.
- Issue title must match the bug section title.
- Issue body must be generated from the corresponding bug section.
- If issue creation tools are unavailable, pause and ask user for permission to continue with local-only tracking.
- After fix design approval, post a comment on the issue containing:
  - chosen approach name
  - 2-3 sentence summary
  - spec file path + bug section reference
  - plan file path once available
```

- [ ] **Step 5: Add transition rule to writing-plans**

Append this exact transition line near the end of `SKILL.md`:

```markdown
After all bugs in the current session have approved fix designs, invoke `writing-plans` and produce one implementation plan that references each bug's `#<issue_number>`.
```

- [ ] **Step 6: Commit skill scaffold**

Run:

```bash
cd /home/ddehaven/projects/superpowers
git add skills/bug-tracker/SKILL.md
git commit -m "feat(skill): add bug-tracker workflow with session spec and github issue correlation"
```

Expected: one new committed file, no unrelated changes staged.

---

### Task 2: Add `/bug` slash prompt that delegates to the skill

**Files:**
- Create: `/home/ddehaven/projects/faery-tale-rs/.github/prompts/bug.prompt.md`

- [ ] **Step 1: Create prompt directory**

Run:

```bash
cd /home/ddehaven/projects/faery-tale-rs
mkdir -p .github/prompts
```

Expected: `.github/prompts` exists.

- [ ] **Step 2: Create `bug.prompt.md` with argument input**

Create `/home/ddehaven/projects/faery-tale-rs/.github/prompts/bug.prompt.md` with:

```markdown
---
name: bug
description: Investigate and record a bug from a short description, then track it to issue and planning
argument-hint: Short bug description
agent: agent
---

User-reported bug: ${input:Bug description}

Load and follow the `bug-tracker` skill from superpowers.

Requirements:
- Investigate deeply before proposing fixes.
- Append to session bug spec in docs/superpowers/specs/YYYY-MM-DD-bugs.md.
- File one GitHub issue per bug and record issue ID in the section header.
- When fix design is approved, comment on the linked issue.
- Transition to writing-plans when user indicates bug intake is complete.
```

- [ ] **Step 3: Validate prompt metadata fields are present**

Run:

```bash
cd /home/ddehaven/projects/faery-tale-rs
grep -nE '^(name|description|argument-hint|agent):' .github/prompts/bug.prompt.md
```

Expected output includes four lines for `name`, `description`, `argument-hint`, and `agent`.

- [ ] **Step 4: Commit prompt file**

Run:

```bash
cd /home/ddehaven/projects/faery-tale-rs
git add .github/prompts/bug.prompt.md
git commit -m "feat(prompt): add /bug slash prompt delegating to bug-tracker skill"
```

Expected: one new committed prompt file.

---

### Task 3: Add discoverability updates in superpowers docs

**Files:**
- Modify: `/home/ddehaven/projects/superpowers/README.md`

- [ ] **Step 1: Add bug-tracker to Skills Library section**

In `/home/ddehaven/projects/superpowers/README.md`, under debugging or collaboration skills, add:

```markdown
- **bug-tracker** - Session-based bug intake from short descriptions with deep investigation, GitHub issue correlation, and planning handoff
```

- [ ] **Step 2: Add one mention in basic workflow**

In "The Basic Workflow", add a line after brainstorming and before writing-plans:

```markdown
- **bug-tracker** - For bug-driven work, capture and investigate bugs, create issue-linked session specs, then hand off to writing-plans
```

- [ ] **Step 3: Verify README contains two `bug-tracker` mentions**

Run:

```bash
cd /home/ddehaven/projects/superpowers
grep -n "bug-tracker" README.md
```

Expected: two matching lines.

- [ ] **Step 4: Commit README update**

Run:

```bash
cd /home/ddehaven/projects/superpowers
git add README.md
git commit -m "docs: document bug-tracker skill in workflow and skills list"
```

Expected: README-only commit.

---

### Task 4: Add explicit skill-trigger regression coverage

**Files:**
- Create: `/home/ddehaven/projects/superpowers/tests/explicit-skill-requests/prompts/use-bug-tracker.txt`
- Modify: `/home/ddehaven/projects/superpowers/tests/explicit-skill-requests/run-all.sh`

- [ ] **Step 1: Create explicit request prompt fixture**

Create `/home/ddehaven/projects/superpowers/tests/explicit-skill-requests/prompts/use-bug-tracker.txt`:

```text
Use the bug-tracker skill. I want to capture a bug where opening the inventory sometimes freezes the UI after a quick map transition.
```

- [ ] **Step 2: Add test invocation in `run-all.sh`**

Append this block before summary output in `/home/ddehaven/projects/superpowers/tests/explicit-skill-requests/run-all.sh`:

```bash
# Test: use bug-tracker
echo ">>> Test 5: use-bug-tracker"
if "$SCRIPT_DIR/run-test.sh" "bug-tracker" "$PROMPTS_DIR/use-bug-tracker.txt"; then
    PASSED=$((PASSED + 1))
    RESULTS="$RESULTS\nPASS: use-bug-tracker"
else
    FAILED=$((FAILED + 1))
    RESULTS="$RESULTS\nFAIL: use-bug-tracker"
fi
echo ""
```

- [ ] **Step 3: Run explicit-skill test for bug-tracker only**

Run:

```bash
cd /home/ddehaven/projects/superpowers
tests/explicit-skill-requests/run-test.sh bug-tracker tests/explicit-skill-requests/prompts/use-bug-tracker.txt 3
```

Expected: output contains `PASS: Skill 'bug-tracker' was triggered`.

- [ ] **Step 4: Commit test updates**

Run:

```bash
cd /home/ddehaven/projects/superpowers
git add tests/explicit-skill-requests/prompts/use-bug-tracker.txt tests/explicit-skill-requests/run-all.sh
git commit -m "test: add explicit skill request coverage for bug-tracker"
```

Expected: test fixture + runner update committed together.

---

### Task 5: End-to-end dry run of `/bug` flow and planning handoff

**Files:**
- Create/Modify during run: `/home/ddehaven/projects/faery-tale-rs/docs/superpowers/specs/YYYY-MM-DD-bugs.md`
- Create during handoff: `/home/ddehaven/projects/faery-tale-rs/docs/superpowers/plans/YYYY-MM-DD-<topic>.md`

- [ ] **Step 1: Run manual chat dry-run with `/bug` prompt**

In VS Code chat, run:

```text
/bug inventory freezes after quick map transition
```

Expected behavior:
- agent investigates code paths first
- creates/appends `docs/superpowers/specs/YYYY-MM-DD-bugs.md`
- files issue and updates section header to `Bug #<id>`
- asks whether to add another bug or move to fix design

- [ ] **Step 2: Add a second bug in same session**

In same chat session, run:

```text
/bug status text sometimes uses wrong tint after leaving intro scene
```

Expected behavior:
- same `YYYY-MM-DD-bugs.md` file reused
- second `## Bug #<id>` section appended
- second issue filed and linked in header

- [ ] **Step 3: Move to fix design and verify issue comments**

In same session, run:

```text
move to fix design
```

Expected behavior:
- 2-3 approaches proposed per bug with approval loop
- approved design written into each bug section under `### Fix Design`
- comment posted on each corresponding issue with chosen approach summary

- [ ] **Step 4: Verify transition to writing-plans**

In same session, approve all bug designs and request planning.

Expected behavior:
- `writing-plans` invoked
- one plan generated with task groups that each include issue references (e.g., `#123`)

- [ ] **Step 5: Commit workspace prompt/spec/plan artifacts if desired**

Run:

```bash
cd /home/ddehaven/projects/faery-tale-rs
git status --short
```

If the generated spec/plan docs should be kept, commit them. If they were only for dry-run validation, remove them before commit.

---

## Self-Review Checklist

- [ ] Every requirement from [docs/superpowers/specs/2026-03-31-bug-tracker-design.md](docs/superpowers/specs/2026-03-31-bug-tracker-design.md) maps to at least one task above
- [ ] No placeholder language (`TBD`, `TODO`, `implement later`) remains in this plan
- [ ] Session-spec behavior is explicit: first `/bug` creates, later `/bug` appends
- [ ] Issue correlation behavior is explicit in both skill content and manual verification
- [ ] Final handoff explicitly invokes writing-plans and produces issue-referenced tasks
