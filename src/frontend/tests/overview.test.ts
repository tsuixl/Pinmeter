import { describe, it, expect } from "vitest";
import { overviewSeries } from "../src/features/overview/useOverviewViewModel";
import { chartPaths } from "../src/features/monitoring/chart";
import { DemoClient } from "../src/shared/client/demo-client";

describe("overview CPU and GPU coexistence", () => {
  it("reads five independent metrics without overwriting CPU history", () => {
    const history = new DemoClient().getSnapshot().state!.history;
    const frame = history.at(-1)!;
    const series = overviewSeries("demo-gpu-0");
    expect(new Set(series.map((item) => item.color)).size).toBe(5);
    expect(series[0].read(frame)).toBe(frame.cpu);
    expect(series[1].read(frame)).toBe(frame.cpu_temperature);
    expect(series[2].read(frame)).toBe(frame.memory);
    expect(series[3].read(frame)).toBe(
      frame.gpus.find((d) => d.id === "demo-gpu-0")!.readings.usage,
    );
    expect(series[4].read(frame)).toBe(
      frame.gpus.find((d) => d.id === "demo-gpu-0")!.readings.temperature,
    );
    expect(
      series.filter((item) => item.temperature).map((item) => item.id),
    ).toEqual(["cpu_temperature", "gpu_temperature"]);
  });

  it("keeps integrated GPU zero, missing temperatures and device removal separate", () => {
    const frame = new DemoClient().getSnapshot().state!.history.at(-1)!;
    const series = overviewSeries("demo-gpu-1");
    expect(series[3].read(frame).value).toBe(0);
    expect(series[4].read(frame).status).toBe("unsupported");
    const frames = [0, 1, 2].map((i) => ({
      ...frame,
      elapsed_ms: i * 1000,
      gpus: i === 1 ? [] : frame.gpus,
    }));
    expect(chartPaths(frames, series[3], 0, 2000, 100)).toHaveLength(2);
    expect(chartPaths(frames, series[4], 0, 2000, 100)).toHaveLength(0);
    expect(chartPaths(frames, series[0], 0, 2000, 100)).toHaveLength(1);
    expect(overviewSeries("removed")[3].read(frame).value).toBeNull();
  });
});
