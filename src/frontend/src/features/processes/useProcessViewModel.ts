import { useCallback, useEffect, useMemo, useState } from "react";
import { usePageUiState } from "../../shared/state/page-ui-state";
import type { ProcessSnapshotDto } from "../../shared/contracts/monitor";
import type { MonitorClient } from "../../shared/client/monitor-client";
import { usePageQuery } from "../monitoring/usePageQuery";
import {
  defaultProcessSort,
  processRows,
  processSnapshotSort,
  toggleProcessSort,
  type ProcessMode,
  type ProcessSortKey,
} from "./process-model";
export function useProcessViewModel(
  client: MonitorClient,
  initialSort = "cpu",
) {
  const [sorting, setSorting] = usePageUiState("process.sort", () =>
    defaultProcessSort(initialSort),
  );
  const { key: sort, direction: sortDirection } = sorting;
  const [mode, setMode] = usePageUiState<ProcessMode>(
    "process.mode",
    "applications",
  );
  const [search, setSearch] = usePageUiState("process.search", "");
  const [paused, setPaused] = usePageUiState("process.paused", false);
  const [pinned, setPinned] = usePageUiState<{
    id: string;
    name: string;
  } | null>("process.pinned", null);
  const [expanded, setExpanded] = usePageUiState<Set<string>>(
    "process.expanded",
    () => new Set(),
  );
  const [limit, setLimit] = usePageUiState("process.limit", 10);
  const [pausedData, setPausedData] = usePageUiState<ProcessSnapshotDto | null>(
    "process.snapshot",
    null,
  );
  const [copyMessage, setCopyMessage] = useState("");
  const snapshotSort = processSnapshotSort(sort);
  const load = useCallback(
    () =>
      client.getProcessSnapshot
        ? client.getProcessSnapshot(snapshotSort)
        : Promise.reject(new Error("当前客户端不支持进程排行")),
    [client, snapshotSort],
  );
  const query = usePageQuery(
    client,
    load,
    2000,
    !paused,
    paused ? pausedData : null,
  );
  useEffect(() => {
    setPausedData(paused ? query.data : null);
  }, [paused, query.data, setPausedData]);
  const data = query.error && !paused ? null : query.data;
  const result = useMemo(
    () =>
      processRows(
        data,
        mode,
        sort,
        search,
        pinned?.id ?? null,
        expanded,
        limit,
        sortDirection,
      ),
    [data, mode, sort, search, pinned, expanded, limit, sortDirection],
  );
  return {
    ...query,
    sort,
    sortDirection,
    setSort: (key: string) => setSorting(defaultProcessSort(key)),
    sortBy: (key: ProcessSortKey) =>
      setSorting((current) => toggleProcessSort(current, key)),
    ...result,
    mode,
    setMode: (next: string) => {
      setMode(next as ProcessMode);
      setLimit(10);
      setPinned(null);
    },
    search,
    setSearch: (value: string) => {
      setSearch(value);
      setLimit(10);
    },
    paused,
    setPaused,
    pinned,
    pin: (row: { id: string; name: string }) =>
      setPinned((current) => (current?.id === row.id ? null : row)),
    clearPin: () => setPinned(null),
    expanded,
    toggle: (id: string) =>
      setExpanded((current) => {
        const next = new Set(current);
        if (next.has(id)) next.delete(id);
        else next.add(id);
        return next;
      }),
    showMore: () => setLimit((current) => current + 20),
    hasMore: result.total > limit,
    copyMessage,
    copy: async (value: string) => {
      try {
        await navigator.clipboard.writeText(value);
        setCopyMessage(`已复制 ${value}`);
      } catch {
        setCopyMessage("无法访问剪贴板，请选中名称或 PID 手动复制。");
      }
    },
  };
}
