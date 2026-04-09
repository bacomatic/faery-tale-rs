---
description: "Run an experiment to verify a research claim — writes and executes a verification script in tools/, reports pass/fail results"
agent: "experimenter"
argument-hint: "Describe what to verify (e.g., 'validate all source citations in RESEARCH.md', 'extract direction vectors from fsubs.asm', 'verify combat damage formula')"
---
Run an experiment to verify the research claim described below. Follow the full experiment workflow: check for existing tools, write or extend a script, run it, save results, and report findings.

**Experiment:** {{ input }}

## Step 1: Understand the claim

Read the relevant documentation and source code to understand what is being claimed. Identify the specific files, lines, data tables, or formulas involved.

## Step 2: Complete the Reuse Checklist (MANDATORY)

List the `tools/` directory and read the Tool Inventory in the experimenter agent definition. For each existing tool, state whether it applies to this experiment and why. You must explicitly write one of:
- "Extending `<script>.py` because: ..."
- "Importing from `<script>.py` because it provides: ..."
- "Creating new script because no existing tool provides: ... Checked: `<tool1>` (no, because ...), `<tool2>` (no, because ...)"

**Do not proceed to Step 3 without completing this step.**

## Step 3: Write or extend a verification script

Create a new script in `tools/` (or extend an existing one) that mechanically verifies the claim. Follow the naming conventions:
- `validate_*.py` — documentation accuracy checks
- `extract_*.py` — data extraction from source
- `verify_*.py` — formula/algorithm testing
- `decode_*.*` — binary asset parsing

## Step 4: Self-review the script before running

Re-read your script and check:
- Does it test what you think it tests, or does it assume the answer by construction?
- Are expected values derived independently from the source, or hardcoded from the same documentation you're trying to verify? (The latter proves nothing.)
- Does it handle malformed input or missing files gracefully?

## Step 5: Run the experiment

Execute the script from the repo root. Capture all output. Read the FULL output — do not skim.

## Step 6: Save results

Write structured results to `tools/results/` including: experiment name, command to reproduce, pass/fail status, and detailed findings.

## Step 6: Report

Present a clear summary:
- **Status**: PASS, FAIL, PARTIAL, or NEEDS_HUMAN_REVIEW
- **Findings**: what was verified and any mismatches found
- **Action items**: what should be corrected in documentation, if anything
- **Reproduce**: the exact command to re-run this experiment
