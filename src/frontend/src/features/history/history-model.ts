import type {
  ArchiveSnapshotDto,
  MinuteBucketDto,
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
  const spacing = data.resolution_ms;
  const byMinute = new Map(data.buckets.map((b) => [b.at_ms, b]));
  const latest = Math.floor(data.now_ms / spacing) * spacing;
  const count = Math.min(
    1441,
    Math.floor((latest - data.from_ms) / spacing) + 1,
  );
  return Array.from({ length: count }, (_, i) => {
    const at = data.from_ms + i * spacing;
    const bucket = byMinute.get(at);
    const time = Math.min(at + spacing, data.now_ms);
    const reading = (
      value: number | null | undefined,
      rate = false,
    ): ReadingDto =>
      value === null || value === undefined
        ? { ...emptyReading, status: "stale", detail: "此时段未采集到有效数据" }
        : {
            ...emptyReading,
            value,
            text: rate ? bytes(value, true) : value.toFixed(1),
            unit: rate ? "" : "%",
            status: "normal",
            valid_at_ms: time,
            source: "本地聚合历史",
            semantic: "archive.weighted_average",
            detail: "时段内有效样本的时间加权平均值",
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

export function historyPeaks(buckets: MinuteBucketDto[]) {
  return (
    [
      ["CPU", "cpu", "cpu_max", "cpu_max_at_ms", "cpu_coverage_ms", false],
      [
        "内存",
        "memory",
        "memory_max",
        "memory_max_at_ms",
        "memory_coverage_ms",
        false,
      ],
      [
        "下载",
        "download",
        "download_max",
        "download_max_at_ms",
        "network_coverage_ms",
        true,
      ],
      [
        "上传",
        "upload",
        "upload_max",
        "upload_max_at_ms",
        "network_coverage_ms",
        true,
      ],
    ] as const
  ).map(([label, meanKey, maxKey, timeKey, coverageKey, rate]) => {
    let peak: number | null = null;
    let at: number | null = null;
    let weighted = 0;
    let covered = 0;
    for (const bucket of buckets) {
      const value = bucket[maxKey];
      if (value !== null && (peak === null || value > peak)) {
        peak = value;
        at = bucket[timeKey];
      }
      const mean = bucket[meanKey];
      if (mean !== null) {
        weighted += mean * bucket[coverageKey];
        covered += bucket[coverageKey];
      }
    }
    const format = (value: number | null) =>
      value === null ? "—" : rate ? bytes(value, true) : `${value.toFixed(1)}%`;
    return {
      id: meanKey,
      label,
      average: format(covered ? weighted / covered : null),
      peak: format(peak),
      at,
      occurred:
        peak === null
          ? "未记录采样峰值"
          : at === null
            ? "旧数据：发生时间未知"
            : new Date(at).toLocaleString(),
      covered: coverage(covered),
    };
  });
}
