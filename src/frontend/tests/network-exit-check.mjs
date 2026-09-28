import { waitForSettingsSave } from "./settings-helpers.mjs";
import { chromium, expect } from "@playwright/test";
import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
const assets = fileURLToPath(
  new URL(
    "../../../docs/development/v0.1.0-app-network-control/assets/",
    import.meta.url,
  ),
);
await mkdir(assets, { recursive: true });
const browser = await chromium.launch({ channel: "chrome", headless: true });
const page = await browser.newPage({ viewport: { width: 1150, height: 980 } });
const errors = [];
page.on("pageerror", (e) => errors.push(String(e)));
page.setDefaultTimeout(8000);
try {
  // Browser-only window adapter exercises the real ViewModel and dialog; no OS rules are touched.
  await page.route("**/src/shared/client/window-client.ts*", async (route) => {
    const response = await route.fetch();
    const body = (await response.text()).replace(
      "if (!isTauri()) return undefined;",
      `if (!isTauri()) {
      globalThis.__exitStatus = { stage: "idle", detail: "" };
      globalThis.__exitCalls = 0;
      return {
        platform: "windows", initialTheme: "light",
        readState: async () => ({ maximized: false, fullscreen: false }),
        onResize: async () => () => {}, minimize: async () => {}, toggleMaximize: async () => {}, show: async () => {},
        readExitState: async () => globalThis.__exitStatus,
        onExitState: async listener => { globalThis.__emitExit = value => { globalThis.__exitStatus = value; listener(value); }; return () => {}; },
        close: async () => globalThis.__emitExit({ stage: "confirm", detail: "退出后，已禁用的应用仍无法联网；限速会停止。" }),
        confirmExit: async () => { globalThis.__exitCalls++; globalThis.__emitExit({ stage: "releasing", detail: "正在解除网络限制…" }); },
        cancelExit: async () => globalThis.__emitExit({ stage: "idle", detail: "" }),
      };
    }`,
    );
    assert.notEqual(
      body,
      await response.text(),
      "Window fixture was not installed",
    );
    await route.fulfill({ response, body });
  });
  await page.goto("http://127.0.0.1:1420/?demo=1");
  await page.getByRole("button", { name: "网络", exact: true }).waitFor();
  await page.evaluate(async () => {
    const url = performance
      .getEntriesByType("resource")
      .find((e) => e.name.includes("/src/shared/client/demo-client.ts")).name;
    const { DemoClient } = await import(url);
    const original = DemoClient.prototype.setAppNetworkMonitoring;
    DemoClient.prototype.setAppNetworkMonitoring = function (enabled) {
      globalThis.__client = this;
      return original.call(this, enabled);
    };
  });
  for (const theme of ["light", "dark"]) {
    await page.getByRole("button", { name: "设置", exact: true }).click();
    const toggle = page.getByRole("switch", {
      name: "退出 Pinmeter 时解除所有网络限制",
    });
    await expect(toggle).toBeChecked();
    await page
      .getByText(theme === "light" ? "浅色" : "深色", { exact: true })
      .click();
    await waitForSettingsSave(page);
    await page.waitForFunction(
      (value) => document.documentElement.dataset.theme === value,
      theme,
    );
    await page.waitForTimeout(1000);
    await page
      .getByText("退出 Pinmeter 时解除所有网络限制", { exact: true })
      .scrollIntoViewIfNeeded();
    await page.screenshot({
      path: `${assets}/exit-${theme}-settings.png`,
      animations: "disabled",
    });
    await page
      .getByText("退出 Pinmeter 时解除所有网络限制", { exact: true })
      .click();
    await expect(
      page.getByText(
        "退出后仅保留网络禁用，限速仍会停止。下次打开会重新应用已启用规则。",
        { exact: true },
      ),
    ).toBeVisible();
    await waitForSettingsSave(page);
    await page.getByRole("button", { name: "网络", exact: true }).click();
    await page
      .locator(".app-network")
      .getByRole("button", { name: "开始监控" })
      .click();
    await page.evaluate(async () => {
      const c = globalThis.__client;
      await c.changeNetworkControl({
        id: "app-0",
        action: "limits",
        download: 32000,
        upload: 16000,
        expectedRevision: c.getSnapshot().state.network_control.revision,
      });
    });
    const rules = page.getByRole("region", { name: "已配置网络规则" });
    await rules
      .getByRole("button", { name: "浏览器（演示） 网络控制" })
      .click();
    await page.getByRole("menuitem", { name: "禁用网络", exact: true }).click();
    await rules.getByRole("button", { name: "解除全部限制" }).click();
    await expect(rules.getByText("未启用", { exact: true })).toBeVisible();
    const saved = await page.evaluate(
      () => globalThis.__client.getSnapshot().state.network_control.rules[0],
    );
    assert.equal(saved.enabled, false);
    assert.equal(saved.blocked, true);
    assert.equal(saved.download, 32000);
    await rules.scrollIntoViewIfNeeded();
    await page.screenshot({ path: `${assets}/exit-${theme}-inactive.png` });
    await rules
      .getByRole("button", { name: "浏览器（演示） 网络控制" })
      .click();
    await page
      .getByRole("menuitem", { name: "启用已保存规则", exact: true })
      .click();
    await expect(rules.getByText("网络已禁用", { exact: true })).toBeVisible();
    await rules
      .getByRole("button", { name: "浏览器（演示） 网络控制" })
      .click();
    await page
      .getByRole("menuitem", { name: "停用规则（保留配置）", exact: true })
      .click();
    await expect(rules.getByText("未启用", { exact: true })).toBeVisible();
    await rules
      .getByRole("button", { name: "浏览器（演示） 网络控制" })
      .click();
    await page
      .getByRole("menuitem", { name: "设置限速…", exact: true })
      .click();
    await expect(
      page.getByRole("alertdialog", { name: "设置应用限速" }),
    ).toBeVisible();
    // Native Alt+F4/close can arrive while the limit dialog is open.
    await page.evaluate(() =>
      globalThis.__emitExit({
        stage: "confirm",
        detail: "退出后，已禁用的应用仍无法联网；限速会停止。",
      }),
    );
    await expect(
      page.getByRole("alertdialog", { name: "退出后保留网络禁用？" }),
    ).toBeVisible();
    await page.screenshot({ path: `${assets}/exit-${theme}-confirm.png` });
    await page.getByRole("button", { name: "返回应用", exact: true }).click();
    await expect(
      page.getByRole("alertdialog", { name: "设置应用限速" }),
    ).toBeVisible();
    await page.getByRole("button", { name: "取消", exact: true }).click();
    await expect(page.getByRole("alertdialog")).toHaveCount(0);
    await page.evaluate(() =>
      globalThis.__emitExit({
        stage: "failed",
        detail: "未退出 Pinmeter。浏览器的网络限制未能解除，请重试。",
      }),
    );
    await expect(
      page.getByRole("alertdialog", { name: "未能解除全部限制" }),
    ).toBeVisible();
    await page.screenshot({ path: `${assets}/exit-${theme}-failed.png` });
    await page.getByRole("button", { name: "重试并退出", exact: true }).click();
    await expect(
      page.getByRole("alertdialog", { name: "正在退出 Pinmeter" }),
    ).toBeVisible();
    await page.keyboard.press("Escape");
    await expect(page.getByRole("alertdialog")).toBeVisible();
    await page.getByRole("button", { name: "返回应用", exact: true }).click();
    await expect(page.getByRole("alertdialog")).toBeVisible();
    await page.evaluate(() =>
      globalThis.__emitExit({ stage: "idle", detail: "" }),
    );
    await page.getByRole("button", { name: "设置", exact: true }).click();
    await page
      .getByText("退出 Pinmeter 时解除所有网络限制", { exact: true })
      .click();
    await waitForSettingsSave(page);
  }
  assert.equal(await page.evaluate(() => globalThis.__exitCalls), 2);
  await page.setViewportSize({ width: 460, height: 800 });
  await expect(
    page.getByRole("switch", { name: "退出 Pinmeter 时解除所有网络限制" }),
  ).toBeChecked();
  await page.waitForTimeout(1000);
  await page
    .getByText("退出 Pinmeter 时解除所有网络限制", { exact: true })
    .scrollIntoViewIfNeeded();
  assert(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  );
  await page.screenshot({ path: `${assets}/exit-narrow-settings.png` });
  assert.deepEqual(errors, []);
  // Capture the actual upstream component states used by this change.
  await page.unrouteAll();
  await page.setViewportSize({ width: 1000, height: 700 });
  for (const story of [
    "forms-switch--with-label",
    "forms-switch--disabled",
    "forms-switch--dark-mode",
  ]) {
    await page.goto(
      `https://main--6a5a658b3681fcc010430db5.chromatic.com/iframe.html?id=${story}&viewMode=story`,
    );
    await page
      .locator("#storybook-root input")
      .first()
      .waitFor({ state: "attached" });
    await page.screenshot({ path: `${assets}/sakani-${story}.png` });
  }
  console.log(
    "PASS: exit preference, retained inactive rules, enable/disable, release all, exit dialog states, themes and narrow layout; browser fixtures only",
  );
} finally {
  await browser.close();
}
