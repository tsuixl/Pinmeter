import { invoke, isTauri } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { FontCatalogDto, TraySnapshotDto } from "../contracts/monitor";
import { builtinFontCatalog } from "../fonts";
export type TrayPage = "overview" | "cpu" | "memory" | "network";
export interface TrayPanelClient {
  read(): Promise<TraySnapshotDto>;
  fonts(): Promise<FontCatalogDto>;
  ready(): Promise<void>;
  hide(): Promise<void>;
  open(page: TrayPage): Promise<void>;
  onVisible(listener: (visible: boolean) => void): Promise<() => void>;
  demo: boolean;
}
export async function createTrayPanelClient(): Promise<TrayPanelClient> {
  if (import.meta.env.DEV && new URLSearchParams(location.search).has("demo")) {
    const { DemoClient } = await import("./demo-client");
    const demo = new DemoClient();
    demo.start();
    return {
      demo: true,
      async read() {
        const state = demo.getSnapshot().state!;
        return {
          frame: state.frame,
          history: state.history.slice(-61),
          theme: new URLSearchParams(location.search).get("theme") ?? "light",
          font_family: state.settings.font_family,
          font_style: state.settings.font_style,
          network: "演示网卡",
        };
      },
      async fonts() {
        return builtinFontCatalog;
      },
      async ready() {},
      async hide() {
        document.body.dataset.panelHidden = "true";
      },
      async open(page) {
        location.assign(`/?demo=1&fromPanel=${page}`);
      },
      async onVisible() {
        return () => {};
      },
    };
  }
  if (!isTauri()) throw new Error("快捷面板只能在 Pinmeter 桌面应用中使用");
  return {
    demo: false,
    read: () => invoke("get_tray_snapshot"),
    fonts: () => invoke("get_tray_font_catalog"),
    ready: () => invoke("complete_tray_panel"),
    hide: () => invoke("hide_tray_panel"),
    open: (page) => invoke("open_tray_detail", { page }),
    onVisible: (listener) =>
      listen<boolean>("tray-panel-visible", ({ payload }) => listener(payload)),
  };
}
