#!/usr/bin/env bash
# Rebuild the RAG index. Set INDEX_INCREMENTAL=true to skip --reset.
set -euo pipefail

# Load local env overrides (existing exported vars still win).
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

# Defaults
OLLAMA_URL="${OLLAMA_URL:-http://127.0.0.1:11434}"
EMBED_MODEL="${EMBED_MODEL:-nomic-embed-text}"
INDEX_DB="${INDEX_DB:-.rag/index.sqlite}"
INDEX_ROOT="${INDEX_ROOT:-.}"
INDEX_EXT="${INDEX_EXT:-rs,md,toml}"
INDEX_CHUNK_LINES="${INDEX_CHUNK_LINES:-40}"
INDEX_OVERLAP_LINES="${INDEX_OVERLAP_LINES:-8}"
INDEX_INCREMENTAL="${INDEX_INCREMENTAL:-false}"
INDEX_RESET="${INDEX_RESET:-true}"

# Check Ollama reachability
if ! curl -sf "${OLLAMA_URL}/api/tags" > /dev/null 2>&1; then
    echo "ERROR: Ollama not reachable at ${OLLAMA_URL}" >&2
    echo "Start Ollama first: ollama serve" >&2
    exit 1
fi

# Check model availability
if ! curl -sf "${OLLAMA_URL}/api/tags" | grep -q "${EMBED_MODEL}"; then
    echo "WARNING: Model '${EMBED_MODEL}' not found in Ollama. Proceeding anyway." >&2
fi

# Build index args
ARGS=(
  --db "${INDEX_DB}"
  --root "${INDEX_ROOT}"
  --chunk-lines "${INDEX_CHUNK_LINES}"
  --overlap-lines "${INDEX_OVERLAP_LINES}"
  --ext "${INDEX_EXT}"
  --model "${EMBED_MODEL}"
  --ollama-url "${OLLAMA_URL}"
)

if [ "${INDEX_INCREMENTAL}" = "true" ] || [ "${INDEX_RESET}" = "false" ]; then
    echo "RAG: incremental reindex..."
else
    echo "RAG: full reindex (--reset)..."
    ARGS=(--reset "${ARGS[@]}")
fi

cargo run --bin rag -- index "${ARGS[@]}"
