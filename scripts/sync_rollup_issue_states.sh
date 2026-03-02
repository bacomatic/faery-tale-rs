#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
STATUS_FILE="$ROOT_DIR/plan_status.toml"
TODAY="$(date +%F)"
STRICT_OPEN=0

usage() {
    cat <<'USAGE'
Usage: bash scripts/sync_rollup_issue_states.sh [--strict-open]

Default behavior:
    - CLOSED GitHub rollup issue -> task state "done"
    - OPEN GitHub rollup issue -> no state change

--strict-open:
    - CLOSED GitHub rollup issue -> task state "done"
    - OPEN GitHub rollup issue -> task state "in_progress"
USAGE
}

if [[ $# -gt 1 ]]; then
    usage
    exit 2
fi

if [[ $# -eq 1 ]]; then
    case "$1" in
        --strict-open)
            STRICT_OPEN=1
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "ERROR: Unknown argument: $1"
            usage
            exit 2
            ;;
    esac
fi

if [[ ! -f "$STATUS_FILE" ]]; then
  echo "ERROR: Missing plan_status.toml at $STATUS_FILE"
  exit 1
fi

if ! command -v gh >/dev/null 2>&1; then
  echo "ERROR: gh CLI not found in PATH"
  exit 1
fi

SYNC_STRICT_OPEN="$STRICT_OPEN" python3 - <<'PY'
import datetime
import os
import json
import re
import subprocess
from pathlib import Path

root = Path.cwd()
status_file = root / "plan_status.toml"
today = datetime.date.today().isoformat()
strict_open = os.environ.get("SYNC_STRICT_OPEN", "0") == "1"

text = status_file.read_text()

proc = subprocess.run(
    [
        "gh",
        "issue",
        "list",
        "--state",
        "all",
        "--limit",
        "500",
        "--json",
        "number,state",
    ],
    check=True,
    capture_output=True,
    text=True,
)
issues = json.loads(proc.stdout)
state_by_number = {str(item["number"]): item["state"] for item in issues}

blocks = text.split("[[task]]")
out = [blocks[0]]
changed = 0
missing_issues = []

for block in blocks[1:]:
    id_match = re.search(r'^id = "([^"]+)"', block, re.M)
    parent_match = re.search(r'^parent = "([^"]+)"', block, re.M)
    issue_match = re.search(r'^issue = "([^"]+)"', block, re.M)
    state_match = re.search(r'^state = "([^"]+)"', block, re.M)

    if not id_match:
        out.append("[[task]]" + block)
        continue

    task_id = id_match.group(1)
    has_parent = bool(parent_match)
    current_issue = issue_match.group(1) if issue_match else ""
    current_state = state_match.group(1) if state_match else ""

    # Only rollups with real issue numbers are synced from GitHub state.
    if task_id.endswith("-001") and not has_parent and re.fullmatch(r"#\d+", current_issue):
        issue_num = current_issue[1:]
        gh_state = state_by_number.get(issue_num)
        if gh_state is None:
            missing_issues.append((task_id, current_issue))
        else:
            # One-way safety: CLOSED -> done.
            if gh_state == "CLOSED" and current_state != "done":
                block = re.sub(r'^state = "[^"]+"', 'state = "done"', block, flags=re.M)
                if re.search(r'^updated = "[^"]+"', block, re.M):
                    block = re.sub(r'^updated = "[^"]+"', f'updated = "{today}"', block, flags=re.M)
                else:
                    block = block.rstrip() + f'\nupdated = "{today}"\n'
                changed += 1
            # Optional strict mode: OPEN -> in_progress.
            elif strict_open and gh_state == "OPEN" and current_state not in ("in_progress", "done"):
                block = re.sub(r'^state = "[^"]+"', 'state = "in_progress"', block, flags=re.M)
                if re.search(r'^updated = "[^"]+"', block, re.M):
                    block = re.sub(r'^updated = "[^"]+"', f'updated = "{today}"', block, flags=re.M)
                else:
                    block = block.rstrip() + f'\nupdated = "{today}"\n'
                changed += 1

    out.append("[[task]]" + block)

new_text = "".join(out)

if changed > 0:
    new_text = re.sub(r'^last_updated = "[^"]+"', f'last_updated = "{today}"', new_text, flags=re.M)
    status_file.write_text(new_text)

print(f"sync_rollup_issue_states: updated {changed} rollup task(s)")
print(f"sync_rollup_issue_states: strict_open={'on' if strict_open else 'off'}")
if missing_issues:
    print("sync_rollup_issue_states: warning: issue references not found on GitHub:")
    for task_id, issue in missing_issues:
        print(f"  - {task_id}: {issue}")
PY

echo "sync_rollup_issue_states: done"
