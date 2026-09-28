import type { ClientSnapshot } from "../../shared/client/monitor-client";
import type { ReadingDto } from "../../shared/contracts/monitor";
import { statusLabels, type Page } from "./useMonitorViewModel";

export interface HealthIssue {
  title: string;
  detail: string;
  page: Page;
}
export function monitorHealth(snapshot: ClientSnapshot) {
  const state = snapshot.state;
  const issues: HealthIssue[] = [];
  let warming = false;
  const check = (
    title: string,
    reading: Pick<ReadingDto, "status" | "detail"> | undefined,
    page: Page,
  ) => {
    if (!reading || reading.status === "warming") warming = true;
    else if (["failed", "stale", "permission_denied"].includes(reading.status))
      issues.push({
        title,
        detail: reading.detail || statusLabels[reading.status],
        page,
      });
  };
  if (snapshot.connected && state) {
    check("CPU 使用率", state.frame?.cpu, "cpu");
    check("物理内存", state.frame?.memory, "memory");
    check("下载速度", state.frame?.download, "network");
    check("上传速度", state.frame?.upload, "network");
    check("CPU 温度", state.cpu_temperature, "cpu");
    check("逻辑处理器", state.cpu_processors, "cpu");
    if (state.gpu.status !== "normal") check("GPU 采集", state.gpu, "gpu");
    else {
      for (const device of state.gpu.devices) {
        check(`${device.name} 使用率`, device.readings.usage, "gpu");
        check(`${device.name} 温度`, device.readings.temperature, "gpu");
      }
    }
    if (
      state.app_network.incomplete ||
      ["failed", "stale", "permission_denied", "incomplete"].includes(
        state.app_network.status,
      )
    )
      issues.push({
        title: "应用流量统计",
        detail: state.app_network.detail,
        page: "network",
      });
    const control = state.network_control;
    if (
      control?.supported &&
      (!control.available ||
        !control.firewall_available ||
        !control.driver_available ||
        control.rules.some((r) => ["failed", "partial"].includes(r.status)))
    )
      issues.push({
        title: "网络限制待核对",
        detail:
          control.detail || "部分限制未确认生效或解除，可查看规则或重新解除。",
        page: "network",
      });
    if (
      state.settings.taskbar.enabled &&
      !state.settings.taskbar.hidden &&
      state.desktop.stage !== "visible"
    )
      issues.push({
        title: "任务栏读数未显示",
        detail: state.desktop.detail,
        page: "settings",
      });
    if (state.diagnostic)
      issues.push({
        title: "采集与配置",
        detail: state.diagnostic,
        page: "settings",
      });
  }
  const restrictions =
    state?.network_control?.rules.filter(
      (r) =>
        r.inbound_blocked ||
        r.outbound_blocked ||
        r.limiting ||
        (r.enabled && (r.blocked || r.download !== null || r.upload !== null)),
    ).length ?? 0;
  return {
    issues,
    restrictions,
    label: !snapshot.connected
      ? "连接中断"
      : issues.length
        ? `${issues.length} 项需关注`
        : warming
          ? "等待采样"
          : "监控正常",
  };
}
