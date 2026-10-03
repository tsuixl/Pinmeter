import type {
  ProcessSnapshotDto,
  ReadingStatus,
} from "../../shared/contracts/monitor";

export type ProcessMode = "applications" | "processes";
export interface ProcessDisplayRow {
  id: string;
  name: string;
  pid: number | null;
  cpu: number | null;
  cpu_status: ReadingStatus;
  working_set: number | null;
  memory_status: ReadingStatus;
  application_id: string;
  process_count: number;
  child: boolean;
}
export function processRows(
  data: ProcessSnapshotDto | null,
  mode: ProcessMode,
  sort: string,
  search: string,
  pinnedId: string | null,
  expanded: Set<string>,
  limit: number,
) {
  if (!data) return { rows: [], total: 0, pinnedMissing: !!pinnedId };
  const needle = search.trim().toLocaleLowerCase();
  const matches = (row: { name: string; pid?: number | null }) =>
    row.name.toLocaleLowerCase().includes(needle) ||
    (row.pid != null && String(row.pid).includes(needle));
  const processes: ProcessDisplayRow[] = data.rows.map((row) => ({
    ...row,
    process_count: 1,
    child: false,
  }));
  const members = new Map<string, ProcessDisplayRow[]>();
  for (const row of processes) {
    const group = members.get(row.application_id) ?? [];
    group.push(row);
    members.set(row.application_id, group);
  }
  const roots: ProcessDisplayRow[] =
    mode === "processes"
      ? processes.filter(matches)
      : data.applications
          .filter((app) => matches(app) || members.get(app.id)?.some(matches))
          .map((app) => ({
            ...app,
            pid: null,
            application_id: app.id,
            child: false,
          }));
  roots.sort((a, b) => {
    const left = sort === "memory" ? a.working_set : a.cpu;
    const right = sort === "memory" ? b.working_set : b.cpu;
    return (
      (left === null ? 1 : 0) - (right === null ? 1 : 0) ||
      (right ?? 0) - (left ?? 0) ||
      a.id.localeCompare(b.id)
    );
  });
  const pin = roots.find((r) => r.id === pinnedId);
  const selected = pin
    ? [pin, ...roots.filter((r) => r.id !== pinnedId)].slice(0, limit)
    : roots.slice(0, limit);
  const rows: ProcessDisplayRow[] = [];
  for (const root of selected) {
    rows.push(root);
    if (mode === "applications" && (expanded.has(root.id) || !!needle))
      rows.push(
        ...(members.get(root.id) ?? [])
          .filter((p) => !needle || matches(p))
          .map((p) => ({ ...p, child: true })),
      );
  }
  return {
    rows,
    total: roots.length,
    pinnedMissing: !!pinnedId && !roots.some((r) => r.id === pinnedId),
  };
}
