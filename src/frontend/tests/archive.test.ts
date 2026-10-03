import { describe, expect, it } from "vitest";
import { archiveFrames } from "../src/features/history/history-model";
import { chartPaths, type ChartSeries } from "../src/features/monitoring/chart";
import { demoArchive } from "../src/shared/client/demo-history";
describe("local minute history", () => {
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
