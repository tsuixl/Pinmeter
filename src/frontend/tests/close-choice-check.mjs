import { waitForSettingsSave } from "./settings-helpers.mjs";
import { chromium, expect } from "@playwright/test";
import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const assets = fileURLToPath(
  new URL(
    "../../../docs/development/v0.1.0-desktop-runtime/assets/",
    import.meta.url,
  ),
);
await mkdir(assets, { recursive: true });
const browser = await chromium.launch({ channel: "chrome", headless: true });
const page = await browser.newPage({ viewport: { width: 1150, height: 980 } });
const errors = [];
page.on("pageerror", (error) => errors.push(String(error)));
page.setDefaultTimeout(8000);
try {
  // Exercise the actual ViewModel/Modal with a browser-only native adapter.
  await page.route("**/src/shared/client/window-client.ts*", async (route) => {
    const response = await route.fetch();
    const original = await response.text();
    const body = original.replace(
      "if (!isTauri()) return undefined;",
      `if (!isTauri()) {
      globalThis.__closeCalls = [];
      globalThis.__closeStatus = {stage: "idle", detail: ""};
      return {
        platform: "windows", initialTheme: "light",
        readState: async () => ({maximized: false, fullscreen: false}),
        onResize: async () => () => {}, minimize: async () => {}, toggleMaximize: async () => {}, show: async () => {},
        readExitState: async () => globalThis.__closeStatus,
        onExitState: async listener => { globalThis.__emitClose = value => {globalThis.__closeStatus=value;listener(value);};return () => {}; },
        close: async () => globalThis.__emitClose({stage: "choose_close", detail: ""}),
        cancelExit: async () => globalThis.__emitClose({stage: "idle", detail: ""}),
        confirmExit: async () => {},
        resolveClose: async (action, remember) => {
          globalThis.__closeCalls.push({action, remember});
          if (globalThis.__failClose) throw new Error("模拟配置保存失败");
          if (globalThis.__holdClose) await new Promise(resolve => globalThis.__releaseClose=resolve);
          globalThis.__emitClose({stage: action === "exit" ? "releasing" : "idle", detail: ""});
        }
      };
    }`,
    );
    assert.notEqual(body, original, "Native fixture must be installed");
    await route.fulfill({ response, body });
  });
  await page.goto("http://127.0.0.1:1420/?demo=1");
  await page.getByRole("button", { name: "设置", exact: true }).click();
  const preference = page.getByRole("combobox", {
    name: "关闭窗口时",
    exact: true,
  });
  await expect(preference).toContainText("每次询问");
  await preference.click();
  await page.getByRole("option", { name: "最小化到托盘", exact: true }).click();
  await waitForSettingsSave(page);
  await page.getByRole("button", { name: "总览", exact: true }).click();
  await page.getByRole("button", { name: "设置", exact: true }).click();
  await expect(preference).toContainText("最小化到托盘");
  await preference.click();
  await page.getByRole("option", { name: "每次询问", exact: true }).click();
  await waitForSettingsSave(page);

  const open = () =>
    page.evaluate(() =>
      globalThis.__emitClose({ stage: "choose_close", detail: "" }),
    );
  const dialog = page.getByRole("alertdialog", {
    name: "关闭 Pinmeter",
    exact: true,
  });
  const remember = dialog.getByRole("checkbox", {
    name: /^记住我的选择/,
  });
  for (const theme of ["light", "dark"]) {
    await page
      .getByText(theme === "light" ? "浅色" : "深色", { exact: true })
      .click();
    await waitForSettingsSave(page);
    await page.waitForFunction(
      (value) => document.documentElement.dataset.theme === value,
      theme,
    );
    await open();
    await expect(dialog).toBeVisible();
    await expect(
      dialog.getByRole("button", { name: "最小化到托盘", exact: true }),
    ).toHaveClass(/button--primary/);
    await expect(
      dialog.getByRole("button", { name: "退出", exact: true }),
    ).not.toHaveClass(/button--primary/);
    await expect(remember).not.toBeChecked();
    await page.screenshot({
      path: `${assets}/close-${theme}.png`,
      animations: "disabled",
    });
    await dialog.getByText("记住我的选择", { exact: true }).click();
    await expect(remember).toBeChecked();
    await page.screenshot({
      path: `${assets}/close-${theme}-remember.png`,
      animations: "disabled",
    });
    await dialog
      .getByRole("button", { name: "最小化到托盘", exact: true })
      .click();
    await expect(dialog).toHaveCount(0);
    assert.deepEqual(
      await page.evaluate(() => globalThis.__closeCalls.at(-1)),
      { action: "minimize", remember: true },
    );
    await open();
    await expect(remember).not.toBeChecked();
    const count = await page.evaluate(() => globalThis.__closeCalls.length);
    await page.keyboard.press("Escape");
    await expect(dialog).toHaveCount(0);
    assert.equal(
      await page.evaluate(() => globalThis.__closeCalls.length),
      count,
    );
  }
  await open();
  await page.evaluate(() => {
    globalThis.__failClose = true;
  });
  await dialog.getByText("记住我的选择", { exact: true }).click();
  await dialog.getByRole("button", { name: "退出", exact: true }).click();
  await expect(
    dialog.getByText("Error: 模拟配置保存失败", { exact: true }),
  ).toBeVisible();
  await expect(remember).toBeChecked();
  await page.screenshot({ path: `${assets}/close-save-error.png` });
  await page.evaluate(() => {
    globalThis.__failClose = false;
    globalThis.__holdClose = true;
  });
  await dialog.getByRole("button", { name: "退出", exact: true }).click();
  await expect(remember).toBeDisabled();
  const count = await page.evaluate(() => globalThis.__closeCalls.length);
  await page.keyboard.press("Escape");
  await expect(dialog).toBeVisible();
  await dialog.getByRole("button", { name: "退出", exact: true }).click();
  assert.equal(
    await page.evaluate(() => globalThis.__closeCalls.length),
    count,
  );
  await page.evaluate(() => globalThis.__releaseClose());
  await expect(
    page.getByRole("alertdialog", { name: "正在退出 Pinmeter" }),
  ).toBeVisible();
  assert.deepEqual(await page.evaluate(() => globalThis.__closeCalls.at(-1)), {
    action: "exit",
    remember: true,
  });
  await page.evaluate(() => {
    globalThis.__emitClose({ stage: "idle", detail: "" });
    globalThis.__holdClose = false;
  });
  await page.setViewportSize({ width: 460, height: 800 });
  await open();
  await expect(dialog).toBeVisible();
  assert(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  );
  await page.screenshot({
    path: `${assets}/close-narrow.png`,
    animations: "disabled",
  });
  await dialog
    .getByRole("button", { name: "最小化到托盘", exact: true })
    .click();
  await expect(dialog).toHaveCount(0);
  assert.deepEqual(await page.evaluate(() => globalThis.__closeCalls.at(-1)), {
    action: "minimize",
    remember: false,
  });
  assert.deepEqual(errors, []);
  console.log(
    "PASS: close choice, preference editing, remember/reset, save failure, pending/reentry, exit handoff, themes and narrow layout (browser adapter).",
  );
} finally {
  await browser.close();
}
