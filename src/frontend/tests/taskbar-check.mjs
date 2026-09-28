import { waitForSettingsSave } from "./settings-helpers.mjs";
import { chromium, expect } from "@playwright/test";
import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const assets = fileURLToPath(
  new URL(
    "../../../docs/development/v0.1.0-taskbar-display/assets/",
    import.meta.url,
  ),
);
await mkdir(assets, { recursive: true });
const browser = await chromium.launch({ channel: "chrome", headless: true });
const page = await browser.newPage({ viewport: { width: 1150, height: 1000 } });
const errors = [];
page.on("pageerror", (error) => errors.push(String(error)));
page.setDefaultTimeout(8000);
try {
  await page.goto("http://127.0.0.1:1420/?demo=1");
  await page.getByRole("button", { name: "设置", exact: true }).click();
  const toggle = page.getByRole("switch", {
    name: "启用任务栏显示",
    exact: true,
  });
  const cpu = page.getByRole("checkbox", {
    name: "CPU 使用率与温度",
    exact: true,
  });
  const gpu = page.getByRole("checkbox", {
    name: "GPU 使用率与温度",
    exact: true,
  });
  const memory = page.getByRole("checkbox", {
    name: "内存使用率",
    exact: true,
  });
  const layout = page.getByRole("combobox", { name: "读数布局", exact: true });
  const preview = page.getByLabel("任务栏读数布局预览");
  await expect(
    page.getByRole("button", { name: "应用", exact: true }),
  ).toHaveCount(0);
  await expect(toggle).not.toBeChecked();
  await expect(cpu).toBeDisabled();
  await expect(gpu).toBeDisabled();
  await expect(layout).toBeDisabled();
  await page.getByText("启用任务栏显示", { exact: true }).click();
  await expect(cpu).toBeEnabled();
  await expect(cpu).toBeChecked();
  await expect(gpu).toBeChecked();
  await expect(memory).toBeChecked();
  await waitForSettingsSave(page);
  await expect(
    page.getByText("演示预览，不会嵌入系统任务栏", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("任务栏设置尚未应用", { exact: true }),
  ).toHaveCount(0);
  for (const theme of ["light", "dark"]) {
    await page
      .getByText(theme === "light" ? "浅色" : "深色", { exact: true })
      .click();
    await waitForSettingsSave(page);
    await page.waitForFunction(
      (value) => document.documentElement.dataset.theme === value,
      theme,
    );
    await expect(preview.getByText("GPU", { exact: true })).toBeVisible();
    await expect(preview.getByText("(57°C)", { exact: true })).toBeVisible();
    await expect(preview.getByText("(49°C)", { exact: true })).toBeVisible();
    await expect(preview).toHaveCSS("background-color", "rgba(0, 0, 0, 0)");
    await preview.scrollIntoViewIfNeeded();
    const columns = await preview
      .locator(".taskbar-preview-hardware > div")
      .evaluateAll((rows) =>
        rows.map((row) => ({
          percent: row.querySelector("strong > span").getBoundingClientRect().x,
          temperature: row
            .querySelector(".taskbar-preview-temperature")
            .getBoundingClientRect().x,
        })),
      );
    assert.equal(columns.length, 2);
    assert(
      Math.abs(columns[0].percent - columns[1].percent) < 0.5,
      "CPU 2% and GPU 18% must align at the percent sign",
    );
    assert(
      Math.abs(columns[0].temperature - columns[1].temperature) < 0.5,
      "temperature columns must align",
    );
    await page.screenshot({
      path: `${assets}/settings-${theme}-double.png`,
      animations: "disabled",
    });
    await layout.evaluate((element) =>
      element.scrollIntoView({ block: "center" }),
    );
    await layout.click();
    await page.getByRole("option", { name: "单行横排" }).click();
    await expect(preview).toHaveClass(/taskbar-preview-single/);
    await page.getByText("CPU 使用率与温度", { exact: true }).click();
    await expect(preview.getByText("CPU", { exact: true })).toHaveCount(0);
    await expect(preview.getByText("内存", { exact: true })).toHaveCount(1);
    await page.getByText("内存使用率", { exact: true }).click();
    await expect(preview.getByText("内存", { exact: true })).toHaveCount(0);
    await expect(preview.getByText("MB/s", { exact: true })).toHaveCount(1);
    await page.getByText("GPU 使用率与温度", { exact: true }).click();
    await expect(preview.getByText("GPU", { exact: true })).toHaveCount(0);
    await expect(preview.getByText("(49°C)", { exact: true })).toHaveCount(0);
    await page.getByText("GPU 使用率与温度", { exact: true }).click();
    await waitForSettingsSave(page);
    await layout.evaluate((element) =>
      element.scrollIntoView({ block: "center" }),
    );
    await layout.click();
    await page.getByRole("option", { name: "双行紧凑" }).click();
    await page.getByText("CPU 使用率与温度", { exact: true }).click();
    await page.getByText("内存使用率", { exact: true }).click();
    await waitForSettingsSave(page);
    await expect(preview).not.toHaveClass(/taskbar-preview-single/);
    await expect(cpu).toBeChecked();
  }
  // Intercept the demo repository only: verify real VM pending/error/draft behavior.
  await page.evaluate(async () => {
    const url = performance
      .getEntriesByType("resource")
      .find((e) => e.name.includes("/src/shared/client/demo-client.ts")).name;
    const { DemoClient } = await import(url);
    const original = DemoClient.prototype.updateSettings;
    DemoClient.prototype.updateSettings = function (settings) {
      globalThis.__taskbarClient = this;
      globalThis.__restoreSave = () => {
        DemoClient.prototype.updateSettings = original;
      };
      return new Promise((_, reject) => {
        globalThis.__failSave = () => reject(new Error("模拟写入失败"));
      });
    };
  });
  await layout.evaluate((element) =>
    element.scrollIntoView({ block: "center" }),
  );
  await layout.click();
  await page.getByRole("option", { name: "单行横排" }).click();
  await expect(toggle).toBeDisabled();
  await expect(cpu).toBeDisabled();
  await expect(gpu).toBeDisabled();
  await expect(layout).toBeDisabled();
  await page.screenshot({ path: `${assets}/settings-pending.png` });
  await page.evaluate(() => globalThis.__failSave());
  await expect(
    page.getByRole("alert").filter({ hasText: "模拟写入失败" }),
  ).toBeVisible();
  await expect(preview).not.toHaveClass(/taskbar-preview-single/);
  await page
    .getByRole("alert")
    .filter({ hasText: "模拟写入失败" })
    .scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${assets}/settings-error.png` });
  await page.evaluate(() => globalThis.__restoreSave());
  await layout.evaluate((element) =>
    element.scrollIntoView({ block: "center" }),
  );
  await layout.click();
  await page.getByRole("option", { name: "单行横排" }).click();
  await waitForSettingsSave(page);
  // A backend failure must remain visible even after settings are confirmed.
  await page.evaluate(() => {
    const c = globalThis.__taskbarClient,
      snap = c.getSnapshot();
    c.publish({
      ...snap,
      state: {
        ...snap.state,
        desktop: {
          ...snap.state.desktop,
          stage: "no_space",
          detail: "任务栏空间不足，已保留托盘入口",
        },
      },
    });
  });
  await expect(
    page.getByText("任务栏空间不足，已保留托盘入口", { exact: true }),
  ).toBeVisible();
  await expect(toggle).toBeChecked();
  await page.keyboard.press("Escape");
  await expect(page.getByRole("option", { name: "单行横排" })).toBeHidden();
  await page.setViewportSize({ width: 460, height: 850 });
  await preview.scrollIntoViewIfNeeded();
  assert(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  );
  assert(
    await page
      .locator(".content")
      .evaluate((e) => e.scrollWidth <= e.clientWidth),
    "Settings must not overflow the page when the single-row preview scrolls.",
  );
  await page.screenshot({ path: `${assets}/settings-narrow-single.png` });
  assert.deepEqual(errors, []);
  console.log(
    "PASS: taskbar immediate save/confirmed state, defaults, layout, metric selection, themes, pending, failed save rollback, unavailable display and narrow layout (browser demo only).",
  );
} finally {
  await browser.close();
}
