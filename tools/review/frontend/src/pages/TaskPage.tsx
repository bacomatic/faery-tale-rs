import { useCallback, useEffect, useRef, useState } from "react";
import { api, errText } from "../api";
import HistoryPanel from "../components/HistoryPanel";
import ItemCard, { type Mark } from "../components/ItemCard";
import ProblemList from "../components/ProblemList";
import StateBadge from "../components/StateBadge";
import VerdictBar from "../components/VerdictBar";
import type { ItemMark, TaskDetail, Verdict } from "../types";

type Marks = Record<string, Mark>;
type SaveState = "idle" | "saving" | "saved" | "error";

const EMPTY: Mark = { status: null, note: "" };
const keyOf = (resource: string, id: string) => `${resource}/${id}`;

// Item IDs never contain "/", so the last "/" separates resource from id.
function toItemMarks(marks: Marks): ItemMark[] {
  return Object.entries(marks)
    .filter(([, m]) => m.status || m.note.trim())
    .map(([k, m]) => {
      const i = k.lastIndexOf("/");
      return { resource: k.slice(0, i), id: k.slice(i + 1), status: m.status, note: m.note };
    });
}

export default function TaskPage({ task }: { task: string }) {
  const [detail, setDetail] = useState<TaskDetail | null>(null);
  const [marks, setMarks] = useState<Marks>({});
  const [notes, setNotes] = useState("");
  const [err, setErr] = useState<string | null>(null);
  const [save, setSave] = useState<SaveState>("idle");
  const [busy, setBusy] = useState(false);
  const [flash, setFlash] = useState<string | null>(null);
  const [historyTick, setHistoryTick] = useState(0);
  const dirty = useRef(false);
  const timer = useRef<number | undefined>(undefined);

  const load = useCallback(async () => {
    const d = await api.task(task);
    const m: Marks = {};
    for (const it of d.items_detail) if (it.carried_ok) m[keyOf(it.resource, it.id)] = { status: "ok", note: "" };
    for (const it of d.draft?.items ?? []) m[keyOf(it.resource, it.id)] = { status: it.status, note: it.note };
    dirty.current = false;
    setDetail(d);
    setMarks(m);
    setNotes(d.draft?.notes ?? "");
  }, [task]);

  useEffect(() => {
    load().catch((e) => setErr(errText(e)));
  }, [load]);

  useEffect(() => {
    if (!dirty.current) return;
    setSave("saving");
    timer.current = window.setTimeout(async () => {
      try {
        const items = toItemMarks(marks);
        if (!items.length && !notes.trim()) await api.deleteDraft(task);
        else await api.putDraft(task, { notes, items });
        setSave("saved");
      } catch (e) {
        setSave("error");
        setErr(errText(e));
      }
    }, 500);
    return () => window.clearTimeout(timer.current);
  }, [marks, notes, task]);

  const setMark = (key: string, m: Mark) => {
    dirty.current = true;
    setFlash(null);
    setMarks((prev) => ({ ...prev, [key]: m }));
  };
  const setTaskNotes = (s: string) => {
    dirty.current = true;
    setFlash(null);
    setNotes(s);
  };

  const submit = async (verdict: Verdict) => {
    window.clearTimeout(timer.current);
    setBusy(true);
    setErr(null);
    try {
      await api.submit({ task, verdict, notes, items: toItemMarks(marks) });
      await load();
      setSave("idle");
      setFlash(`${verdict} recorded in assets/tasks/review_results.json`);
      setHistoryTick((n) => n + 1);
    } catch (e) {
      setErr(errText(e));
    } finally {
      setBusy(false);
    }
  };

  if (!detail) return err ? <p className="error">{err}</p> : <p className="muted">Loading…</p>;

  const items = detail.items_detail;
  const statuses = items.map((it) => (marks[keyOf(it.resource, it.id)] ?? EMPTY).status);
  const problemCount = statuses.filter((s) => s === "problem").length;
  const unmarkedCount = statuses.filter((s) => !s).length;
  const carriedCount = items.filter((it) => it.carried_ok).length;

  return (
    <>
      <div className="task-head">
        <h1>
          {detail.task} — {detail.title}
        </h1>
        <StateBadge state={detail.state} />
        <span className={`save save-${save}`}>
          {save === "saving" ? "saving draft…" : save === "saved" ? "draft saved" : save === "error" ? "draft not saved" : ""}
        </span>
      </div>
      {detail.summary && <p className="summary">{detail.summary}</p>}
      {detail.stale && (
        <p className="panel panel-warn">
          The last ACCEPT is stale: covered files or the item set changed since it was submitted.
        </p>
      )}
      {detail.task_problems.length > 0 && (
        <section className="panel panel-error">
          <h2>Validation errors (submit disabled)</h2>
          <ProblemList problems={detail.task_problems} />
        </section>
      )}
      {carriedCount > 0 && (
        <p className="panel">
          {carriedCount} of {items.length} items were OK in the previous round and their files are unchanged; they are
          pre-marked OK and collapsed.
        </p>
      )}
      {err && <p className="panel panel-error">{err}</p>}
      {flash && <p className="panel panel-ok">{flash}</p>}

      {items.map((it) => {
        const k = keyOf(it.resource, it.id);
        return (
          <ItemCard key={`${k}:${it.carried_ok}`} item={it} mark={marks[k] ?? EMPTY} onChange={(m) => setMark(k, m)} />
        );
      })}

      <VerdictBar
        notes={notes}
        onNotes={setTaskNotes}
        problemCount={problemCount}
        unmarkedCount={unmarkedCount}
        invalid={detail.task_problems.length > 0}
        busy={busy}
        onSubmit={submit}
      />
      <HistoryPanel task={task} refresh={historyTick} />
    </>
  );
}
