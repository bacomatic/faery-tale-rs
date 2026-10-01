import { useState } from "react";
import { api, errText } from "../api";
import type { CompareResult, StaleChanges } from "../types";

// Why the last ACCEPT is stale: the changed files (by hash) and, on request, whether their
// content really changed (pixels for PNGs, structure for JSON) against the accepted git version.
// Items whose changed files are all *trivial* (identical content or <= trivial_max_changes JSON
// paths) and that were OK last round can be re-marked OK right here; the rest need a normal look.
export default function StalePanel({
  task,
  changes,
  okThen,
  onOkItems,
}: {
  task: string;
  changes: StaleChanges;
  okThen: Set<string>;
  onOkItems: (items: { key: string; note: string }[]) => void;
}) {
  const [cmp, setCmp] = useState<CompareResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [open, setOpen] = useState<Record<string, boolean>>({});
  const [done, setDone] = useState(false);

  const run = async () => {
    setBusy(true);
    setErr(null);
    setDone(false);
    try {
      setCmp(await api.compare(task));
    } catch (e) {
      setErr(errText(e));
    } finally {
      setBusy(false);
    }
  };

  // item key -> changed files touching it, and whether every one of them is trivial
  const perItem = new Map<string, { files: string[]; trivial: boolean }>();
  for (const f of cmp?.files ?? []) {
    const trivial = f.status === "changed" && !!f.result?.trivial;
    for (const k of f.items) {
      const e = perItem.get(k) ?? { files: [], trivial: true };
      e.files.push(f.file);
      e.trivial &&= trivial;
      perItem.set(k, e);
    }
  }
  const trivialItems = [...perItem].filter(([k, e]) => e.trivial && okThen.has(k));
  const reviewItems = [...perItem].filter(([k, e]) => !(e.trivial && okThen.has(k)));

  const okTrivial = () => {
    onOkItems(
      trivialItems.map(([key, e]) => ({
        key,
        note: `OK'd from the diff view: ${e.files.map((f) => f.replace(/^assets\//, "")).join(", ")} — trivial change vs accepted ${cmp?.accepted_at.slice(0, 10)}`,
      })),
    );
    setDone(true);
  };

  const byFile = new Map(cmp?.files.map((f) => [f.file, f]) ?? []);
  return (
    <section className="panel panel-warn stale">
      <p>
        The last ACCEPT is stale: {changes.files.length} covered file{changes.files.length === 1 ? "" : "s"} changed
        {changes.items_added.length + changes.items_removed.length > 0 && " and the item set changed"} since it was
        submitted.{" "}
        <button disabled={busy || !changes.files.length} onClick={run} title="Fetch the accepted version from git history and compare content">
          {busy ? "comparing…" : cmp ? "Compare again" : "Compare with accepted version"}
        </button>
      </p>
      {err && <p className="error">{err}</p>}
      {cmp && (
        <p className={cmp.effectively_unchanged ? "ok" : ""}>
          {cmp.effectively_unchanged
            ? "Nothing changed in content — every file is identical once decoded (re-encoded or reformatted only)."
            : `${cmp.content_changed} of ${cmp.files.length} file${cmp.files.length === 1 ? "" : "s"} really changed.`}
        </p>
      )}
      {cmp && trivialItems.length > 0 && (
        <p>
          <button className="ok" disabled={done} onClick={okTrivial}>
            {done ? "marked OK" : `OK ${trivialItems.length} item${trivialItems.length === 1 ? "" : "s"} with trivial changes`}
          </button>{" "}
          <span className="small muted">
            (identical content or ≤ {cmp.trivial_max_changes} JSON changes, and OK last round):{" "}
            {trivialItems.map(([k]) => k).join(", ")}
          </span>
        </p>
      )}
      {cmp && reviewItems.length > 0 && (
        <p className="small">
          <span className="warn">Needs normal review:</span> {reviewItems.map(([k]) => k).join(", ")}
        </p>
      )}
      {(changes.items_added.length > 0 || changes.items_removed.length > 0) && (
        <p className="small">
          {changes.items_added.length > 0 && <>items added: <code>{changes.items_added.join(", ")}</code> </>}
          {changes.items_removed.length > 0 && <>items removed: <code>{changes.items_removed.join(", ")}</code></>}
        </p>
      )}
      <ul className="problems">
        {changes.files.map((f) => {
          const r = byFile.get(f.file)?.result;
          const commit = byFile.get(f.file)?.accepted_commit;
          const details = r?.kind === "json" && r.changes.length > 0;
          return (
            <li key={f.file}>
              <span className={`status status-${f.status}`}>{f.status}</span> <code>{f.file}</code>
              {r && (
                <>
                  {" — "}
                  <span className={r.identical ? "ok" : r.trivial ? "" : "warn"}>{r.summary}</span>
                  {r.trivial && !r.identical && <span className="muted small"> (trivial)</span>}
                  {commit && <span className="muted small"> (accepted @ {commit})</span>}
                  {details && (
                    <button className="link" onClick={() => setOpen((o) => ({ ...o, [f.file]: !o[f.file] }))}>
                      {open[f.file] ? "hide" : "show"}
                    </button>
                  )}
                  {details && open[f.file] && (
                    <ul className="json-changes small">
                      {r.changes.map((c, i) => (
                        <li key={i}>
                          <code>{c.path || "(root)"}</code> {c.change}
                          {"then" in c && <>: <code>{JSON.stringify(c.then)}</code></>}
                          {"now" in c && <> → <code>{JSON.stringify(c.now)}</code></>}
                        </li>
                      ))}
                      {r.truncated && <li className="muted">… more</li>}
                    </ul>
                  )}
                </>
              )}
            </li>
          );
        })}
      </ul>
    </section>
  );
}
