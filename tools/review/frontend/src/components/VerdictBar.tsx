import { useState } from "react";
import type { Verdict } from "../types";

// Sticky sub-header under the top bar: verdict buttons always in reach, task notes fold out.
// Mirrors the server's submit rules (store.Store.submit); the server still enforces them.
export default function VerdictBar({
  notes,
  onNotes,
  problemCount,
  unmarkedCount,
  okCount,
  total,
  invalid,
  busy,
  onSubmit,
}: {
  notes: string;
  onNotes: (s: string) => void;
  problemCount: number;
  unmarkedCount: number;
  okCount: number;
  total: number;
  invalid: boolean;
  busy: boolean;
  onSubmit: (v: Verdict) => void;
}) {
  const [showNotes, setShowNotes] = useState(false);
  const acceptWhy = invalid
    ? "fix validation errors first"
    : problemCount
      ? `${problemCount} item(s) marked Problem`
      : null;
  const rejectWhy = invalid
    ? "fix validation errors first"
    : !problemCount && !notes.trim()
      ? "mark a Problem item or write a task note"
      : null;
  const open = showNotes || notes.trim().length > 0;
  return (
    <section className="verdict">
      <div className="verdict-row">
        <button className="accept" disabled={busy || !!acceptWhy} title={acceptWhy ?? ""} onClick={() => onSubmit("ACCEPT")}>
          ACCEPT
        </button>
        <button className="reject" disabled={busy || !!rejectWhy} title={rejectWhy ?? ""} onClick={() => onSubmit("REJECT")}>
          REJECT
        </button>
        <span className="small tally">
          <span className="ok">{okCount} OK</span> · <span className={problemCount ? "warn" : ""}>{problemCount} Problem</span> ·{" "}
          <span className="muted">{unmarkedCount} unmarked</span> <span className="muted">/ {total}</span>
        </span>
        <span className="muted small">
          {acceptWhy && `ACCEPT blocked: ${acceptWhy}. `}
          {rejectWhy && `REJECT blocked: ${rejectWhy}.`}
        </span>
        <button className="link" onClick={() => setShowNotes(!open)} disabled={notes.trim().length > 0}>
          {open ? "notes" : "+ notes"}
        </button>
      </div>
      {open && (
        <textarea
          className="note"
          placeholder="Task-level notes"
          value={notes}
          onChange={(e) => onNotes(e.target.value)}
        />
      )}
    </section>
  );
}
