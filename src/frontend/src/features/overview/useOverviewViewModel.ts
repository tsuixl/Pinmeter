import { useEffect, useState } from "react";
import type { ReadingDto } from "../../shared/contracts/monitor";
import { useGpuViewModel, type GpuKey } from "../gpu/useGpuViewModel";
import { type ChartSeries } from "../monitoring/chart";
import {
  emptyReading,
  statusLabels,
  useMonitorViewModel,
  type Page,
} from "../monitoring/useMonitorViewModel";

export function overviewSeries(
  gpuId: string | undefined,
  temperatureKey: GpuKey = "temperature",
  temperatureLabel = "GPU 温度",
): ChartSeries[] {
  const missing: ReadingDto = {
    ...emptyReading,
    status: "unsupported",
    detail: "此时没有该显卡的有效采样",
  };
  return [
    {
      id: "cpu",
      label: "CPU 使用率",
      color: "var(--color-chart-5)",
      read: (f) => f.cpu,
    },
    {
      id: "cpu_temperature",
      label: "CPU 温度",
      color: "var(--color-chart-2)",
      temperature: true,
      read: (f) => f.cpu_temperature,
    },
    {
      id: "memory",
      label: "内存",
      color: "var(--color-chart-3)",
      read: (f) => f.memory,
    },
    {
      id: "gpu_usage",
      label: "GPU 使用率",
      color: "var(--color-chart-1)",
      read: (f) =>
        f.gpus.find((d) => d.id === gpuId)?.readings.usage ?? missing,
    },
    {
      id: "gpu_temperature",
      label: temperatureLabel,
      color: "var(--color-chart-4)",
      temperature: true,
      read: (f) =>
        f.gpus.find((d) => d.id === gpuId)?.readings[temperatureKey] ?? missing,
    },
  ];
}

const text = (reading: ReadingDto) =>
  reading.status === "normal" ? `${reading.text} ${reading.unit}`.trim() : "—";

export function useOverviewViewModel(
  monitor: ReturnType<typeof useMonitorViewModel>,
  gpu: ReturnType<typeof useGpuViewModel>,
) {
  const [hidden, setHidden] = useState<string[]>([]);
  const [group, setGroup] = useState("usage");
  useEffect(() => {
    monitor.setAnchor(null);
  }, [gpu.selected?.id, monitor.setAnchor]);
  const series = overviewSeries(
    gpu.selected?.id,
    gpu.temperature.key,
    gpu.temperature.label,
  );
  const groupSeries = series.filter(
    (item) =>
      group === "all" ||
      (group === "temperature" ? item.temperature : !item.temperature),
  );
  const card = (
    id: string,
    title: string,
    reading: ReadingDto,
    page: Page,
    description?: string,
  ) => ({
    id,
    title,
    page,
    value: text(reading),
    description:
      reading.status === "normal"
        ? (description ?? statusLabels[reading.status])
        : reading.detail || statusLabels[reading.status],
  });
  return {
    cards: [
      card(
        "cpu",
        "CPU 使用率",
        monitor.reading("cpu"),
        "cpu",
        monitor.cpuModel,
      ),
      card(
        "cpu_temperature",
        "CPU 温度",
        monitor.temperature,
        "cpu",
        monitor.cpuModel,
      ),
      card(
        "gpu_usage",
        "GPU 使用率",
        gpu.reading("usage"),
        "gpu",
        gpu.selected?.name,
      ),
      card(
        "gpu_temperature",
        gpu.temperature.label,
        gpu.reading(gpu.temperature.key),
        "gpu",
        gpu.temperature.key === "vr_soc_temperature"
          ? gpu.temperature.description
          : gpu.selected?.name,
      ),
      card(
        "memory",
        "内存",
        monitor.reading("memory"),
        "memory",
        `${monitor.frame?.memory_used ?? "—"} / ${monitor.frame?.memory_total ?? "—"}`,
      ),
      {
        id: "network",
        title: "网速",
        page: "network" as Page,
        value: `↓ ${text(monitor.reading("download"))}\n↑ ${text(monitor.reading("upload"))}`,
        description: `${monitor.networkName} · ${monitor.state?.settings.network_id ? "手动选择" : "自动选择"}${[
          "download",
          "upload",
        ]
          .flatMap((key) => {
            const reading = monitor.reading(key as "download" | "upload");
            return reading.status === "normal"
              ? []
              : [
                  ` · ${key === "download" ? "下载" : "上传"}${statusLabels[reading.status]}`,
                ];
          })
          .join("")}`,
      },
    ],
    series,
    groupSeries,
    group,
    setGroup,
    visibleSeries: groupSeries.filter((item) => !hidden.includes(item.id)),
    hidden,
    toggle: (id: string) =>
      setHidden((previous) =>
        previous.includes(id)
          ? previous.filter((key) => key !== id)
          : [...previous, id],
      ),
  };
}
