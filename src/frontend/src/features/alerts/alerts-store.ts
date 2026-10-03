import type { MonitorClient } from "../../shared/client/monitor-client";
import type {
  AlertsConfigDto,
  AlertsSnapshotDto,
} from "../../shared/contracts/monitor";

interface AlertsState {
  data: AlertsSnapshotDto | null;
  loading: boolean;
  pending: boolean;
  error: string;
}

// One read-only cache per client. Main banner and settings share a single timer;
// confirmed configuration and event ownership remain in the backend.
export class AlertsStore {
  private state: AlertsState = {
    data: null,
    loading: true,
    pending: false,
    error: "",
  };
  private listeners = new Set<() => void>();
  private timer: ReturnType<typeof setInterval> | undefined;
  private unsubscribeClient: (() => void) | undefined;
  private readGeneration = 0;
  private reading = false;
  private alive = false;
  private mutationError = false;
  private visible = true;
  constructor(private readonly client: MonitorClient) {}
  getSnapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    if (this.listeners.size === 1) this.start();
    return () => {
      this.listeners.delete(listener);
      if (this.listeners.size === 0) this.stop();
    };
  };
  private publish(patch: Partial<AlertsState>) {
    this.state = { ...this.state, ...patch };
    this.listeners.forEach((listener) => listener());
  }
  private isVisible() {
    return (
      (typeof document === "undefined" || !document.hidden) &&
      this.client.getSnapshot().nativeVisible !== false
    );
  }
  private onVisibility = () => {
    const visible = this.isVisible();
    if (visible && !this.visible) void this.refresh();
    this.visible = visible;
  };
  private start() {
    this.alive = true;
    this.visible = this.isVisible();
    this.unsubscribeClient = this.client.subscribe(this.onVisibility);
    if (typeof document !== "undefined")
      document.addEventListener("visibilitychange", this.onVisibility);
    this.timer = setInterval(() => void this.refresh(), 5000);
    void this.refresh();
  }
  private stop() {
    this.alive = false;
    this.readGeneration++;
    this.reading = false;
    clearInterval(this.timer);
    this.unsubscribeClient?.();
    if (typeof document !== "undefined")
      document.removeEventListener("visibilitychange", this.onVisibility);
  }
  refresh = async () => {
    if (!this.alive || !this.isVisible() || this.reading || this.state.pending)
      return;
    if (!this.client.getAlertsSnapshot) {
      this.publish({ loading: false, error: "当前宿主不支持占用提醒" });
      return;
    }
    this.reading = true;
    const generation = ++this.readGeneration;
    try {
      const data = await this.client.getAlertsSnapshot();
      if (this.alive && generation === this.readGeneration)
        this.publish({
          data,
          error: this.mutationError ? this.state.error : "",
          loading: false,
        });
    } catch (error) {
      if (this.alive && generation === this.readGeneration)
        this.publish({ error: String(error), loading: false });
    } finally {
      if (generation === this.readGeneration) this.reading = false;
    }
  };
  private async mutate(action: () => Promise<AlertsSnapshotDto>) {
    if (this.state.pending) return false;
    this.readGeneration++;
    this.reading = false;
    this.mutationError = false;
    this.publish({ pending: true, error: "" });
    try {
      const data = await action();
      this.publish({ data, loading: false, pending: false });
      return true;
    } catch (error) {
      this.mutationError = true;
      this.publish({ error: String(error), pending: false });
      return false;
    }
  }
  save = (config: AlertsConfigDto, expectedRevision: string) =>
    this.mutate(async () => {
      if (!this.client.updateAlerts)
        throw new Error("当前宿主无法保存占用提醒");
      return this.client.updateAlerts(config, expectedRevision);
    });
  acknowledge = (id: string) =>
    this.mutate(async () => {
      if (!this.client.acknowledgeAlert)
        throw new Error("当前宿主无法确认提醒");
      return this.client.acknowledgeAlert(id);
    });
  clear = (throughId: string) =>
    this.mutate(async () => {
      if (!this.client.clearAlertEvents)
        throw new Error("当前宿主无法清除提醒");
      return this.client.clearAlertEvents(throughId);
    });
}

const stores = new WeakMap<MonitorClient, AlertsStore>();
export function alertsStore(client: MonitorClient) {
  let store = stores.get(client);
  if (!store) {
    store = new AlertsStore(client);
    stores.set(client, store);
  }
  return store;
}
