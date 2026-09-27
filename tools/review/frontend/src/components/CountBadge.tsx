import type { CountResult } from "../types";

export default function CountBadge({ r }: { r: CountResult }) {
  const cls = r.error ? "count count-error" : r.ok ? "count count-ok" : "count count-bad";
  const text = r.error ? `${r.label}: ? / ${r.expect}` : `${r.label}: ${r.actual} / ${r.expect}`;
  const title = r.error ?? (r.ok ? "actual matches expected" : `actual ${r.actual} ≠ expected ${r.expect}`);
  return (
    <span className={cls} title={title}>
      {text}
    </span>
  );
}
