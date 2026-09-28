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
let state = await invoke("get_monitor_state");
await invoke("update_settings", {
  settings: { ...state.settings, interval_ms: 1000 },
  expectedRevision: state.settings.revision,
});
await pollState(
  invoke,
  (s) => s.settings.revision === s.applied_settings_revision,
);
const before = await invoke("get_monitor_state");
await invoke("perform_desktop_action", { action: "minimize" });
await page.waitForTimeout(500);
await page.evaluate(() => {
  window.pinmeterTestMutations = 0;
  window.pinmeterTestObserver = new MutationObserver(
    () => window.pinmeterTestMutations++,
  );
  window.pinmeterTestObserver.observe(document.querySelector("main"), {
    subtree: true,
    childList: true,
    characterData: true,
  });
});
await page.waitForTimeout(4500);
const count = await page.evaluate(() => window.pinmeterTestMutations);
assert.equal(count, 0, "minimized UI must not render");
state = await invoke("get_monitor_state");
assert(BigInt(state.frame.cursor) > BigInt(before.frame.cursor));
await invoke("perform_desktop_action", { action: "activate" });
await page.waitForFunction(() => window.pinmeterTestMutations > 0);
await page.evaluate(() => window.pinmeterTestObserver.disconnect());
state = await invoke("get_monitor_state");
assert.equal(state.session_id, before.session_id);
assert(state.history.length <= 301);
for (const interval of [2000, 5000, 1000]) {
  state = await invoke("get_monitor_state");
  await invoke("update_settings", {
    settings: { ...state.settings, interval_ms: interval },
    expectedRevision: state.settings.revision,
  });
  await pollState(
    invoke,
    (s) => s.settings.revision === s.applied_settings_revision,
  );
  const first = (await invoke("get_monitor_state")).frame;
  const second = (
    await pollState(
      invoke,
      (s) => BigInt(s.frame.cursor) > BigInt(first.cursor),
    )
  ).frame;
  const delta = second.elapsed_ms - first.elapsed_ms;
  assert(
    delta >= interval && delta < interval + 1000,
    `${interval}: actual ${delta}`,
  );
}
console.log(
  "PASS: minimized no DOM updates, sampling continues, reactivation resumes, same session, actual 1/2/5s sample intervals.",
);
await browser.close();
