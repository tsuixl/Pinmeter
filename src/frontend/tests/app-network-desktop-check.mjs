import { chromium } from "@playwright/test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { pollState } from "./desktop-helpers.mjs";
const assets = fileURLToPath(
  new URL(
    "../../../docs/development/v0.1.0-app-network-ranking/assets/",
    import.meta.url,
  ),
);
const browser = await chromium.connectOverCDP("http://127.0.0.1:9223");
const page = browser.contexts()[0].pages()[0];
page.setDefaultTimeout(15000);
const invoke = (command, args = {}) =>
  page.evaluate(
    ({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args),
    { command, args },
  );
const initial = await invoke("get_monitor_state");
try {
  await page.getByRole("button", { name: "网络", exact: true }).click();
  await page
    .locator(".app-network")
    .getByRole("button", { name: "开始监控" })
    .click();
  let state = await pollState(
    invoke,
    (s) => s.app_network.status === "normal" && s.app_network.apps.length > 0,
    125000,
  );
  assert(
    state.app_network.apps.some((a) =>
      a.icon?.startsWith("data:image/png;base64,"),
    ),
  );
  for (const direction of ["download", "upload"]) {
    const sum =
      state.app_network.apps.reduce(
        (s, a) => s + (a.traffic[direction + "_share"] ?? 0),
        0,
      ) + (state.app_network.other[direction + "_share"] ?? 0);
    assert(sum === 0 || Math.abs(sum - 100) < 0.0001);
  }
  const session = state.app_network.session;
  for (const theme of ["light", "dark"]) {
    state = await invoke("get_monitor_state");
    await invoke("update_settings", {
      settings: { ...state.settings, theme, interval_ms: 1000 },
      expectedRevision: state.settings.revision,
    });
    await page.waitForFunction(
      (theme) => document.documentElement.dataset.theme === theme,
      theme,
    );
    await page.locator(".app-network").scrollIntoViewIfNeeded();
    const expand = page
      .locator(".app-network")
      .getByRole("button", { name: /^展开 / })
      .first();
    if (await expand.count()) await expand.click();
    await page.waitForTimeout(350);
    await page.screenshot({ path: assets + `ranking-native-${theme}.png` });
  }
  await invoke("perform_desktop_action", { action: "minimize" });
  state = await pollState(invoke, (s) => !s.app_network.running);
  assert.equal(state.app_network.status, "disabled");
  assert(state.app_network.apps.every((a) => a.traffic.download === null));
  await invoke("perform_desktop_action", { action: "activate" });
  await page
    .locator(".app-network")
    .getByRole("button", { name: "开始监控" })
    .click();
  state = await pollState(
    invoke,
    (s) => s.app_network.status === "normal",
    125000,
  );
  assert.notEqual(state.app_network.session, session);
  await page.getByRole("button", { name: "CPU", exact: true }).click();
  await pollState(invoke, (s) => !s.app_network.running);
  assert.equal((await invoke("get_monitor_state")).frame.cpu.status, "normal");
  console.log(
    "PASS: real application traffic/icons/shares, native themes, minimize stop, fresh session restart and page cleanup.",
  );
} finally {
  await invoke("set_app_network_monitoring", { enabled: false });
  const state = await invoke("get_monitor_state");
  await invoke("update_settings", {
    settings: { ...initial.settings, revision: state.settings.revision },
    expectedRevision: state.settings.revision,
  });
  await browser.close();
}
