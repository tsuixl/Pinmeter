import type {
  MonitorStateDto,
  SettingsDto,
  ReadingStatus,
} from "../contracts/monitor";

export interface ClientSnapshot {
  state: MonitorStateDto | null;
  connected: boolean;
  error: string | null;
  demo: boolean;
  nativeTheme?: "light" | "dark" | null;
}
export interface NetworkControlChange {
  id: string;
  action: "limits" | "block" | "restore" | "clear" | "enable" | "disable";
  download: number | null;
  upload: number | null;
  expectedRevision: string;
}
export interface MonitorClient {
  getArchiveSnapshot?(
    dayStartMs: number,
  ): Promise<import("../contracts/monitor").ArchiveSnapshotDto>;
  getProcessSnapshot?(
    sort: string,
  ): Promise<import("../contracts/monitor").ProcessSnapshotDto>;
  getDiskSnapshot?(
    diskId: string | null,
  ): Promise<import("../contracts/monitor").DiskSnapshotDto>;
  prepareDiagnostics?(): Promise<
    import("../contracts/monitor").DiagnosticPreviewDto
  >;
  exportDiagnostics?(
    token: string,
  ): Promise<import("../contracts/monitor").DiagnosticExportDto>;
  reconnect?(): Promise<void>;
  getHardwareInfo?(): Promise<import("../contracts/monitor").HardwareInfoDto>;
  temperatureDriverMissing?(): Promise<boolean>;
  installTemperatureDriver?(): Promise<string>;
  onDesktopNavigate?(listener: (page: string) => void): Promise<() => void>;
  requestExit?(): Promise<void>;
  releaseAllNetworkControl?(expectedRevision: string): Promise<void>;
  changeNetworkControl?(change: NetworkControlChange): Promise<void>;
  getSnapshot(): ClientSnapshot;
  subscribe(listener: () => void): () => void;
  start(): () => void;
  updateSettings(settings: SettingsDto): Promise<SettingsDto>;
  recoverWindow(): Promise<void>;
  setAppNetworkMonitoring?(enabled: boolean): Promise<void>;
  setIpViewActive?(active: boolean): Promise<void>;
  refreshIp?(): Promise<void>;
  refreshIpChecks?(section: string): Promise<void>;
  setScenario?(status: ReadingStatus): void;
}
export abstract class ObservableClient implements MonitorClient {
  protected snapshot: ClientSnapshot = {
    state: null,
    connected: false,
    error: null,
    demo: false,
  };
  private listeners = new Set<() => void>();
  getSnapshot = () => this.snapshot;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };
  protected publish(snapshot: ClientSnapshot) {
    this.snapshot = snapshot;
    this.listeners.forEach((listener) => listener());
  }
  abstract start(): () => void;
  abstract updateSettings(settings: SettingsDto): Promise<SettingsDto>;
  abstract recoverWindow(): Promise<void>;
}
