import { describe, it, expect } from "vitest";
import {
  overviewCards,
  overviewSeries,
} from "../src/features/overview/useOverviewViewModel";
import { chartPaths } from "../src/features/monitoring/chart";
import { DemoClient } from "../src/shared/client/demo-client";
import { gpuTemperature } from "../src/features/gpu/useGpuViewModel";
import { emptyReading } from "../src/features/monitoring/useMonitorViewModel";
import type { ReadingDto } from "../src/shared/contracts/monitor";

function cardSources() {
  const state = new DemoClient().getSnapshot().state!;
  const frame = state.frame!;
  const selected = state.gpu.devices[0];
  return {
    monitor: {
      state,
      frame,
      cpuModel: state.cpu_model!,
      temperature: state.cpu_temperature,
      networkName: state.selected_interface!.name,
      reading: (
        key: "cpu" | "cpu_temperature" | "memory" | "download" | "upload",
      ) => frame[key],
    },
    gpu: {
      selected,
      temperature: gpuTemperature(selected.readings),
      reading: (key: string) => selected.readings[key] ?? emptyReading,
    },
  };
}

describe("overview resource cards", () => {
  it("merges utilization and temperature into four resource cards with one device description", () => {
    const { monitor, gpu } = cardSources();
    const cards = overviewCards(monitor, gpu);
    expect(cards.map((card) => card.id)).toEqual([
      "cpu",
      "gpu",
      "memory",
      "network",
    ]);
    expect(cards[0].readings.map((reading) => reading.id)).toEqual([
      "cpu",
      "cpu_temperature",
    ]);
    expect(cards[0].description).toBe(monitor.cpuModel);
    expect(cards[1].description).toBe(gpu.selected.name);
    expect(cards[2].description).toBe(
      `${monitor.frame.memory_used} / ${monitor.frame.memory_total}`,
    );
    expect(cards[3].readings.map((reading) => reading.id)).toEqual([
      "download",
      "upload",
    ]);
  });

  it.each(["unsupported", "permission_denied", "failed", "stale"] as const)(
    "keeps a normal zero utilization independent from %s temperature",
    (status) => {
      const { monitor, gpu } = cardSources();
      const zero: ReadingDto = {
        ...monitor.reading("cpu"),
        value: 0,
        text: "0.0",
        unit: "%",
        status: "normal",
      };
      const cards = overviewCards(
        {
          ...monitor,
          reading: (key) => (key === "cpu" ? zero : monitor.reading(key)),
          temperature: {
            ...monitor.temperature,
            status,
            value: null,
            text: "0",
            detail: "温度读数不可用",
          },
        },
        gpu,
      );
      expect(cards[0].readings[0]).toMatchObject({
        value: "0.0 %",
        status: "normal",
        statusLabel: "",
      });
      expect(cards[0].readings[1]).toMatchObject({
        value: "—",
        status,
        detail: "温度读数不可用",
      });
      expect(cards[0].readings[1].statusLabel).not.toBe("");
    },
  );

  it("preserves the actual VR SoC label and the selected GPU's values", () => {
    const { monitor, gpu } = cardSources();
    const vrSoc: ReadingDto = {
      ...emptyReading,
      value: 53,
      text: "53",
      unit: "°C",
      status: "normal",
    };
    const readings: Record<string, ReadingDto> = {
      ...gpu.selected.readings,
      temperature: { ...emptyReading, status: "unsupported" as const },
      vr_soc_temperature: vrSoc,
    };
    const card = overviewCards(monitor, {
      ...gpu,
      temperature: gpuTemperature(readings),
      reading: (key) => readings[key] ?? emptyReading,
    })[1];
    expect(card.description).toBe(gpu.selected.name);
    expect(card.readings[1]).toMatchObject({
      label: "VR SoC 温度",
      value: "53 °C",
      status: "normal",
    });
  });
});

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
