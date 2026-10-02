import { useEffect, useState } from "react";
import { api, errText } from "../api";
import type { Round } from "../types";

export default function HistoryPanel({ task, refresh }: { task: string; refresh: number }) {
  const [rounds, setRounds] = useState<Round[] | null>(null);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => {
    api.reviews(task).then(setRounds).catch((e) => setErr(errText(e)));
  }, [task, refresh]);

  return (
    <section className="card history">
      <h2>History</h2>
      {err && <p className="error">{err}</p>}
      {rounds && rounds.length === 0 && <p className="muted">No rounds yet.</p>}
      {rounds &&
        [...rounds].reverse().map((r, i) => {
          const flagged = r.items.filter((m) => m.status === "problem" || m.note);
          return (
            <div key={i} className="round">
              <div>
                <span className={`badge state-${r.verdict}`}>{r.verdict}</span>{" "}
                <span className="muted small">
                  {r.submitted_at} · {r.items.filter((m) => m.status === "ok").length} OK ·{" "}
                  {r.items.filter((m) => m.status === "problem").length} Problem ·{" "}
                  {r.items.filter((m) => !m.status).length} unmarked · {Object.keys(r.files).length} files hashed
                </span>
              </div>
              {r.notes && <p className="round-notes">{r.notes}</p>}
              {flagged.length > 0 && (
                <ul className="small">
                  {flagged.map((m) => (
                    <li key={`${m.resource}/${m.id}`}>
                      <code>{m.resource || "."}/{m.id}</code> {m.status ?? "unmarked"}
                      {m.note && (
                        <>
                          : <span className="round-note">{m.note}</span>
                        </>
                      )}
                    </li>
                  ))}
                </ul>
              )}
            </div>
          );
        })}
    </section>
  );
}
