import { waitForSettingsSave } from "./settings-helpers.mjs";
import { chromium } from "@playwright/test";
import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";

await mkdir("test-results", { recursive: true });
const browser = await chromium.connectOverCDP("http://127.0.0.1:18800");
const context = await browser.newContext({
  viewport: { width: 1100, height: 760 },
});
const page = await context.newPage();
const errors = [];
page.on("pageerror", (e) => errors.push(String(e)));
await page.goto("http://127.0.0.1:1420/?demo=1");
await page.getByRole("heading", { name: "系统总览" }).waitFor();
for (const theme of ["light", "dark"]) {
  await page.getByRole("button", { name: "设置", exact: true }).click();
  await page
    .getByText(theme === "dark" ? "深色" : "浅色", { exact: true })
    .click();
  await waitForSettingsSave(page);
  await page.waitForFunction(
    (t) => document.documentElement.dataset.theme === t,
    theme,
  );
  for (const label of ["总览", "CPU", "内存", "网络", "设置"]) {
    await page.getByRole("button", { name: label, exact: true }).click();
    assert.equal(
      await page.locator("h1").innerText(),
      label === "总览" ? "系统总览" : label,
    );
    await page.screenshot({ path: `test-results/${theme}-${label}.png` });
    assert(
      await page
        .locator(".content")
        .evaluate((el) => el.scrollWidth <= el.clientWidth + 1),
      `${theme} ${label} overflow`,
    );
  }
}
for (const zoom of [1, 1.5, 2]) {
  await page.setViewportSize({
    width: Math.round(880 / zoom),
    height: Math.round(600 / zoom),
  });
  for (const label of ["总览", "CPU", "内存", "网络", "设置"]) {
    await page.getByRole("button", { name: label, exact: true }).click();
    assert(
      await page
        .locator(".content")
        .evaluate((el) => el.scrollWidth <= el.clientWidth + 1),
      `${zoom} ${label} overflow`,
    );
  }
  await page.screenshot({ path: `test-results/zoom-${zoom}.png` });
}
await page.setViewportSize({ width: 1100, height: 760 });
await page.getByRole("button", { name: "总览", exact: true }).click();
for (const status of [
  "warming",
  "failed",
  "stale",
  "unsupported",
  "permission_denied",
]) {
  await page.getByRole("combobox", { name: "演示状态" }).click();
  await page
    .getByRole("option", {
      name: {
        warming: "采样中",
        failed: "采集失败",
        stale: "数据过期",
        unsupported: "不支持",
        permission_denied: "权限不足",
      }[status],
      exact: true,
    })
    .click();
  assert(
    (await page.locator(".metric-link").allTextContents()).every((v) =>
      v.includes("—"),
    ),
  );
}
await page.screenshot({ path: "test-results/unavailable.png" });
await page.getByRole("combobox", { name: "演示状态" }).click();
await page.getByRole("option", { name: "实时", exact: true }).click();
await page.getByRole("tab", { name: "1 分钟", exact: true }).click();
const graph = page.locator(".chart-svg").first();
await graph.focus();
await page.keyboard.press("ArrowLeft");
await page.getByRole("button", { name: "返回实时" }).click();
const rect = await graph.boundingBox();
await page.mouse.move(rect.x + rect.width * 0.3, rect.y + 40);
await page.mouse.down();
await page.mouse.move(rect.x + rect.width * 0.7, rect.y + 40, { steps: 5 });
await page.mouse.up();
await page.getByRole("button", { name: "返回实时" }).waitFor();
await page.goto("http://127.0.0.1:1420/");
await page
  .getByText("请从 Pinmeter 桌面应用查看真实数据。浏览器未连接采集服务。")
  .waitFor();
assert(
  (await page.locator(".metric-link").allTextContents()).every((v) =>
    v.includes("—"),
  ),
);
assert.deepEqual(errors, []);
console.log(
  "PASS: five pages, themes, equivalent zoom layouts, five unavailable states, keyboard/pointer history, unavailable browser.",
);
await context.close();
await browser.close();
