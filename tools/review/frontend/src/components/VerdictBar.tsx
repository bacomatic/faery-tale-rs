import type { Verdict } from "../types";

// Mirrors the server's submit rules (store.Store.submit); the server still enforces them.
export default function VerdictBar({
  notes,
  onNotes,
  problemCount,
  unmarkedCount,
  invalid,
  busy,
  onSubmit,
}: {
  notes: string;
  onNotes: (s: string) => void;
  problemCount: number;
  unmarkedCount: number;
  invalid: boolean;
  busy: boolean;
  onSubmit: (v: Verdict) => void;
}) {
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
  return (
    <section className="card verdict">
      <h2>Verdict</h2>
      <textarea
        className="note"
        placeholder="Task-level notes"
        value={notes}
        onChange={(e) => onNotes(e.target.value)}
      />
      <div className="verdict-row">
        <button className="accept" disabled={busy || !!acceptWhy} title={acceptWhy ?? ""} onClick={() => onSubmit("ACCEPT")}>
          ACCEPT
        </button>
        <button className="reject" disabled={busy || !!rejectWhy} title={rejectWhy ?? ""} onClick={() => onSubmit("REJECT")}>
          REJECT
        </button>
        <span className="muted small">
          {unmarkedCount > 0 && `${unmarkedCount} item(s) unmarked. `}
          {acceptWhy && `ACCEPT blocked: ${acceptWhy}. `}
          {rejectWhy && `REJECT blocked: ${rejectWhy}.`}
        </span>
      </div>
    </section>
  );
}
