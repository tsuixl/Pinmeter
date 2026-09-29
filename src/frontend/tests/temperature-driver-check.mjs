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
      window.__installCalls = 0;
      DemoClient.prototype.temperatureDriverMissing = async () =>
        window.__driverMissing;
      DemoClient.prototype.installTemperatureDriver = () => {
        window.__installCalls++;
        return new Promise((resolve, reject) => {
          window.__finishInstall = (outcome) => {
            const failures = {
              network: "官方驱动下载失败，请检查网络后重试",
              checksum: "驱动安装器校验失败，未执行安装，请重新下载",
              canceled: "已取消管理员授权，可重新下载安装",
            };
            if (failures[outcome]) reject(failures[outcome]);
            else if (outcome === "restart")
              resolve(
                "驱动安装器要求重启电脑，请保存工作并手动重启后查看温度。",
              );
            else {
              window.__driverMissing = false;
              resolve("PawnIO 已安装，温度采集将在 30 秒内自动重试。");
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
    const install = page.locator(".temperature-driver button").first();
    const recheck = page.getByRole("button", { name: "重新检测", exact: true });
    await page
      .getByRole("button", { name: "下载安装驱动", exact: true })
      .waitFor();
    await page.waitForTimeout(1100);
    assert.equal(
      await page.evaluate(() => window.__installCalls),
      0,
      "Entering CPU must not install a driver",
    );
    await page.screenshot({
      path: assets + "driver-download-" + theme + ".png",
    });
    let calls = 0;
    for (const outcome of [
      "network",
      "checksum",
      "canceled",
      "restart",
      "installed",
    ]) {
      await install.click();
      calls++;
      await page.waitForFunction(
        (expected) => window.__installCalls === expected,
        calls,
      );
      await page.getByText("正在下载并安装驱动", { exact: true }).waitFor();
      assert(await install.isDisabled());
      assert(await recheck.isDisabled());
      await install.evaluate((button) => button.click());
      assert.equal(await page.evaluate(() => window.__installCalls), calls);
      if (outcome === "network")
        await page.screenshot({
          path: assets + "driver-install-pending-" + theme + ".png",
        });
      await page.evaluate((result) => window.__finishInstall(result), outcome);
      await recheck.waitFor({ state: "visible" });
      await page.waitForFunction(
        () => !document.querySelector(".temperature-driver button").disabled,
      );
      await page.waitForTimeout(350);
      const text = await page.locator(".temperature-driver").innerText();
      if (outcome === "network") assert.match(text, /下载失败/);
      if (outcome === "checksum") {
        assert.match(text, /校验失败/);
        await page.screenshot({
          path: assets + "driver-download-error-" + theme + ".png",
        });
      }
      if (outcome === "canceled") assert.match(text, /已取消管理员授权/);
      if (outcome === "restart") {
        assert.match(text, /手动重启/);
        assert.equal(
          await page
            .getByRole("button", { name: "下载安装驱动", exact: true })
            .count(),
          1,
        );
        await page.screenshot({
          path: assets + "driver-install-restart-" + theme + ".png",
        });
      }
    }
    assert.equal(
      await page
        .getByRole("button", { name: "下载安装驱动", exact: true })
        .count(),
      0,
    );
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
    await page
      .getByRole("button", { name: "下载安装驱动", exact: true })
      .waitFor();
    await recheck.click();
    await page.getByText(/尚未检测到 PawnIO/).waitFor();
    assert.equal(
      await page.evaluate(() => window.__installCalls),
      calls,
      "Detection must not download/install",
    );
    assert(
      await page.evaluate(
        () => document.documentElement.scrollWidth <= innerWidth,
      ),
    );
    await page.waitForTimeout(1200);
    await page.screenshot({
      path: assets + "driver-download-narrow-" + theme + ".png",
    });
    await context.close();
  }
  assert.deepEqual(errors, []);
  console.log(
    "PASS: click-only online installation, pending duplicates, download/checksum/UAC failures, reboot result, automatic detection, resumed readings, light/dark/narrow presentation (simulated installer).",
  );
} finally {
  await browser.close();
}
