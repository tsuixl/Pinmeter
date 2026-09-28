import { useEffect, useState } from "react";
import type {
  FrameDto,
  GpuSnapshotDto,
  ReadingDto,
} from "../../shared/contracts/monitor";
import { emptyReading } from "../monitoring/useMonitorViewModel";

export type GpuKey =
  | "usage"
  | "dedicated_used"
  | "shared_used"
  | "memory_total"
  | "temperature"
  | "vr_soc_temperature"
  | "core_clock"
  | "memory_clock";
const missing: ReadingDto = {
  ...emptyReading,
  status: "unsupported",
  detail: "此显卡未提供该指标",
};
export function gpuTemperature(readings: Record<string, ReadingDto> = {}) {
  const fallback =
    (!readings.temperature || readings.temperature.status === "unsupported") &&
    readings.vr_soc_temperature &&
    readings.vr_soc_temperature.status !== "unsupported";
  return {
    key: (fallback ? "vr_soc_temperature" : "temperature") as GpuKey,
    label: fallback ? "GPU VR SoC 温度" : "GPU 核心温度",
    description: fallback
      ? "未提供核心温度，显示 VR SoC 传感器读数"
      : "GPU 核心传感器",
  };
}
export function projectGpuHistory(
  history: FrameDto[],
  id: string,
  first: GpuKey,
  second: GpuKey,
  temperature = false,
): FrameDto[] {
  return history.map((frame) => {
    const gpu = frame.gpus.find((g) => g.id === id);
    const absent = { ...missing, detail: "此时没有该显卡的有效采样" };
    return {
      ...frame,
      cpu: gpu?.readings[first] ?? absent,
      ...(temperature
        ? { cpu_temperature: gpu?.readings[second] ?? absent }
        : { memory: gpu?.readings[second] ?? absent }),
    };
  });
}
export function useGpuViewModel(
  gpu: GpuSnapshotDto | undefined,
  connected: boolean,
  history: FrameDto[],
) {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const [anchor, setAnchor] = useState<number | null>(null);
  const devices = gpu?.devices ?? [];
  const selected =
    devices.find((d) => d.id === selectedId) ??
    devices.reduce<(typeof devices)[number] | undefined>(
      (best, device) =>
        !best ||
        (device.readings.memory_total?.value ?? 0) >
          (best.readings.memory_total?.value ?? 0)
          ? device
          : best,
      undefined,
    );
  useEffect(() => {
    setSelectedId(selected?.id ?? null);
    setAnchor(null);
  }, [selected?.id]);
  const reading = (key: GpuKey): ReadingDto => {
    const value = selected?.readings[key] ?? missing;
    return connected
      ? value
      : {
          ...value,
          value: null,
          text: "—",
          status: "stale",
          detail: "采集服务未连接",
        };
  };
  return {
    devices,
    selected,
    temperature: gpuTemperature(selected?.readings),
    setSelectedId,
    reading,
    anchor,
    setAnchor,
    status: connected ? (gpu?.status ?? "warming") : "stale",
    detail: connected ? (gpu?.detail ?? "等待 GPU 采样") : "采集服务未连接",
    history: (first: GpuKey, second: GpuKey, temperature = false) =>
      projectGpuHistory(
        history,
        selected?.id ?? "",
        first,
        second,
        temperature,
      ),
  };
}
