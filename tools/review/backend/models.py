"""Pydantic models for the review app: verify.json schema, review rounds, drafts.

Run `python tools/review/backend/models.py` to regenerate verify.schema.json.
"""
from __future__ import annotations

import json
from pathlib import Path
from typing import Annotated, Literal, Optional

from pydantic import BaseModel, ConfigDict, Field, StringConstraints, model_validator

TaskId = Annotated[str, StringConstraints(pattern=r"^T\d+(\.\d+)*$")]
ItemId = Annotated[str, StringConstraints(pattern=r"^[A-Za-z0-9][A-Za-z0-9_.-]*$")]

ViewKind = Literal[
    "palette", "table", "text", "image", "json",
    "image-grid", "audio", "markdown", "code", "sprite",
]
Status = Optional[Literal["ok", "problem"]]
Verdict = Literal["ACCEPT", "REJECT"]


class Strict(BaseModel):
    model_config = ConfigDict(extra="forbid")


# --- verify.json -----------------------------------------------------------

class Count(Strict):
    label: str
    file: Optional[str] = None
    path: str = ""
    glob: Optional[str] = None
    expect: int = Field(ge=0)

    @model_validator(mode="after")
    def _one_kind(self) -> "Count":
        if (self.file is None) == (self.glob is None):
            raise ValueError("count needs exactly one of 'file' or 'glob'")
        if self.glob is not None and self.path:
            raise ValueError("'path' is only valid with 'file'")
        return self


class TaskInfo(Strict):
    title: str
    summary: str = ""


class Item(Strict):
    id: ItemId
    task: TaskId
    title: str
    view: ViewKind
    files: list[str] = Field(min_length=1)
    look_for: str
    citations: list[str] = []
    counts: list[Count] = []


class VerifyFile(Strict):
    schema_version: Literal[1]
    tasks: dict[TaskId, TaskInfo] = {}
    items: list[Item] = []


# --- review rounds & drafts ------------------------------------------------

class ItemMark(Strict):
    resource: str
    id: str
    status: Status = None
    note: str = ""


class Round(Strict):
    task: str
    submitted_at: str
    verdict: Verdict
    notes: str = ""
    items: list[ItemMark]
    files: dict[str, str]


class ResultsFile(Strict):
    schema_version: Literal[1] = 1
    rounds: list[Round] = []


class DraftBody(Strict):
    notes: str = ""
    items: list[ItemMark] = []


class Draft(DraftBody):
    task: str
    updated_at: str


class Submission(Strict):
    task: str
    verdict: Verdict
    notes: str = ""
    items: list[ItemMark] = []


class Problem(BaseModel):
    file: str
    task: Optional[str] = None
    item: Optional[str] = None
    message: str


SCHEMA_PATH = Path(__file__).with_name("verify.schema.json")


def verify_schema() -> dict:
    return VerifyFile.model_json_schema()


if __name__ == "__main__":
    SCHEMA_PATH.write_text(json.dumps(verify_schema(), indent=2) + "\n")
    print(f"wrote {SCHEMA_PATH}")
