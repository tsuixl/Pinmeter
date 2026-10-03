import { afterEach, describe, expect, it, vi } from "vitest";
import type { MonitorClient } from "../src/shared/client/monitor-client";
import type { AlertsSnapshotDto } from "../src/shared/contracts/monitor";
import { defaultAlertsConfig } from "../src/shared/client/alert-defaults";
import { AlertsStore } from "../src/features/alerts/alerts-store";

function snapshot(revision = "0"): AlertsSnapshotDto {
  return {
    config: defaultAlertsConfig(),
    settings_revision: revision,
    revision,
    events: [],
    unread_count: 0,
    local_time_available: true,
    quiet_now: false,
    delivery_detail: "test",
  };
}
function fixture(read: () => Promise<AlertsSnapshotDto>) {
  let visible = true;
  const listeners = new Set<() => void>();
  const client = {
    getAlertsSnapshot: vi.fn(read),
    updateAlerts: vi.fn(async () => snapshot("2")),
    getSnapshot: () => ({
      state: null,
      nativeVisible: visible,
      connected: true,
      error: null,
      demo: false,
    }),
    subscribe: (fn: () => void) => {
      listeners.add(fn);
      return () => {
        listeners.delete(fn);
      };
    },
  } as unknown as MonitorClient;
  return {
    client,
    setVisible(value: boolean) {
      visible = value;
      listeners.forEach((fn) => fn());
    },
  };
}
afterEach(() => vi.useRealTimers());

describe("alerts read cache", () => {
  it("shares one read timer, pauses hidden requests and stops after the last subscriber", async () => {
    vi.useFakeTimers();
    const f = fixture(async () => snapshot());
    const store = new AlertsStore(f.client);
    const first = store.subscribe(() => {});
    const second = store.subscribe(() => {});
    await Promise.resolve();
    expect(f.client.getAlertsSnapshot).toHaveBeenCalledTimes(1);
    f.setVisible(false);
    await vi.advanceTimersByTimeAsync(20000);
    expect(f.client.getAlertsSnapshot).toHaveBeenCalledTimes(1);
    f.setVisible(true);
    await Promise.resolve();
    expect(f.client.getAlertsSnapshot).toHaveBeenCalledTimes(2);
    first();
    await vi.advanceTimersByTimeAsync(5000);
    expect(f.client.getAlertsSnapshot).toHaveBeenCalledTimes(3);
    second();
    await vi.advanceTimersByTimeAsync(10000);
    expect(f.client.getAlertsSnapshot).toHaveBeenCalledTimes(3);
  });

  it("does not overwrite a confirmed save with an earlier read response", async () => {
    let resolveRead!: (value: AlertsSnapshotDto) => void;
    const f = fixture(
      () =>
        new Promise((resolve) => {
          resolveRead = resolve;
        }),
    );
    const store = new AlertsStore(f.client);
    const stop = store.subscribe(() => {});
    expect(await store.save(defaultAlertsConfig(), "1")).toBe(true);
    resolveRead(snapshot("1"));
    await Promise.resolve();
    expect(store.getSnapshot().data?.settings_revision).toBe("2");
    stop();
  });

  it("rejects duplicate writes while keeping confirmed state on failure", async () => {
    let rejectSave!: (reason: Error) => void;
    const f = fixture(async () => snapshot("3"));
    f.client.updateAlerts = vi.fn(
      () =>
        new Promise<AlertsSnapshotDto>((_resolve, reject) => {
          rejectSave = reject;
        }),
    );
    const store = new AlertsStore(f.client);
    const stop = store.subscribe(() => {});
    await Promise.resolve();
    const first = store.save(defaultAlertsConfig(), "3");
    expect(await store.save(defaultAlertsConfig(), "3")).toBe(false);
    rejectSave(new Error("disk failed"));
    expect(await first).toBe(false);
    expect(f.client.updateAlerts).toHaveBeenCalledTimes(1);
    expect(store.getSnapshot().data?.settings_revision).toBe("3");
    expect(store.getSnapshot().error).toContain("disk failed");
    stop();
  });
});
