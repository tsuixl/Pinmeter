import { describe, expect, it } from "vitest";
import type { AppHistoryRowDto } from "../src/shared/contracts/monitor";
import {
  historyRows,
  appHistoryFrames,
} from "../src/features/history/app-history-model";
import { demoAppHistory } from "../src/shared/client/demo-app-history";
import { chartPaths, type ChartSeries } from "../src/features/monitoring/chart";
describe("application traffic history", () => {
  const row = (
    id: string,
    count: string,
    name = "app.exe",
  ): AppHistoryRowDto => ({
    id,
    name,
    path: id,
    received: count,
    transmitted: "0",
    total: count,
  });
  it("sorts full integer byte counts without losing precision or mixing special buckets", () => {
    const rows = [
      row("app:c:/a.exe", "9007199254740992"),
      row("app:d:/a.exe", "9007199254740993"),
      row("unknown", "9999999999999999999", "未归属"),
    ];
    expect(historyRows(rows, "", "total").map((r) => r.id)).toEqual([
      "app:d:/a.exe",
      "app:c:/a.exe",
      "unknown",
    ]);
    expect(historyRows(rows, "D:/", "total").map((r) => r.id)).toEqual([
      "app:d:/a.exe",
    ]);
    expect(rows[0].id).toBe("app:c:/a.exe");
  });
  it("keeps the saved app selectable and leaves unrecorded minutes empty", () => {
    const now = Date.now();
    const data = demoAppHistory(
      "24h",
      "app:c:\\program files\\browser\\浏览器.exe",
      new Date().setHours(0, 0, 0, 0),
      now - 86400_000,
      now,
    );
    const frames = appHistoryFrames(data);
    const series: ChartSeries = {
      id: "download",
      label: "下载",
      color: "",
      network: true,
      maxGapMs: 90_000,
      read: (f) => f.download,
    };
    expect(data.selected_name).toBe("浏览器.exe");
    expect(
      chartPaths(frames, series, data.from_ms, data.through_ms, 1e6),
    ).toHaveLength(2);
    expect(frames.some((f) => f.download.value === null)).toBe(true);
  });
});
