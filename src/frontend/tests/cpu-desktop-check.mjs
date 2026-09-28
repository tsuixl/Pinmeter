import { chromium } from "@playwright/test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { pollState } from "./desktop-helpers.mjs";

const assets = fileURLToPath(
  new URL(
    "../../../docs/development/v0.1.0-main-window/assets/",
    import.meta.url,
  ),
);
const browser = await chromium.connectOverCDP("http://127.0.0.1:9223");
const page = browser.contexts()[0].pages()[0];
page.setDefaultTimeout(10000);
const invoke = (command, args = {}) =>
  page.evaluate(
    ({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args),
    { command, args },
  );
const initial = await invoke("get_monitor_state");
try {
  for (const theme of ["light", "dark"]) {
    const state = await invoke("get_monitor_state");
    await invoke("update_settings", {
      settings: { ...state.settings, theme, interval_ms: 1000 },
      expectedRevision: state.settings.revision,
    });
    const ready = await pollState(
      invoke,
      (s) =>
        s.settings.revision === s.applied_settings_revision &&
        s.cpu_processors.processors.length > 0 &&
        s.cpu_processors.processors.every((p) => p.usage.status === "normal"),
    );
    assert.equal(
      ready.cpu_processors.processors.length,
      Number(process.argv[2] ?? ready.cpu_processors.processors.length),
    );
    assert(
      ready.cpu_processors.processors.every(
        (p) => p.usage.value >= 0 && p.usage.value <= 100,
      ),
    );
    assert(ready.history.every((frame) => !("cpu_processors" in frame)));
    await page.getByRole("button", { name: "CPU", exact: true }).click();
    await page.waitForFunction(
      (t) => document.documentElement.dataset.theme === t,
      theme,
    );
    await page.locator(".cpu-processors").scrollIntoViewIfNeeded();
    await page.waitForFunction(
      (count) =>
        document.querySelectorAll('.cpu-bar-column[data-valid="true"]')
          .length === count,
      ready.cpu_processors.processors.length,
    );
    await page.waitForTimeout(350);
    await page.screenshot({ path: `${assets}cpu-bar-native-${theme}.png` });
    await page.locator(".cpu-bar-column").first().hover();
    await page.getByRole("tooltip").waitFor();
    assert.match(
      await page.getByRole("tooltip").innerText(),
      /逻辑处理器.*\n.*占用率/s,
    );
    await page.screenshot({
      path: `${assets}cpu-bar-native-${theme}-hover.png`,
    });
    const axisX = (await page.locator(".cpu-bar-axis").boundingBox()).x;
    await page.locator(".processor-scroll").hover();
    await page.mouse.wheel(600, 0);
    await page.waitForFunction(
      () => document.querySelector(".processor-scroll").scrollLeft > 0,
    );
    assert.equal((await page.locator(".cpu-bar-axis").boundingBox()).x, axisX);
    await page.locator(".cpu-bar-column").first().focus();
    await page.keyboard.press("End");
    await page.getByRole("tooltip").waitFor();
    assert.equal(
      await page.locator(":focus").getAttribute("data-processor-id"),
      ready.cpu_processors.processors.at(-1).id,
    );
    await page.waitForFunction(() =>
      document
        .querySelector(".cpu-bar-tooltip-title")
        ?.textContent?.includes(document.activeElement.dataset.processorId),
    );
    await page.screenshot({
      path: assets + "cpu-bar-native-" + theme + "-scroll.png",
    });
    await page.keyboard.press("Home");
    await page.keyboard.press("Escape");
    console.log(
      `Native ${theme}: ${ready.cpu_processors.processors.length} processors, DPR ${await page.evaluate(() => devicePixelRatio)}`,
    );
  }
  for (const interval_ms of [2000, 5000]) {
    const state = await invoke("get_monitor_state");
    await invoke("update_settings", {
      settings: { ...state.settings, interval_ms },
      expectedRevision: state.settings.revision,
    });
    await pollState(
      invoke,
      (s) =>
        s.settings.revision === s.applied_settings_revision &&
        s.cpu_processors.processors.length > 0 &&
        s.cpu_processors.processors.every((p) => p.usage.status === "normal"),
    );
    await page.getByRole("button", { name: "查看指标说明" }).click();
    assert(
      (await page.locator(".explanation").innerText()).includes(
        `${interval_ms / 1000} 秒`,
      ),
    );
    await page.keyboard.press("Escape");
  }
  console.log(
    "PASS: native CPU inventory, valid readings, light/dark, current-only payload and 1/2/5-second sampling.",
  );
} finally {
  const state = await invoke("get_monitor_state");
  await invoke("update_settings", {
    settings: { ...initial.settings, revision: state.settings.revision },
    expectedRevision: state.settings.revision,
  });
  await browser.close();
}
