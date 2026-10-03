import type {
  AppHistoryRowDto,
  AppHistorySnapshotDto,
  ReadingDto,
} from "../../shared/contracts/monitor";
import { emptyReading } from "../monitoring/useMonitorViewModel";
import { plotFrame } from "../monitoring/plot-frame";
import { bytes } from "./history-model";
export type HistorySort = "total" | "received" | "transmitted" | "name";
const names = new Intl.Collator("zh-CN", {
  numeric: true,
  sensitivity: "base",
});
export function historyRows(
  rows: AppHistoryRowDto[],
  search: string,
  sort: HistorySort,
) {
  const term = search.trim().toLocaleLowerCase();
  return [...rows]
    .filter(
      (row) =>
        !term || `${row.name} ${row.path}`.toLocaleLowerCase().includes(term),
    )
    .sort((a, b) => {
      const aSpecial = !a.id.startsWith("app:");
      const bSpecial = !b.id.startsWith("app:");
      if (aSpecial !== bSpecial) return aSpecial ? 1 : -1;
      if (sort === "name")
        return names.compare(a.name, b.name) || a.id.localeCompare(b.id);
      const left = BigInt(a[sort]);
      const right = BigInt(b[sort]);
      return (
        (left > right ? -1 : left < right ? 1 : 0) || a.id.localeCompare(b.id)
      );
    });
}
export function appHistoryFrames(data: AppHistorySnapshotDto) {
  const reading = (value: number | null, at: number): ReadingDto =>
    value === null
      ? {
          ...emptyReading,
          status: "stale",
          detail: "未记录、采集缺失或无法确认应用归属",
        }
      : {
          ...emptyReading,
          value,
          status: "normal",
          text: bytes(value, true),
          unit: "",
          valid_at_ms: at,
          source: "应用流量历史",
          semantic: "app_network.observed_minute_rate",
          detail: "分钟内有效采集区间的平均速率",
        };
  return data.points.map((p) =>
    plotFrame(p.at_ms, p.at_ms, "app-history", {
      download: reading(p.download, p.at_ms),
      upload: reading(p.upload, p.at_ms),
    }),
  );
}
