"""FastAPI routes for the review app. Run with:

    mise exec -- python -m uvicorn app:app --app-dir tools/review/backend --host 127.0.0.1 --port 8765
"""
from __future__ import annotations

from pathlib import Path
from typing import Optional

from fastapi import FastAPI, HTTPException
from fastapi.responses import FileResponse
from fastapi.staticfiles import StaticFiles

from models import DraftBody, Submission
from store import NotFound, Rejected, Store

REPO_ROOT = Path(__file__).resolve().parents[3]
DIST = Path(__file__).resolve().parents[1] / "frontend" / "dist"


def create_app(store: Store, dist: Optional[Path] = DIST) -> FastAPI:
    app = FastAPI(title="Faery Tale asset review")

    def call(fn, *args):
        try:
            return fn(*args)
        except NotFound as e:
            raise HTTPException(404, str(e))
        except Rejected as e:
            raise HTTPException(409, str(e))
        except ValueError as e:
            raise HTTPException(400, str(e))

    @app.get("/api/tasks")
    def tasks():
        bundle = store.load()
        return [store.task_summary(bundle, t) for t in bundle.task_ids()]

    @app.get("/api/tasks/{task}")
    def task(task: str):
        return call(store.task_detail, task)

    @app.get("/api/validation")
    def validation():
        return [p.model_dump() for p in store.load().problems]

    @app.get("/api/source")
    def source(path: str, start: Optional[int] = None, end: Optional[int] = None,
               block: bool = False):
        return call(store.source_lines, path, start, end, block)

    @app.get("/files/{path:path}")
    def files(path: str):
        try:
            return FileResponse(call(store.asset_file, path))
        except HTTPException as e:
            raise HTTPException(404, "not found") if e.status_code == 400 else e

    @app.get("/api/drafts/{task}")
    def get_draft(task: str):
        draft = call(store.get_draft, task)
        if draft is None:
            raise HTTPException(404, "no draft")
        return draft

    @app.put("/api/drafts/{task}")
    def put_draft(task: str, body: DraftBody):
        return call(store.put_draft, task, body)

    @app.delete("/api/drafts/{task}", status_code=204)
    def delete_draft(task: str):
        call(store.delete_draft, task)

    @app.post("/api/reviews", status_code=201)
    def submit(sub: Submission):
        return call(store.submit, sub)

    @app.get("/api/reviews")
    def reviews(task: Optional[str] = None):
        return store.rounds(task)

    if dist is not None and dist.is_dir():
        app.mount("/", StaticFiles(directory=dist, html=True), name="frontend")

    return app


app = create_app(Store(REPO_ROOT))
