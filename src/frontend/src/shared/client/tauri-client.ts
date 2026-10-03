import { Channel, invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type {
  HistoryDto,
  MonitorBatchDto,
  SettingsDto,
} from "../contracts/monitor";
import { type NetworkControlChange, ObservableClient } from "./monitor-client";
import { acceptBatch, mergeHistory } from "../state/history";

export class TauriMonitorClient extends ObservableClient {
  getAppHistory(range: string, appId: string | null, dayStartMs: number) {
    return invoke<import("../contracts/monitor").AppHistorySnapshotDto>(
      "get_app_history",
      { range, appId, dayStartMs },
    );
  }
  private fontCatalog?: Promise<import("../contracts/monitor").FontCatalogDto>;
  getFontCatalog(refresh = false) {
    if (refresh || !this.fontCatalog) {
      this.fontCatalog = invoke<import("../contracts/monitor").FontCatalogDto>(
        "get_font_catalog",
        { refresh },
      ).catch((error) => {
        this.fontCatalog = undefined;
        throw error;
      });
    }
    return this.fontCatalog;
  }
  getArchiveSnapshot(dayStartMs: number) {
    return invoke<import("../contracts/monitor").ArchiveSnapshotDto>(
      "get_archive_snapshot",
      { dayStartMs },
    );
  }
  getProcessSnapshot(sort: string) {
    return invoke<import("../contracts/monitor").ProcessSnapshotDto>(
      "get_process_snapshot",
      { sort },
    );
  }
  getDiskSnapshot(diskId: string | null) {
    return invoke<import("../contracts/monitor").DiskSnapshotDto>(
      "get_disk_snapshot",
      { diskId },
    );
  }
  prepareDiagnostics() {
    return invoke<import("../contracts/monitor").DiagnosticPreviewDto>(
      "prepare_diagnostics",
    );
  }
  exportDiagnostics(token: string) {
    return invoke<import("../contracts/monitor").DiagnosticExportDto>(
      "export_diagnostics",
      { token },
    );
  }
  async reconnect() {
    if (this.active && this.nativeVisible && !document.hidden)
      await this.connect();
  }
  getHardwareInfo() {
    return invoke<import("../contracts/monitor").HardwareInfoDto>(
      "get_hardware_info",
    );
  }
  temperatureDriverMissing() {
    return invoke<boolean>("temperature_driver_missing");
  }
  installTemperatureDriver() {
    return invoke<string>("install_temperature_driver");
  }
  onDesktopNavigate(listener: (page: string) => void) {
    return listen<string>("desktop-navigate", ({ payload }) =>
      listener(payload),
    );
  }
  requestExit() {
    return invoke<void>("request_app_exit", { confirmed: false });
  }
  private generation = 0;
  private subscription: string | null = null;
  private active = false;
  private nativeVisible = true;
  private lastMessage = 0;
  private retryAt = 0;
  private networkActions = Promise.resolve();
  private ipActions = Promise.resolve();
  private ipWanted = false;
  start() {
    if (!isTauri()) {
      this.publish({
        ...this.snapshot,
        error: "请从 Pinmeter 桌面应用查看真实数据。浏览器未连接采集服务。",
      });
      return () => {};
    }
    this.active = true;
    this.nativeVisible = false;
    const nativeWindow = getCurrentWindow();
    const publishTheme = (nativeTheme: "light" | "dark" | null) => {
      if (this.active) this.publish({ ...this.snapshot, nativeTheme });
    };
    let themeChanged = false;
    const unlistenTheme = nativeWindow.onThemeChanged(({ payload }) => {
      themeChanged = true;
      publishTheme(payload);
    });
    void unlistenTheme
      .then(() => nativeWindow.theme())
      .then((theme) => {
        if (!themeChanged) publishTheme(theme);
      })
      .catch((error) => console.error("Native theme unavailable", error));
    const visibility = () => {
      if (this.snapshot.nativeVisible !== this.nativeVisible)
        this.publish({ ...this.snapshot, nativeVisible: this.nativeVisible });
      if (document.hidden || !this.nativeVisible) {
        this.disconnect();
      } else void this.connect();
    };
    document.addEventListener("visibilitychange", visibility);
    let visibilityEventSeen = false;
    const unlisten = listen<boolean>("monitor-visibility", (event) => {
      visibilityEventSeen = true;
      this.nativeVisible = event.payload;
      visibility();
    });
    void unlisten
      .then(() => nativeWindow.isVisible())
      .then((visible) => {
        if (this.active && !visibilityEventSeen) {
          this.nativeVisible = visible;
          visibility();
        }
      })
      .catch((error) => console.error("无法读取窗口可见状态", error));
    const timer = window.setInterval(() => {
      if (!this.active || document.hidden || !this.nativeVisible) return;
      const timeout = Math.max(
        1000,
        (this.snapshot.state?.settings.interval_ms ?? 1000) * 3,
      );
      if (
        (this.subscription && Date.now() - this.lastMessage >= timeout) ||
        (!this.subscription && Date.now() >= this.retryAt)
      ) {
        this.publish({
          ...this.snapshot,
          connected: false,
          error: "连接中断，正在重新连接…",
        });
        void this.connect();
      }
    }, 1000);
    visibility();
    return () => {
      this.active = false;
      clearInterval(timer);
      document.removeEventListener("visibilitychange", visibility);
      void unlisten.then((release) => release());
      void unlistenTheme.then((release) => release()).catch(() => {});
      this.disconnect();
    };
  }
  async releaseAllNetworkControl(expectedRevision: string) {
    await invoke("release_all_network_control", { expectedRevision });
  }
  private disconnect() {
    ++this.generation;
    const id = this.subscription;
    this.subscription = null;
    if (id)
      void invoke("unsubscribe_monitor", { subscriptionId: id }).catch(
        () => {},
      );
  }
  private async connect() {
    this.disconnect();
    const generation = this.generation;
    this.retryAt = Date.now() + 5000;
    let seq = 0n;
    const channel = new Channel<MonitorBatchDto>();
    let queue = Promise.resolve();
    channel.onmessage = (batch) => {
      queue = queue
        .then(async () => {
          if (generation !== this.generation || !this.active) return;
          if (BigInt(batch.delivery_seq) !== seq + 1n)
            throw new Error("数据批次不连续");
          if (seq === 0n && batch.kind !== "bootstrap")
            throw new Error("缺少初始状态");
          this.subscription = batch.subscription_id;
          let state = acceptBatch(this.snapshot.state, batch);
          if (batch.requires_history_sync) {
            const history = await invoke<HistoryDto>("get_history", {
              after: this.snapshot.state?.history.at(-1)?.cursor ?? "0",
            });
            if (generation !== this.generation) return;
            if (history.session_id !== state.session_id)
              throw new Error("监控会话已变化");
            state = {
              ...state,
              history: mergeHistory(state.history, history.frames),
            };
          }
          seq = BigInt(batch.delivery_seq);
          this.lastMessage = Date.now();
          this.publish({
            ...this.snapshot,
            state,
            connected: true,
            error: null,
            demo: false,
          });
          await invoke("ack_monitor_batch", {
            subscriptionId: batch.subscription_id,
            deliverySeq: batch.delivery_seq,
          });
        })
        .catch((error) => {
          if (generation !== this.generation) return;
          this.disconnect();
          this.publish({
            ...this.snapshot,
            connected: false,
            error: String(error),
          });
        });
    };
    try {
      const id = await invoke<string>("subscribe_monitor", { channel });
      if (generation !== this.generation)
        await invoke("unsubscribe_monitor", { subscriptionId: id });
      else {
        this.subscription = id;
        void this.applyIpActive().catch(() => {});
      }
    } catch (error) {
      if (generation === this.generation)
        this.publish({
          ...this.snapshot,
          connected: false,
          error: String(error),
        });
    }
  }
  async updateSettings(settings: SettingsDto) {
    const session = this.snapshot.state?.session_id;
    const saved = await invoke<SettingsDto>("update_settings", {
      settings,
      expectedRevision: settings.revision,
    });
    const current = this.snapshot.state;
    if (
      current &&
      current.session_id === session &&
      BigInt(saved.revision) >= BigInt(current.settings.revision)
    ) {
      this.publish({
        ...this.snapshot,
        state: { ...current, settings: saved },
      });
    }
    return saved;
  }
  setIpViewActive(active: boolean) {
    this.ipWanted = active;
    return this.applyIpActive();
  }
  private applyIpActive() {
    const active =
      this.ipWanted && this.active && this.nativeVisible && !document.hidden;
    const action = this.ipActions
      .catch(() => {})
      .then(() => invoke<void>("set_ip_view_active", { active }));
    this.ipActions = action;
    return action;
  }
  async refreshIp() {
    await invoke("refresh_ip");
  }
  async refreshIpChecks(section: string) {
    await invoke("refresh_ip_checks", { section });
  }
  async changeNetworkControl(change: NetworkControlChange) {
    await invoke("change_network_control", { ...change });
  }
  async recoverWindow() {
    await invoke("perform_desktop_action", { action: "recover_window" });
  }
  setAppNetworkMonitoring(enabled: boolean) {
    const action = this.networkActions
      .catch(() => {})
      .then(() => invoke<void>("set_app_network_monitoring", { enabled }));
    this.networkActions = action;
    return action;
  }
}
