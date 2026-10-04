import { useEffect } from "react";
import type { ReadingDto } from "../../shared/contracts/monitor";
import { usePageUiState } from "../../shared/state/page-ui-state";
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

type OverviewMonitor = Pick<
  ReturnType<typeof useMonitorViewModel>,
  "reading" | "temperature" | "cpuModel" | "frame" | "networkName" | "state"
>;
type OverviewGpu = Pick<
  ReturnType<typeof useGpuViewModel>,
  "reading" | "temperature" | "selected"
>;

export function overviewCards(monitor: OverviewMonitor, gpu: OverviewGpu) {
  const readout = (
    id: string,
    label: string,
    reading: ReadingDto,
    secondary = false,
  ) => ({
    id,
    label,
    value: text(reading),
    status: reading.status,
    statusLabel:
      reading.status === "normal" ? "" : statusLabels[reading.status],
    detail: reading.detail,
    secondary,
  });
  const memory = monitor.reading("memory");
  return [
    {
      id: "cpu",
      title: "CPU",
      page: "cpu" as Page,
      readings: [
        readout("cpu", "使用率", monitor.reading("cpu")),
        readout("cpu_temperature", "温度", monitor.temperature, true),
      ],
      description: monitor.cpuModel,
    },
    {
      id: "gpu",
      title: "GPU",
      page: "gpu" as Page,
      readings: [
        readout("gpu_usage", "使用率", gpu.reading("usage")),
        readout(
          "gpu_temperature",
          gpu.temperature.label.replace(/^GPU /, ""),
          gpu.reading(gpu.temperature.key),
          true,
        ),
      ],
      description: gpu.selected?.name ?? "暂无可用显卡",
    },
    {
      id: "memory",
      title: "内存",
      page: "memory" as Page,
      readings: [readout("memory", "使用率", memory)],
      description:
        memory.status === "normal"
          ? `${monitor.frame?.memory_used ?? "—"} / ${monitor.frame?.memory_total ?? "—"}`
          : "物理内存",
    },
    {
      id: "network",
      title: "网速",
      page: "network" as Page,
      readings: [
        readout("download", "↓ 下载", monitor.reading("download")),
        readout("upload", "↑ 上传", monitor.reading("upload")),
      ],
      description: `${monitor.networkName} · ${monitor.state?.settings.network_id ? "手动选择" : "自动选择"}`,
    },
  ];
}

export function useOverviewViewModel(
  monitor: ReturnType<typeof useMonitorViewModel>,
  gpu: ReturnType<typeof useGpuViewModel>,
) {
  const [hidden, setHidden] = usePageUiState<string[]>("overview.hidden", []);
  const [group, setGroup] = usePageUiState("overview.group", "usage");
  const gpuId = gpu.selected?.id ?? null;
  const [lastGpuId, setLastGpuId] = usePageUiState<string | null>(
    "overview.gpuId",
    gpuId,
  );
  useEffect(() => {
    if (gpuId === lastGpuId) return;
    monitor.setAnchor(null);
    setLastGpuId(gpuId);
  }, [gpuId, lastGpuId, monitor.setAnchor, setLastGpuId]);
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
  return {
    cards: overviewCards(monitor, gpu),
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
