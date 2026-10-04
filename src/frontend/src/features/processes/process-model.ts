import type {
  ProcessSnapshotDto,
  ReadingStatus,
} from "../../shared/contracts/monitor";

export type ProcessMode = "applications" | "processes";
export type ProcessSortKey = "name" | "pid" | "cpu" | "memory";
export type ProcessSortDirection = "asc" | "desc";
export interface ProcessSort {
  key: ProcessSortKey;
  direction: ProcessSortDirection;
}

export function defaultProcessSort(key: string): ProcessSort {
  const normalized =
    key === "name" || key === "pid" || key === "memory" ? key : "cpu";
  return {
    key: normalized,
    direction: normalized === "name" || normalized === "pid" ? "asc" : "desc",
  };
}

export function toggleProcessSort(
  current: ProcessSort,
  key: ProcessSortKey,
): ProcessSort {
  return current.key === key
    ? { key, direction: current.direction === "asc" ? "desc" : "asc" }
    : defaultProcessSort(key);
}

export function processSnapshotSort(key: ProcessSortKey): "cpu" | "memory" {
  return key === "memory" ? "memory" : "cpu";
}

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

const processNameOrder = new Intl.Collator(undefined, {
  numeric: true,
  sensitivity: "base",
});

function sortValue(row: ProcessDisplayRow, key: ProcessSortKey) {
  if (key === "name") return row.name;
  if (key === "pid") return row.pid;
  const value = key === "memory" ? row.working_set : row.cpu;
  const status = key === "memory" ? row.memory_status : row.cpu_status;
  return status === "normal" && value !== null && Number.isFinite(value)
    ? value
    : null;
}

function compareRows(
  left: ProcessDisplayRow,
  right: ProcessDisplayRow,
  key: ProcessSortKey,
  direction: ProcessSortDirection,
) {
  const a = sortValue(left, key);
  const b = sortValue(right, key);
  // Validity is independent of direction. Returning zero preserves equal rows,
  // including application groups, whose PID is deliberately unknown.
  if (a === null || b === null) return Number(a === null) - Number(b === null);
  const comparison =
    typeof a === "string" && typeof b === "string"
      ? processNameOrder.compare(a, b)
      : (a as number) - (b as number);
  return direction === "asc" ? comparison : -comparison;
}

export function processRows(
  data: ProcessSnapshotDto | null,
  mode: ProcessMode,
  sort: ProcessSortKey,
  search: string,
  pinnedId: string | null,
  expanded: Set<string>,
  limit: number,
  direction: ProcessSortDirection = "desc",
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
  const compare = (a: ProcessDisplayRow, b: ProcessDisplayRow) =>
    compareRows(a, b, sort, direction);
  roots.sort(compare);
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
          .sort(compare)
          .map((p) => ({ ...p, child: true })),
      );
  }
  return {
    rows,
    total: roots.length,
    pinnedMissing: !!pinnedId && !roots.some((r) => r.id === pinnedId),
  };
}
