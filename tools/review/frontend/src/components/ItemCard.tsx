import { useState } from "react";
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

export const itemDomId = (item: { resource: string; id: string }) => `item-${item.resource}/${item.id}`;

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
  const [all, setAll] = useState(false);
  const [collapsed, setCollapsed] = useState(item.carried_ok);
  const shown = all ? item.resolved_files : item.resolved_files.slice(0, MAX_FILES);
  return (
    <article id={itemDomId(item)} className={`card item mark-${mark.status ?? "none"}`}>
      <header className="item-head">
        <div>
          <h3>{item.title}</h3>
          <div className="muted small">
            <code>{item.resource || "."}/{item.id}</code> · view <code>{item.view}</code>
            {item.carried_ok && <> · OK in previous round, files unchanged</>}
          </div>
        </div>
        {item.carried_ok && (
          <button onClick={() => setCollapsed(!collapsed)}>{collapsed ? "Show" : "Hide"}</button>
        )}
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
      {!collapsed && (
        <>
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
        {item.resolved_files.length > MAX_FILES && (
          <button onClick={() => setAll(!all)}>
            {all ? `Show first ${MAX_FILES} only` : `Show ${item.resolved_files.length - MAX_FILES} more files`}
          </button>
        )}
      </div>
      <textarea
        className="note"
        placeholder="Note (optional)"
        value={mark.note}
        onChange={(e) => onChange({ ...mark, note: e.target.value })}
      />
        </>
      )}
    </article>
  );
}
