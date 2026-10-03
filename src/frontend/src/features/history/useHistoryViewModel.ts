import { useCallback, useMemo, useState } from "react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import { usePageQuery } from "../monitoring/usePageQuery";
import { archiveFrames } from "./history-model";
export function useHistoryViewModel(client: MonitorClient) {
  const load = useCallback(() => {
    const start = new Date();
    start.setHours(0, 0, 0, 0);
    return client.getArchiveSnapshot
      ? client.getArchiveSnapshot(start.getTime())
      : Promise.reject(new Error("当前客户端不支持本地历史"));
  }, [client]);
  const query = usePageQuery(client, load, 10_000);
  const [range, setRange] = useState("86400000");
  const [anchor, setAnchor] = useState<number | null>(null);
  const frames = useMemo(
    () => (query.data ? archiveFrames(query.data) : []),
    [query.data],
  );
  return { ...query, frames, range, setRange, anchor, setAnchor };
}
