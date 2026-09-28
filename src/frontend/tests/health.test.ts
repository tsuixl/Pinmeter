import { describe, expect, it } from "vitest";
import { DemoClient } from "../src/shared/client/demo-client";
import { monitorHealth } from "../src/features/monitoring/health";

describe("global monitoring health", () => {
  it("does not report unsupported optional readings or valid zeroes as failures", () => {
    const snapshot = new DemoClient().getSnapshot();
    const state = snapshot.state!;
    state.cpu_temperature.status = "unsupported";
    state.frame!.cpu.value = 0;
    state.network_control!.supported = false;
    state.network_control!.available = false;
    expect(monitorHealth(snapshot).issues).toEqual([]);
  });
  it("reports a sensor failure even when basic metrics and the connection are normal", () => {
    const snapshot = new DemoClient().getSnapshot();
    snapshot.state!.cpu_temperature.status = "failed";
    snapshot.state!.cpu_temperature.detail = "sensor disconnected";
    expect(monitorHealth(snapshot).issues).toContainEqual({
      title: "CPU 温度",
      detail: "sensor disconnected",
      page: "cpu",
    });
  });
  it("does not call an unavailable helper recovered just because there are no saved rules", () => {
    const snapshot = new DemoClient().getSnapshot();
    snapshot.state!.network_control!.available = false;
    expect(
      monitorHealth(snapshot).issues.some(
        (issue) => issue.title === "网络限制待核对",
      ),
    ).toBe(true);
    snapshot.connected = false;
    expect(monitorHealth(snapshot).label).toBe("连接中断");
  });
  it("surfaces whole-provider and control capability failures without active rules or devices", () => {
    const snapshot = new DemoClient().getSnapshot();
    snapshot.state!.gpu = {
      devices: [],
      status: "failed",
      detail: "GPU helper exited",
    };
    snapshot.state!.network_control!.available = true;
    snapshot.state!.network_control!.firewall_available = false;
    expect(monitorHealth(snapshot).issues.map((issue) => issue.title)).toEqual([
      "GPU 采集",
      "网络限制待核对",
    ]);
  });
});
