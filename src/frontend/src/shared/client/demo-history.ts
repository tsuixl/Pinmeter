import type { ArchiveSnapshotDto } from "../contracts/monitor";
export function demoArchive(dayStartMs: number): ArchiveSnapshotDto {
  const now = Date.now();
  const minute = Math.floor(now / 60_000) * 60_000;
  return {
    loading: false,
    notice: "",
    persistence_error: "",
    saved_at_ms: minute,
    lost_samples: "0",
    clock_discontinuities: "0",
    now_ms: now,
    day_start_ms: dayStartMs,
    received: "5243923000",
    transmitted: "403885000",
    network_coverage_ms: Math.min(now - dayStartMs, 6 * 3600_000),
    networks: ["演示以太网"],
    buckets: Array.from({ length: 1440 }, (_, i) => {
      const cpu = 20 + 15 * Math.sin(i / 10);
      return {
        at_ms: minute - (1439 - i) * 60_000,
        cpu,
        cpu_min: cpu - 3,
        cpu_max: cpu + 5,
        memory: 45 + 6 * Math.sin(i / 70),
        cpu_coverage_ms: 60_000,
        memory_coverage_ms: 60_000,
        download: 1e6 + 6e5 * Math.sin(i / 20),
        upload: 2e5 + 1e5 * Math.sin(i / 13),
        network_coverage_ms: 60_000,
        network_key: "demo",
      };
    }).filter((_, i) => !(i > 620 && i < 900)),
  };
}
