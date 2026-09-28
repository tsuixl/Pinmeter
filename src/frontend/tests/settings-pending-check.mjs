import { chromium, expect } from "@playwright/test";
import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { waitForSettingsSave } from "./settings-helpers.mjs";

const assets = "../../docs/development/v0.1.0-preferences/assets";
await mkdir(assets, { recursive: true });
const browser = await chromium.launch({ channel: "chrome", headless: true });
const page = await browser.newPage({ viewport: { width: 1150, height: 980 } });
const errors = [];
page.on("pageerror", (error) => errors.push(String(error)));
try {
  await page.goto("http://127.0.0.1:1420/?demo=1");
  await page.getByRole("button", { name: "设置", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "应用", exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "放弃更改", exact: true }),
  ).toHaveCount(0);
  await page.evaluate(async () => {
    const url = performance
      .getEntriesByType("resource")
      .find((e) => e.name.includes("/src/shared/client/demo-client.ts")).name;
    const { DemoClient } = await import(url);
    const original = DemoClient.prototype.updateSettings;
    globalThis.__saves = [];
    DemoClient.prototype.updateSettings = function (settings) {
      globalThis.__settingsClient = this;
      globalThis.__saves.push(settings);
      if (globalThis.__holdSave)
        return new Promise((resolve, reject) => {
          globalThis.__finishSave = () =>
            original.call(this, settings).then(resolve, reject);
          globalThis.__rejectSave = () => reject(new Error("模拟写入失败"));
        });
      return original.call(this, settings);
    };
  });
  for (const theme of ["light", "dark"]) {
    await page
      .getByText(theme === "light" ? "浅色" : "深色", { exact: true })
      .click();
    await waitForSettingsSave(page);
    await expect(
      page.getByRole("radio", {
        name: theme === "light" ? "浅色" : "深色",
        exact: true,
      }),
    ).toBeChecked();
    await page.waitForFunction(
      (t) => document.documentElement.dataset.theme === t,
      theme,
    );
    await page.screenshot({ path: `${assets}/instant-${theme}.png` });
  }
  const interval = page.getByRole("combobox", {
    name: "采样间隔",
    exact: true,
  });
  await interval.click();
  await page.getByRole("option", { name: "2 秒", exact: true }).click();
  await waitForSettingsSave(page);
  assert.equal(
    await page.evaluate(
      () =>
        globalThis.__settingsClient.getSnapshot().state.settings.interval_ms,
    ),
    2000,
  );
  const toggle = page.getByRole("switch", {
    name: "启用任务栏显示",
    exact: true,
  });
  await page.evaluate(() => {
    globalThis.__holdSave = true;
  });
  await page.getByText("启用任务栏显示", { exact: true }).click();
  await expect(toggle).toBeChecked();
  await expect(toggle).toBeDisabled();
  await expect(
    page.getByRole("button", { name: "恢复默认", exact: true }),
  ).toBeDisabled();
  assert.equal(
    await page.evaluate(
      () =>
        globalThis.__settingsClient.getSnapshot().state.settings.taskbar
          .enabled,
    ),
    false,
  );
  const count = await page.evaluate(() => globalThis.__saves.length);
  await page.screenshot({ path: `${assets}/instant-pending.png` });
  await page.evaluate(() => globalThis.__rejectSave());
  await expect(toggle).not.toBeChecked();
  await expect(toggle).toBeEnabled();
  await expect(
    page.getByRole("alert").filter({ hasText: "模拟写入失败" }),
  ).toBeVisible();
  await page
    .getByRole("alert")
    .filter({ hasText: "模拟写入失败" })
    .scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${assets}/instant-error.png` });
  assert.equal(await page.evaluate(() => globalThis.__saves.length), count);
  await page.getByText("启用任务栏显示", { exact: true }).click();
  await page.evaluate(() => globalThis.__finishSave());
  await waitForSettingsSave(page);
  await expect(toggle).toBeChecked();
  assert.equal(
    await page.evaluate(
      () =>
        globalThis.__settingsClient.getSnapshot().state.settings.taskbar
          .enabled,
    ),
    true,
  );
  await page.evaluate(() => {
    globalThis.__holdSave = false;
  });
  await page.getByRole("button", { name: "总览", exact: true }).click();
  await page.getByRole("button", { name: "设置", exact: true }).click();
  await expect(toggle).toBeChecked();
  await expect(interval).toContainText("2 秒");
  await page.getByRole("button", { name: "恢复默认", exact: true }).click();
  await waitForSettingsSave(page);
  await expect(toggle).not.toBeChecked();
  await expect(interval).toContainText("1 秒");
  const revisions = await page.evaluate(() =>
    globalThis.__saves.map((s) => s.revision),
  );
  for (let i = 1; i < revisions.length; i++)
    assert(BigInt(revisions[i]) >= BigInt(revisions[i - 1]));
  await page.setViewportSize({ width: 460, height: 800 });
  await page.screenshot({ path: `${assets}/instant-narrow.png` });
  assert(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  );
  assert.deepEqual(errors, []);
  console.log(
    "PASS: instant theme/interval/taskbar saves, pending guard, failure rollback, retry, navigation, reset and narrow layout.",
  );
} finally {
  await browser.close();
}
