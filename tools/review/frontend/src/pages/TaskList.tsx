import { useEffect, useState } from "react";
import { api, errText } from "../api";
import ProblemList from "../components/ProblemList";
import StateBadge from "../components/StateBadge";
import type { Problem, TaskSummary } from "../types";

export default function TaskList() {
  const [tasks, setTasks] = useState<TaskSummary[] | null>(null);
  const [problems, setProblems] = useState<Problem[]>([]);
  const [err, setErr] = useState<string | null>(null);

  useEffect(() => {
    Promise.all([api.tasks(), api.validation()])
      .then(([t, p]) => {
        setTasks(t);
        setProblems(p);
      })
      .catch((e) => setErr(errText(e)));
  }, []);

  if (err) return <p className="error">{err}</p>;
  if (!tasks) return <p className="muted">Loading…</p>;

  const unattached = problems.filter((p) => !p.task);
  return (
    <>
      <h1>Tasks</h1>
      {unattached.length > 0 && (
        <section className="panel panel-error">
          <h2>verify.json problems not tied to a task</h2>
          <ProblemList problems={unattached} />
        </section>
      )}
      {tasks.length === 0 ? (
        <p className="muted">No <code>verify.json</code> files found under <code>assets/</code>.</p>
      ) : (
        <table className="tasks">
          <thead>
            <tr>
              <th>Task</th>
              <th>Title</th>
              <th>State</th>
              <th>Items</th>
              <th>Problems</th>
              <th>Last round</th>
            </tr>
          </thead>
          <tbody>
            {tasks.map((t) => (
              <tr key={t.task}>
                <td>
                  <a href={`#/task/${encodeURIComponent(t.task)}`}>{t.task}</a>
                </td>
                <td>{t.title}</td>
                <td>
                  <StateBadge state={t.state} />
                  {t.state === "draft" && t.latest_verdict && (
                    <span className="muted small"> (last: {t.latest_verdict}{t.stale ? ", stale" : ""})</span>
                  )}
                </td>
                <td>{t.items}</td>
                <td className={t.problems ? "error" : "muted"}>{t.problems}</td>
                <td className="muted small">{t.latest_at ?? "—"}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </>
  );
}
