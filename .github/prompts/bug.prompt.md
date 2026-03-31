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
