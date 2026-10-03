import { describe, expect, it } from "vitest";
import { plotFrame } from "../src/features/monitoring/plot-frame";
import { chartPaths } from "../src/features/monitoring/chart";
import { emptyReading } from "../src/features/monitoring/useMonitorViewModel";
describe("disk history", () => {
  it("breaks a curve when the selected device disappears", () => {
    const reading = { ...emptyReading, status: "normal" as const, value: 12 };
    const frames = [
      plotFrame(0, 0, "1", { download: reading }),
      plotFrame(2000, 2000, "1", { download: undefined }),
      plotFrame(4000, 4000, "1", { download: reading }),
    ];
    expect(chartPaths(frames, "download", 0, 4000, 100)).toHaveLength(2);
    expect(frames[1].download.value).toBeNull();
  });
});
