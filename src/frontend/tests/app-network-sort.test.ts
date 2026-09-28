import { describe, expect, it } from "vitest";
import { sortedApps } from "../src/features/app-network/useAppNetworkViewModel";
import type { AppNetworkRowDto } from "../src/shared/contracts/monitor";

const app = (
  id: string,
  name: string,
  download: number | null,
  upload = 0,
): AppNetworkRowDto => ({
  id,
  name,
  path: id,
  icon: null,
  processes: [],
  traffic: {
    download,
    upload,
    received: "0",
    sent: "0",
    download_share: null,
    upload_share: null,
  },
});
describe("application network sorting", () => {
  it("keeps missing readings after valid zeroes in both directions without mutating the snapshot", () => {
    const apps = [
      app("missing", "Missing", null),
      app("busy", "Busy", 500),
      app("idle", "Idle", 0),
    ];
    expect(
      sortedApps(apps, { key: "download", order: "desc" }).map((a) => a.id),
    ).toEqual(["busy", "idle", "missing"]);
    expect(
      sortedApps(apps, { key: "download", order: "asc" }).map((a) => a.id),
    ).toEqual(["idle", "busy", "missing"]);
    expect(apps[0].id).toBe("missing");
  });
  it("sorts names naturally and ties deterministically", () => {
    const apps = [
      app("z", "app10", 1),
      app("b", "App2", 1),
      app("a", "app2", 1),
    ];
    expect(
      sortedApps(apps, { key: "name", order: "asc" }).map((a) => a.id),
    ).toEqual(["a", "b", "z"]);
    expect(
      sortedApps(apps, { key: "name", order: "desc" }).map((a) => a.id),
    ).toEqual(["z", "a", "b"]);
  });
  it("uses the selected direction rather than the opposite rate", () => {
    const apps = [app("download", "A", 500, 1), app("upload", "B", 1, 500)];
    expect(sortedApps(apps, { key: "upload", order: "desc" })[0].id).toBe(
      "upload",
    );
    expect(sortedApps(apps, { key: "upload", order: "asc" })[0].id).toBe(
      "download",
    );
  });
});
