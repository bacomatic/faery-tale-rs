#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PLAN_FILE="$ROOT_DIR/PLAN.md"
STATUS_FILE="$ROOT_DIR/plan_status.toml"

if [[ ! -f "$PLAN_FILE" ]]; then
  echo "ERROR: Missing PLAN.md at $PLAN_FILE"
  exit 1
fi

if [[ ! -f "$STATUS_FILE" ]]; then
  echo "ERROR: Missing plan_status.toml at $STATUS_FILE"
  exit 1
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

all_ids_file="$tmp_dir/all_ids.txt"
rollup_ids_file="$tmp_dir/rollup_ids.txt"
parent_ids_file="$tmp_dir/parent_ids.txt"
index_ids_file="$tmp_dir/index_ids.txt"
invalid_states_file="$tmp_dir/invalid_states.txt"
issue_errors_file="$tmp_dir/issue_errors.txt"

grep '^id = "' "$STATUS_FILE" | sed -E 's/^id = "([^"]+)".*/\1/' | sort -u > "$all_ids_file"
grep '^id = ".*-001"' "$STATUS_FILE" | sed -E 's/^id = "([^"]+)".*/\1/' | sort -u > "$rollup_ids_file"

if grep -q '^parent = "' "$STATUS_FILE"; then
  grep '^parent = "' "$STATUS_FILE" | sed -E 's/^parent = "([^"]+)".*/\1/' | sort -u > "$parent_ids_file"
else
  : > "$parent_ids_file"
fi

awk '
  /^## Status Index/ { in_index=1; next }
  in_index && /^### Completed/ { in_index=0; next }
  in_index { print }
' "$PLAN_FILE" \
  | grep -oE '\[[a-z0-9-]+\]' \
  | tr -d '[]' \
  | sort -u > "$index_ids_file" || true

grep '^state = "' "$STATUS_FILE" \
  | sed -E 's/^state = "([^"]+)".*/\1/' \
  | grep -Ev '^(todo|in_progress|blocked|done)$' > "$invalid_states_file" || true

awk '
BEGIN {
  RS="\\[\\[task\\]\\]";
  FS="\n";
}

NR == 1 { next }

{
  id="";
  state="";
  issue="";
  provenance="";
  evidence="";

  for (i = 1; i <= NF; i++) {
    line = $i;
    if (line ~ /^id = "/) {
      sub(/^id = "/, "", line);
      sub(/".*/, "", line);
      id = line;
    } else if (line ~ /^state = "/) {
      sub(/^state = "/, "", line);
      sub(/".*/, "", line);
      state = line;
    } else if (line ~ /^issue = "/) {
      sub(/^issue = "/, "", line);
      sub(/".*/, "", line);
      issue = line;
    } else if (line ~ /^provenance = "/) {
      sub(/^provenance = "/, "", line);
      sub(/".*/, "", line);
      provenance = line;
    } else if (line ~ /^evidence = "/) {
      sub(/^evidence = "/, "", line);
      sub(/".*/, "", line);
      evidence = line;
    }
  }

  if (id == "") {
    next;
  }

  if (state == "done") {
    if (issue == "") {
      print "Task " id " is state=done but missing issue field.";
      next;
    }

    if (issue != "pre-issues" && issue != "n/a" && issue !~ /^#[0-9]+$/) {
      print "Task " id " has invalid issue value: " issue " (expected pre-issues, n/a, or #<number>).";
    }

    if (issue == "pre-issues") {
      if (provenance != "completed-before-github-issues") {
        print "Task " id " uses issue=pre-issues but has invalid provenance: " provenance;
      }
      if (evidence == "") {
        print "Task " id " uses issue=pre-issues but is missing evidence.";
      }
    }
  } else {
    if (issue == "pre-issues") {
      print "Task " id " is not done but has issue=pre-issues.";
    }
  }
}
' "$STATUS_FILE" > "$issue_errors_file"

errors=0

if [[ ! -s "$rollup_ids_file" ]]; then
  echo "ERROR: No rollup task IDs found (expected IDs ending in -001)."
  errors=1
fi

if [[ -s "$invalid_states_file" ]]; then
  echo "ERROR: Invalid task states found in plan_status.toml:"
  sed 's/^/  - /' "$invalid_states_file"
  errors=1
fi

if ! grep -q '^issue_tracking = "github"$' "$STATUS_FILE"; then
  echo "ERROR: Missing or invalid issue_tracking metadata in plan_status.toml (expected issue_tracking = \"github\")."
  errors=1
fi

if ! grep -q '^issues_enabled_on = "[0-9][0-9][0-9][0-9]-[0-9][0-9]-[0-9][0-9]"$' "$STATUS_FILE"; then
  echo "ERROR: Missing or invalid issues_enabled_on metadata in plan_status.toml (expected YYYY-MM-DD)."
  errors=1
fi

if [[ -s "$issue_errors_file" ]]; then
  echo "ERROR: Issue/provenance metadata problems in plan_status.toml:"
  sed 's/^/  - /' "$issue_errors_file"
  errors=1
fi

while IFS= read -r parent_id; do
  if ! grep -qx "$parent_id" "$all_ids_file"; then
    echo "ERROR: parent points to unknown task ID: $parent_id"
    errors=1
  fi
done < "$parent_ids_file"

while IFS= read -r rollup_id; do
  if ! grep -q "^parent = \"$rollup_id\"" "$STATUS_FILE"; then
    echo "ERROR: Rollup task has no child tasks: $rollup_id"
    errors=1
  fi
done < "$rollup_ids_file"

missing_in_status="$tmp_dir/missing_in_status.txt"
missing_in_plan="$tmp_dir/missing_in_plan.txt"

comm -23 "$index_ids_file" "$rollup_ids_file" > "$missing_in_status" || true
comm -13 "$index_ids_file" "$rollup_ids_file" > "$missing_in_plan" || true

if [[ -s "$missing_in_status" ]]; then
  echo "ERROR: PLAN status index IDs missing as rollups in plan_status.toml:"
  sed 's/^/  - /' "$missing_in_status"
  errors=1
fi

if [[ -s "$missing_in_plan" ]]; then
  echo "ERROR: Rollup IDs in plan_status.toml missing from PLAN status index:"
  sed 's/^/  - /' "$missing_in_plan"
  errors=1
fi

rollup_count=$(wc -l < "$rollup_ids_file" | tr -d ' ')
index_count=$(wc -l < "$index_ids_file" | tr -d ' ')

if [[ "$errors" -ne 0 ]]; then
  echo
  echo "plan_sync_check: FAILED"
  exit 1
fi

echo "plan_sync_check: OK"
echo "  rollups in plan_status.toml: $rollup_count"
echo "  IDs in PLAN.md status index: $index_count"