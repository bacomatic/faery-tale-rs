# Local RAG Guide

This repository includes a local RAG CLI binary: `rag`.

It stores chunk text and embeddings in a local SQLite DB (default: `.rag/index.sqlite`) and uses Ollama for local embeddings and local answer generation.

## Source of Truth Policy

RAG is for discovery and navigation, not canonical project state.

- Canonical machine-readable task state remains `plan_status.toml`.
- Canonical human-readable task narrative remains `PLAN.md`.
- `scripts/plan_sync_check.sh` and related plan tooling rely on exact structured plan data.
- Use RAG to find relevant code/docs quickly, then confirm state transitions and task updates directly in plan files.
- When there is a conflict between RAG output and plan files, trust the plan files.

## Prerequisites

Install and run Ollama, then pull models:

    $ ollama pull nomic-embed-text
    $ ollama pull llama3.2

## Core Commands

Index the repository (defaults to `.rs`, `.md`, `.toml`):

    $ cargo run --bin rag -- index --reset

Query top matches:

    $ cargo run --bin rag -- query "where is page flip handled"

Bias retrieval toward source code files:

    $ cargo run --bin rag -- query --code-first "where is page flip handled"

Restrict retrieval to file types:

    $ cargo run --bin rag -- query --only-ext rs "where is page flip handled"

End-to-end answer (retrieve + generate):

    $ cargo run --bin rag -- ask "how does intro music start"

Answer with sources and code-first ranking:

    $ cargo run --bin rag -- ask --code-first --show-sources "where is page flip handled"

Restrict answer retrieval to Rust files:

    $ cargo run --bin rag -- ask --only-ext rs --show-sources "where is page flip handled"

## One-command Demo Script

Run index + ask in one command:

    $ bash scripts/rag_demo.sh "where is page flip handled"

Use the provided environment template for quick setup:

    $ cp .env.example .env
    $ bash scripts/rag_demo.sh "where is page flip handled"

Optional agent defaults live in `agent_defaults.toml`.
Use `scripts/agent_bootstrap.sh` to run consistency checks and verify local RAG prerequisites.

    $ bash scripts/agent_bootstrap.sh

`scripts/rag_demo.sh` automatically loads `.env.local` (preferred) or `.env` if present.
Explicitly exported variables still take precedence over file values.

The script preflights:
- `ollama` installed
- Ollama server reachable
- embedding + generation models present

By default, the ask step is code-biased and code-filtered (`ASK_CODE_FIRST=true`, `ONLY_EXT=rs`).

### Script Environment Variables

- `OLLAMA_URL` — Ollama server URL (default: `http://127.0.0.1:11434`)
- `EMBED_MODEL` — embedding model (default: `nomic-embed-text`)
- `INDEX_EXT` — indexed file extensions, comma-separated (default: `rs,md,toml`)
- `INDEX_ROOT` — root path to index (default: `.`)
- `INDEX_DB` — SQLite DB path (default: `.rag/index.sqlite`)
- `INDEX_CHUNK_LINES` — chunk size in lines (default: `40`)
- `INDEX_OVERLAP_LINES` — chunk overlap lines (default: `8`)
- `INDEX_RESET` — reset DB before index (`true`/`false`, default: `true`)
- `INDEX_INCREMENTAL` — only index changed/untracked git files (`true`/`false`, default: `false`)
- `ASK_LLM_MODEL` — generation model (default: `llama3.2`)
- `ONLY_EXT` — ask/query extension filter (default: `rs`)
- `ASK_TOP_K` — retrieval top-k for ask (default: `5`)
- `ASK_MAX_CONTEXT_CHARS` — context size cap for ask prompt (default: `8000`)
- `ASK_SHOW_SOURCES` — include sources in output (`true`/`false`, default: `true`)
- `ASK_CODE_FIRST` — boost code files in retrieval (`true`/`false`, default: `true`)
- `ASK_CONFIDENCE_FALLBACK` — auto-retry code questions with strict code retrieval when top source is weak (`true`/`false`, default: `true`)
- `ASK_MIN_SOURCE_SCORE` — fallback threshold for first source score (default: `0.70`)

Example overrides:

    $ INDEX_EXT=rs INDEX_DB=.rag/code.sqlite ASK_TOP_K=3 ASK_MAX_CONTEXT_CHARS=4000 bash scripts/rag_demo.sh "where is page flip handled"
    $ INDEX_INCREMENTAL=true INDEX_RESET=false bash scripts/rag_demo.sh "what changed in map rendering"
    $ ASK_SHOW_SOURCES=false ASK_CODE_FIRST=false ONLY_EXT=md,toml bash scripts/rag_demo.sh "plan status"

## CLI Options (quick reference)

- `index --db <path>` change SQLite location
- `index --root <path>` index a different root
- `index --chunk-lines` / `--overlap-lines` tune chunking
- `index --ext` control indexed extensions
- `--ollama-url` and `--model` select embedding endpoint/model
- `ask --llm-model` select generation model
- `ask --max-context-chars` cap prompt context size
- `query --code-first` and `ask --code-first` boost `.rs` chunks, de-prioritize planning/status docs
- `query --only-ext <ext[,ext...]>` and `ask --only-ext <ext[,ext...]>` filter retrieval by extension
