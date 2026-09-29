// Mirrors tools/review/backend/models.py and the dicts built in store.py.

export type Status = "ok" | "problem" | null;
export type Verdict = "ACCEPT" | "REJECT";
export type TaskState = "not_reviewed" | "draft" | "ACCEPT" | "REJECT" | "stale" | "invalid";

export interface Problem {
  file: string;
  task: string | null;
  item: string | null;
  message: string;
}

export interface TaskSummary {
  task: string;
  title: string;
  state: TaskState;
  latest_verdict: Verdict | null;
  latest_at: string | null;
  stale: boolean;
  has_draft: boolean;
  items: number;
  problems: number;
}

export interface Count {
  label: string;
  file: string | null;
  path: string;
  glob: string | null;
  expect: number;
}

export interface CountResult {
  label: string;
  expect: number;
  actual: number | null;
  ok: boolean;
  error: string | null;
}

export interface ItemDetail {
  resource: string;
  id: string;
  task: string;
  title: string;
  view: string;
  files: string[];
  look_for: string;
  citations: string[];
  counts: Count[];
  resolved_files: string[];
  count_results: CountResult[];
  carried_ok: boolean;
  problems: Problem[];
}

export interface ItemMark {
  resource: string;
  id: string;
  status: Status;
  note: string;
}

export interface Round {
  task: string;
  submitted_at: string;
  verdict: Verdict;
  notes: string;
  items: ItemMark[];
  files: Record<string, string>;
}

export interface Draft {
  task: string;
  notes: string;
  items: ItemMark[];
  updated_at: string;
}

export interface SourceLines {
  path: string;
  start: number;
  end: number;
  lines: string[];
}

export interface TaskDetail extends TaskSummary {
  summary: string;
  resource: string;
  items_detail: ItemDetail[];
  task_problems: Problem[];
  latest_round: Round | null;
  draft: Draft | null;
}
