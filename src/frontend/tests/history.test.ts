import { describe, it, expect } from "vitest";
import { DemoClient } from "../src/shared/client/demo-client";
import { mergeHistory, acceptBatch } from "../src/shared/state/history";
import {
  chartPaths,
  clampAnchor,
  areaPath,
  networkCeiling,
  temperatureDomain,
} from "../src/features/monitoring/chart";

const state = new DemoClient().getSnapshot().state!;
const frame = state.history[0];
describe("bounded history and chart continuity", () => {
  it("keeps a saved setting when an older batch arrives, but accepts a new session", () => {
    const saved = {
      ...state,
      settings: { ...state.settings, revision: "9", theme: "dark" },
    };
    const batch = {
      kind: "bootstrap",
      subscription_id: "s",
      delivery_seq: "1",
      history_from: "0",
      history_to: "0",
      requires_history_sync: false,
      state: {
        ...state,
        settings: { ...state.settings, revision: "8", theme: "light" },
      },
    };
    expect(acceptBatch(saved, batch).settings).toEqual(saved.settings);
    expect(
      acceptBatch(saved, {
        ...batch,
        state: { ...batch.state, session_id: "new" },
      }).settings.revision,
    ).toBe("8");
    expect(
      acceptBatch(saved, {
        ...batch,
        state: {
          ...batch.state,
          settings: { ...batch.state.settings, revision: "10" },
        },
      }).settings.revision,
    ).toBe("10");
  });
  it("keeps temperature gaps independent and scales Celsius without clipping", () => {
    const frames = [0, 1, 2, 3].map((i) => ({
      ...frame,
      elapsed_ms: i * 1000,
      cpu_temperature: {
        ...frame.cpu_temperature,
        value: i === 0 ? -20 : i === 1 ? null : 120,
        status: i === 1 ? ("stale" as const) : ("normal" as const),
      },
    }));
    const [floor, ceiling] = temperatureDomain([-20, 120]);
    expect([floor, ceiling]).toEqual([-20, 120]);
    expect(temperatureDomain([0, 60])).toEqual([0, 100]);
    const paths = chartPaths(
      frames,
      "cpu_temperature",
      0,
      3000,
      ceiling,
      floor,
    );
    expect(paths).toHaveLength(2);
    expect(paths[0]).toContain(",180.00");
    expect(paths[1]).toContain(",0.00");
    expect(chartPaths(frames, "cpu", 0, 3000, 100)).toHaveLength(1);
  });
  it("fills each valid segment independently and uses stable readable network bounds", () => {
    expect(areaPath("M300.00,120.00 L400.00,90.00")).toBe(
      "M300.00,120.00 L400.00,90.00 L400.00,180 L300.00,180 Z",
    );
    expect(networkCeiling([1100, 1350])).toBe(2000);
    expect(networkCeiling([1500, 1900])).toBe(2000);
    expect(networkCeiling([0])).toBe(1000);
  });
  it("deduplicates replay and evicts old samples by time and count", () => {
    const incoming = Array.from({ length: 700 }, (_, i) => ({
      ...frame,
      cursor: String(i),
      elapsed_ms: i * 1000,
    }));
    const result = mergeHistory(incoming, incoming.slice(-10));
    expect(result).toHaveLength(301);
    expect(result[0].elapsed_ms).toBe(399000);
  });
  it("clears old sessions at bootstrap", () => {
    const result = acceptBatch(state, {
      kind: "bootstrap",
      subscription_id: "s",
      delivery_seq: "1",
      history_from: "0",
      history_to: "0",
      requires_history_sync: false,
      state: { ...state, session_id: "new", history: [] },
    });
    expect(result.history).toEqual([]);
  });
  it("breaks curves for invalid samples and network identity changes", () => {
    const frames = [0, 1, 2, 3].map((i) => ({
      ...frame,
      elapsed_ms: i * 1000,
      network_id: i < 2 ? "a" : "b",
    }));
    expect(chartPaths(frames, "download", 0, 3000, 1e6)).toHaveLength(2);
    expect(chartPaths(frames, "cpu", 0, 3000, 100)).toHaveLength(1);
    frames[1] = {
      ...frames[1],
      cpu: { ...frame.cpu, status: "failed", value: null },
    };
    expect(chartPaths(frames, "cpu", 0, 3000, 100)).toHaveLength(2);
  });
  it("keeps real zero and clamps a dragged viewport to retained data", () => {
    expect(
      chartPaths(
        [{ ...frame, cpu: { ...frame.cpu, value: 0, status: "normal" } }],
        "cpu",
        0,
        300000,
        100,
      )[0],
    ).toContain(",180.00");
    expect(clampAnchor(-100, 1000, 301000, 60000)).toBe(61000);
  });
});
