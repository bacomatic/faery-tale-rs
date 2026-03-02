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

if ! grep -q '^## Issue Map (Rollups)$' "$PLAN_FILE"; then
  echo "ERROR: PLAN.md is missing '## Issue Map (Rollups)' section."
  exit 1
fi

if ! grep -q '^## Status Index (source of truth for humans)$' "$PLAN_FILE"; then
  echo "ERROR: PLAN.md is missing '## Status Index (source of truth for humans)' section."
  exit 1
fi

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

map_lines="$tmp_dir/map_lines.txt"
new_plan="$tmp_dir/PLAN.new.md"

awk '
BEGIN {
  RS="\[\[task\]\]";
  FS="\n";
}

NR == 1 { next }

{
  id="";
  issue="";
  parent="";

  for (i = 1; i <= NF; i++) {
    line = $i;
    if (line ~ /^id = "/) {
      sub(/^id = "/, "", line);
      sub(/".*/, "", line);
      id = line;
    } else if (line ~ /^issue = "/) {
      sub(/^issue = "/, "", line);
      sub(/".*/, "", line);
      issue = line;
    } else if (line ~ /^parent = "/) {
      sub(/^parent = "/, "", line);
      sub(/".*/, "", line);
      parent = line;
    }
  }

  # Only rollups: IDs ending in -001 with no parent.
  if (id ~ /-001$/ && parent == "") {
    if (issue == "") {
      issue = "(missing issue)";
    }
    printf "- `%s` → %s\n", id, issue;
  }
}
' "$STATUS_FILE" > "$map_lines"

awk -v mapfile="$map_lines" '
BEGIN {
  in_map = 0;
  inserted = 0;
}

/^## Issue Map \(Rollups\)$/ {
  print;
  print "";

  while ((getline mapline < mapfile) > 0) {
    print mapline;
  }
  close(mapfile);

  print "";
  in_map = 1;
  inserted = 1;
  next;
}

/^## Status Index \(source of truth for humans\)$/ {
  in_map = 0;
  print;
  next;
}

{
  if (!in_map) {
    print;
  }
}

END {
  if (!inserted) {
    exit 2;
  }
}
' "$PLAN_FILE" > "$new_plan"

mv "$new_plan" "$PLAN_FILE"

echo "Issue Map refreshed in PLAN.md from plan_status.toml"
