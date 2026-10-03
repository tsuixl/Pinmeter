import { invoke, isTauri } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { listen } from "@tauri-apps/api/event";
import type { MonitorStateDto, ExitStatusDto } from "../contracts/monitor";

export interface WindowState {
  maximized: boolean;
  fullscreen: boolean;
}
export interface WindowClient {
  platform: string;
  initialTheme: "light" | "dark";
  initialFontFamily?: string;
  initialFontStyle?: string;
  readState(): Promise<WindowState>;
  onResize(listener: () => void): Promise<() => void>;
  minimize(): Promise<void>;
  toggleMaximize(): Promise<void>;
  close(): Promise<void>;
  show(): Promise<void>;
  finishStartup?(): Promise<void>;
  readExitState(): Promise<ExitStatusDto>;
  onExitState(listener: (state: ExitStatusDto) => void): Promise<() => void>;
  confirmExit(): Promise<void>;
  cancelExit(): Promise<void>;
  resolveClose?(action: "minimize" | "exit", remember: boolean): Promise<void>;
}

export async function createWindowClient(): Promise<WindowClient | undefined> {
  if (!isTauri()) return undefined;
  const window = getCurrentWindow();
  const [platform, state, nativeTheme] = await Promise.all([
    invoke<string>("get_desktop_platform"),
    invoke<MonitorStateDto>("get_monitor_state"),
    window.theme(),
  ]);
  const preference = state.settings.theme;
  return {
    platform,
    initialFontFamily: state.settings.font_family,
    initialFontStyle: state.settings.font_style,
    initialTheme:
      preference === "light" || preference === "dark"
        ? preference
        : (nativeTheme ?? "light"),
    async readState() {
      const [maximized, fullscreen] = await Promise.all([
        window.isMaximized(),
        window.isFullscreen(),
      ]);
      return { maximized, fullscreen };
    },
    onResize: (listener) => window.onResized(listener),
    minimize: () => invoke("minimize_to_tray"),
    toggleMaximize: () => window.toggleMaximize(),
    close: () => window.close(),
    readExitState: () => invoke("get_app_exit_state"),
    onExitState: (listener) =>
      listen<ExitStatusDto>("pinmeter-exit", ({ payload }) =>
        listener(payload),
      ),
    confirmExit: () => invoke("request_app_exit", { confirmed: true }),
    cancelExit: () => invoke("cancel_app_exit"),
    resolveClose: (action, remember) =>
      invoke("resolve_app_close", { action, remember }),
    show: () => window.show(),
    finishStartup: () => invoke("complete_window_startup"),
  };
}

export async function showStartupError(error: unknown) {
  document.getElementById("root")!.textContent =
    `Pinmeter 启动失败：${String(error)}`;
  if (isTauri()) await getCurrentWindow().show();
}
