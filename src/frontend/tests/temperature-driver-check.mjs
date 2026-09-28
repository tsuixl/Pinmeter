import { chromium } from "@playwright/test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { waitForSettingsSave } from "./settings-helpers.mjs";

const assets = fileURLToPath(
  new URL(
    "../../../docs/development/v0.1.0-cpu-temperature/assets/",
    import.meta.url,
  ),
);
const browser = await chromium.connectOverCDP("http://127.0.0.1:18800");
const context = await browser.newContext({
  viewport: { width: 1100, height: 900 },
});
const page = await context.newPage();
const errors = [];
page.on("pageerror", (error) => errors.push(String(error)));
try {
  for (const theme of ["light", "dark"]) {
    await page.goto("http://127.0.0.1:1420/?demo=1");
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
      window.__installCalls = 0;
      DemoClient.prototype.temperatureDriverMissing = async () =>
        window.__driverMissing;
      DemoClient.prototype.installTemperatureDriver = () => {
        window.__installCalls++;
        return new Promise((resolve, reject) => {
          window.__finishInstall = (success) => {
            if (!success) reject("已取消管理员授权，可重新安装");
            else {
              window.__driverMissing = false;
              resolve("PawnIO 已安装，温度采集将在 30 秒内自动重试");
            }
          };
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
    const install = page.getByRole("button", {
      name: "安装温度驱动",
      exact: true,
    });
    await install.waitFor();
    await page.waitForTimeout(1100);
    await page.screenshot({ path: assets + `driver-missing-${theme}.png` });
    await install.click();
    const pending = page.getByRole("button", {
      name: "正在安装…",
      exact: true,
    });
    assert(await pending.isDisabled());
    await pending.evaluate((button) => button.click());
    assert.equal(await page.evaluate(() => window.__installCalls), 1);
    await page.screenshot({ path: assets + `driver-pending-${theme}.png` });
    await page.evaluate(() => window.__finishInstall(false));
    const retry = page.getByRole("button", { name: "重试安装", exact: true });
    await retry.waitFor();
    await page.waitForTimeout(350);
    assert.match(
      await page.locator(".temperature-driver").innerText(),
      /已取消管理员授权/,
    );
    await page.screenshot({ path: assets + `driver-error-${theme}.png` });
    await retry.click();
    await page.waitForFunction(() => window.__installCalls === 2);
    await page.evaluate(() => window.__finishInstall(true));
    await page.getByText("温度驱动安装结果", { exact: true }).waitFor();
    assert.equal(await page.locator(".temperature-driver button").count(), 0);
    await page.waitForTimeout(1200);
    assert.match(
      await page.locator(".cpu-temperature").innerText(),
      /\d+\.\d °C/,
    );
    await page.screenshot({ path: assets + `driver-installed-${theme}.png` });
    await page.getByRole("button", { name: "内存", exact: true }).click();
    assert.equal(await page.locator(".temperature-driver").count(), 0);
  }
  assert.deepEqual(errors, []);
  console.log(
    "PASS: missing-driver install, pending duplicate prevention, cancellation/retry, verified completion, resumed readings and light/dark themes (simulated driver)",
  );
} finally {
  await context.close();
  await browser.close();
}
