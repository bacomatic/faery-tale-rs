import { useState } from "react";
import type { ItemDetail } from "../types";
import type { Mark } from "./ItemCard";

// Second row of the sticky header: the item currently scrolled into view, with its OK/Problem
// buttons and note, plus prev/next jumps — so marking never needs a scroll back up to the card.
export default function CurrentItemBar({
  item,
  index,
  total,
  mark,
  onChange,
  onJump,
}: {
  item: ItemDetail | null;
  index: number;
  total: number;
  mark: Mark;
  onChange: (m: Mark) => void;
  onJump: (delta: number) => void;
}) {
  const [showNote, setShowNote] = useState(false);
  if (!item) return null;
  const toggle = (s: "ok" | "problem") => onChange({ ...mark, status: mark.status === s ? null : s });
  const open = showNote || mark.note.trim().length > 0;
  return (
    <div className={`current-item mark-${mark.status ?? "none"}`}>
      <div className="current-row">
      <button className="link" disabled={index <= 0} onClick={() => onJump(-1)} title="previous item">
        ↑
      </button>
      <button className="link" disabled={index >= total - 1} onClick={() => onJump(1)} title="next item">
        ↓
      </button>
      <span className="muted small">
        {index + 1}/{total}
      </span>
      <span className="current-title" title={item.title}>
        {item.title}
      </span>
      <div className="mark-buttons">
        <button className={`ok ${mark.status === "ok" ? "active" : ""}`} onClick={() => toggle("ok")}>
          OK
        </button>
        <button className={`problem ${mark.status === "problem" ? "active" : ""}`} onClick={() => toggle("problem")}>
          Problem
        </button>
      </div>
      <button className="link note-toggle" onClick={() => setShowNote(!open)} disabled={mark.note.trim().length > 0}>
        {open ? "note" : "+ note"}
      </button>
      </div>
      {open && (
        <textarea
          className="note"
          placeholder="Note (optional)"
          value={mark.note}
          onChange={(e) => onChange({ ...mark, note: e.target.value })}
        />
      )}
    </div>
  );
}
