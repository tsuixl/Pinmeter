import { waitForSettingsSave } from "./settings-helpers.mjs";
import { chromium } from "@playwright/test";
import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
const assets = fileURLToPath(
  new URL(
    "../../../docs/development/v0.1.0-app-network-ranking/assets/",
    import.meta.url,
  ),
);
await mkdir(assets, { recursive: true });
const browser = process.env.BROWSER_CDP
  ? await chromium.connectOverCDP(process.env.BROWSER_CDP)
  : await chromium.launch({ channel: "chrome", headless: true });
const context = await browser.newContext({
  viewport: { width: 1150, height: 980 },
});
const page = await context.newPage();
page.setDefaultTimeout(12000);
const errors = [];
page.on("pageerror", (error) => errors.push(String(error)));
try {
  await page.goto("http://127.0.0.1:1420/?demo=1");
  await page.evaluate(async () => {
    const url = performance
      .getEntriesByType("resource")
      .find((e) => e.name.includes("/src/shared/client/demo-client.ts"))?.name;
    if (!url) throw new Error("Demo module missing");
    const { DemoClient } = await import(url);
    const original = DemoClient.prototype.setAppNetworkMonitoring;
    DemoClient.prototype.setAppNetworkMonitoring = function (enabled) {
      window.__networkClient = this;
      return original.call(this, enabled);
    };
  });
  for (const theme of ["light", "dark"]) {
    await page.getByRole("button", { name: "设置", exact: true }).click();
    await page
      .getByText(theme === "light" ? "浅色" : "深色", { exact: true })
      .click();
    await waitForSettingsSave(page);
    await page.getByRole("button", { name: "网络", exact: true }).click();
    const ranking = page.locator(".app-network");
    const start = ranking.getByRole("button", { name: "开始监控" });
    if (await start.count()) await start.click();
    await ranking
      .getByRole("button", { name: "展开 浏览器（演示）" })
      .waitFor();
    assert.match(await ranking.innerText(), /60.0%/);
    const appOrder = () =>
      ranking
        .getByRole("button", { name: /^(展开|收起) / })
        .evaluateAll((buttons) =>
          buttons.map((button) => button.getAttribute("aria-label").slice(3)),
        );
    const descending = ["浏览器（演示）", "下载工具（演示）", "云盘（演示）"];
    assert.deepEqual(await appOrder(), descending);
    assert.equal(
      await ranking.locator('th[aria-sort="descending"]').innerText(),
      "下载速度",
    );
    await ranking.getByRole("button", { name: /^下载速度，/ }).click();
    assert.deepEqual(await appOrder(), [...descending].reverse());
    await ranking.getByRole("button", { name: /^下载速度，/ }).click();
    assert.deepEqual(await appOrder(), descending);
    await ranking.getByRole("button", { name: "展开 浏览器（演示）" }).click();
    assert.match(await ranking.innerText(), /PID 1000/);
    const applicationLabel = await ranking
      .getByText("浏览器（演示）", { exact: true })
      .boundingBox();
    const processLabel = await ranking
      .getByText("PID 1000", { exact: true })
      .boundingBox();
    assert(
      processLabel.x >= applicationLabel.x + 16,
      "process label must be visibly indented beyond its application name",
    );
    await ranking.getByRole("button", { name: /^上传速度，/ }).click();
    assert.equal(
      await ranking.locator('th[aria-sort="descending"]').innerText(),
      "上传速度",
    );
    await ranking.getByRole("button", { name: /^上传速度，/ }).click();
    assert.deepEqual(await appOrder(), [...descending].reverse());
    const nameHeader = ranking.getByRole("button", { name: /^应用名称，/ });
    await nameHeader.click();
    assert.deepEqual(await appOrder(), descending);
    assert.match(await ranking.locator("thead").innerText(), /上传占比/);
    await nameHeader.press("Enter");
    assert.deepEqual(await appOrder(), [...descending].reverse());
    await nameHeader.press("Space");
    assert.deepEqual(await appOrder(), descending);
    await page.waitForTimeout(1200);
    assert.equal(
      await nameHeader.evaluate((button) => button === document.activeElement),
      true,
    );
    assert.deepEqual(await appOrder(), descending);
    assert.match(
      await ranking.locator("tbody tr").nth(1).innerText(),
      /PID 1000/,
    );
    await ranking.scrollIntoViewIfNeeded();
    await page.waitForTimeout(350);
    await page.screenshot({ path: `${assets}ranking-${theme}-expanded.png` });
    await ranking.getByRole("button", { name: "收起 浏览器（演示）" }).focus();
    await page.keyboard.press("Enter");
    assert.doesNotMatch(await ranking.innerText(), /PID 1000/);
  }
  for (const status of [
    "permission_denied",
    "failed",
    "stale",
    "warming",
    "unsupported",
  ]) {
    await page.evaluate(
      (status) => window.__networkClient.setScenario(status),
      status,
    );
    const ranking = page.locator(".app-network");
    assert.doesNotMatch(await ranking.innerText(), /60.0%/);
    await ranking.scrollIntoViewIfNeeded();
    if (status === "permission_denied")
      await page.screenshot({ path: `${assets}ranking-permission.png` });
  }
  await page.evaluate(() => window.__networkClient.setScenario("normal"));
  await page.setViewportSize({ width: 420, height: 800 });
  await page.locator(".app-network").scrollIntoViewIfNeeded();
  await page
    .locator(".app-network")
    .getByRole("button", { name: /^上传速度，/ })
    .click();
  assert.equal(
    await page.locator('.app-network th[aria-sort="descending"]').innerText(),
    "上传速度",
  );
  assert(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  );
  await page.locator(".app-network-table table").evaluate((table) => {
    table.parentElement.scrollLeft = 0;
  });
  await page.screenshot({ path: `${assets}ranking-narrow.png` });
  await page.getByRole("button", { name: "CPU", exact: true }).click();
  assert.equal(
    await page.evaluate(
      () => window.__networkClient.getSnapshot().state.app_network.running,
    ),
    true,
  );
  await page.getByRole("button", { name: "网络", exact: true }).click();
  await page
    .locator(".app-network")
    .getByRole("button", { name: "停止监控" })
    .waitFor();
  await page
    .locator(".app-network")
    .getByRole("button", { name: "停止监控" })
    .click();
  assert.equal(
    await page.evaluate(
      () => window.__networkClient.getSnapshot().state.app_network.running,
    ),
    false,
  );
  assert.deepEqual(errors, []);
  console.log(
    "PASS: ranking sorting, grouping, states, themes, page continuity and explicit stop.",
  );
} finally {
  await context.close();
  await browser.close();
}
