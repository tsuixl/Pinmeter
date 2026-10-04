import { describe, expect, it } from "vitest";
import { defaultAlertsConfig } from "../src/shared/client/alert-defaults";
import { alertSaveRevision } from "../src/features/alerts/useAlertsEditor";

describe("reminder draft revision across settings categories", () => {
  it("accepts a revision advanced by an unrelated immediate setting", () => {
    const base = defaultAlertsConfig();
    expect(
      alertSaveRevision(base, "2", {
        alerts: structuredClone(base),
        revision: "4",
      }),
    ).toBe("4");
  });
  it("does not overwrite rules changed by another entry point", () => {
    const base = defaultAlertsConfig();
    const changed = structuredClone(base);
    changed.cpu.enabled = true;
    expect(
      alertSaveRevision(base, "2", { alerts: changed, revision: "4" }),
    ).toBe("2");
  });
  it("keeps the original conflict boundary when the connection has no settings", () => {
    expect(alertSaveRevision(defaultAlertsConfig(), "2")).toBe("2");
  });
});
