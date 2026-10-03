import {
  useCallback,
  useMemo,
  useRef,
  useState,
  useSyncExternalStore,
} from "react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import { usePageQuery } from "../monitoring/usePageQuery";
import {
  appHistoryFrames,
  historyRows,
  type HistorySort,
} from "./app-history-model";
export function useAppHistoryViewModel(
  client: MonitorClient,
  initialAppId: string | null = null,
) {
  const snapshot = useSyncExternalStore(client.subscribe, client.getSnapshot);
  const [range, setRange] = useState(initialAppId ? "24h" : "today");
  const [selectedId, setSelectedId] = useState<string | null>(initialAppId);
  const [search, setSearch] = useState("");
  const [sort, setSort] = useState<HistorySort>("total");
  const [limit, setLimit] = useState(10);
  const [anchor, setAnchor] = useState<number | null>(null);
  const [pending, setPending] = useState(false);
  const [actionError, setActionError] = useState("");
  const busy = useRef(false);
  const load = useCallback(() => {
    const start = new Date();
    start.setHours(0, 0, 0, 0);
    return client.getAppHistory
      ? client.getAppHistory(range, selectedId, start.getTime())
      : Promise.reject(new Error("当前客户端不支持应用历史"));
  }, [client, range, selectedId]);
  const query = usePageQuery(client, load, 5_000);
  const data = query.data?.range === range ? query.data : null;
  const selected = data?.selected_id === selectedId ? data : null;
  const rows = useMemo(
    () => historyRows(data?.rows ?? [], search, sort),
    [data, search, sort],
  );
  const frames = useMemo(
    () => (selected ? appHistoryFrames(selected) : []),
    [selected],
  );
  const network = snapshot.state?.app_network;
  const status = snapshot.connected ? (network?.status ?? "warming") : "stale";
  const operate = async (action: () => Promise<unknown>) => {
    if (busy.current) return;
    busy.current = true;
    setPending(true);
    setActionError("");
    try {
      await action();
    } catch (e) {
      setActionError(String(e));
    } finally {
      busy.current = false;
      setPending(false);
    }
  };
  return {
    data,
    frames,
    selected,
    range,
    search,
    sort,
    anchor,
    setAnchor,
    pending,
    error: actionError || query.error,
    selectedId,
    rows: rows.slice(0, limit),
    totalRows: rows.length,
    showMore: () => setLimit((n) => n + 20),
    setRange: (value: string) => {
      setRange(value);
      setAnchor(null);
      setLimit(10);
    },
    setSearch: (value: string) => {
      setSearch(value);
      setLimit(10);
    },
    setSort: (value: HistorySort) => {
      setSort(value);
      setLimit(10);
    },
    select: (id: string | null) => {
      setSelectedId(id);
      setAnchor(null);
    },
    running: network?.running ?? false,
    status,
    detail: network?.detail ?? "",
    canRecord:
      snapshot.connected &&
      status !== "unsupported" &&
      !!client.setAppNetworkMonitoring,
    autoRecord: snapshot.state?.settings.record_app_traffic_on_start ?? false,
    record: () =>
      operate(() => client.setAppNetworkMonitoring!(!network?.running)),
    changeAuto: (value: boolean) =>
      operate(async () => {
        const settings = client.getSnapshot().state?.settings;
        if (!settings) throw new Error("设置尚未加载");
        await client.updateSettings({
          ...settings,
          record_app_traffic_on_start: value,
        });
      }),
  };
}
