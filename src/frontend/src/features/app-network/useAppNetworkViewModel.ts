import { useEffect, useState, useSyncExternalStore } from "react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import type {
  AppNetworkDto,
  AppNetworkRowDto,
  AppTrafficDto,
} from "../../shared/contracts/monitor";

export function formatTraffic(value: number | null, rate = true) {
  if (value === null) return "—";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let index = 0;
  while (value >= 1000 && index < units.length - 1) {
    value /= 1000;
    index++;
  }
  return `${value.toFixed(index ? 1 : 0)} ${units[index]}${rate ? "/s" : ""}`;
}
export type SortKey = "name" | "download" | "upload";
export interface AppNetworkSort {
  key: SortKey;
  order: "asc" | "desc";
}
const names = new Intl.Collator("zh-CN", {
  numeric: true,
  sensitivity: "base",
});
export function sortedApps(apps: AppNetworkRowDto[], sort: AppNetworkSort) {
  return [...apps].sort((a, b) => {
    let comparison: number;
    if (sort.key === "name") comparison = names.compare(a.name, b.name);
    else {
      const left = a.traffic[sort.key];
      const right = b.traffic[sort.key];
      // Missing readings always follow valid zeroes, in either direction.
      if (left === null && right !== null) return 1;
      if (right === null && left !== null) return -1;
      comparison = (left ?? 0) - (right ?? 0);
    }
    return (
      (sort.order === "asc" ? comparison : -comparison) ||
      a.id.localeCompare(b.id)
    );
  });
}
export const networkStatusLabels: Record<string, string> = {
  disabled: "未监控",
  authorizing: "启动中",
  warming: "采样中",
  normal: "实时",
  incomplete: "统计不完整",
  stale: "数据过期",
  failed: "采集失败",
  permission_denied: "权限不足",
  unsupported: "不支持",
};
export interface DisplayRow {
  id: string;
  name: string;
  path: string;
  traffic: AppTrafficDto;
  download: number | null;
  upload: number | null;
  app?: AppNetworkRowDto;
  process?: boolean;
  observed?: boolean;
}
export function useAppNetworkViewModel(client: MonitorClient, active = true) {
  const snapshot = useSyncExternalStore(client.subscribe, client.getSnapshot);
  const [sorting, setSorting] = useState<{
    sort: AppNetworkSort;
    direction: "download" | "upload";
  }>({ sort: { key: "download", order: "desc" }, direction: "download" });
  const { sort, direction } = sorting;
  const [expanded, setExpanded] = useState<Set<string>>(new Set());
  const [search, setSearch] = useState("");
  const [pinned, setPinned] = useState<{ id: string; name: string } | null>(
    null,
  );
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const data = snapshot.state?.app_network;
  useEffect(() => setExpanded(new Set()), [data?.session]);
  const status = snapshot.connected ? (data?.status ?? "warming") : "stale";
  const live = status === "normal";
  const safeTraffic = (traffic: AppTrafficDto): AppTrafficDto =>
    live
      ? traffic
      : {
          ...traffic,
          download: null,
          upload: null,
          download_share: null,
          upload_share: null,
        };
  const rows: DisplayRow[] = [];
  const apps = sortedApps(
    (active ? (data?.apps ?? []) : []).map((app) => ({
      ...app,
      traffic: safeTraffic(app.traffic),
    })),
    sort,
  );
  const needle = search.trim().toLocaleLowerCase();
  const matching = apps.filter((app) =>
    `${app.name}\n${app.path}`.toLocaleLowerCase().includes(needle),
  );
  const visibleApps = pinned
    ? [
        ...matching.filter((a) => a.id === pinned.id),
        ...matching.filter((a) => a.id !== pinned.id),
      ]
    : matching;
  for (const app of visibleApps) {
    rows.push({
      ...app,
      download: app.traffic.download,
      upload: app.traffic.upload,
      app,
    });
    if (expanded.has(app.id))
      for (const process of app.processes) {
        rows.push({
          id: `${app.id}/${process.id}`,
          name: `PID ${process.pid}`,
          path: "",
          traffic: safeTraffic(process.traffic),
          download: live ? process.traffic.download : null,
          upload: live ? process.traffic.upload : null,
          process: true,
          observed: live && process.observed,
        });
      }
  }
  for (const [key, name] of [
    ["other", "其他已归属"],
    ["unknown", "未归属"],
  ] as const) {
    const traffic = data?.[key];
    if (
      !needle &&
      traffic &&
      (traffic.received !== "0" || traffic.sent !== "0")
    )
      rows.push({
        id: key,
        name,
        path: "",
        traffic: safeTraffic(traffic),
        download: live ? traffic.download : null,
        upload: live ? traffic.upload : null,
      });
  }
  return {
    data: data as AppNetworkDto | undefined,
    rows,
    search,
    setSearch,
    pinned,
    pin: (app: AppNetworkRowDto) =>
      setPinned((current) =>
        current?.id === app.id ? null : { id: app.id, name: app.name },
      ),
    clearPin: () => setPinned(null),
    pinnedMissing: !!pinned && !apps.some((a) => a.id === pinned.id),
    selectedId,
    select: setSelectedId,
    totalApps: apps.length,
    matchingApps: matching.length,
    status,
    expanded,
    direction,
    sort,
    pending,
    error,
    detail: snapshot.connected
      ? data?.detail
      : "采集服务未连接，旧速率和占比已隐藏",
    canStart:
      snapshot.connected &&
      !!client.setAppNetworkMonitoring &&
      status !== "unsupported",
    setSort: (key: SortKey) =>
      setSorting((previous) => ({
        sort: {
          key,
          order:
            previous.sort.key === key
              ? previous.sort.order === "asc"
                ? "desc"
                : "asc"
              : key === "name"
                ? "asc"
                : "desc",
        },
        direction: key === "name" ? previous.direction : key,
      })),
    toggle: (id: string) =>
      setExpanded((previous) => {
        const next = new Set(previous);
        if (next.has(id)) next.delete(id);
        else next.add(id);
        return next;
      }),
    setMonitoring: async (enabled: boolean) => {
      setPending(true);
      setError(null);
      try {
        await client.setAppNetworkMonitoring?.(enabled);
      } catch (failure) {
        setError(String(failure));
      } finally {
        setPending(false);
      }
    },
  };
}
