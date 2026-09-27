import type { Problem } from "../types";

export default function ProblemList({ problems }: { problems: Problem[] }) {
  if (!problems.length) return null;
  return (
    <ul className="problems">
      {problems.map((p, i) => (
        <li key={i}>
          <code>{p.file}</code>
          {p.item && <> · item <code>{p.item}</code></>}: {p.message}
        </li>
      ))}
    </ul>
  );
}
