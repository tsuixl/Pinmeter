import { describe, expect, it } from "vitest";
import { cpuUsageStrength } from "../src/features/monitoring/useCpuProcessorChart";
import { emptyReading } from "../src/features/monitoring/useMonitorViewModel";

describe("CPU chart display rules", () => {
  it("uses an absolute percentage scale and keeps unavailable distinct from genuine zero", () => {
    const processor = (
      value: number | null,
      status = "normal" as typeof emptyReading.status,
    ) => ({ id: "0,0", usage: { ...emptyReading, value, status } });
    expect(cpuUsageStrength(processor(0))).toBe(0.12);
    expect(cpuUsageStrength(processor(50))).toBe(0.56);
    expect(cpuUsageStrength(processor(100))).toBe(1);
    expect(cpuUsageStrength(processor(10))).toBeLessThan(0.21);
    expect(cpuUsageStrength(processor(null))).toBeNull();
    expect(cpuUsageStrength(processor(Number.NaN))).toBeNull();
    expect(cpuUsageStrength(processor(0, "failed"))).toBeNull();
  });
});
