import type { TaskState } from "../types";

const LABELS: Record<TaskState, string> = {
  not_reviewed: "not reviewed",
  draft: "draft",
  ACCEPT: "ACCEPT",
  REJECT: "REJECT",
  stale: "stale ACCEPT",
  invalid: "invalid",
};

export default function StateBadge({ state }: { state: TaskState }) {
  return <span className={`badge state-${state}`}>{LABELS[state]}</span>;
}
