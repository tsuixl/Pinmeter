import { useCallback, useMemo, useState } from "react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import { usePageQuery } from "../monitoring/usePageQuery";
import { archiveFrames } from "./history-model";
import { usePageUiState } from "../../shared/state/page-ui-state";
export function useHistoryViewModel(client: MonitorClient) {
  const [range, setRange] = usePageUiState("history.range", "86400000");
  const [revision, refresh] = useState(0);
  const load = useCallback(() => {
    const start = new Date();
    start.setHours(0, 0, 0, 0);
    return client.getArchiveSnapshot
      ? client.getArchiveSnapshot(start.getTime(), Number(range))
      : Promise.reject(new Error("当前客户端不支持本地历史"));
  }, [client, range, revision]);
  const query = usePageQuery(client, load, 10_000);
  const data =
    query.data &&
    query.data.now_ms - query.data.from_ms >= Number(range) &&
    query.data.now_ms - query.data.from_ms <
      Number(range) + query.data.resolution_ms
      ? query.data
      : null;
  const [anchor, setAnchor] = usePageUiState<number | null>(
    "history.anchor",
    null,
  );
  const frames = useMemo(() => (data ? archiveFrames(data) : []), [data]);
  return {
    ...query,
    data,
    frames,
    range,
    setRange,
    anchor,
    setAnchor,
    revision,
    refresh: () => refresh((value) => value + 1),
  };
}
