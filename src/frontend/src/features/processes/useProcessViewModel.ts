import { useCallback, useState } from "react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import { usePageQuery } from "../monitoring/usePageQuery";
export function useProcessViewModel(client: MonitorClient) {
  const [sort, setSort] = useState("cpu");
  const load = useCallback(
    () =>
      client.getProcessSnapshot
        ? client.getProcessSnapshot(sort)
        : Promise.reject(new Error("当前客户端不支持进程排行")),
    [client, sort],
  );
  const query = usePageQuery(load);
  return {
    ...query,
    sort,
    setSort,
    rows: !query.error && query.data?.sort === sort ? query.data.rows : [],
  };
}
