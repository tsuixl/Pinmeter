import type {
  ArchiveSnapshotDto,
  ReadingDto,
} from "../../shared/contracts/monitor";
import { emptyReading } from "../monitoring/useMonitorViewModel";
import { plotFrame } from "../monitoring/plot-frame";
export function bytes(value: number | string, rate = false) {
  let scaled = Number(value);
  let unit = 0;
  while (scaled >= 1000 && unit < 6) {
    scaled /= 1000;
    unit++;
  }
  return `${scaled.toFixed(unit ? 2 : 0)} ${["B", "KB", "MB", "GB", "TB", "PB", "EB"][unit]}${rate ? "/s" : ""}`;
}
export function coverage(ms: number) {
  const seconds = Math.floor(ms / 1000);
  return seconds >= 3600
    ? `${Math.floor(seconds / 3600)} 小时 ${Math.floor((seconds % 3600) / 60)} 分钟`
    : seconds >= 60
      ? `${Math.floor(seconds / 60)} 分 ${seconds % 60} 秒`
      : `${seconds} 秒`;
}
export function archiveFrames(data: ArchiveSnapshotDto) {
  const byMinute = new Map(data.buckets.map((b) => [b.at_ms, b]));
  const latest = Math.floor(data.now_ms / 60_000) * 60_000;
  return Array.from({ length: 1441 }, (_, i) => {
    const at = latest - (1440 - i) * 60_000;
    const bucket = byMinute.get(at);
    const time = Math.min(at + 60_000, data.now_ms);
    const reading = (
      value: number | null | undefined,
      rate = false,
    ): ReadingDto =>
      value === null || value === undefined
        ? { ...emptyReading, status: "stale", detail: "此分钟未采集到有效数据" }
        : {
            ...emptyReading,
            value,
            text: rate ? bytes(value, true) : value.toFixed(1),
            unit: rate ? "" : "%",
            status: "normal",
            valid_at_ms: time,
            source: "本地分钟历史",
            semantic: "minute.average",
            detail: "分钟内有效样本的时间加权平均值",
          };
    const frame = plotFrame(time, time, "archive", {
      cpu: reading(bucket?.cpu),
      memory: reading(bucket?.memory),
      download: reading(bucket?.download, true),
      upload: reading(bucket?.upload, true),
    });
    frame.network_generation = bucket?.network_key ?? "missing";
    return frame;
  });
}
