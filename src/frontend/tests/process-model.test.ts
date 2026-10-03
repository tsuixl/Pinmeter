import { describe, expect, it } from "vitest";
import { processRows } from "../src/features/processes/process-model";
import type { ProcessSnapshotDto } from "../src/shared/contracts/monitor";

function sample(): ProcessSnapshotDto {
  return {
    sort: "cpu",
    status: "normal",
    detail: "",
    sampled_at_ms: 1000,
    total: 12,
    unreadable: 0,
    truncated: false,
    rows: Array.from({ length: 12 }, (_, i) => ({
      id: `pid:${i}`,
      pid: i + 10,
      name: i < 2 ? "same.exe" : `worker-${i}.exe`,
      application_id: `app:${i}`,
      cpu: 12 - i,
      cpu_status: "normal",
      working_set: i * 100,
      memory_status: "normal",
    })),
    applications: Array.from({ length: 12 }, (_, i) => ({
      id: `app:${i}`,
      name: i < 2 ? "same.exe" : `worker-${i}.exe`,
      cpu: 12 - i,
      cpu_status: "normal",
      working_set: i * 100,
      memory_status: "normal",
      process_count: 1,
    })),
  };
}
describe("process investigation", () => {
  it("searches beyond the default top ten and keeps same-name identities separate", () => {
    const data = sample();
    expect(
      processRows(data, "processes", "cpu", "21", null, new Set(), 10).rows[0]
        .pid,
    ).toBe(21);
    const found = processRows(
      data,
      "applications",
      "cpu",
      "same.exe",
      null,
      new Set(),
      10,
    );
    expect(found.total).toBe(2);
    expect(found.rows.filter((r) => !r.child).map((r) => r.id)).toEqual([
      "app:0",
      "app:1",
    ]);
  });
  it("pins by identity despite changing rank and never replaces an exited target by name", () => {
    const data = sample();
    expect(
      processRows(data, "processes", "cpu", "", "pid:11", new Set(), 10).rows[0]
        .id,
    ).toBe("pid:11");
    const missing = processRows(
      data,
      "processes",
      "cpu",
      "",
      "old-process",
      new Set(),
      10,
    );
    expect(missing.pinnedMissing).toBe(true);
    expect(missing.rows.some((r) => r.id === "old-process")).toBe(false);
  });
  it("uses backend aggregates and keeps invalid values after valid zero", () => {
    const data = sample();
    data.applications[0].cpu = null;
    data.applications[0].cpu_status = "permission_denied";
    data.applications[1].cpu = 0;
    const result = processRows(
      data,
      "applications",
      "cpu",
      "same.exe",
      null,
      new Set(["app:0"]),
      10,
    );
    expect(result.rows.filter((r) => !r.child).map((r) => r.cpu)).toEqual([
      0,
      null,
    ]);
    expect(result.rows.find((r) => r.id === "app:0")?.cpu_status).toBe(
      "permission_denied",
    );
  });
});
