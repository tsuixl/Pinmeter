import { chromium } from "@playwright/test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { pollState } from "./desktop-helpers.mjs";

const assets = fileURLToPath(
  new URL(
    "../../../docs/development/v0.1.0-cpu-temperature/assets/",
    import.meta.url,
  ),
);
const browser = await chromium.connectOverCDP("http://127.0.0.1:9223");
const page = browser.contexts()[0].pages()[0];
const invoke = (command, args = {}) =>
  page.evaluate(
    ({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args),
    { command, args },
  );
const initial = await invoke("get_monitor_state");
const expected = process.argv[2] ?? "unsupported";
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
        s.cpu_temperature.status === expected &&
        s.cpu_processors.processors.length > 0 &&
        s.cpu_processors.processors.every((p) => p.usage.status === "normal"),
    );
    assert.equal(ready.cpu_temperature.unit, "°C");
    assert(ready.history.every((frame) => frame.cpu_temperature.unit === "°C"));
    assert(ready.history.length <= 301);
    if (expected === "normal") {
      assert(Number.isFinite(ready.cpu_temperature.value));
      assert(
        ready.cpu_temperature.value >= -50 &&
          ready.cpu_temperature.value <= 150,
      );
    } else assert.equal(ready.cpu_temperature.value, null);
    await page.getByRole("button", { name: "CPU", exact: true }).click();
    await page.waitForFunction(
      (t) => document.documentElement.dataset.theme === t,
      theme,
    );
    await page.waitForTimeout(350);
    const card = page.locator(".cpu-temperature");
    await card.scrollIntoViewIfNeeded();
    await page.waitForFunction(
      (status) =>
        document
          .querySelector(".cpu-temperature")
          ?.textContent?.includes(status === "normal" ? "°C" : "—"),
      expected,
    );
    const usage = await page
      .locator(".cpu-summary > :first-child")
      .boundingBox();
    const temperature = await card.boundingBox();
    assert.equal(usage.y, temperature.y);
    assert(Math.abs(usage.width - temperature.width) < 1);
    assert(temperature.x >= usage.x + usage.width);
    if (expected === "normal") {
      await page.waitForFunction(() =>
        document.querySelector('[data-metric="cpu_temperature"] .series'),
      );
      const svg = await page.locator(".trend.dual-axis svg").boundingBox();
      await page.mouse.move(svg.x + svg.width - 2, svg.y + svg.height / 2);
      await page.getByRole("tooltip").waitFor();
      assert.match(await page.getByRole("tooltip").innerText(), /CPU 温度/);
      assert.match(await page.getByRole("tooltip").innerText(), /°C/);
    }
    if (expected === "unsupported") {
      assert.match(await card.innerText(), /PawnIO/);
    }
    assert.equal(
      await page.getByRole("button", { name: "授权读取温度" }).count(),
      0,
    );
    await page.screenshot({
      path: assets + `temperature-native-${expected}-${theme}.png`,
    });
    console.log(
      JSON.stringify({
        theme,
        temperature: ready.cpu_temperature,
        processors: ready.cpu_processors.processors.length,
      }),
    );
  }
  console.log(
    "PASS: native temperature history, independent CPU readings, equal-width row, trend tooltip and themes",
  );
} finally {
  const state = await invoke("get_monitor_state");
  await invoke("update_settings", {
    settings: { ...initial.settings, revision: state.settings.revision },
    expectedRevision: state.settings.revision,
  });
  await browser.close();
}
