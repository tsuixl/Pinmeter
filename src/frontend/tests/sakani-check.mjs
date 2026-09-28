import { waitForSettingsSave } from "./settings-helpers.mjs";
import { chromium } from "@playwright/test";
import assert from "node:assert/strict";
import { copyFile, mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
const assets = fileURLToPath(
  new URL(
    "../../../docs/development/v0.1.0-main-window/assets",
    import.meta.url,
  ),
);
await mkdir(assets, { recursive: true });
const browser = await chromium.connectOverCDP("http://127.0.0.1:18800");
const context = await browser.newContext({
  viewport: { width: 1100, height: 760 },
});
const page = await context.newPage();
const errors = [];
page.on("pageerror", (error) => errors.push(String(error)));
try {
  await page.goto("http://127.0.0.1:1420/?demo=1");
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
    await page.waitForTimeout(350);
    assert.equal(
      await page.evaluate(() => document.fonts.check('14px "Geist Variable"')),
      true,
    );
    for (const [key, label] of Object.entries({
      overview: "总览",
      cpu: "CPU",
      memory: "内存",
      network: "网络",
      settings: "设置",
    })) {
      await page.getByRole("button", { name: label, exact: true }).click();
      await page.waitForTimeout(350);
      await page.screenshot({ path: `${assets}/sakani-${theme}-${key}.png` });
    }
    const combo = page.getByRole("combobox", { name: "采样间隔", exact: true });
    await combo.focus();
    await page.keyboard.press("Enter");
    await page.getByRole("option", { name: "2 秒", exact: true }).waitFor();
    await page.screenshot({
      path: `${assets}/sakani-${theme}-select-open.png`,
    });
    await page.keyboard.press("ArrowDown");
    await page.keyboard.press("Enter");
    assert.match(await combo.innerText(), /秒/);
    await page.getByRole("button", { name: "CPU", exact: true }).click();
    const graph = page.locator(".chart-svg").first();
    await graph.scrollIntoViewIfNeeded();
    const bounds = await graph.boundingBox();
    await page.mouse.move(
      bounds.x + bounds.width * 0.75,
      bounds.y + bounds.height * 0.5,
    );
    await page.getByRole("tooltip").waitFor();
    assert.equal(await page.locator(".chart-crosshair").count(), 1);
    assert.equal(await page.locator(".active-point").count(), 1);
    assert((await graph.locator('path[fill^="url"]').count()) > 0);
    await page.screenshot({
      path: `${assets}/sakani-${theme}-chart-hover.png`,
    });
  }
  await page.getByRole("button", { name: "总览", exact: true }).click();
  // Test an entirely unavailable history as well as failed current readings.
  await page.evaluate(async () => {
    const url = performance
      .getEntriesByType("resource")
      .find((e) => e.name.includes("/src/shared/client/demo-client.ts"))?.name;
    if (!url) throw new Error("Missing demo client");
    const { DemoClient } = await import(url);
    const original = DemoClient.prototype.setScenario;
    DemoClient.prototype.setScenario = function (status) {
      original.call(this, status);
      this.publish({
        ...this.snapshot,
        state: { ...this.snapshot.state, history: [this.snapshot.state.frame] },
      });
    };
  });
  await page.getByRole("combobox", { name: "演示状态" }).click();
  await page.getByRole("option", { name: "采集失败", exact: true }).click();
  await page.keyboard.press("Escape");
  await page.waitForTimeout(350);
  const cards = await page.locator(".metric-link").allTextContents();
  assert.equal(cards.length, 4);
  assert(cards.every((t) => t.includes("—")));
  await page.getByText("等待有效采样", { exact: true }).waitFor();
  assert.equal(await page.locator(".chart-svg .series").count(), 0);
  await page.locator(".trend").scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${assets}/sakani-chart-unavailable.png` });
  assert.deepEqual(errors, []);
  await copyFile(
    new URL("../test-results/settings-pending.png", import.meta.url),
    `${assets}/sakani-settings-pending.png`,
  );
  await copyFile(
    new URL("../test-results/settings-error.png", import.meta.url),
    `${assets}/sakani-settings-error.png`,
  );
  console.log(
    "PASS: Sakani five pages x two themes, local Geist, keyboard Select, area/cursor/dot/tooltip, unavailable chart and saved visual evidence.",
  );
} finally {
  await context.close();
  await browser.close();
}
