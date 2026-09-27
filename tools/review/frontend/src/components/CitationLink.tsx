import { useState } from "react";
import { api, errText } from "../api";
import type { SourceLines } from "../types";

const CITE = /^((?:src|reference)\/[^:]+):(\d+)(?:-(\d+))?$/;

// `block`: for a single-line cite, show through the end of the `{ ... }` block starting there
// (used for table metadata, which only records the declaration line).
export default function CitationLink({ cite, block = false }: { cite: string; block?: boolean }) {
  const [open, setOpen] = useState(false);
  const [src, setSrc] = useState<SourceLines | null>(null);
  const [err, setErr] = useState<string | null>(null);
  const m = cite.match(CITE);
  if (!m) return <code className="cite">{cite}</code>;
  const [, path, s, e] = m;

  const toggle = () => {
    setOpen(!open);
    if (!src && !err)
      api
        .source(path, Number(s), e === undefined ? null : Number(e), block && e === undefined)
        .then(setSrc)
        .catch((x) => setErr(errText(x)));
  };

  return (
    <span className="cite-wrap">
      <button className={`cite-link ${open ? "active" : ""}`} onClick={toggle}>
        {cite}
      </button>
      {open && (
        <div className="cite-pop">
          {err && <p className="error">{err}</p>}
          {!src && !err && <p className="muted">Loading…</p>}
          {src && (
            <pre>
              <div className="muted">
                {src.path}:{src.start}
                {src.end !== src.start && `-${src.end}`}
              </div>
              {src.lines.map((line, i) => (
                <div key={i}>
                  <span className="lineno">{src.start + i}</span>
                  {line}
                </div>
              ))}
            </pre>
          )}
        </div>
      )}
    </span>
  );
}
