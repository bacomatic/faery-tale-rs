import { useState } from "react";
import CitationLink from "../components/CitationLink";
import { JsonTree, isPrimitive, type Json } from "./JsonViewer";

type Obj = { [k: string]: Json };
const isObj = (v: Json): v is Obj => !!v && typeof v === "object" && !Array.isArray(v);
const cell = (v: Json) => (isPrimitive(v) ? String(v) : JSON.stringify(v));

function Meta({ d }: { d: Obj }) {
  const src = typeof d.source === "string" ? d.source : null;
  const line = typeof d.line === "number" ? d.line : null;
  const cite = src && line && /^(src|reference)\//.test(src) ? `${src}:${line}` : null;
  if (!d.name && !cite && !d.semantics) return null;
  return (
    <div className="table-meta small">
      {typeof d.name === "string" && <code>{d.name}</code>}
      {cite && <CitationLink cite={cite} block />}
      {Array.isArray(d.shape) && <span className="muted">shape {JSON.stringify(d.shape)}</span>}
      {typeof d.semantics === "string" && <div className="muted">{d.semantics}</div>}
    </div>
  );
}

function RowsTable({ fields, rows }: { fields: string[]; rows: Obj[] }) {
  return (
    <div className="table-scroll">
      <table className="data">
        <thead>
          <tr>
            <th className="idx">#</th>
            {fields.map((f) => (
              <th key={f}>{f}</th>
            ))}
          </tr>
        </thead>
        <tbody>
          {rows.map((r, i) => (
            <tr key={i}>
              <td className="idx">{i}</td>
              {fields.map((f) => (
                <td key={f}>{f in r ? cell(r[f]) : ""}</td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function ValuesGrid({ values }: { values: Json[] }) {
  const flat = values.every(isPrimitive);
  const [cols, setCols] = useState(values.length % 16 && values.length % 8 === 0 ? 8 : 16);
  const rows: Json[][] = flat
    ? Array.from({ length: Math.ceil(values.length / cols) }, (_, i) => values.slice(i * cols, (i + 1) * cols))
    : values.map((v) => (Array.isArray(v) ? v : [v]));
  const width = Math.max(...rows.map((r) => r.length));
  return (
    <>
      {flat && (
        <div className="zoom small">
          <span className="muted">{values.length} values · per row:</span>
          {[4, 8, 16, 32].map((n) => (
            <button key={n} className={n === cols ? "active" : ""} onClick={() => setCols(n)}>
              {n}
            </button>
          ))}
        </div>
      )}
      <div className="table-scroll">
        <table className="data">
          <thead>
            <tr>
              <th className="idx">{flat ? "start" : "#"}</th>
              {Array.from({ length: width }, (_, i) => (
                <th key={i}>{i}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {rows.map((r, i) => (
              <tr key={i}>
                <td className="idx">{flat ? i * cols : i}</td>
                {r.map((v, j) => (
                  <td key={j}>{cell(v)}</td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </>
  );
}

// Renders {fields, rows}, {values}, arrays of objects, or arrays of primitives; else a JSON tree.
export default function TableViewer({ data }: { data: Json }) {
  if (isObj(data) && Array.isArray(data.rows) && data.rows.every(isObj)) {
    const fields = Array.isArray(data.fields)
      ? data.fields.map(String)
      : [...new Set(data.rows.flatMap((r) => Object.keys(r as Obj)))];
    return (
      <>
        <Meta d={data} />
        <RowsTable fields={fields} rows={data.rows as Obj[]} />
      </>
    );
  }
  if (isObj(data) && Array.isArray(data.values)) {
    return (
      <>
        <Meta d={data} />
        <ValuesGrid values={data.values} />
      </>
    );
  }
  if (Array.isArray(data) && data.length && data.every(isObj)) {
    const fields = [...new Set(data.flatMap((r) => Object.keys(r as Obj)))];
    return <RowsTable fields={fields} rows={data as Obj[]} />;
  }
  if (Array.isArray(data) && data.length) return <ValuesGrid values={data} />;
  return <JsonTree data={data} />;
}
