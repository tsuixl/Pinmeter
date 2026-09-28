import { waitForSettingsSave } from "./settings-helpers.mjs";
import { chromium } from "@playwright/test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";

const desktop = process.argv.includes("--desktop");
const browser = await chromium.connectOverCDP(
  `http://127.0.0.1:${desktop ? 9223 : 18800}`,
);
const context = desktop
  ? browser.contexts()[0]
  : await browser.newContext({ viewport: { width: 1280, height: 1000 } });
const page = desktop ? context.pages()[0] : await context.newPage();
const assets = fileURLToPath(
  new URL(
    "../../../docs/development/v0.1.0-main-window/assets/",
    import.meta.url,
  ),
);
let initial;
const invoke = (command, args = {}) =>
  page.evaluate(
    ({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args),
    { command, args },
  );
try {
  if (!desktop) await page.goto("http://127.0.0.1:1420/?demo=1");
  else initial = await invoke("get_monitor_state");
  const model = desktop ? initial.cpu_model : "AMD Ryzen 9 9950X3D（演示）";
  assert(model);
  await page.getByRole("button", { name: "总览", exact: true }).click();
  for (const metric of ["cpu", "cpu_temperature"]) {
    await page
      .locator(`.overview-metric[data-metric="${metric}"]`)
      .getByText(model, { exact: true })
      .waitFor();
  }
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
    await page.getByRole("button", { name: "CPU", exact: true }).click();
    assert.equal(
      await page
        .locator(".cpu-summary")
        .getByText(model, { exact: true })
        .count(),
      2,
    );
    const trigger = page.getByRole("button", { name: "查看指标说明" });
    await trigger.scrollIntoViewIfNeeded();
    await page.waitForTimeout(600);
    assert.equal(await page.getByRole("dialog").count(), 0);
    await page.screenshot({
      path:
        assets + `metric-info-${desktop ? "native-" : ""}${theme}-closed.png`,
    });
    await trigger.focus();
    await page.keyboard.press("Enter");
    await page.getByRole("dialog").waitFor();
    assert.match(
      await page.getByRole("dialog").innerText(),
      /统计范围.*全部逻辑处理器/,
    );
    assert.match(await page.getByRole("dialog").innerText(), /来源/);
    await page.screenshot({
      path: assets + `metric-info-${desktop ? "native-" : ""}${theme}-open.png`,
    });
    await page.keyboard.press("Escape");
    assert.equal(await page.getByRole("dialog").count(), 0);
    assert(await trigger.evaluate((n) => n === document.activeElement));
    await trigger.click();
    await page.locator(".page-heading h1").click();
    assert.equal(await page.getByRole("dialog").count(), 0);
    await page.getByRole("button", { name: "内存", exact: true }).click();
    await trigger.click();
    assert.match(await page.getByRole("dialog").innerText(), /物理内存/);
    await trigger.click();
    assert.equal(await page.getByRole("dialog").count(), 0);
  }
  if (!desktop) {
    await page.setViewportSize({ width: 420, height: 800 });
    await page.getByRole("button", { name: "CPU", exact: true }).click();
    await page.getByRole("button", { name: "查看指标说明" }).click();
    const box = await page.getByRole("dialog").boundingBox();
    assert(box.x >= 0 && box.x + box.width <= 420 && box.y >= 0);
    await page.screenshot({ path: assets + "metric-info-narrow.png" });
    await page.keyboard.press("Escape");
    // Exercise missing metadata independently of valid CPU readings.
    await page.evaluate(async () => {
      const url = performance
        .getEntriesByType("resource")
        .find((e) => e.name.includes("/src/shared/client/demo-client.ts")).name;
      const { DemoClient } = await import(url);
      const publish = DemoClient.prototype.publish;
      DemoClient.prototype.publish = function (snapshot) {
        window.__infoClient = this;
        publish.call(this, {
          ...snapshot,
          state: { ...snapshot.state, cpu_model: null },
        });
      };
    });
    await page.waitForFunction(() => !!window.__infoClient);
    assert.equal(
      await page
        .locator(".cpu-summary")
        .getByText("CPU 型号未知", { exact: true })
        .count(),
      2,
    );
    await page.evaluate(() => window.__infoClient.setScenario("failed"));
    assert.equal(
      await page
        .locator(".cpu-summary")
        .getByText("CPU 型号未知", { exact: true })
        .count(),
      0,
    );
  }
  console.log(
    `PASS: ${desktop ? "native" : "browser"} CPU model ${model}, collapsed metric info, click/keyboard/dismiss, themes${desktop ? "" : ", narrow, unknown model and failure"}`,
  );
} finally {
  if (desktop && initial) {
    const state = await invoke("get_monitor_state");
    await invoke("update_settings", {
      settings: { ...initial.settings, revision: state.settings.revision },
      expectedRevision: state.settings.revision,
    });
  }
  if (!desktop) await context.close();
  await browser.close();
}
