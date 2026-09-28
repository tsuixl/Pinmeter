import { expect, it } from "vitest";
import { DemoClient } from "../src/shared/client/demo-client";
import { acceptBatch } from "../src/shared/state/history";
import { ipClient } from "../src/shared/client/ip-client";
it("keeps omitted IP updates but never reuses another session's result", () => {
  const state = new DemoClient().getSnapshot().state!;
  const batch = {
    kind: "update",
    subscription_id: "s",
    delivery_seq: "2",
    history_from: "0",
    history_to: "0",
    requires_history_sync: false,
    state: { ...state, ip: undefined },
  };
  expect(acceptBatch(state, batch).ip).toBe(state.ip);
  expect(
    acceptBatch(state, { ...batch, kind: "bootstrap" }).ip,
  ).toBeUndefined();
  expect(
    acceptBatch(state, {
      ...batch,
      state: { ...batch.state, session_id: "new" },
    }).ip,
  ).toBeUndefined();
});
it("IP client projects the shared authoritative snapshot", () => {
  const monitor = new DemoClient();
  const client = ipClient(monitor);
  expect(client.getSnapshot()).toBe(monitor.getSnapshot().state?.ip);
  expect(client.isConnected()).toBe(true);
});
