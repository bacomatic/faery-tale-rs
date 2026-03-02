#!/usr/bin/env bash
set -euo pipefail

# Auto-load local env file if present (existing exported vars still win).
if [[ -f ".env.local" ]]; then
  set -a
  # shellcheck disable=SC1091
  source ".env.local"
  set +a
elif [[ -f ".env" ]]; then
  set -a
  # shellcheck disable=SC1091
  source ".env"
  set +a
fi

QUESTION="${1:-how does intro music start}"
OLLAMA_URL="${OLLAMA_URL:-http://127.0.0.1:11434}"
EMBED_MODEL="${EMBED_MODEL:-nomic-embed-text}"
INDEX_EXT="${INDEX_EXT:-rs,md,toml}"
INDEX_ROOT="${INDEX_ROOT:-.}"
INDEX_DB="${INDEX_DB:-.rag/index.sqlite}"
INDEX_CHUNK_LINES="${INDEX_CHUNK_LINES:-40}"
INDEX_OVERLAP_LINES="${INDEX_OVERLAP_LINES:-8}"
INDEX_RESET="${INDEX_RESET:-true}"
INDEX_INCREMENTAL="${INDEX_INCREMENTAL:-false}"
ONLY_EXT="${ONLY_EXT:-rs}"
ASK_TOP_K="${ASK_TOP_K:-5}"
ASK_LLM_MODEL="${ASK_LLM_MODEL:-llama3.2}"
ASK_MAX_CONTEXT_CHARS="${ASK_MAX_CONTEXT_CHARS:-8000}"
ASK_SHOW_SOURCES="${ASK_SHOW_SOURCES:-true}"
ASK_CODE_FIRST="${ASK_CODE_FIRST:-true}"
ASK_CONFIDENCE_FALLBACK="${ASK_CONFIDENCE_FALLBACK:-true}"
ASK_MIN_SOURCE_SCORE="${ASK_MIN_SOURCE_SCORE:-0.70}"

is_true() {
  local value
  value="$(echo "$1" | tr '[:upper:]' '[:lower:]')"
  [[ "$value" == "1" || "$value" == "true" || "$value" == "yes" || "$value" == "on" ]]
}

is_false() {
  local value
  value="$(echo "$1" | tr '[:upper:]' '[:lower:]')"
  [[ "$value" == "0" || "$value" == "false" || "$value" == "no" || "$value" == "off" ]]
}

require_bool() {
  local name value
  name="$1"
  value="$2"
  if ! is_true "$value" && ! is_false "$value"; then
    echo "error: ${name} must be a boolean (true/false/1/0/yes/no/on/off), got '${value}'."
    exit 1
  fi
}

require_uint() {
  local name value
  name="$1"
  value="$2"
  if ! [[ "$value" =~ ^[0-9]+$ ]]; then
    echo "error: ${name} must be a non-negative integer, got '${value}'."
    exit 1
  fi
}

require_float() {
  local name value
  name="$1"
  value="$2"
  if ! [[ "$value" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
    echo "error: ${name} must be a non-negative number, got '${value}'."
    exit 1
  fi
}

contains_ext() {
  local list target token
  list="$1"
  target="$2"
  IFS=',' read -r -a token <<< "$list"
  for t in "${token[@]}"; do
    if [[ "$(echo "$t" | tr '[:upper:]' '[:lower:]' | sed 's/^\.//')" == "$target" ]]; then
      return 0
    fi
  done
  return 1
}

question_is_code_like() {
  echo "$QUESTION" | grep -Eiq 'where|how|impl|implement|function|method|struct|enum|module|file|code|handled|render|parse|load|scene|audio|page flip|bug|fix|src/'
}

run_ask_command() {
  local force_code_first="$1"
  local force_only_ext="$2"

  local -a ask_only_ext_args=()
  if [[ -n "$force_only_ext" ]]; then
    ask_only_ext_args=(--only-ext "$force_only_ext")
  elif [[ -n "$ONLY_EXT" ]]; then
    ask_only_ext_args=(--only-ext "$ONLY_EXT")
  fi

  local -a ask_flags=()
  if is_true "$ASK_SHOW_SOURCES"; then
    ask_flags+=(--show-sources)
  fi
  if [[ "$force_code_first" == "true" ]] || is_true "$ASK_CODE_FIRST"; then
    ask_flags+=(--code-first)
  fi

  cargo run --bin rag -- ask --db "${INDEX_DB}" --top-k "${ASK_TOP_K}" --max-context-chars "${ASK_MAX_CONTEXT_CHARS}" --llm-model "${ASK_LLM_MODEL}" --model "${EMBED_MODEL}" --ollama-url "${OLLAMA_URL}" "${ask_flags[@]}" "${ask_only_ext_args[@]}" "${QUESTION}"
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    cat <<'EOF'
Usage: bash scripts/rag_demo.sh [QUESTION]

Runs local RAG end-to-end:
1) Re-index repository chunks into .rag/index.sqlite
2) Ask one question and print answer + sources

Environment variables:
  OLLAMA_URL   Ollama server URL (default: http://127.0.0.1:11434)
  EMBED_MODEL  Embedding model for indexing/query embedding (default: nomic-embed-text)
  INDEX_EXT    Comma-separated file extensions to index (default: rs,md,toml)
  INDEX_ROOT   Root path to index (default: .)
  INDEX_DB     SQLite DB path for index/query (default: .rag/index.sqlite)
  INDEX_CHUNK_LINES  Chunk size in lines for index (default: 40)
  INDEX_OVERLAP_LINES  Chunk overlap in lines for index (default: 8)
  INDEX_RESET  Reset DB before indexing (default: true)
  INDEX_INCREMENTAL  Index only changed files from git status (default: false)
  ASK_LLM_MODEL LLM model for answer generation (default: llama3.2)
  ONLY_EXT     Retrieval extension filter for ask/query (default: rs)
  ASK_TOP_K    Number of retrieved chunks for ask (default: 5)
  ASK_MAX_CONTEXT_CHARS  Max chars of retrieved context sent to LLM (default: 8000)
  ASK_SHOW_SOURCES  Include source list in output (default: true)
  ASK_CODE_FIRST  Bias retrieval toward source code files (default: true)
  ASK_CONFIDENCE_FALLBACK  Re-run with strict code retrieval if source confidence is weak (default: true)
  ASK_MIN_SOURCE_SCORE  Threshold for top source score before fallback (default: 0.70)

Examples:
  bash scripts/rag_demo.sh
  bash scripts/rag_demo.sh "where is page flip handled"
  INDEX_DB=.rag/code.sqlite INDEX_EXT=rs bash scripts/rag_demo.sh "where is page flip handled"
  INDEX_INCREMENTAL=true INDEX_RESET=false bash scripts/rag_demo.sh "what changed in map rendering"
  INDEX_EXT=rs bash scripts/rag_demo.sh "where is page flip handled"
  ONLY_EXT=rs bash scripts/rag_demo.sh "where is page flip handled"
  ASK_LLM_MODEL=llama3.2 bash scripts/rag_demo.sh "where is page flip handled"
  ASK_TOP_K=3 bash scripts/rag_demo.sh "where is page flip handled"
  ASK_MAX_CONTEXT_CHARS=4000 bash scripts/rag_demo.sh "where is page flip handled"
  ASK_CONFIDENCE_FALLBACK=true ASK_MIN_SOURCE_SCORE=0.75 bash scripts/rag_demo.sh "where is page flip handled"
  ASK_SHOW_SOURCES=false ASK_CODE_FIRST=false bash scripts/rag_demo.sh "plan status"
  ONLY_EXT=md,toml bash scripts/rag_demo.sh "plan status"
EOF
    exit 0
fi

require_uint "INDEX_CHUNK_LINES" "$INDEX_CHUNK_LINES"
require_uint "INDEX_OVERLAP_LINES" "$INDEX_OVERLAP_LINES"
require_uint "ASK_TOP_K" "$ASK_TOP_K"
require_uint "ASK_MAX_CONTEXT_CHARS" "$ASK_MAX_CONTEXT_CHARS"
require_bool "INDEX_RESET" "$INDEX_RESET"
require_bool "INDEX_INCREMENTAL" "$INDEX_INCREMENTAL"
require_bool "ASK_SHOW_SOURCES" "$ASK_SHOW_SOURCES"
require_bool "ASK_CODE_FIRST" "$ASK_CODE_FIRST"
require_bool "ASK_CONFIDENCE_FALLBACK" "$ASK_CONFIDENCE_FALLBACK"
require_float "ASK_MIN_SOURCE_SCORE" "$ASK_MIN_SOURCE_SCORE"

if [[ "$INDEX_CHUNK_LINES" -eq 0 ]]; then
  echo "error: INDEX_CHUNK_LINES must be > 0."
  exit 1
fi
if [[ "$INDEX_OVERLAP_LINES" -ge "$INDEX_CHUNK_LINES" ]]; then
  echo "error: INDEX_OVERLAP_LINES must be less than INDEX_CHUNK_LINES."
  exit 1
fi
if [[ "$ASK_TOP_K" -eq 0 ]]; then
  echo "error: ASK_TOP_K must be > 0."
  exit 1
fi

if is_true "$INDEX_INCREMENTAL" && is_true "$INDEX_RESET"; then
  echo "warning: INDEX_INCREMENTAL=true with INDEX_RESET=true would drop prior index state; forcing INDEX_RESET=false."
  INDEX_RESET="false"
fi

if ! command -v ollama >/dev/null 2>&1; then
  echo "error: ollama is not installed."
  echo "install from https://ollama.com/download or your distro package manager."
  exit 1
fi

if ! curl -fsS "${OLLAMA_URL}/api/tags" >/dev/null 2>&1; then
  echo "error: ollama server not reachable at ${OLLAMA_URL}."
  echo "start it with: ollama serve"
  exit 1
fi

tags_json="$(curl -fsS "${OLLAMA_URL}/api/tags")"

if ! echo "$tags_json" | grep -Eo '"name":"[^"]+"' | sed -E 's/"name":"([^"]+)"/\1/' | grep -Eq "^${EMBED_MODEL}(:|$)"; then
  echo "error: embedding model '${EMBED_MODEL}' is missing."
  echo "pull it with: ollama pull ${EMBED_MODEL}"
  exit 1
fi

if ! echo "$tags_json" | grep -Eo '"name":"[^"]+"' | sed -E 's/"name":"([^"]+)"/\1/' | grep -Eq "^${ASK_LLM_MODEL}(:|$)"; then
  echo "error: ask LLM model '${ASK_LLM_MODEL}' is missing."
  echo "pull it with: ollama pull ${ASK_LLM_MODEL}"
  exit 1
fi

echo "==> Re-indexing local RAG DB"
INDEX_ARGS=(
  --db "${INDEX_DB}"
  --root "${INDEX_ROOT}"
  --chunk-lines "${INDEX_CHUNK_LINES}"
  --overlap-lines "${INDEX_OVERLAP_LINES}"
  --ext "${INDEX_EXT}"
  --model "${EMBED_MODEL}"
  --ollama-url "${OLLAMA_URL}"
)
if is_true "$INDEX_RESET"; then
  INDEX_ARGS=(--reset "${INDEX_ARGS[@]}")
fi

if is_true "$INDEX_INCREMENTAL"; then
  if git -C "${INDEX_ROOT}" rev-parse --is-inside-work-tree >/dev/null 2>&1; then
    mapfile -t changed_paths < <(
      {
        git -C "${INDEX_ROOT}" diff --name-only --diff-filter=ACMR HEAD --
        git -C "${INDEX_ROOT}" ls-files --others --exclude-standard
      } | awk 'NF' | sort -u
    )

    if [[ "${#changed_paths[@]}" -eq 0 ]]; then
      if [[ -f "${INDEX_DB}" ]] && ! is_true "$INDEX_RESET"; then
        echo "==> Incremental mode: no changed files detected; skipping index step"
      else
        echo "==> Incremental mode: no changed files detected; running full index"
        cargo run --bin rag -- index "${INDEX_ARGS[@]}"
      fi
    else
      for changed_path in "${changed_paths[@]}"; do
        INDEX_ARGS+=(--path "$changed_path")
      done
      echo "==> Incremental mode: indexing ${#changed_paths[@]} changed/untracked files"
      cargo run --bin rag -- index "${INDEX_ARGS[@]}"
    fi
  else
    echo "==> Incremental mode requested but INDEX_ROOT is not a git work tree; running full index"
    cargo run --bin rag -- index "${INDEX_ARGS[@]}"
  fi
else
  cargo run --bin rag -- index "${INDEX_ARGS[@]}"
fi

echo "==> Asking: ${QUESTION}"
ask_output="$(run_ask_command "false" "")"

if is_true "$ASK_CONFIDENCE_FALLBACK" && is_true "$ASK_SHOW_SOURCES" && question_is_code_like; then
  top_source_line="$(echo "$ask_output" | awk '/^\[[0-9]+\] [0-9]+(\.[0-9]+)? /{print; exit}')"
  top_score="$(echo "$top_source_line" | awk '{print $2}')"
  top_path_with_range="$(echo "$top_source_line" | awk '{print $3}')"
  top_path="${top_path_with_range%%:*}"

  if [[ -z "${top_score:-}" ]]; then
    top_score="0"
  fi

  low_score="$(awk -v s="$top_score" -v t="$ASK_MIN_SOURCE_SCORE" 'BEGIN { print (s < t) ? 1 : 0 }')"
  top_not_rs=0
  if [[ -n "$top_path" && "$top_path" != *.rs ]]; then
    top_not_rs=1
  fi

  strict_already=0
  if is_true "$ASK_CODE_FIRST" && contains_ext "$ONLY_EXT" "rs"; then
    strict_already=1
  fi

  if [[ "$strict_already" -eq 0 ]] && { [[ "$low_score" -eq 1 ]] || [[ "$top_not_rs" -eq 1 ]]; }; then
    echo "==> Confidence gate: weak/non-code top source (score=${top_score}); retrying with --code-first --only-ext rs"
    ask_output="$(run_ask_command "true" "rs")"
  fi
fi

echo "$ask_output"
