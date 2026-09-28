import { waitForSettingsSave } from "./settings-helpers.mjs";
import { chromium } from "@playwright/test";
import assert from "node:assert/strict";
import { pollState } from "./desktop-helpers.mjs";
const browser = await chromium.connectOverCDP("http://127.0.0.1:9223");
const page = browser.contexts()[0].pages()[0];
const invoke = (command, args = {}) =>
  page.evaluate(
    ({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args),
    { command, args },
  );
await page.getByText("本机实时数据", { exact: true }).waitFor();
const initial = await invoke("get_monitor_state");
assert.equal(initial.protocol_version, 1);
await page.getByRole("button", { name: "设置", exact: true }).click();
await page.getByText("深色", { exact: true }).click();
await page.getByRole("combobox", { name: "采样间隔", exact: true }).click();
await page.getByRole("option", { name: "2 秒", exact: true }).click();
await waitForSettingsSave(page);
await page.getByText("设置已保存并生效", { exact: true }).waitFor();
let state = await invoke("get_monitor_state");
assert.equal(state.settings.interval_ms, 2000);
assert.equal(state.settings.theme, "dark");
assert.equal(state.settings.revision, state.applied_settings_revision);
await page.screenshot({ path: "test-results/s3-settings.png" });
await assert.rejects(
  invoke("update_settings", {
    settings: initial.settings,
    expectedRevision: initial.settings.revision,
  }),
  /配置版本冲突/,
);
for (const interval of [5000, 1000]) {
  state = await invoke("get_monitor_state");
  await invoke("update_settings", {
    settings: { ...state.settings, interval_ms: interval },
    expectedRevision: state.settings.revision,
  });
  await pollState(
    invoke,
    (s) => s.settings.revision === s.applied_settings_revision,
  );
  state = await invoke("get_monitor_state");
  assert.equal(state.settings.interval_ms, interval);
}
state = await invoke("get_monitor_state");
await invoke("update_settings", {
  settings: { ...state.settings, network_id: "test-removed-interface" },
  expectedRevision: state.settings.revision,
});
await pollState(
  invoke,
  (s) =>
    s.settings.revision === s.applied_settings_revision &&
    s.selected_interface === null,
);
state = await invoke("get_monitor_state");
assert.equal(state.frame.download.value, null);
assert.notEqual(state.frame.download.status, "normal");
await page.getByRole("button", { name: "网络", exact: true }).click();
await page.screenshot({ path: "test-results/s3-removed-interface.png" });
await invoke("update_settings", {
  settings: {
    ...state.settings,
    network_id: null,
    theme: "dark",
    interval_ms: 2000,
  },
  expectedRevision: state.settings.revision,
});
await pollState(
  invoke,
  (s) =>
    s.settings.revision === s.applied_settings_revision &&
    s.frame.download.status === "normal",
);
console.log(
  "PASS: real readings, theme save, 1/2/5 second configuration, stale revision rejection, manual missing interface, recovery. Leave dark/2s for restart verification.",
);
await browser.close();
