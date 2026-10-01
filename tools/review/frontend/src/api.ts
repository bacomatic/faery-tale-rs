import type { CompareResult, Draft, ItemMark, Problem, Round, SourceLines, TaskDetail, TaskSummary, Verdict } from "./types";

export class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message);
  }
}

async function req<T>(method: string, url: string, body?: unknown): Promise<T> {
  const r = await fetch(url, {
    method,
    headers: body === undefined ? undefined : { "Content-Type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  if (!r.ok) {
    let msg: unknown = r.statusText;
    try {
      msg = (await r.json()).detail ?? msg;
    } catch {
      /* non-JSON error body */
    }
    throw new ApiError(r.status, typeof msg === "string" ? msg : JSON.stringify(msg));
  }
  return (r.status === 204 ? undefined : await r.json()) as T;
}

const t = encodeURIComponent;

export const api = {
  tasks: () => req<TaskSummary[]>("GET", "/api/tasks"),
  task: (task: string) => req<TaskDetail>("GET", `/api/tasks/${t(task)}`),
  compare: (task: string) => req<CompareResult>("GET", `/api/tasks/${t(task)}/compare`),
  validation: () => req<Problem[]>("GET", "/api/validation"),
  reviews: (task: string) => req<Round[]>("GET", `/api/reviews?task=${t(task)}`),
  source: (path: string, start: number, end: number | null, block = false) =>
    req<SourceLines>(
      "GET",
      `/api/source?path=${t(path)}&start=${start}${end === null ? "" : `&end=${end}`}${block ? "&block=true" : ""}`,
    ),
  putDraft: (task: string, body: { notes: string; items: ItemMark[] }) =>
    req<Draft>("PUT", `/api/drafts/${t(task)}`, body),
  deleteDraft: (task: string) => req<void>("DELETE", `/api/drafts/${t(task)}`),
  submit: (body: { task: string; verdict: Verdict; notes: string; items: ItemMark[] }) =>
    req<Round>("POST", "/api/reviews", body),
};

export const fileUrl = (assetPath: string) =>
  "/files/" + assetPath.split("/").map(encodeURIComponent).join("/");

export const errText = (e: unknown) => (e instanceof Error ? e.message : String(e));
