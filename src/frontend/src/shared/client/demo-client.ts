// Loaded only by the development entry with ?demo=1. Never a production fallback.
import type {
  FrameDto,
  ReadingDto,
  ReadingStatus,
  SettingsDto,
  AppNetworkDto,
} from "../contracts/monitor";
import { type NetworkControlChange, ObservableClient } from "./monitor-client";
import { mergeHistory } from "../state/history";
import { demoGpu } from "./demo-gpu";
import { demoIp } from "./demo-ip";
import { demoHardware } from "./demo-hardware";
import { demoDisk } from "./demo-disk";
import { demoArchive } from "./demo-history";
import { demoAppHistory } from "./demo-app-history";
import { builtinFontCatalog } from "../fonts";
import { defaultAlertsConfig } from "./alert-defaults";
export class DemoClient extends ObservableClient {
  private alertEvents: import("../contracts/monitor").AlertEventDto[] =
    typeof location !== "undefined" &&
    new URLSearchParams(location.search).get("alerts") === "events"
      ? [
          {
            id: "1",
            metric: "cpu",
            at_ms: Date.now() - 60000,
            value: 96,
            threshold_percent: 90,
            duration_seconds: 30,
            acknowledged: false,
          },
        ]
      : [];
  private alertRevision = 0;
  async getAlertsSnapshot(): Promise<
    import("../contracts/monitor").AlertsSnapshotDto
  > {
    return {
      config: this.snapshot.state?.settings.alerts ?? defaultAlertsConfig(),
      settings_revision: this.snapshot.state?.settings.revision ?? "0",
      revision: String(this.alertRevision),
      events: this.alertEvents,
      unread_count: this.alertEvents.filter((e) => !e.acknowledged).length,
      local_time_available: true,
      quiet_now: false,
      delivery_detail:
        "演示提醒：未写入文件，不发送系统弹窗。事件退出后清空，最多 20 条。",
    };
  }
  async updateAlerts(
    config: import("../contracts/monitor").AlertsConfigDto,
    expectedRevision: string,
  ) {
    if (
      typeof location !== "undefined" &&
      new URLSearchParams(location.search).get("alerts") === "failed"
    )
      throw new Error("演示：无法写入提醒设置，已确认规则保持不变");
    const state = this.snapshot.state;
    if (!state || state.settings.revision !== expectedRevision)
      throw new Error("设置版本冲突，请重新载入后编辑");
    await this.updateSettings({ ...state.settings, alerts: config });
    return this.getAlertsSnapshot();
  }
  async acknowledgeAlert(id: string) {
    this.alertEvents = this.alertEvents.map((event) =>
      event.id === id ? { ...event, acknowledged: true } : event,
    );
    this.alertRevision++;
    return this.getAlertsSnapshot();
  }
  async clearAlertEvents(throughId: string) {
    this.alertEvents = this.alertEvents.filter(
      (event) => BigInt(event.id) > BigInt(throughId),
    );
    this.alertRevision++;
    return this.getAlertsSnapshot();
  }
  private traceStarted = 0;
  private traceDuration = 60;
  private traceStopped = 0;
  async getTraceSnapshot(): Promise<
    import("../contracts/monitor").TraceSnapshotDto
  > {
    const elapsed = this.traceStarted
      ? Math.min(
          this.traceDuration,
          ((this.traceStopped || Date.now()) - this.traceStarted) / 1000,
        )
      : 0;
    const samples = Math.floor(elapsed / 2);
    const active =
      !!this.traceStarted && !this.traceStopped && elapsed < this.traceDuration;
    return {
      token: String(this.traceStarted),
      active,
      remaining_seconds: active ? Math.ceil(this.traceDuration - elapsed) : 0,
      samples,
      content: this.traceStarted
        ? JSON.stringify(
            {
              demo: true,
              description: "演示记录，不代表本机实际读数",
              duration_seconds: elapsed,
              observations: Array.from({ length: samples }, (_, i) => ({
                at_ms: this.traceStarted + i * 2000,
                cpu: 22 + (i % 5),
                memory: 45,
                top_cpu_processes: [{ name: "浏览器.exe", pid: 2048, cpu: 12 }],
              })),
            },
            null,
            2,
          )
        : "",
    };
  }
  async startTrace(seconds: number) {
    this.traceStarted = Date.now();
    this.traceStopped = 0;
    this.traceDuration = seconds;
    return this.getTraceSnapshot();
  }
  async stopTrace() {
    this.traceStopped = Date.now();
    return this.getTraceSnapshot();
  }
  async exportTrace(_token: string) {
    return { saved: false, path: null };
  }
  private appHistoryFrom: number | null =
    typeof location !== "undefined" &&
    new URLSearchParams(location.search).get("appHistory") === "empty"
      ? null
      : Date.now() - 86400_000;
  private appHistoryThrough = Date.now() - 60_000;
  async getAppHistory(range: string, appId: string | null, dayStartMs: number) {
    if (
      typeof location !== "undefined" &&
      new URLSearchParams(location.search).get("appHistory") === "failed"
    )
      throw new Error("演示：本地应用历史暂不可用");
    if (this.networkEnabled) this.appHistoryThrough = Date.now();
    // The older network demo uses short IDs; keep its fixtures compatible while
    // resolving the same executable in the history demo for detail navigation.
    const aliases: Record<string, string> = {
      "app-0": "app:c:\\program files\\browser\\浏览器.exe",
      "app-1": "app:c:\\tools\\下载器.exe",
      "app-2": "app:c:\\program files\\drive\\云盘.exe",
    };
    const resolved = appId ? (aliases[appId] ?? appId) : null;
    const result = demoAppHistory(
      range,
      resolved,
      dayStartMs,
      this.appHistoryFrom,
      this.appHistoryThrough,
      this.status !== "normal",
    );
    return {
      ...result,
      selected_id: appId,
      rows: result.rows.map((row) =>
        row.id === resolved && appId ? { ...row, id: appId } : row,
      ),
    };
  }
  async getFontCatalog() {
    return builtinFontCatalog;
  }
  private basicHistoryClearedAt = 0;
  async getArchiveSnapshot(dayStartMs: number, rangeMs = 86400000) {
    return demoArchive(dayStartMs, rangeMs, this.basicHistoryClearedAt);
  }
  async getHistoryStorage() {
    return {
      basic_bytes: this.basicHistoryClearedAt ? "128" : "624384",
      applications_bytes: this.appHistoryFrom ? "3124820" : "128",
      basic_limit_bytes: "83886080",
      applications_limit_bytes: "33554432",
      detail: "演示数据，不读取本机文件。",
    };
  }
  async clearHistory(scope: "basic" | "applications") {
    if (scope === "basic") this.basicHistoryClearedAt = Date.now();
    else this.appHistoryFrom = this.networkEnabled ? Date.now() : null;
  }
  async exportHistory(
    _scope: "basic" | "applications",
    _fromMs: number,
    _throughMs: number,
    _includePaths: boolean,
  ): Promise<import("../contracts/monitor").HistoryExportDto> {
    throw new Error("浏览器演示不生成本机历史文件；请在桌面应用确认导出。");
  }
  async getProcessSnapshot(
    sort: string,
  ): Promise<import("../contracts/monitor").ProcessSnapshotDto> {
    const status = this.getSnapshot().state?.frame?.cpu.status ?? "normal";
    const rows: import("../contracts/monitor").ProcessRowDto[] =
      status === "normal"
        ? Array.from({ length: 16 }, (_, i) => ({
            id: `demo-${i}`,
            application_id: `demo-app-${i % 6}`,
            pid: 2048 + i * 4,
            name: [
              "浏览器.exe",
              "编辑器.exe",
              "Pinmeter.exe",
              "文件管理器.exe",
              "终端.exe",
              "浏览器.exe",
            ][i % 6],
            cpu: 12 / (i + 1),
            cpu_status: "normal",
            working_set: 500_000_000 / (i + 1),
            memory_status: "normal",
          }))
        : [];
    return {
      sort,
      status,
      detail: "演示进程数据",
      sampled_at_ms: Date.now(),
      total: rows.length,
      unreadable: 3,
      truncated: false,
      rows,
      applications: Array.from(new Set(rows.map((r) => r.application_id))).map(
        (id) => {
          const members = rows.filter((r) => r.application_id === id);
          return {
            id,
            name: members[0].name,
            cpu: members.reduce((sum, r) => sum + (r.cpu ?? 0), 0),
            cpu_status: "normal",
            working_set: members.reduce(
              (sum, r) => sum + (r.working_set ?? 0),
              0,
            ),
            memory_status: "normal",
            process_count: members.length,
          };
        },
      ),
    };
  }
  async getDiskSnapshot(id: string | null) {
    return demoDisk(
      id,
      this.getSnapshot().state?.frame?.cpu.status ?? "normal",
    );
  }
  async prepareDiagnostics() {
    const generated_at_ms = Date.now();
    return {
      token: String(generated_at_ms),
      generated_at_ms,
      content: JSON.stringify(
        {
          demo: true,
          application_version: "0.1.3",
          generated_at_ms,
          system: { os: "windows", architecture: "x86_64" },
          readings: [
            { metric: "cpu", status: "normal" },
            { metric: "cpu_temperature", status: "unsupported" },
          ],
          privacy: "演示快照，不包含路径、IP 或进程名单",
        },
        null,
        2,
      ),
    };
  }
  async exportDiagnostics(_token: string) {
    return { saved: false, path: null };
  }
  private hardware = demoHardware();
  async getHardwareInfo() {
    return this.hardware;
  }
  private status: ReadingStatus = "normal";
  private cursor = 300;
  private networkEnabled = false;
  private startAt = Date.now() - 300_000;
  constructor() {
    super();
    const adapter = {
      id: "demo-adapter",
      name: "Wi-Fi（演示）",
      up: true,
      physical: true,
    };
    const history = Array.from({ length: 300 }, (_, i) => this.frame(i + 1));
    this.snapshot = {
      connected: true,
      demo: true,
      error: null,
      state: {
        autostart: {
          available: true,
          enabled: false,
          detail: "演示环境，不修改系统启动项",
        },
        desktop: {
          supported: true,
          stage: "disabled",
          detail: "演示环境，不嵌入系统任务栏",
          revision: "0",
        },
        network_control: {
          supported: true,
          revision: "0",
          version: "0",
          available: true,
          driver_available: true,
          firewall_available: true,
          detail: "演示，不修改系统网络",
          rules: [],
        },
        ip: demoIp(),
        cpu_model: "AMD Ryzen 9 9950X3D（演示）",
        protocol_version: 1,
        session_id: "demo-session",
        platform: "演示环境",
        applied_settings_revision: "0",
        capabilities_version: "0",
        settings: {
          close_action: "ask",
          autostart: false,
          start_in_tray: false,
          record_app_traffic_on_start: false,
          alerts: defaultAlertsConfig(),
          onboarding_completed: false,
          revision: "0",
          theme: "system",
          font_family: "harmonyos_sans_sc",
          font_style: "auto",
          interval_ms: 1000,
          network_id: null,
          release_network_on_exit: true,
          taskbar: {
            enabled: false,
            hidden: false,
            layout: "double",
            cpu: true,
            gpu: true,
            memory: true,
            network: true,
            cpu_temperature: true,
            gpu_temperature: true,
            gpu_id: null,
            order: ["network", "cpu", "gpu", "memory"],
          },
        },
        interfaces: [adapter],
        selected_interface: adapter,
        capabilities: [],
        diagnostic: null,
        frame: history.at(-1)!,
        cpu_processors: this.processors(),
        cpu_temperature: this.temperature(),
        gpu: demoGpu(this.status, this.cursor, Date.now()),
        app_network: this.appNetwork(),
        history,
      },
    };
  }
  async changeNetworkControl(change: NetworkControlChange) {
    const state = this.snapshot.state!;
    const control = state.network_control!;
    if (control.revision !== change.expectedRevision)
      throw new Error("规则已变化，请重新打开菜单");
    const old = control.rules.find((r) => r.id === change.id);
    const app = state.app_network.apps.find((a) => a.id === change.id);
    if (!old && !app) throw new Error("应用不在演示列表中");
    const rule = old
      ? { ...old }
      : {
          id: app!.id,
          name: app!.name,
          path: app!.path,
          download: null as number | null,
          upload: null as number | null,
          blocked: false,
          enabled: true,
          status: "applied",
          detail: "演示规则，不修改系统网络",
          inbound_blocked: false,
          outbound_blocked: false,
          limiting: false,
        };
    if (change.action === "limits") {
      rule.download = change.download;
      rule.upload = change.upload;
    }
    if (change.action === "block") {
      rule.blocked = true;
      rule.enabled = true;
    }
    if (change.action === "enable") rule.enabled = true;
    if (change.action === "disable") rule.enabled = false;
    if (change.action === "restore") rule.blocked = false;
    rule.status = rule.enabled ? "applied" : "disabled";
    rule.detail = "演示规则，不修改系统网络";
    rule.inbound_blocked = rule.outbound_blocked = rule.enabled && rule.blocked;
    rule.limiting =
      rule.enabled &&
      !rule.blocked &&
      (rule.download !== null || rule.upload !== null);
    const rules = control.rules.filter((r) => r.id !== rule.id);
    if (
      change.action !== "clear" &&
      (rule.blocked || rule.download !== null || rule.upload !== null)
    )
      rules.push(rule);
    const revision = String(Number(control.revision) + 1);
    this.publish({
      ...this.snapshot,
      state: {
        ...state,
        network_control: { ...control, revision, version: revision, rules },
      },
    });
  }
  async releaseAllNetworkControl(expectedRevision: string) {
    const state = this.snapshot.state!;
    const control = state.network_control!;
    if (control.revision !== expectedRevision)
      throw new Error("规则已变化，请重试");
    this.publish({
      ...this.snapshot,
      state: {
        ...state,
        network_control: {
          ...control,
          revision: String(BigInt(control.revision) + 1n),
          version: String(BigInt(control.version) + 1n),
          rules: control.rules.map((rule) => ({
            ...rule,
            enabled: false,
            status: "disabled",
            inbound_blocked: false,
            outbound_blocked: false,
            limiting: false,
          })),
        },
      },
    });
  }
  private temperature(): ReadingDto {
    const value = 55 + Math.sin(this.cursor / 9) * 7;
    return {
      value: this.status === "normal" ? value : null,
      text: this.status === "normal" ? value.toFixed(1) : "—",
      unit: "°C",
      status: this.status,
      valid_at_ms: this.status === "normal" ? Date.now() : null,
      source: "演示数据",
      semantic: "cpu.package_die_or_core_max_celsius",
      detail: this.status === "normal" ? "" : "开发用温度状态样例",
    };
  }
  async setIpViewActive(_active: boolean) {}
  async refreshIpChecks(_section: string) {
    await this.refreshIp();
  }
  async refreshIp() {
    if (this.snapshot.state)
      this.publish({
        ...this.snapshot,
        state: { ...this.snapshot.state, ip: demoIp() },
      });
  }
  private appNetwork(): AppNetworkDto {
    const traffic = (
      download: number,
      upload: number,
      share: number | null,
    ) => ({
      download:
        this.networkEnabled && this.status === "normal" ? download : null,
      upload: this.networkEnabled && this.status === "normal" ? upload : null,
      received: String(Math.round(download * 35)),
      sent: String(Math.round(upload * 35)),
      download_share: this.status === "normal" ? share : null,
      upload_share: this.status === "normal" ? share : null,
    });
    return {
      session: this.networkEnabled ? "1" : "0",
      running: this.networkEnabled,
      status: this.networkEnabled ? this.status : "disabled",
      detail: this.networkEnabled
        ? "演示数据，不代表真实应用流量"
        : "开始后查看演示应用流量",
      incomplete: false,
      limited: false,
      apps: this.networkEnabled
        ? ["浏览器（演示）", "下载工具（演示）", "云盘（演示）"].map(
            (name, index) => ({
              id: `app-${index}`,
              name,
              icon: null,
              path: `C:/Demo/app-${index}.exe`,
              traffic: traffic(
                [600000, 300000, 100000][index],
                [60000, 30000, 10000][index],
                [60, 30, 10][index],
              ),
              processes: [0, 1].map((i) => ({
                id: `${index}-${i}`,
                pid: 1000 + index * 10 + i,
                observed: true,
                traffic: traffic(
                  [300000, 150000, 50000][index],
                  [30000, 15000, 5000][index],
                  null,
                ),
              })),
            }),
          )
        : [],
      unknown: traffic(0, 0, null),
      other: traffic(0, 0, null),
    };
  }
  async setAppNetworkMonitoring(enabled: boolean) {
    if (enabled && this.appHistoryFrom === null)
      this.appHistoryFrom = Date.now();
    this.networkEnabled = enabled;
    this.publish({
      ...this.snapshot,
      state: { ...this.snapshot.state!, app_network: this.appNetwork() },
    });
  }
  private processors() {
    return {
      status: "normal" as const,
      detail: "",
      processors: Array.from({ length: 16 }, (_, index) => {
        const value = 35 + Math.sin(this.cursor / 8 + index) * 30;
        return {
          id: `0,${index}`,
          usage: {
            value: this.status === "normal" ? value : null,
            text: this.status === "normal" ? value.toFixed(1) : "—",
            unit: "%",
            status: this.status,
            valid_at_ms: this.status === "normal" ? Date.now() : null,
            source: "演示数据",
            semantic: "cpu.logical_processor.busy_time",
            detail: this.status === "normal" ? "" : "开发用状态样例",
          },
        };
      }),
    };
  }
  private frame(n: number): FrameDto {
    const at = this.startAt + n * 1000;
    const reading = (
      value: number,
      unit: string,
      semantic: string,
    ): ReadingDto => ({
      value: this.status === "normal" ? value : null,
      text: this.status === "normal" ? value.toFixed(1) : "—",
      unit,
      status: this.status,
      valid_at_ms: this.status === "normal" ? at : null,
      source: "演示数据",
      semantic,
      detail: this.status === "normal" ? "" : "开发用状态样例",
    });
    const frame: FrameDto = {
      gpus: demoGpu(n >= 160 && n <= 172 ? "failed" : this.status, n, at)
        .devices,
      cursor: String(n),
      at_ms: at,
      elapsed_ms: n * 1000,
      generation: "0",
      network_generation: "0",
      network_id: "demo-adapter",
      cpu: reading(
        20 + Math.sin(n / 12) * 8 + Math.sin(n / 3) * 3,
        "%",
        "cpu.busy_time",
      ),
      memory: reading(46 + Math.sin(n / 30) * 2, "%", "memory.physical"),
      cpu_temperature: reading(
        55 + Math.sin(n / 9) * 7,
        "°C",
        "cpu.package_die_or_core_max_celsius",
      ),
      download: reading(
        1.6 + Math.sin(n / 11) * 0.9,
        "MB/s",
        "network.bytes_per_second",
      ),
      upload: reading(
        110 + Math.sin(n / 14) * 70,
        "KB/s",
        "network.bytes_per_second",
      ),
      memory_used: "7.4 GiB",
      memory_total: "16.0 GiB",
    };
    if (frame.download.value !== null) frame.download.value *= 1e6;
    if (frame.upload.value !== null) frame.upload.value *= 1000;
    if (n >= 160 && n <= 172)
      for (const key of [
        "cpu",
        "cpu_temperature",
        "memory",
        "download",
        "upload",
      ] as const)
        frame[key] = {
          ...frame[key],
          value: null,
          text: "—",
          status: "failed",
        };
    return frame;
  }
  start() {
    const timer = window.setInterval(() => {
      if (document.hidden || !this.snapshot.state) return;
      const frame = this.frame(++this.cursor);
      this.publish({
        ...this.snapshot,
        state: {
          ...this.snapshot.state,
          frame,
          cpu_processors: this.processors(),
          cpu_temperature: this.temperature(),
          gpu: demoGpu(this.status, this.cursor, Date.now()),
          app_network: this.appNetwork(),
          history: mergeHistory(this.snapshot.state.history, [frame]),
        },
      });
    }, 1000);
    return () => clearInterval(timer);
  }
  setScenario(status: ReadingStatus) {
    this.status = status;
    const frame = this.frame(++this.cursor);
    this.publish({
      ...this.snapshot,
      state: {
        ...this.snapshot.state!,
        frame,
        cpu_processors: this.processors(),
        cpu_temperature: this.temperature(),
        gpu: demoGpu(this.status, this.cursor, Date.now()),
        app_network: this.appNetwork(),
        history: mergeHistory(this.snapshot.state!.history, [frame]),
      },
    });
  }
  async updateSettings(settings: SettingsDto) {
    if (settings.revision !== this.snapshot.state!.settings.revision)
      throw new Error("配置版本已变化，请刷新后重试");
    this.publish({
      ...this.snapshot,
      state: {
        ...this.snapshot.state!,
        settings: {
          ...settings,
          revision: String(BigInt(settings.revision) + 1n),
        },
        applied_settings_revision: String(BigInt(settings.revision) + 1n),
        autostart: {
          available: true,
          enabled: settings.autostart,
          detail: settings.autostart
            ? "已开启自启（演示，不修改系统启动项）"
            : "未开启开机自启（演示）",
        },
        desktop: {
          supported: true,
          stage: settings.taskbar.enabled ? "preview" : "disabled",
          detail: settings.taskbar.enabled
            ? "演示预览，不会嵌入系统任务栏"
            : "任务栏显示已关闭（演示）",
          revision: String(BigInt(settings.revision) + 1n),
        },
      },
    });
    return this.snapshot.state!.settings;
  }
  async recoverWindow() {}
}
