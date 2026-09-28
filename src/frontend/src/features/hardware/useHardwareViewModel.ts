import { useEffect, useState } from "react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import type {
  HardwareInfoDto,
  HardwareItemDto,
  MonitorStateDto,
} from "../../shared/contracts/monitor";

const categories = [
  ["board", "主板"],
  ["cpu", "处理器"],
  ["memory", "内存"],
  ["gpu", "显卡"],
  ["display", "显示器"],
  ["disk", "硬盘"],
  ["audio", "音频设备"],
  ["network", "网卡"],
] as const;

export function summarizeHardware(id: string, items: HardwareItemDto[]) {
  const join = (values: string[]) =>
    [...new Set(values.filter(Boolean))].join(" / ");
  if (id === "memory") {
    const total = items.find((item) => item.name.startsWith("已安装 "));
    const modules = items.filter((item) => item !== total);
    const brands = join(modules.map((item) => item.name.split(/\s+/)[0]));
    const types = join(
      modules.flatMap((item) =>
        [item.name, ...item.details].flatMap(
          (detail) => detail.match(/\b(?:LP)?DDR\d\b/)?.[0] ?? [],
        ),
      ),
    );
    const speeds = join(
      modules.flatMap((item) =>
        item.details.flatMap(
          (detail) => detail.match(/^配置速率 (\d+)/)?.[1] ?? [],
        ),
      ),
    );
    const slots = total?.details
      .find((detail) => detail.includes("插槽"))
      ?.replace(/\s*\/\s*/g, "/")
      .replace("已使用", "");
    return (
      [
        brands,
        total?.name.replace(/^已安装 /, ""),
        types && speeds
          ? `${types}-${speeds}`
          : types || (speeds ? `配置速率 ${speeds}` : ""),
        slots ? `(${slots})` : "",
      ]
        .filter(Boolean)
        .join(" ") || join(items.map((item) => item.name))
    );
  }
  const physical = items.filter(
    (item) =>
      !item.details.some((detail) => detail.includes("虚拟")) &&
      !/virtual/i.test(item.name) &&
      !(id === "network" && /bluetooth/i.test(item.name)),
  );
  const selected =
    ["gpu", "network", "audio"].includes(id) && physical.length
      ? physical
      : items;
  return selected
    .map((item) => {
      const capacity = item.details.find((detail) =>
        id === "gpu"
          ? detail.startsWith("显存容量 ")
          : id === "disk" && /^\d.* GiB$/.test(detail),
      );
      if (capacity)
        return `${item.name} (${capacity.replace("显存容量 ", "")})`;
      if (id === "display") {
        const resolution = item.details.find((detail) =>
          detail.startsWith("首选 "),
        );
        return resolution ? `${item.name} (${resolution})` : item.name;
      }
      return item.name;
    })
    .join(" / ");
}

export function formatUptime(boot: number | null | undefined, now: number) {
  if (!boot || boot > now) return "—";
  const minutes = Math.floor((now - boot) / 60_000);
  if (minutes < 1) return "不足 1 分钟";
  const days = Math.floor(minutes / 1440);
  const hours = Math.floor((minutes % 1440) / 60);
  return `${days ? `${days} 天 ` : ""}${hours} 小时 ${minutes % 60} 分钟`;
}

export function useHardwareViewModel(
  client: MonitorClient,
  state: MonitorStateDto | null,
  connected: boolean,
) {
  const [info, setInfo] = useState<HardwareInfoDto | null>(null);
  const [error, setError] = useState("");
  const [copyState, setCopyState] = useState("");
  const [expanded, setExpanded] = useState(false);
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    let active = true;
    let timer: ReturnType<typeof setTimeout> | undefined;
    setInfo(null);
    setError("");
    if (!client.getHardwareInfo) {
      setError("此环境暂未提供硬件信息");
      return;
    }
    const read = async () => {
      try {
        const result = await client.getHardwareInfo!();
        if (!active) return;
        setInfo(result);
        if (result.status === "loading")
          timer = setTimeout(() => void read(), 500);
      } catch (reason) {
        if (active) setError(String(reason));
      }
    };
    void read();
    return () => {
      active = false;
      clearTimeout(timer);
    };
  }, [client]);
  useEffect(() => {
    const timer = setInterval(() => {
      if (!document.hidden) setNow(Date.now());
    }, 10_000);
    return () => clearInterval(timer);
  }, []);

  const loading = !error && (!info || info.status === "loading");
  const issue =
    error ||
    (info?.status === "failed" || info?.status === "unsupported"
      ? info.detail
      : "");
  const sections = categories.map(([id, title]) => {
    const section = info?.sections.find((entry) => entry.id === id);
    let items: HardwareItemDto[] =
      section?.items.map((item) => ({ ...item, details: [...item.details] })) ??
      [];
    if (id === "cpu" && state?.cpu_model) {
      if (!items.length) items = [{ name: state.cpu_model, details: [] }];
      else if (items.length === 1) items[0].name = state.cpu_model;
    }
    if (id === "gpu") {
      for (const gpu of state?.gpu.devices ?? []) {
        let item = items.find(
          (entry) =>
            entry.name.trim().toLowerCase() === gpu.name.trim().toLowerCase(),
        );
        if (!item) {
          item = { name: gpu.name, details: [] };
          items.push(item);
        }
        const capacity = gpu.readings.memory_total;
        if (
          connected &&
          capacity?.status === "normal" &&
          capacity.value !== null
        ) {
          item.details.unshift(
            `显存容量 ${(capacity.value / 1024 ** 3).toLocaleString("zh-CN", { maximumFractionDigits: 1 })} GiB`,
          );
        }
      }
    }
    return {
      id,
      title,
      items,
      summary: summarizeHardware(id, items),
      error: section?.error,
      empty: loading ? "正在读取…" : issue ? "暂不可用" : "系统未提供",
    };
  });
  const board = sections.find((section) => section.id === "board")?.items[0]
    ?.name;
  const device = info?.model || info?.manufacturer || "此电脑";
  const deviceDetail = info?.model
    ? info.manufacturer
    : board || (loading ? "正在读取设备信息…" : "整机型号未提供");
  const uptime = formatUptime(info?.boot_at_ms, now);
  const bootLabel = info?.boot_at_ms
    ? `启动于 ${new Date(info.boot_at_ms).toLocaleString("zh-CN", { hour12: false })}`
    : "系统启动时间未提供";
  const copy = async () => {
    const text = [
      "Pinmeter · 硬件信息",
      `设备：${device}${expanded && deviceDetail ? ` · ${deviceDetail}` : ""}`,
      `系统：${info?.system ?? "未提供"}${expanded && info?.system_detail ? ` ${info.system_detail}` : ""}`,
      `运行时间：${uptime}`,
      ...sections.map(
        (section) =>
          `${section.title}：${section.items.length ? (expanded ? section.items.map((item) => [item.name, ...item.details].join(" · ")).join("\n  ") : section.summary) : section.error || section.empty}`,
      ),
    ].join("\n");
    try {
      await navigator.clipboard.writeText(text);
      setCopyState("已复制硬件信息");
    } catch {
      setCopyState("复制失败，请重试");
    }
  };
  return {
    info,
    loading,
    issue,
    sections,
    device,
    deviceDetail,
    uptime,
    bootLabel,
    copy,
    copyState,
    expanded,
    toggleExpanded: () => setExpanded((value) => !value),
  };
}
