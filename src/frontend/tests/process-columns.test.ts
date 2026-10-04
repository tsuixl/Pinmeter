import { describe, expect, it } from "vitest";
import {
  clampColumnWidth,
  defaultColumnWidths,
  minimumColumnWidths,
  moveProcessColumn,
  processColumnOrder,
} from "../src/features/processes/process-columns";
describe("process columns", () => {
  it("moves the selected column before or after its target without losing a column", () => {
    expect(moveProcessColumn(processColumnOrder, "cpu", "name", false)).toEqual(
      ["cpu", "name", "pid", "working_set", "id"],
    );
    expect(
      moveProcessColumn(processColumnOrder, "name", "working_set", true),
    ).toEqual(["pid", "cpu", "working_set", "name", "id"]);
    expect(moveProcessColumn(processColumnOrder, "pid", "pid", true)).toEqual(
      processColumnOrder,
    );
  });
  it("keeps controls readable when shrinking and bounds oversized or invalid widths", () => {
    expect(clampColumnWidth("name", 10)).toBe(minimumColumnWidths.name);
    expect(clampColumnWidth("cpu", 5000)).toBe(1200);
    expect(clampColumnWidth("pid", NaN)).toBe(minimumColumnWidths.pid);
    expect(clampColumnWidth("cpu", 175.7)).toBe(176);
  });
  it("fits defaults to the available desktop width while narrow containers scroll", () => {
    const wide = defaultColumnWidths(1280),
      narrow = defaultColumnWidths(420);
    expect(Object.values(wide).reduce((a, b) => a + b, 0)).toBeCloseTo(
      1280,
      -1,
    );
    expect(narrow).toEqual(minimumColumnWidths);
    expect(Object.keys(wide)).toEqual([...processColumnOrder]);
  });
});
