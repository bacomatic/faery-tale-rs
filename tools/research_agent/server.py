"""FastAPI HTTP server for the FTA research agent.

Endpoints:
  POST /query          — synchronous query → JSON response
  POST /query/stream   — streaming query → SSE token stream
  GET  /health         — liveness check

Run with:
  tools/run.sh research_agent/server.py [--port 8765]
  python3 tools/research_agent/server.py [--port 8765]
"""
from __future__ import annotations

import asyncio
import logging
import sys
import uuid
import time
from pathlib import Path
from typing import Optional

# Allow running as `python3 tools/research_agent/server.py` directly
if __name__ == "__main__" and __package__ is None:
    _tools_dir = Path(__file__).parent.parent
    if str(_tools_dir) not in sys.path:
        sys.path.insert(0, str(_tools_dir))
    import runpy
    runpy.run_module("research_agent.server", run_name="__main__", alter_sys=True)
    sys.exit(0)

import uvicorn
from fastapi import FastAPI
from fastapi.responses import StreamingResponse
from pydantic import BaseModel

from .agent import Session, create_agent
from .config import Settings

logger = logging.getLogger(__name__)

# In-process session store: session_id → Session
_sessions: dict[str, Session] = {}
_graph = None


def _get_graph():
    global _graph
    if _graph is None:
        _graph = create_agent()
    return _graph


def _get_or_create_session(session_id: str) -> Session:
    if session_id not in _sessions:
        settings = Settings()
        _sessions[session_id] = Session(
            graph=_get_graph(),
            max_history_turns=settings.agent_max_history_turns,
        )
    return _sessions[session_id]


class QueryRequest(BaseModel):
    query: str
    session_id: Optional[str] = None


class UsageInfo(BaseModel):
    prompt_tokens: int
    completion_tokens: int
    tokens_per_sec: float


class QueryResponse(BaseModel):
    answer: str
    sources: list[str]
    session_id: str
    usage: Optional[UsageInfo] = None


def create_app() -> FastAPI:
    settings = Settings()
    app = FastAPI(title="FTA Research Agent")

    @app.get("/health")
    def health():
        return {"status": "ok", "model": settings.openai_model}

    @app.post("/query", response_model=QueryResponse)
    def query(req: QueryRequest):
        sid = req.session_id or str(uuid.uuid4())
        session = _get_or_create_session(sid)
        start = time.monotonic()
        answer, sources, usage = session.query(req.query)
        elapsed = time.monotonic() - start
        usage_info = UsageInfo(**usage) if usage else None
        if usage:
            logger.info(
                "POST /query session=%s latency=%.2fs prompt=%d completion=%d tok/s=%.1f",
                sid[:8],
                elapsed,
                usage["prompt_tokens"],
                usage["completion_tokens"],
                usage["tokens_per_sec"],
            )
        else:
            logger.info("POST /query session=%s latency=%.2fs (tokens: unavailable)", sid[:8], elapsed)
        return QueryResponse(
            answer=answer,
            sources=sources,
            session_id=sid,
            usage=usage_info,
        )

    @app.post("/query/stream")
    async def query_stream(req: QueryRequest):
        sid = req.session_id or str(uuid.uuid4())
        session = _get_or_create_session(sid)

        async def event_generator():
            # Run query in a thread so we don't block the event loop
            loop = asyncio.get_event_loop()
            answer, sources, usage = await loop.run_in_executor(
                None, session.query, req.query
            )
            # Stream the answer word-by-word as SSE events
            for word in answer.split(" "):
                yield f"data: {word} \n\n"
                await asyncio.sleep(0)
            stats = ""
            if usage:
                stats = (
                    f"[{usage['prompt_tokens']}p + {usage['completion_tokens']}c "
                    f"| {usage['tokens_per_sec']} tok/s]"
                )
            yield f"event: done\ndata: {stats}\n\n"

        return StreamingResponse(event_generator(), media_type="text/event-stream")

    return app


def main():
    import argparse
    parser = argparse.ArgumentParser(description="FTA Research Agent HTTP server")
    parser.add_argument("--port", type=int, default=None)
    args = parser.parse_args()

    settings = Settings()
    port = args.port or settings.agent_server_port

    logging.basicConfig(
        level=getattr(logging, settings.agent_log_level.upper(), logging.INFO),
        format="%(asctime)s %(levelname)s %(name)s — %(message)s",
    )

    # Validate LLM reachable before starting server
    import httpx
    try:
        httpx.get(f"{settings.openai_base_url.rstrip('/')}/models", timeout=5)
    except Exception as exc:
        logger.error("LLM endpoint unreachable at %s: %s", settings.openai_base_url, exc)
        sys.exit(1)

    uvicorn.run(create_app(), host="127.0.0.1", port=port, log_level="warning")


if __name__ == "__main__":
    main()
