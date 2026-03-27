#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel)"
cd "$ROOT_DIR"

required_files=(
  "README.md"
  "AGENTS.md"
  "PLAN.md"
  "plan_status.toml"
  "RESEARCH.md"
  "research_index.toml"
  "scripts/plan_sync_check.sh"
)

for required in "${required_files[@]}"; do
  if [[ ! -e "$required" ]]; then
    echo "missing required file: $required"
    exit 1
  fi
done

echo "docs link check passed"
