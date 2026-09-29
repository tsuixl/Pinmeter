import { chromium } from "@playwright/test";
import assert from "node:assert/strict";
import fs from "node:fs";
import { fileURLToPath } from "node:url";
import { waitForSettingsSave } from "./settings-helpers.mjs";

const assets = fileURLToPath(
  new URL(
    "../../../docs/development/v0.1.0-cpu-temperature/assets/",
    import.meta.url,
  ),
);
fs.mkdirSync(assets, { recursive: true });
const browser = await chromium.launch({ channel: "msedge", headless: true });
const errors = [];
try {
  for (const theme of ["light", "dark"]) {
    const context = await browser.newContext({
      viewport: { width: 1100, height: 900 },
    });
    const page = await context.newPage();
    page.on("pageerror", (error) => errors.push(String(error)));
    await page.goto(
      process.env.PINMETER_UI_URL || "http://127.0.0.1:1420/?demo=1",
    );
    await page.getByRole("button", { name: "设置", exact: true }).click();
    await page
      .getByText(theme === "light" ? "浅色" : "深色", { exact: true })
      .click();
    await waitForSettingsSave(page);
    await page.evaluate(async () => {
      const url = performance
        .getEntriesByType("resource")
        .find((e) => e.name.includes("/src/shared/client/demo-client.ts")).name;
      const { DemoClient } = await import(url);
      window.__driverMissing = true;
      window.__downloadCalls = 0;
      DemoClient.prototype.temperatureDriverMissing = async () =>
        window.__driverMissing;
      DemoClient.prototype.openTemperatureDriverDownload = () => {
        window.__downloadCalls++;
        return new Promise((resolve, reject) => {
          window.__finishOpen = (success) =>
            success
              ? resolve()
              : reject("浏览器无法启动，请手动访问 https://pawnio.eu/");
        });
      };
      const temperature = DemoClient.prototype.temperature;
      DemoClient.prototype.temperature = function () {
        const reading = temperature.call(this);
        return window.__driverMissing
          ? {
              ...reading,
              value: null,
              text: "—",
              status: "unsupported",
              detail: "未安装 CPU 温度驱动（PawnIO）",
            }
          : reading;
      };
    });
    await page.getByRole("button", { name: "CPU", exact: true }).click();
    const download = page.getByRole("button", {
      name: "前往官网下载",
      exact: true,
    });
    const recheck = page.getByRole("button", { name: "重新检测", exact: true });
    await download.waitFor();
    await page.waitForTimeout(1100);
    await page.screenshot({
      path: assets + "driver-download-" + theme + ".png",
    });
    await download.click();
    await page.waitForFunction(() => window.__downloadCalls === 1);
    assert(await download.isDisabled());
    assert(await recheck.isDisabled());
    await download.evaluate((button) => button.click());
    assert.equal(await page.evaluate(() => window.__downloadCalls), 1);
    await page.evaluate(() => window.__finishOpen(false));
    await page
      .getByText("浏览器无法启动，请手动访问 https://pawnio.eu/", {
        exact: true,
      })
      .waitFor();
    await page.waitForTimeout(350);
    await page.screenshot({
      path: assets + "driver-download-error-" + theme + ".png",
    });
    await download.click();
    await page.waitForFunction(() => window.__downloadCalls === 2);
    await page.evaluate(() => window.__finishOpen(true));
    await page.getByText(/已打开 PawnIO 官网/).waitFor();
    assert(
      await download.isVisible(),
      "Opening the website must not claim the driver is installed",
    );
    await recheck.click();
    await page.getByText(/尚未检测到 PawnIO/).waitFor();
    await page.evaluate(() => {
      window.__driverMissing = false;
    });
    await recheck.click();
    await page.getByText(/已检测到 PawnIO/).waitFor();
    assert.equal(await download.count(), 0);
    await page.waitForTimeout(1200);
    assert.match(
      await page.locator(".cpu-temperature").innerText(),
      /\d+\.\d °C/,
    );
    await page.screenshot({
      path: assets + "driver-detected-" + theme + ".png",
    });
    await page.getByRole("button", { name: "内存", exact: true }).click();
    assert.equal(await page.locator(".temperature-driver").count(), 0);
    await page.evaluate(() => {
      window.__driverMissing = true;
    });
    await page.setViewportSize({ width: 460, height: 850 });
    await page.getByRole("button", { name: "CPU", exact: true }).click();
    await download.waitFor();
    await page.waitForTimeout(1200);
    assert(
      !/已检测到 PawnIO/.test(
        await page.locator(".temperature-driver").innerText(),
      ),
    );
    assert(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    );
    await page.screenshot({
      path: assets + "driver-download-narrow-" + theme + ".png",
    });
    await context.close();
  }
  assert.deepEqual(errors, []);
  console.log(
    "PASS: publisher download, duplicate prevention, opening failure/retry, explicit detection, resumed readings and light/dark/narrow presentation (simulated driver).",
  );
} finally {
  await browser.close();
}
