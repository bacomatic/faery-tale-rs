#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel)"
cd "$ROOT_DIR"

required_files=(
  "README.md"
  "AGENTS.md"
  "RAG.md"
  "PLAN.md"
  "plan_status.toml"
  "DECODE.md"
  "RESEARCH.md"
  "research_index.toml"
  "scripts/plan_sync_check.sh"
  "scripts/rag_demo.sh"
  ".env.example"
)

for required in "${required_files[@]}"; do
  if [[ ! -e "$required" ]]; then
    echo "missing required file: $required"
    exit 1
  fi
done

if ! grep -q 'RAG.md' README.md; then
  echo "README.md must reference RAG.md"
  exit 1
fi

if ! grep -q 'RAG.md' AGENTS.md; then
  echo "AGENTS.md must reference RAG.md"
  exit 1
fi

if ! grep -q 'plan_status.toml' RAG.md || ! grep -q 'PLAN.md' RAG.md; then
  echo "RAG.md must include source-of-truth references to PLAN.md and plan_status.toml"
  exit 1
fi

echo "docs link check passed"
