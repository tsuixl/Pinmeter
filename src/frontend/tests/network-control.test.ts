import { describe, expect, it } from "vitest";
import {
  controllable,
  limitField,
  parseLimit,
} from "../src/features/app-network-control/useNetworkControlViewModel";
describe("network control inputs", () => {
  it("keeps unlimited separate from zero and converts decimal units", () => {
    expect(
      parseLimit({ mode: "unlimited", value: "0", unit: "KB/s" }),
    ).toBeNull();
    expect(parseLimit({ mode: "limited", value: "1.25", unit: "MB/s" })).toBe(
      1_250_000,
    );
    for (const value of [
      "",
      "0",
      "-1",
      "NaN",
      "Infinity",
      "15.999",
      "1000001",
    ]) {
      expect(() =>
        parseLimit({ mode: "limited", value, unit: "KB/s" }),
      ).toThrow();
    }
    for (const rate of [null, 16_000, 1_234_567, 1_000_000_000])
      expect(parseLimit(limitField(rate))).toBe(rate);
  });
  it("does not offer control for system, Pinmeter or unknown identities", () => {
    for (const path of [
      "",
      "System",
      "C:\\Windows\\System32\\svchost.exe",
      "C:\\Apps\\pinmeter-host.exe",
    ]) {
      expect(controllable({ id: "x", name: "x", path })).toBe(false);
    }
    expect(
      controllable({ id: "x", name: "x", path: "C:\\Apps\\browser.exe" }),
    ).toBe(true);
  });
});
