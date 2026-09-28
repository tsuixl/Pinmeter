import { chromium, expect } from "@playwright/test";
import assert from "node:assert/strict";
import fs from "node:fs";
import { fileURLToPath } from "node:url";
import { waitForSettingsSave } from "./settings-helpers.mjs";

const assets = fileURLToPath(
  new URL(
    "../../../docs/development/v0.1.0-hardware-info/assets/",
    import.meta.url,
  ),
);
fs.mkdirSync(assets, { recursive: true });
const browser = await chromium.connectOverCDP("http://127.0.0.1:18800");
const context = await browser.newContext({
  viewport: { width: 1100, height: 850 },
  permissions: ["clipboard-read", "clipboard-write"],
});
const page = await context.newPage();
const errors = [];
page.on("pageerror", (error) => errors.push(String(error)));
try {
  await page.goto("http://127.0.0.1:1420/?demo=1");
  await page.getByRole("button", { name: "硬件信息", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "复制信息", exact: true }),
  ).toBeEnabled();
  assert.equal(await page.locator(".hardware-row").count(), 8);
  assert.equal(await page.locator(".hardware-row-summary").count(), 8);
  assert.equal(await page.locator(".hardware-row li").count(), 0);
  await expect(page.locator('[data-hardware="memory"]')).toContainText(
    "48 GiB",
  );
  await expect(page.locator('[data-hardware="board"]')).not.toContainText(
    "BIOS",
  );
  const contentBottom = await page
    .locator(".content")
    .evaluate((el) => el.getBoundingClientRect().bottom);
  assert(
    (await page.locator('[data-hardware="network"]').boundingBox()).y +
      (await page.locator('[data-hardware="network"]').boundingBox()).height <=
      contentBottom,
    "All eight overview rows should fit in the standard viewport",
  );
  assert.equal(await page.getByLabel("趋势时间范围").count(), 0);
  await page.getByRole("button", { name: "复制信息", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("已复制硬件信息");
  assert.match(
    await page.evaluate(() => navigator.clipboard.readText()),
    /主板：.*ROG/,
  );
  assert(
    !/BIOS/.test(await page.evaluate(() => navigator.clipboard.readText())),
  );
  await page.getByRole("button", { name: "详细信息", exact: true }).click();
  await expect(page.locator('[data-hardware="board"]')).toContainText(
    "BIOS 2202",
  );
  await expect(
    page.getByRole("button", { name: "收起详情", exact: true }),
  ).toHaveAttribute("aria-expanded", "true");
  await page.getByRole("button", { name: "复制信息", exact: true }).click();
  assert.match(
    await page.evaluate(() => navigator.clipboard.readText()),
    /BIOS 2202/,
  );
  await page.getByRole("button", { name: "收起详情", exact: true }).click();
  for (const theme of ["light", "dark"]) {
    await page.getByRole("button", { name: "设置", exact: true }).click();
    await page
      .getByText(theme === "light" ? "浅色" : "深色", { exact: true })
      .click();
    await waitForSettingsSave(page);
    await page.getByRole("button", { name: "硬件信息", exact: true }).click();
    await expect(
      page.getByRole("button", { name: "复制信息", exact: true }),
    ).toBeEnabled();
    await page.waitForFunction(
      (t) => document.documentElement.dataset.theme === t,
      theme,
    );
    await page.getByRole("button", { name: "复制信息", exact: true }).focus();
    await page.waitForTimeout(350);
    await page.screenshot({ path: `${assets}hardware-compact-${theme}.png` });
  }
  await page.setViewportSize({ width: 440, height: 850 });
  assert(
    await page
      .locator(".hardware-page")
      .evaluate((el) => el.scrollWidth <= el.clientWidth + 1),
  );
  await page.screenshot({ path: `${assets}hardware-compact-narrow.png` });
  // The demo fixture is replaced at the module boundary to exercise loading and category failure.
  await page.route("**/demo-hardware.ts*", (route) =>
    route.fulfill({
      contentType: "application/javascript",
      body: `export function demoHardware() { return {status:'loading',detail:'',sections:[]}; }`,
    }),
  );
  await page.reload();
  await page.getByRole("button", { name: "硬件信息", exact: true }).click();
  await expect(
    page.getByRole("button", { name: "复制信息", exact: true }),
  ).toBeDisabled();
  await expect(page.locator(".hardware-rows")).toHaveAttribute(
    "aria-busy",
    "true",
  );
  await page.screenshot({ path: `${assets}hardware-loading.png` });
  await page.unroute("**/demo-hardware.ts*");
  await page.route("**/demo-hardware.ts*", (route) =>
    route.fulfill({
      contentType: "application/javascript",
      body: `export function demoHardware() { return {status:'ready',detail:'',sections:[{id:'display',items:[],error:'系统未能提供此类信息'}]}; }`,
    }),
  );
  await page.reload();
  await page.getByRole("button", { name: "硬件信息", exact: true }).click();
  await expect(page.locator('[data-hardware="display"]')).toContainText(
    "系统未能提供此类信息",
  );
  await expect(page.locator('[data-hardware="cpu"]')).toContainText("Ryzen");
  await page.screenshot({ path: `${assets}hardware-partial.png` });
  await page.unroute("**/demo-hardware.ts*");
  await page.route("**/demo-hardware.ts*", (route) =>
    route.fulfill({
      contentType: "application/javascript",
      body: `export function demoHardware() { return {status:'failed',detail:'硬件查询超时',sections:[]}; }`,
    }),
  );
  await page.reload();
  await page.getByRole("button", { name: "硬件信息", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("硬件查询超时");
  await expect(
    page.getByRole("button", { name: "复制信息", exact: true }),
  ).toBeDisabled();
  assert.deepEqual(errors, []);
  console.log(
    "Hardware UI: eight categories, copy, navigation, light/dark, narrow, loading, partial failure and failed query passed.",
  );
} finally {
  await context.close();
  await browser.close();
}
