import { chromium } from "@playwright/test";
import { execFileSync } from "node:child_process";
import { copyFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";
import { pollState } from "./desktop-helpers.mjs";
const root = fileURLToPath(new URL("../../../", import.meta.url));
const assets = fileURLToPath(
  new URL(
    "../../../docs/development/v0.1.0-main-window/assets/",
    import.meta.url,
  ),
);
const browser = await chromium.connectOverCDP("http://127.0.0.1:9223");
const page = browser.contexts()[0].pages()[0];
await page.waitForLoadState("domcontentloaded");
await page.getByRole("button", { name: "CPU", exact: true }).waitFor();
const invoke = (command, args = {}) =>
  page.evaluate(
    ({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args),
    { command, args },
  );
const initial = (await invoke("get_monitor_state")).settings;
try {
  for (const theme of ["light", "dark", "system"]) {
    const state = await invoke("get_monitor_state");
    await invoke("update_settings", {
      settings: { ...state.settings, theme },
      expectedRevision: state.settings.revision,
    });
    await pollState(
      invoke,
      (s) => s.settings.revision === s.applied_settings_revision,
    );
    const expected =
      theme === "system"
        ? await invoke("plugin:window|theme", { label: "main" })
        : theme;
    await page.waitForFunction(
      (t) => document.documentElement.dataset.theme === t,
      expected,
    );
    await page.getByRole("button", { name: "CPU", exact: true }).click();
    await page.waitForTimeout(400);
    const window = JSON.parse(
      execFileSync(
        "powershell",
        [
          "-ExecutionPolicy",
          "Bypass",
          "-File",
          "tools/window-check.ps1",
          "-Action",
          "capture",
        ],
        { cwd: root, encoding: "utf8" },
      ),
    );
    const resolved = await page.evaluate(
      () => document.documentElement.dataset.theme,
    );
    assert.equal(window.ThemeResult, 0);
    assert.equal(window.DarkCaption, resolved === "dark");
    await copyFile(
      `${root}src/backend/target/measurements/release-window.png`,
      `${assets}sakani-native-${theme}.png`,
    );
    console.log(
      `PASS: ${theme}, resolved ${resolved}, native dark caption ${window.DarkCaption}, DPI ${window.Dpi}`,
    );
  }
} finally {
  const state = await invoke("get_monitor_state");
  await invoke("update_settings", {
    settings: { ...initial, revision: state.settings.revision },
    expectedRevision: state.settings.revision,
  });
  await browser.close();
}
