# Design: FTA Research Agent

**Date**: 2026-06-07
**Status**: approved
**Branch**: research

## Summary

Add a locally-hosted LLM research agent that answers queries about *The Faery Tale Adventure* codebase by reasoning over the reference documentation. The agent replaces context-mode for information retrieval — instead of semantic search it uses a LangGraph ReAct loop to decide which docs to read, then synthesises an answer with source citations.

The backend is any OpenAI-compatible server (LM Studio, Ollama, etc.), configured via env vars. The agent runs as both an ACP-style HTTP server and an interactive REPL.

---

## Architecture

```
tools/research_agent/
  __init__.py
  agent.py        # LangGraph ReAct graph — core reasoning loop
  tools.py        # LangChain tools: list_dir, read_file, search_text, read_source_file
  server.py       # FastAPI HTTP server  (POST /query, POST /query/stream, GET /health)
  repl.py         # Interactive REPL with persistent in-process session memory
  config.py       # Pydantic settings — reads .env and env vars
  AGENT.md        # Entry document loaded as the system prompt
tools/.env.example          # Committed template; user copies to .env (gitignored)
tools/requirements.txt      # Extended with new dependencies
tools/README.md             # New section documenting the agent
.github/agents/research.agent.md   # Agent definition for Copilot discovery
```

Entry points (via the existing `tools/run.sh` venv wrapper):

```bash
tools/run.sh research_agent/server.py [--port 8765]
tools/run.sh research_agent/repl.py
```

---

## Components

### agent.py — LangGraph ReAct Loop

- Uses `langgraph` `create_react_agent` with a `ChatOpenAI` backend
- System prompt is the full content of `AGENT.md`, prepended once per session
- Tools available: `list_directory`, `read_file`, `search_text`, `read_source_file`
- Graph runs until the model emits a final answer or hits `AGENT_MAX_TOOL_STEPS` (default 15)
- On step-limit hit: returns partial answer with a note that reasoning was truncated
- Returns: `{answer: str, sources: list[str], usage: TokenUsage | None}`

### tools.py — LangChain Tools

| Tool | Signature | Notes |
|------|-----------|-------|
| `list_directory` | `(path: str) → str` | Lists files in a `reference/` subdirectory. Path must be under `reference/`. |
| `read_file` | `(path: str) → str` | Reads any file under `reference/`. |
| `search_text` | `(pattern: str, path: str) → str` | Grep-style regex search. `path` is a file or directory under `reference/`; defaults to all of `reference/` if omitted. Returns matching lines with `file:line` context, capped at 50 results. |
| `read_source_file` | `(path: str) → str` | Reads original `.c`/`.asm`/`.h` source. Gated by `AGENT.md` instructions. |

**Path safety** (all tools):
- Paths are resolved relative to the repo root
- Any `..` traversal or absolute path raises a `ToolError` returned to the model
- `read_file` is restricted to `reference/`
- `read_source_file` is restricted to repo root source files (`.c`, `.asm`, `.h`, `.i`, `.p`)
- Tool errors return a structured message to the model so it can recover, not a Python exception

### server.py — FastAPI HTTP Server

**Endpoints:**

```
POST /query
  Request:  { "query": str, "session_id": str? }
  Response: { "answer": str, "sources": [str], "session_id": str,
              "usage": { "prompt_tokens": int, "completion_tokens": int, "tokens_per_sec": float } | null }

POST /query/stream
  Request:  same as /query
  Response: text/event-stream (SSE) — token-by-token answer stream

GET /health
  Response: { "status": "ok", "model": str }
```

**Session state:**
- Sessions stored in-process dict keyed by `session_id` (UUID)
- Each session holds a `messages` list (LangChain `BaseMessage` history)
- System prompt prepended once at session creation, not re-sent each turn
- Trimmed to `AGENT_MAX_HISTORY_TURNS` (default 20) oldest non-system messages when exceeded
- Sessions lost on server restart — no disk persistence

**Startup:**
- Validates LLM endpoint is reachable on startup; logs clear error and exits if not

**Logging** (each request):
```
POST /query  session=abc123  latency=1.24s  prompt=312  completion=87  tok/s=23.4
```

### repl.py — Interactive REPL

- Persistent session for the life of the process (full message history retained)
- Streams tokens to terminal as they arrive
- After each response: compact stats line `[1.2s | 312 prompt + 87 completion tokens | 23 tok/s]`
  - Falls back to `[tokens: unavailable]` if the local server doesn't return usage data
- Special commands: `/reset` (clear history), `/sources` (show last turn's full source list), `/quit`

### config.py — Settings

Loaded via `pydantic-settings` from `.env` + environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `OPENAI_BASE_URL` | `http://localhost:1234/v1` | LLM API base URL |
| `OPENAI_API_KEY` | `lm-studio` | API key (any non-empty string for local servers) |
| `OPENAI_MODEL` | *(required)* | Model name as the server expects it |
| `AGENT_MAX_HISTORY_TURNS` | `20` | Max conversation turns before trimming |
| `AGENT_MAX_TOOL_STEPS` | `15` | Max ReAct tool calls per query before truncating |
| `AGENT_SERVER_PORT` | `8765` | HTTP server port |
| `AGENT_LOG_LEVEL` | `INFO` | `DEBUG` enables per-tool-call traces |

### AGENT.md — Entry Document

Content:
1. **Role statement**: what the agent is, what questions it can answer
2. **Operating rules**: no guessing, cite sources using `file:LINE` format, escalate gaps
3. **Document index**: one entry per reference file — filename, what questions it answers
4. **world_db.json schema summary**: field names, types, and how to query by region/sector/coordinate (not the full 600KB)
5. **Tool usage guidance**: start with `search_text`, narrow before fetching full files; only call `read_source_file` when the user explicitly requests source-level detail
6. **Source citation format**: `reference/RESEARCH-terrain-combat.md§3.2`, `fmain.c:1609`

---

## Configuration Files

### `.env.example` (committed) and `.env` (gitignored)

Both live at `tools/.env.example` and `tools/.env`. `config.py` loads `tools/.env` explicitly (path resolved relative to the `research_agent/` package directory), so the venv wrapper doesn't need to set a working directory.

```ini
# LM Studio (default)
OPENAI_BASE_URL=http://localhost:1234/v1
OPENAI_API_KEY=lm-studio
OPENAI_MODEL=meta-llama-3.1-8b-instruct

# Ollama alternative:
# OPENAI_BASE_URL=http://localhost:11434/v1
# OPENAI_API_KEY=ollama
# OPENAI_MODEL=llama3.1

# Tuning
AGENT_MAX_HISTORY_TURNS=20
AGENT_MAX_TOOL_STEPS=15
AGENT_SERVER_PORT=8765
AGENT_LOG_LEVEL=INFO
```

### `tools/requirements.txt` additions

```
langchain-openai>=0.2
langgraph>=0.2
fastapi>=0.115
uvicorn[standard]>=0.30
python-dotenv>=1.0
pydantic-settings>=2.0
```

---

## New Files Summary

| File | Purpose |
|------|---------|
| `tools/research_agent/__init__.py` | Package marker |
| `tools/research_agent/agent.py` | LangGraph ReAct loop |
| `tools/research_agent/tools.py` | LangChain tool definitions |
| `tools/research_agent/server.py` | FastAPI HTTP server |
| `tools/research_agent/repl.py` | Interactive REPL |
| `tools/research_agent/config.py` | Pydantic settings |
| `tools/research_agent/AGENT.md` | System prompt / entry document |
| `tools/.env.example` | Configuration template |
| `.github/agents/research.agent.md` | Agent definition for Copilot |

## Modified Files

| File | Change |
|------|--------|
| `tools/requirements.txt` | Add 6 new dependencies |
| `tools/README.md` | New section: Research Agent |
| `.gitignore` | Add `tools/.env` if not already present |

---

## Verification

- [ ] `tools/run.sh research_agent/server.py` starts and `GET /health` returns 200
- [ ] `POST /query` returns a non-empty answer with at least one source
- [ ] `POST /query/stream` streams tokens via SSE
- [ ] REPL starts, accepts a query, streams response, prints token stats
- [ ] `/reset` clears history; follow-up query has no memory of prior turn
- [ ] Path traversal (`../../../etc/passwd`) returns a tool error, not file contents
- [ ] Works with LM Studio URL and with Ollama URL
- [ ] Token stats appear in logs; gracefully handles missing `usage` field

---

## Risks & Considerations

- **Context window**: Local models often have 4k–8k context. Fetching multiple large docs in one session may overflow. AGENT.md must be concise and tool guidance must discourage bulk reads. The step limit (15) also caps runaway tool loops.
- **Model capability**: Smaller local models (7B) may struggle with multi-hop reasoning across several files. The AGENT.md guidance and tool descriptions are the primary mitigation — clear framing helps small models stay on task.
- **world_db.json**: At 600KB this is too large to read in full. The AGENT.md schema summary and `search_text` are the intended access path.
- **No auth on HTTP server**: The server is localhost-only by default. `server.py` should bind to `127.0.0.1` not `0.0.0.0`.
