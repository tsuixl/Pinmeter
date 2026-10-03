import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import type { ReadingDto, ReadingStatus } from "../../shared/contracts/monitor";
export type Page =
  | "overview"
  | "hardware"
  | "cpu"
  | "memory"
  | "gpu"
  | "network"
  | "disk"
  | "processes"
  | "ip"
  | "settings";
export const pageLabels: Record<Page, string> = {
  overview: "总览",
  hardware: "硬件信息",
  cpu: "CPU",
  gpu: "GPU",
  memory: "内存",
  network: "网络",
  disk: "磁盘",
  processes: "进程",
  ip: "IP",
  settings: "设置",
};
export const statusLabels: Record<ReadingStatus, string> = {
  normal: "实时",
  warming: "采样中",
  unsupported: "不支持",
  permission_denied: "权限不足",
  failed: "采集失败",
  stale: "数据过期",
};
export const emptyReading: ReadingDto = {
  value: null,
  text: "—",
  unit: "",
  status: "warming",
  valid_at_ms: null,
  source: "等待采集服务",
  semantic: "",
  detail: "",
};
export function useMonitorViewModel(client: MonitorClient) {
  const snapshot = useSyncExternalStore(client.subscribe, client.getSnapshot);
  const [page, setPage] = useState<Page>("overview");
  const [range, setRange] = useState(300_000);
  const [anchor, setAnchor] = useState<number | null>(null);
  const [driverMissing, setDriverMissing] = useState(false);
  const [driverAction, setDriverAction] = useState<
    "checking" | "installing" | null
  >(null);
  const driverPending = driverAction !== null;
  const driverInstalling = driverAction === "installing";
  const [driverMessage, setDriverMessage] = useState("");
  const [driverError, setDriverError] = useState("");
  const driverBusy = useRef(false);
  const checkTemperatureDriver = async () => {
    if (driverBusy.current || !client.temperatureDriverMissing) return;
    driverBusy.current = true;
    setDriverAction("checking");
    setDriverError("");
    try {
      const missing = await client.temperatureDriverMissing();
      setDriverMissing(missing);
      setDriverMessage(
        missing
          ? "尚未检测到 PawnIO，可以点击“下载安装驱动”。"
          : "已检测到 PawnIO，温度采集将在 30 秒内自动重试；部分设备可能需要重启电脑。",
      );
    } catch (error) {
      setDriverError(String(error));
    } finally {
      driverBusy.current = false;
      setDriverAction(null);
    }
  };
  useEffect(() => {
    if (
      page !== "cpu" ||
      !snapshot.connected ||
      !client.temperatureDriverMissing
    )
      return;
    let active = true;
    setDriverMessage("");
    setDriverError("");
    void client
      .temperatureDriverMissing()
      .then((missing) => {
        if (active) setDriverMissing(missing);
      })
      .catch((error) => {
        if (active) setDriverError(String(error));
      });
    return () => {
      active = false;
    };
  }, [client, page, snapshot.connected]);
  const installTemperatureDriver = async () => {
    if (driverBusy.current || !client.installTemperatureDriver) return;
    driverBusy.current = true;
    setDriverAction("installing");
    setDriverError("");
    setDriverMessage("");
    try {
      setDriverMessage(await client.installTemperatureDriver());
      if (client.temperatureDriverMissing)
        setDriverMissing(await client.temperatureDriverMissing());
    } catch (error) {
      setDriverError(String(error));
    } finally {
      driverBusy.current = false;
      setDriverAction(null);
    }
  };
  useEffect(() => client.start(), [client]);
  useEffect(() => {
    let active = true;
    const unlisten = client.onDesktopNavigate?.((page) => {
      if (active && Object.hasOwn(pageLabels, page)) {
        setPage(page as Page);
        setAnchor(null);
      }
    });
    void unlisten?.catch(() => {});
    return () => {
      active = false;
      void unlisten?.then((release) => release()).catch(() => {});
    };
  }, [client]);
  useEffect(() => {
    setAnchor(null);
  }, [snapshot.state?.session_id]);
  const frame = snapshot.state?.frame;
  const temperature = snapshot.state?.cpu_temperature ?? {
    ...emptyReading,
    unit: "°C",
  };
  const reading = (
    key: "cpu" | "cpu_temperature" | "memory" | "download" | "upload",
  ) => {
    const value = frame?.[key] ?? emptyReading;
    return snapshot.connected
      ? value
      : {
          ...value,
          status: (frame ? "stale" : "failed") as ReadingStatus,
          text: "—",
          value: null,
        };
  };
  return {
    ...snapshot,
    driverMissing,
    driverPending,
    driverInstalling,
    driverMessage,
    driverError,
    installTemperatureDriver,
    checkTemperatureDriver,
    cpuModel: snapshot.state?.cpu_model?.trim() || "CPU 型号未知",
    page,
    range,
    anchor,
    setPage,
    setRange,
    setAnchor,
    reading,
    temperature: snapshot.connected
      ? temperature
      : {
          ...temperature,
          status: "stale" as const,
          value: null,
          text: "—",
          detail: "采集服务未连接",
        },
    processors: (snapshot.state?.cpu_processors.processors ?? []).map(
      (processor) => ({
        ...processor,
        usage: snapshot.connected
          ? processor.usage
          : {
              ...processor.usage,
              status: "stale" as const,
              text: "—",
              value: null,
              detail: "采集服务未连接",
            },
      }),
    ),
    processorsStatus: snapshot.connected
      ? (snapshot.state?.cpu_processors.status ?? "warming")
      : ("failed" as ReadingStatus),
    processorsDetail: snapshot.connected
      ? (snapshot.state?.cpu_processors.detail ?? "等待逻辑处理器采样")
      : "采集服务未连接",
    history: snapshot.state?.history ?? [],
    frame,
    networkName: snapshot.state?.selected_interface?.name ?? "暂无可用网卡",
  };
}
