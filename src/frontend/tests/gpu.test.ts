import { describe, expect, it } from "vitest";
import {
  gpuTemperature,
  projectGpuHistory,
} from "../src/features/gpu/useGpuViewModel";
import { overviewSeries } from "../src/features/overview/useOverviewViewModel";
import { DemoClient } from "../src/shared/client/demo-client";

describe("GPU history projection", () => {
  it("labels VR SoC separately and never mixes it into core temperature history", () => {
    const frames = new DemoClient().getSnapshot().state!.history;
    const readings = frames.at(-1)!.gpus[1].readings;
    const temperature = gpuTemperature(readings);
    expect(temperature.key).toBe("vr_soc_temperature");
    expect(temperature.label).toBe("GPU VR SoC 温度");
    const series = overviewSeries(
      "demo-gpu-1",
      temperature.key,
      temperature.label,
    )[4];
    expect(series.read(frames.at(-1)!).value).toBeGreaterThan(0);
    expect(series.label).toBe(temperature.label);
    const history = projectGpuHistory(
      frames,
      "demo-gpu-1",
      "usage",
      temperature.key,
      true,
    );
    expect(history.at(-1)!.cpu_temperature.value).toBe(
      series.read(frames.at(-1)!).value,
    );
    expect(readings.temperature.value).toBeNull();
    expect(
      gpuTemperature({
        ...readings,
        temperature: { ...readings.vr_soc_temperature, status: "normal" },
      }).key,
    ).toBe("temperature");
    expect(
      gpuTemperature({
        ...readings,
        temperature: { ...readings.temperature, status: "failed" },
      }).key,
    ).toBe("temperature");
    expect(
      gpuTemperature({
        ...readings,
        vr_soc_temperature: {
          ...readings.vr_soc_temperature,
          status: "unsupported",
          value: null,
        },
      }).key,
    ).toBe("temperature");
  });
  it("selects one device and keeps unavailable temperature distinct from zero", () => {
    const client = new DemoClient();
    const frames = client.getSnapshot().state!.history;
    const integrated = projectGpuHistory(
      frames,
      "demo-gpu-1",
      "usage",
      "temperature",
      true,
    );
    expect(integrated.at(-1)!.cpu.value).toBe(0);
    expect(integrated.at(-1)!.cpu_temperature.status).toBe("unsupported");
    expect(integrated.at(-1)!.cpu_temperature.value).toBeNull();
    const discrete = projectGpuHistory(
      frames,
      "demo-gpu-0",
      "usage",
      "temperature",
      true,
    );
    expect(discrete.at(-1)!.cpu.value).toBeGreaterThan(0);
    expect(discrete.some((f) => f.cpu.status === "failed")).toBe(true);
    expect(
      projectGpuHistory(frames, "removed", "usage", "temperature", true).every(
        (f) => f.cpu.value === null,
      ),
    ).toBe(true);
    expect(frames.at(-1)!.cpu.semantic).toBe("cpu.busy_time");
  });
});
