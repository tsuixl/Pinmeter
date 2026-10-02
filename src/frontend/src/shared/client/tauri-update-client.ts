import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { UpdateSnapshotDto } from "../contracts/monitor";
import { acceptUpdateSnapshot, type UpdateClient } from "./update-client";

export class TauriUpdateClient implements UpdateClient {
  private snapshot: UpdateSnapshotDto | null = null;
  private listeners = new Set<() => void>();
  private generation = 0;
  getSnapshot = () => this.snapshot;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };
  private accept(next: UpdateSnapshotDto) {
    const accepted = acceptUpdateSnapshot(this.snapshot, next);
    if (accepted === this.snapshot) return;
    this.snapshot = accepted;
    this.listeners.forEach((listener) => listener());
  }
  start() {
    const generation = ++this.generation;
    let stop: (() => void) | undefined;
    void listen<UpdateSnapshotDto>("pinmeter-update", ({ payload }) => {
      if (generation === this.generation) this.accept(payload);
    })
      .then(async (unlisten) => {
        if (generation !== this.generation) {
          unlisten();
          return;
        }
        stop = unlisten;
        const state = await invoke<UpdateSnapshotDto>("get_update_state");
        if (generation === this.generation) this.accept(state);
      })
      .catch((error) => console.error("无法连接更新服务", error));
    return () => {
      this.generation++;
      stop?.();
    };
  }
  check = () => invoke<void>("check_app_update");
  download = () => invoke<void>("download_app_update");
  install = (confirmed: boolean) =>
    invoke<void>("install_app_update", { confirmed });
  preference = (action: "automatic" | "read" | "dismiss", value?: boolean) =>
    invoke<void>("update_update_preference", { action, value: value ?? null });
  openDownloads = () => invoke<void>("open_update_downloads");
}
