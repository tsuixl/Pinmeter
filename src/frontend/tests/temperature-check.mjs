import { waitForSettingsSave } from "./settings-helpers.mjs";
import { chromium } from "@playwright/test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
const assets = fileURLToPath(
  new URL(
    "../../../docs/development/v0.1.0-cpu-temperature/assets/",
    import.meta.url,
  ),
);
const browser = await chromium.connectOverCDP("http://127.0.0.1:18800");
const context = await browser.newContext({
  viewport: { width: 1100, height: 900 },
});
const page = await context.newPage();
const errors = [];
page.on("pageerror", (error) => errors.push(String(error)));
try {
  await page.goto("http://127.0.0.1:1420/?demo=1");
  await page.getByRole("button", { name: "CPU", exact: true }).click();
  const card = page.locator(".cpu-temperature");
  await card.waitFor();
  await page.evaluate(async () => {
    const url = performance
      .getEntriesByType("resource")
      .find((e) => e.name.includes("/src/shared/client/demo-client.ts")).name;
    const { DemoClient } = await import(url);
    const original = DemoClient.prototype.temperature;
    DemoClient.prototype.temperature = function () {
      window.__temperatureClient = this;
      return window.__temperatureFixture ?? original.call(this);
    };
  });
  await page.waitForFunction(() => !!window.__temperatureClient);
  for (const theme of ["light", "dark"]) {
    await page.getByRole("button", { name: "设置", exact: true }).click();
    await page
      .getByText(theme === "light" ? "浅色" : "深色", { exact: true })
      .click();
    await waitForSettingsSave(page);
    await page.waitForFunction(
      (t) => document.documentElement.dataset.theme === t,
      theme,
    );
    await page.getByRole("button", { name: "CPU", exact: true }).click();
    assert.match(await card.innerText(), /\d+\.\d °C/);
    const summary = await page
      .locator(".cpu-summary > :first-child")
      .boundingBox();
    const temperature = await card.boundingBox();
    assert.equal(summary.y, temperature.y);
    assert(Math.abs(summary.width - temperature.width) < 1);
    assert(temperature.x >= summary.x + summary.width);
    await page.waitForTimeout(350);
    const trend = page.locator(".trend.dual-axis");
    assert.match(await trend.locator(".temperature-axis").innerText(), /°C/);
    assert(
      (await trend
        .locator('[data-metric="cpu_temperature"] .series')
        .count()) >= 2,
    );
    const svg = await trend.locator("svg").boundingBox();
    await page.mouse.move(svg.x + svg.width * 0.9, svg.y + svg.height / 2);
    const tooltip = page.getByRole("tooltip");
    await tooltip.waitFor();
    assert.match(await tooltip.innerText(), /CPU 温度/);
    assert.match(await tooltip.innerText(), /°C/);
    assert.match(await tooltip.innerText(), /%/);
    await page.screenshot({ path: assets + `temperature-${theme}.png` });
  }
  for (const status of [
    "normal",
    "warming",
    "unsupported",
    "permission_denied",
    "failed",
    "stale",
  ]) {
    await page.evaluate((status) => {
      const client = window.__temperatureClient;
      window.__temperatureFixture = {
        ...client.getSnapshot().state.cpu_temperature,
        value: status === "normal" ? 0 : null,
        text: status === "normal" ? "0.0" : "—",
        status,
        detail: status === "normal" ? "" : "温度传感器状态：" + status,
      };
      client.setScenario("normal");
    }, status);
    assert.equal(
      await page.locator(".cpu-bar-column[data-valid=true]").count(),
      16,
    );
    assert(
      (await card.innerText()).includes(status === "normal" ? "0.0 °C" : "—"),
    );
    assert.equal(
      await page.getByRole("button", { name: "授权读取温度" }).count(),
      0,
    );
    if (status === "stale")
      await page.screenshot({ path: assets + "temperature-stale.png" });
  }
  await page.setViewportSize({ width: 420, height: 700 });
  assert(
    await page
      .locator(".content")
      .evaluate((n) => n.scrollWidth <= n.clientWidth + 1),
  );
  await page.screenshot({ path: assets + "temperature-narrow.png" });
  assert.deepEqual(errors, []);
  console.log(
    "PASS: temperature equal-width row, dual-axis trend and tooltip, zero, independent invalid states and themes",
  );
} finally {
  await context.close();
  await browser.close();
}
