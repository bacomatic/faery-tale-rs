#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel)"
cd "$ROOT_DIR"

defaults_file="agent_defaults.toml"

get_toml_string() {
  local key="$1"
  awk -F ' = ' -v k="$key" '$1 == k {gsub(/"/, "", $2); print $2; exit}' "$defaults_file"
}

echo "==> Running plan consistency check"
bash scripts/plan_sync_check.sh

echo "==> Running docs link check"
bash scripts/check_docs_links.sh

if [[ ! -f "$defaults_file" ]]; then
  echo "warning: ${defaults_file} not found; skipping RAG default checks"
  exit 0
fi

ollama_url="$(get_toml_string "ollama_url")"
embed_model="$(get_toml_string "embed_model")"
llm_model="$(get_toml_string "llm_model")"

missing=0

if ! command -v ollama >/dev/null 2>&1; then
  echo "warning: ollama is not installed"
  echo "  install: https://ollama.com/download"
  missing=1
else
  if ! curl -fsS "${ollama_url}/api/tags" >/dev/null 2>&1; then
    echo "warning: ollama server not reachable at ${ollama_url}"
    echo "  start: ollama serve"
    missing=1
  fi

  if ! ollama list | awk '{print $1}' | grep -Eq "^${embed_model}(:|$)"; then
    echo "warning: embedding model missing: ${embed_model}"
    echo "  pull: ollama pull ${embed_model}"
    missing=1
  fi

  if ! ollama list | awk '{print $1}' | grep -Eq "^${llm_model}(:|$)"; then
    echo "warning: LLM model missing: ${llm_model}"
    echo "  pull: ollama pull ${llm_model}"
    missing=1
  fi
fi

echo "==> Suggested next commands"
echo "  make plan-check"
echo "  make docs-check"
echo "  make rag-demo Q=\"where is page flip handled\""
echo "  make sync-issues"

if [[ "$missing" -ne 0 ]]; then
  echo "==> Bootstrap completed with warnings"
else
  echo "==> Bootstrap completed"
fi
