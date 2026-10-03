import { useCallback, useMemo, useState } from "react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import { usePageQuery } from "../monitoring/usePageQuery";
import { processRows, type ProcessMode } from "./process-model";
export function useProcessViewModel(
  client: MonitorClient,
  initialSort = "cpu",
) {
  const [sort, setSort] = useState(initialSort);
  const [mode, setMode] = useState<ProcessMode>("applications");
  const [search, setSearch] = useState("");
  const [paused, setPaused] = useState(false);
  const [pinned, setPinned] = useState<{ id: string; name: string } | null>(
    null,
  );
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [limit, setLimit] = useState(10);
  const [copyMessage, setCopyMessage] = useState("");
  const load = useCallback(
    () =>
      client.getProcessSnapshot
        ? client.getProcessSnapshot(sort)
        : Promise.reject(new Error("当前客户端不支持进程排行")),
    [client, sort],
  );
  const query = usePageQuery(client, load, 2000, !paused);
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
      ),
    [data, mode, sort, search, pinned, expanded, limit],
  );
  return {
    ...query,
    sort,
    setSort,
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
