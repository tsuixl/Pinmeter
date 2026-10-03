import type { ArchiveSnapshotDto } from "../contracts/monitor";
export function demoArchive(
  dayStartMs: number,
  rangeMs = 86400000,
  clearedAt = 0,
): ArchiveSnapshotDto {
  const now = Date.now();
  const spacing =
    rangeMs <= 86400000 ? 60000 : rangeMs <= 604800000 ? 900000 : 3600000;
  const from = Math.floor((now - rangeMs) / spacing) * spacing;
  const minute = Math.floor(now / spacing) * spacing;
  const count = Math.floor((minute - from) / spacing) + 1;
  const period = (start: number, end: number) => {
    const covered = Math.max(0, end - Math.max(start, clearedAt)) * 0.8;
    return {
      from_ms: start,
      through_ms: end,
      cpu_average: covered ? 22.5 : null,
      cpu_coverage_ms: covered,
      memory_average: covered ? 47.2 : null,
      memory_coverage_ms: covered,
      received: Math.round(covered * 1400).toString(),
      transmitted: Math.round(covered * 120).toString(),
      network_coverage_ms: covered,
    };
  };
  return {
    loading: false,
    notice: "",
    persistence_error: "",
    saved_at_ms: minute,
    lost_samples: "0",
    clock_discontinuities: "0",
    now_ms: now,
    day_start_ms: dayStartMs,
    from_ms: from,
    resolution_ms: spacing,
    current_period: period(minute - rangeMs, minute),
    previous_period:
      rangeMs < 2592000000
        ? period(minute - 2 * rangeMs, minute - rangeMs)
        : null,
    received: clearedAt ? "0" : "5243923000",
    transmitted: clearedAt ? "0" : "403885000",
    network_coverage_ms: clearedAt
      ? 0
      : Math.min(now - dayStartMs, 6 * 3600_000),
    networks: ["演示以太网"],
    buckets: Array.from({ length: count }, (_, i) => {
      const cpu = 20 + 15 * Math.sin(i / 10);
      const at = from + i * spacing;
      const memory = 45 + 6 * Math.sin(i / 70);
      return {
        at_ms: at,
        cpu,
        cpu_min: cpu - 3,
        cpu_max: cpu + 5,
        cpu_max_at_ms: i % 17 === 0 ? null : Math.min(at + 5000, now),
        memory,
        memory_min: memory - 2,
        memory_max: memory + 3,
        memory_max_at_ms: Math.min(at + 3000, now),
        cpu_coverage_ms: spacing,
        memory_coverage_ms: spacing,
        download: 1e6 + 6e5 * Math.sin(i / 20),
        upload: 2e5 + 1e5 * Math.sin(i / 13),
        download_max: i % 19 === 0 ? null : 2e6 + 7e5 * Math.sin(i / 20),
        download_max_at_ms: i % 19 === 0 ? null : Math.min(at + 5000, now),
        upload_max: i % 19 === 0 ? null : 4e5 + 2e5 * Math.sin(i / 13),
        upload_max_at_ms: i % 19 === 0 ? null : Math.min(at + 5000, now),
        network_coverage_ms: spacing,
        network_key: "demo",
      };
    }).filter(
      (bucket, i) =>
        bucket.at_ms >= clearedAt && !(i > count * 0.43 && i < count * 0.62),
    ),
  };
}
