import type { ItemDetail, Status } from "../types";
import FileViewer from "../viewers/FileViewer";
import CitationLink from "./CitationLink";
import CountBadge from "./CountBadge";
import ProblemList from "./ProblemList";

export interface Mark {
  status: Status;
  note: string;
}

const MAX_FILES = 24;

export default function ItemCard({
  item,
  mark,
  onChange,
}: {
  item: ItemDetail;
  mark: Mark;
  onChange: (m: Mark) => void;
}) {
  const toggle = (s: Exclude<Status, null>) => onChange({ ...mark, status: mark.status === s ? null : s });
  const shown = item.resolved_files.slice(0, MAX_FILES);
  return (
    <article className={`card item mark-${mark.status ?? "none"}`}>
      <header className="item-head">
        <div>
          <h3>{item.title}</h3>
          <div className="muted small">
            <code>{item.resource || "."}/{item.id}</code> · view <code>{item.view}</code>
          </div>
        </div>
        <div className="mark-buttons">
          <button className={`ok ${mark.status === "ok" ? "active" : ""}`} onClick={() => toggle("ok")}>
            OK
          </button>
          <button
            className={`problem ${mark.status === "problem" ? "active" : ""}`}
            onClick={() => toggle("problem")}
          >
            Problem
          </button>
        </div>
      </header>
      <ProblemList problems={item.problems} />
      <p className="look-for">{item.look_for}</p>
      {item.count_results.length > 0 && (
        <div className="counts">
          {item.count_results.map((r, i) => (
            <CountBadge key={i} r={r} />
          ))}
        </div>
      )}
      {item.citations.length > 0 && (
        <div className="small citations">
          Citations:{" "}
          {item.citations.map((c) => (
            <CitationLink key={c} cite={c} />
          ))}
        </div>
      )}
      <div className="files">
        {shown.map((f) => (
          <FileViewer key={f} path={f} view={item.view} />
        ))}
        {item.resolved_files.length > shown.length && (
          <p className="muted small">+{item.resolved_files.length - shown.length} more files not shown</p>
        )}
      </div>
      <textarea
        className="note"
        placeholder="Note (optional)"
        value={mark.note}
        onChange={(e) => onChange({ ...mark, note: e.target.value })}
      />
    </article>
  );
}
