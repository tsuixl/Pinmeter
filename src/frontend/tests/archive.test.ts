import { describe, expect, it } from "vitest";
import {
  archiveFrames,
  historyPeaks,
} from "../src/features/history/history-model";
import { chartPaths, type ChartSeries } from "../src/features/monitoring/chart";
import { demoArchive } from "../src/shared/client/demo-history";
import { chartTimeLabel } from "../src/features/monitoring/TrendChart";
describe("local minute history", () => {
  it("distinguishes days on long charts while keeping short chart time labels", () => {
    const first = new Date(2026, 9, 1, 8, 30).getTime();
    const second = new Date(2026, 9, 2, 8, 30).getTime();
    expect(chartTimeLabel(first, 86400000)).not.toBe(
      chartTimeLabel(second, 86400000),
    );
    expect(chartTimeLabel(first, 300000)).toBe(chartTimeLabel(second, 300000));
  });
  it("uses one bounded layer and preserves gaps for week and month views", () => {
    for (const range of [604800000, 2592000000]) {
      const data = demoArchive(new Date().setHours(0, 0, 0, 0), range);
      const frames = archiveFrames(data);
      expect(frames.length).toBeLessThanOrEqual(
        range === 604800000 ? 673 : 721,
      );
      expect(frames.some((frame) => frame.cpu.value === null)).toBe(true);
      expect(data.previous_period === null).toBe(range === 2592000000);
    }
  });
  it("weights averages by coverage and never fabricates legacy peak timestamps", () => {
    const [first, second] = demoArchive(0).buckets;
    const buckets = [
      {
        ...first,
        cpu: 100,
        cpu_max: 100,
        cpu_max_at_ms: null,
        cpu_coverage_ms: 1000,
        download_max: null,
        download_max_at_ms: null,
      },
      {
        ...second,
        cpu: 0,
        cpu_max: 0,
        cpu_max_at_ms: null,
        cpu_coverage_ms: 9000,
        download_max: null,
        download_max_at_ms: null,
      },
    ];
    const rows = historyPeaks(buckets);
    expect(rows[0].average).toBe("10.0%");
    expect(rows[0].occurred).toContain("发生时间未知");
    expect(rows[2].peak).toBe("—");
    expect(rows[2].occurred).toBe("未记录采样峰值");
  });
  it("connects adjacent minutes and preserves the offline gap", () => {
    const data = demoArchive(new Date().setHours(0, 0, 0, 0));
    const frames = archiveFrames(data);
    const series: ChartSeries = {
      id: "cpu",
      label: "CPU",
      color: "",
      maxGapMs: 90_000,
      read: (f) => f.cpu,
    };
    const paths = chartPaths(
      frames,
      series,
      data.now_ms - 86400_000,
      data.now_ms,
      100,
    );
    expect(paths).toHaveLength(2);
    expect(paths.every((p) => p.includes(" L"))).toBe(true);
    expect(frames.some((f) => f.cpu.value === null)).toBe(true);
    expect(frames.length).toBe(1441);
  });
});
