import type {
  GpuSnapshotDto,
  ReadingDto,
  ReadingStatus,
} from "../contracts/monitor";
export function demoGpu(
  status: ReadingStatus,
  n: number,
  at: number,
): GpuSnapshotDto {
  const metric = (
    value: number,
    unit: string,
    supported = true,
  ): ReadingDto => {
    const current = supported ? status : "unsupported";
    return {
      value: current === "normal" ? value : null,
      text:
        current === "normal"
          ? (value / (unit === "GiB" ? 1073741824 : 1)).toFixed(2)
          : "—",
      unit,
      status: current,
      valid_at_ms: current === "normal" ? at : null,
      source: "演示数据",
      semantic: "demo.gpu",
      detail: supported
        ? current === "normal"
          ? ""
          : "演示异常状态"
        : "此显卡未提供核心温度",
    };
  };
  return {
    status,
    detail: status === "normal" ? "" : "GPU 演示状态",
    devices: [0, 1].map((i) => ({
      id: `demo-gpu-${i}`,
      name: i
        ? "AMD Radeon 核显（演示）"
        : "NVIDIA GeForce RTX 5070 Ti（演示）",
      readings: {
        usage: metric(i ? 0 : 45 + Math.sin(n / 8) * 25, "%"),
        temperature: metric(55 + Math.sin(n / 12) * 8, "°C", i === 0),
        vr_soc_temperature: metric(56 + Math.sin(n / 12) * 2, "°C", i === 1),
        dedicated_used: metric(
          (i ? 0.01 : 5 + Math.sin(n / 10)) * 1073741824,
          "GiB",
        ),
        shared_used: metric((i ? 0.02 : 0.3) * 1073741824, "GiB"),
        memory_total: metric((i ? 0.5 : 16) * 1073741824, "GiB"),
        core_clock: metric(i ? 600 : 2535, "MHz"),
        memory_clock: metric(i ? 2800 : 14001, "MHz"),
      },
    })),
  };
}
