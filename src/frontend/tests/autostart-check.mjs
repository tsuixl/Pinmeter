import { chromium, expect } from "@playwright/test";
import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { waitForSettingsSave } from "./settings-helpers.mjs";

const assets = "../../docs/development/v0.1.0-preferences/assets";
await mkdir(assets, { recursive: true });
const browser = await chromium.launch({ channel: "chrome", headless: true });
const page = await browser.newPage({ viewport: { width: 1100, height: 1000 } });
const errors = [];
page.on("pageerror", (e) => errors.push(String(e)));
try {
  await page.goto("http://127.0.0.1:1420/?demo=1");
  await page.getByRole("button", { name: "设置", exact: true }).click();
  const toggle = page.getByRole("switch", { name: "开机自启", exact: true });
  await expect(toggle).not.toBeChecked();
  await page.evaluate(async () => {
    const url = performance
      .getEntriesByType("resource")
      .find((e) => e.name.includes("/src/shared/client/demo-client.ts")).name;
    const { DemoClient } = await import(url);
    const original = DemoClient.prototype.updateSettings;
    DemoClient.prototype.updateSettings = function (settings) {
      globalThis.__client = this;
      if (globalThis.__hold)
        return new Promise((resolve, reject) => {
          globalThis.__finish = () =>
            original.call(this, settings).then(resolve, reject);
          globalThis.__reject = () => reject(new Error("系统拒绝注册启动项"));
        });
      return original.call(this, settings);
    };
  });
  for (const theme of ["light", "dark"]) {
    await page
      .getByText(theme === "light" ? "浅色" : "深色", { exact: true })
      .click();
    await waitForSettingsSave(page);
    await toggle.focus();
    await page.screenshot({
      animations: "disabled",
      path: `${assets}/autostart-${theme}-off-focus.png`,
    });
    await page.keyboard.press("Space");
    await waitForSettingsSave(page);
    await expect(toggle).toBeChecked();
    await page.screenshot({
      animations: "disabled",
      path: `${assets}/autostart-${theme}-on.png`,
    });
    await page.evaluate(() => {
      globalThis.__hold = true;
    });
    await page.getByText("开机自启", { exact: true }).click();
    await expect(toggle).toBeDisabled();
    await expect(
      page.getByRole("button", { name: "恢复默认", exact: true }),
    ).toBeDisabled();
    await page.screenshot({
      animations: "disabled",
      path: `${assets}/autostart-${theme}-pending.png`,
    });
    await page.evaluate(() => globalThis.__reject());
    await expect(toggle).toBeChecked();
    await expect(toggle).toBeEnabled();
    await expect(
      page.getByRole("alert").filter({ hasText: "系统拒绝注册启动项" }),
    ).toBeVisible();
    await page.screenshot({
      animations: "disabled",
      path: `${assets}/autostart-${theme}-error.png`,
    });
    await page.getByText("开机自启", { exact: true }).click();
    await page.evaluate(() => globalThis.__finish());
    await waitForSettingsSave(page);
    await expect(toggle).not.toBeChecked();
    await page.evaluate(() => {
      globalThis.__hold = false;
    });
  }
  await page.getByText("开机自启", { exact: true }).click();
  await waitForSettingsSave(page);
  await page.getByRole("button", { name: "总览", exact: true }).click();
  await page.getByRole("button", { name: "设置", exact: true }).click();
  await expect(toggle).toBeChecked();
  await page.getByRole("button", { name: "恢复默认", exact: true }).click();
  await waitForSettingsSave(page);
  await expect(toggle).not.toBeChecked();
  await page.setViewportSize({ width: 460, height: 800 });
  await toggle.scrollIntoViewIfNeeded();
  await page.screenshot({
    animations: "disabled",
    path: `${assets}/autostart-narrow.png`,
  });
  assert(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  );
  await page.evaluate(() => {
    const client = globalThis.__client;
    client.publish({
      ...client.snapshot,
      state: {
        ...client.snapshot.state,
        autostart: {
          available: false,
          enabled: false,
          detail: "当前平台尚不支持开机自启",
        },
      },
    });
  });
  await expect(toggle).toBeDisabled();
  assert.deepEqual(errors, []);
  console.log(
    "PASS: default off, instant enable/disable, keyboard, both themes, pending, rollback, retry, navigation, reset, narrow layout, unsupported platform.",
  );
} finally {
  await browser.close();
}
