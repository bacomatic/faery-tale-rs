import { useState } from "react";
import { JsonTree, type Json } from "./JsonViewer";

// extract() replaces each '%' with datanames[brother-1] (src/fmain2.c:527-530);
// datanames = { "Julian","Phillip","Kevin" } (src/fmain.c:604).
const BROTHERS = ["Julian", "Phillip", "Kevin"];

type Obj = { [k: string]: Json };
const isObj = (v: Json): v is Obj => !!v && typeof v === "object" && !Array.isArray(v);
const allStrings = (vs: Json[]) => vs.length > 0 && vs.every((v) => typeof v === "string");

// Indexed string list: an array of strings, an object of strings, or an object holding exactly
// one array of strings (other primitive keys are shown as metadata).
function extract(data: Json): { entries: [string, string][]; meta: [string, Json][] } | null {
  if (Array.isArray(data) && allStrings(data))
    return { entries: data.map((s, i) => [String(i), s as string]), meta: [] };
  if (!isObj(data)) return null;
  const vals = Object.values(data);
  if (allStrings(vals)) return { entries: Object.entries(data) as [string, string][], meta: [] };
  const lists = Object.entries(data).filter(([, v]) => Array.isArray(v) && allStrings(v));
  if (lists.length !== 1) return null;
  const [key, list] = lists[0] as [string, string[]];
  return {
    entries: list.map((s, i) => [String(i), s]),
    meta: Object.entries(data).filter(([k, v]) => k !== key && (v === null || typeof v !== "object")),
  };
}

function Highlighted({ s }: { s: string }) {
  return (
    <>
      {s.split(/(%)/).map((part, i) =>
        part === "%" ? (
          <mark key={i} className="placeholder">
            {part}
          </mark>
        ) : (
          part
        ),
      )}
    </>
  );
}

function Sample({ s, name }: { s: string; name: string }) {
  return (
    <>
      {s.split(/(%)/).map((part, i) =>
        part === "%" ? (
          <mark key={i} className="substituted">
            {name}
          </mark>
        ) : (
          part
        ),
      )}
    </>
  );
}

export default function StringsViewer({ data }: { data: Json }) {
  const [brother, setBrother] = useState(0);
  const found = extract(data);
  if (!found) return <JsonTree data={data} />;
  const hasName = found.entries.some(([, s]) => s.includes("%"));
  return (
    <div className="strings">
      {found.meta.length > 0 && (
        <div className="muted small">
          {found.meta.map(([k, v]) => (
            <span key={k} className="meta">
              {k}: <code>{JSON.stringify(v)}</code>
            </span>
          ))}
        </div>
      )}
      <div className="zoom small">
        <span className="muted">{found.entries.length} strings</span>
        {hasName && (
          <>
            <span className="muted">· sample name for %:</span>
            {BROTHERS.map((n, i) => (
              <button key={n} className={i === brother ? "active" : ""} onClick={() => setBrother(i)}>
                {n}
              </button>
            ))}
          </>
        )}
      </div>
      <ol className="string-list">
        {found.entries.map(([k, s]) => (
          <li key={k}>
            <span className="idx">{k}</span>
            <span className="str">
              <Highlighted s={s} />
              {s.includes("%") && (
                <>
                  <span className="arrow"> ---&gt; </span>
                  <span className="sample">
                    <Sample s={s} name={BROTHERS[brother]} />
                  </span>
                </>
              )}
            </span>
          </li>
        ))}
      </ol>
    </div>
  );
}
