import { JsonTree, isPrimitive, type Json } from "./JsonViewer";

interface Entry {
  index: number;
  rgb4?: string;
  rgba8: [number, number, number, number];
}

const isEntry = (v: Json): v is Json & Entry =>
  !!v && typeof v === "object" && !Array.isArray(v) && Array.isArray(v.rgba8) && v.rgba8.length === 4;

// Collect swatch groups: an entry list at the root, or entries/entry lists nested in objects
// (e.g. region_overrides.json → "default", "regions.4", "regions.9").
function groups(v: Json, path: string, out: [string, Entry[]][]) {
  if (Array.isArray(v) && v.length && v.every(isEntry)) out.push([path, v as unknown as Entry[]]);
  else if (isEntry(v)) out.push([path, [v]]);
  else if (v && typeof v === "object" && !Array.isArray(v))
    for (const [k, x] of Object.entries(v)) groups(x, path ? `${path}.${k}` : k, out);
}

function Swatch({ e }: { e: Entry }) {
  const [r, g, b, a] = e.rgba8;
  return (
    <div className="swatch" title={`index ${e.index} · ${e.rgb4 ?? ""} · rgba(${r}, ${g}, ${b}, ${a})`}>
      <div className="chip" style={{ background: `rgba(${r}, ${g}, ${b}, ${a / 255})` }} />
      <div className="swatch-label">
        <span>{e.index}</span>
        {e.rgb4 && <code>{e.rgb4}</code>}
      </div>
    </div>
  );
}

export default function PaletteViewer({ data }: { data: Json }) {
  const found: [string, Entry[]][] = [];
  groups(data, "", found);
  if (!found.length) return <JsonTree data={data} />;
  const meta =
    data && typeof data === "object" && !Array.isArray(data)
      ? Object.entries(data).filter(([, v]) => isPrimitive(v))
      : [];
  return (
    <div className="palette">
      {meta.length > 0 && (
        <div className="muted small">
          {meta.map(([k, v]) => (
            <span key={k} className="meta">
              {k}: <code>{JSON.stringify(v)}</code>
            </span>
          ))}
        </div>
      )}
      {found.map(([label, entries]) => (
        <div key={label} className="swatch-group">
          {label && <div className="small json-key">{label}</div>}
          <div className="swatches">
            {entries.map((e, i) => (
              <Swatch key={i} e={e} />
            ))}
          </div>
          {!label && <div className="muted small">{entries.length} entries</div>}
        </div>
      ))}
    </div>
  );
}
