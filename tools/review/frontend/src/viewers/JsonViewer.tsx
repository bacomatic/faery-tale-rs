import { useFetched } from "./useFetched";

export type Json = null | boolean | number | string | Json[] | { [k: string]: Json };

export const isPrimitive = (v: Json) => v === null || typeof v !== "object";

function Node({ name, value, depth }: { name?: string; value: Json; depth: number }) {
  const label = name !== undefined && <span className="json-key">{name}: </span>;
  if (isPrimitive(value)) {
    return (
      <div className="json-line">
        {label}
        <span className={`json-${value === null ? "null" : typeof value}`}>{JSON.stringify(value)}</span>
      </div>
    );
  }
  const entries: [string, Json][] = Array.isArray(value)
    ? value.map((v, i) => [String(i), v])
    : Object.entries(value);
  const [open, close] = Array.isArray(value) ? ["[", "]"] : ["{", "}"];
  const inline = JSON.stringify(value);
  if (entries.every(([, v]) => isPrimitive(v)) && inline.length <= 80) {
    return (
      <div className="json-line">
        {label}
        <span className="json-inline">{inline}</span>
      </div>
    );
  }
  return (
    <details className="json-node" open={depth < 2}>
      <summary>
        {label}
        {open}
        <span className="muted small"> {entries.length} {Array.isArray(value) ? "items" : "keys"} </span>
        {close}
      </summary>
      <div className="json-children">
        {entries.map(([k, v]) => (
          <Node key={k} name={k} value={v} depth={depth + 1} />
        ))}
      </div>
    </details>
  );
}

export function JsonTree({ data }: { data: Json }) {
  return (
    <div className="json-viewer">
      <Node value={data} depth={0} />
    </div>
  );
}

export default function JsonViewer({ path }: { path: string }) {
  const { data, err } = useFetched<Json>(path, (r) => r.json());
  if (err) return <p className="error">{err}</p>;
  if (data === undefined) return <p className="muted">Loading…</p>;
  return <JsonTree data={data} />;
}
