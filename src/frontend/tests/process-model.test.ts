import { describe, expect, it } from "vitest";
import {
  defaultProcessSort,
  processRows,
  processSnapshotSort,
  toggleProcessSort,
  type ProcessSortDirection,
  type ProcessSortKey,
} from "../src/features/processes/process-model";
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

  it("uses column defaults on selection and reverses a repeated header click", () => {
    expect(defaultProcessSort("cpu")).toEqual({
      key: "cpu",
      direction: "desc",
    });
    expect(defaultProcessSort("memory")).toEqual({
      key: "memory",
      direction: "desc",
    });
    expect(defaultProcessSort("name")).toEqual({
      key: "name",
      direction: "asc",
    });
    expect(defaultProcessSort("pid")).toEqual({ key: "pid", direction: "asc" });
    expect(defaultProcessSort("unknown")).toEqual({
      key: "cpu",
      direction: "desc",
    });
    const nameSort = toggleProcessSort(defaultProcessSort("cpu"), "name");
    expect(nameSort).toEqual({ key: "name", direction: "asc" });
    expect(toggleProcessSort(nameSort, "name")).toEqual({
      key: "name",
      direction: "desc",
    });
    expect(toggleProcessSort(nameSort, "memory")).toEqual({
      key: "memory",
      direction: "desc",
    });
  });

  it("only requests supported backend snapshot orders", () => {
    expect(
      ["name", "pid", "cpu", "memory"].map((key) =>
        processSnapshotSort(key as ProcessSortKey),
      ),
    ).toEqual(["cpu", "cpu", "cpu", "memory"]);
  });

  it.each<[ProcessSortKey, ProcessSortDirection, number[]]>([
    ["name", "asc", [1, 2, 0]],
    ["name", "desc", [0, 2, 1]],
    ["pid", "asc", [0, 1, 2]],
    ["pid", "desc", [2, 1, 0]],
    ["cpu", "asc", [2, 1, 0]],
    ["cpu", "desc", [0, 1, 2]],
    ["memory", "asc", [0, 1, 2]],
    ["memory", "desc", [2, 1, 0]],
  ])("sorts flat processes by %s %s", (key, direction, expected) => {
    const data = sample();
    data.rows = data.rows.slice(0, 3);
    data.rows[0].name = "worker-10.exe";
    data.rows[1].name = "Worker-1.exe";
    data.rows[2].name = "worker-2.exe";
    const result = processRows(
      data,
      "processes",
      key,
      "",
      null,
      new Set(),
      10,
      direction,
    );
    expect(result.rows.map((row) => row.id)).toEqual(
      expected.map((i) => `pid:${i}`),
    );
  });

  it.each<["cpu" | "memory", ProcessSortDirection]>([
    ["cpu", "asc"],
    ["cpu", "desc"],
    ["memory", "asc"],
    ["memory", "desc"],
  ])("keeps invalid %s values last for %s order", (key, direction) => {
    const data = sample();
    data.rows = data.rows.slice(0, 5);
    const metric = key === "cpu" ? "cpu" : "working_set";
    const status = key === "cpu" ? "cpu_status" : "memory_status";
    data.rows[0][metric] = null;
    data.rows[1][metric] = 100;
    data.rows[1][status] = "stale";
    data.rows[2][metric] = 0;
    data.rows[3][metric] = 1;
    data.rows[4][metric] = Number.NaN;
    const result = processRows(
      data,
      "processes",
      key,
      "",
      null,
      new Set(),
      10,
      direction,
    );
    expect(result.rows.map((row) => row.id)).toEqual([
      ...(direction === "asc" ? ["pid:2", "pid:3"] : ["pid:3", "pid:2"]),
      "pid:0",
      "pid:1",
      "pid:4",
    ]);
  });

  it("keeps pinned groups together and sorts their children by the selected direction", () => {
    const data = sample();
    data.applications = data.applications.slice(0, 2);
    data.rows = data.rows.slice(0, 4);
    data.rows[0].application_id = "app:1";
    data.rows[1].application_id = "app:0";
    data.rows[2].application_id = "app:1";
    data.rows[3].application_id = "app:0";
    const result = processRows(
      data,
      "applications",
      "memory",
      "",
      "app:1",
      new Set(["app:0", "app:1"]),
      10,
      "asc",
    );
    expect(result.rows.map((row) => [row.id, row.child])).toEqual([
      ["app:1", false],
      ["pid:0", true],
      ["pid:2", true],
      ["app:0", false],
      ["pid:1", true],
      ["pid:3", true],
    ]);
    expect(data.rows.map((row) => row.id)).toEqual([
      "pid:0",
      "pid:1",
      "pid:2",
      "pid:3",
    ]);
    expect(data.applications.map((row) => row.id)).toEqual(["app:0", "app:1"]);
  });

  it.each<ProcessSortDirection>(["asc", "desc"])(
    "sorts group PID children %s while keeping parent order stable",
    (direction) => {
      const data = sample();
      data.applications = [data.applications[1], data.applications[0]];
      data.rows = data.rows.slice(0, 4);
      data.rows[0].application_id = "app:0";
      data.rows[1].application_id = "app:1";
      data.rows[2].application_id = "app:0";
      data.rows[3].application_id = "app:1";
      const result = processRows(
        data,
        "applications",
        "pid",
        "",
        null,
        new Set(["app:0", "app:1"]),
        10,
        direction,
      );
      expect(
        result.rows.filter((row) => !row.child).map((row) => row.id),
      ).toEqual(["app:1", "app:0"]);
      expect(
        result.rows.filter((row) => row.child).map((row) => row.pid),
      ).toEqual(direction === "asc" ? [11, 13, 10, 12] : [13, 11, 12, 10]);
    },
  );

  it("keeps equal-key application identities stable and never pins by a duplicate name", () => {
    const data = sample();
    data.applications = [data.applications[1], data.applications[0]];
    const result = processRows(
      data,
      "applications",
      "name",
      "same",
      "gone:app",
      new Set(),
      10,
      "desc",
    );
    expect(
      result.rows.filter((row) => !row.child).map((row) => row.id),
    ).toEqual(["app:1", "app:0"]);
    expect(result.pinnedMissing).toBe(true);
  });
});
