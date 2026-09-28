import { waitForSettingsSave } from "./settings-helpers.mjs";
import { chromium } from "@playwright/test";
import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
const assets = fileURLToPath(
  new URL(
    "../../../docs/development/v0.1.0-gpu-monitoring/assets/",
    import.meta.url,
  ),
);
await mkdir(assets, { recursive: true });
const browser = await chromium.launch({ channel: "chrome", headless: true });
const context = await browser.newContext({
  viewport: { width: 1280, height: 1100 },
});
const page = await context.newPage();
page.setDefaultTimeout(15000);
const errors = [];
try {
  if (process.argv.includes("--references")) {
    for (const story of [
      "composite-stat-card--grid",
      "composite-stat-card--dark-mode",
      "forms-select--with-label",
      "forms-select--dark-mode",
      "charts-area-chart--default",
      "charts-area-chart--dark-mode",
    ]) {
      await page.goto(
        `https://main--6a5a658b3681fcc010430db5.chromatic.com/iframe.html?id=${story}&viewMode=story`,
      );
      await page.waitForFunction(
        () => document.querySelector("#storybook-root")?.children.length > 0,
      );
      assert(
        !(await page.locator("body").innerText()).includes(
          "Couldn't find story",
        ),
      );
      await page.waitForTimeout(400);
      await page.screenshot({
        path: assets + `reference-${story}.png`,
        fullPage: true,
      });
    }
  }
  page.on("pageerror", (e) => errors.push(String(e)));
  await page.route("**/src/shared/client/demo-client.ts*", async (route) => {
    const response = await route.fetch();
    const body = (await response.text()).replace(
      "super();",
      "super(); window.__gpuClient = this;",
    );
    await route.fulfill({ response, body });
  });
  await page.goto(
    `${process.env.PINMETER_UI_URL ?? "http://127.0.0.1:1420"}/?demo=1`,
  );
  await page.getByRole("button", { name: "GPU", exact: true }).click();
  await page.waitForFunction(() => window.__gpuClient);
  await page.evaluate(() => {
    const client = window.__gpuClient;
    const publish = client.publish;
    client.publish = function (snapshot) {
      if (window.__gpuFixture) snapshot.state.gpu = window.__gpuFixture;
      if (window.__gpuDisconnected) snapshot.connected = false;
      return publish.call(this, snapshot);
    };
  });
  await page.waitForFunction(() => window.__gpuClient);
  const gpu = page.getByRole("region", { name: "GPU 详情" });
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
    await page.getByRole("button", { name: "GPU", exact: true }).click();
    await gpu.locator(".gpu-stats > *").first().waitFor();
    await page.waitForTimeout(500);
    assert.equal(await gpu.locator(".gpu-stats > *").count(), 6);
    assert.match(
      await gpu.getByRole("combobox", { name: "显卡" }).innerText(),
      /NVIDIA/,
    );
    assert.match(await gpu.locator(".gpu-stats").innerText(), /14001.00 MHz/);
    const chart = gpu.locator(".trend").first();
    assert(
      (await chart.locator('[data-metric="cpu"] .series').count()) >= 2,
      "gaps stay split",
    );
    const box = await chart.locator("svg").boundingBox();
    await page.mouse.move(box.x + box.width * 0.85, box.y + box.height / 2);
    await page.getByRole("tooltip").waitFor();
    assert.match(await page.getByRole("tooltip").innerText(), /GPU 核心温度/);
    await page.screenshot({
      path: assets + `gpu-${theme}.png`,
      fullPage: true,
    });
    await gpu.locator(".gpu-charts").scrollIntoViewIfNeeded();
    await page.screenshot({ path: assets + `gpu-${theme}-charts.png` });
    await gpu.locator(".gpu-stats").scrollIntoViewIfNeeded();
    const select = gpu.getByRole("combobox", { name: "显卡" });
    await select.click();
    await page.getByRole("option", { name: /AMD/ }).waitFor();
    await page.waitForTimeout(350);
    await page.screenshot({ path: assets + `gpu-${theme}-select.png` });
    await page.getByRole("option", { name: /AMD/ }).click();
    await page.waitForTimeout(350);
    assert.match(await gpu.locator(".gpu-stats").innerText(), /0.00 %/);
    assert.match(await gpu.locator(".gpu-stats").innerText(), /未提供核心温度/);
    assert.match(
      await gpu.locator(".gpu-stats").innerText(),
      /GPU VR SoC 温度/,
    );
    assert(
      (await gpu.locator('[data-metric="cpu_temperature"] .series').count()) >
        0,
    );
    await page.screenshot({
      path: assets + `gpu-${theme}-vrsoc.png`,
      fullPage: true,
    });
    await page.getByRole("button", { name: "总览", exact: true }).click();
    await page.waitForTimeout(350);
    const overviewTemperature = page.locator(
      '.overview-metric[data-metric="gpu_temperature"]',
    );
    assert.match(await overviewTemperature.innerText(), /GPU VR SoC 温度/);
    assert.match(
      await overviewTemperature.innerText(),
      /未提供核心温度，显示 VR SoC 传感器读数/,
    );
    await page.screenshot({ path: assets + `gpu-${theme}-vrsoc-overview.png` });
    await page.getByRole("button", { name: "GPU", exact: true }).click();
    await select.focus();
    await select.press("Enter");
    await page.getByRole("option", { name: /NVIDIA/ }).click();
  }
  await page.getByRole("tab", { name: "1 分钟", exact: true }).click();
  await page.getByRole("tab", { name: "5 分钟", exact: true }).click();
  const chartSvg = gpu.locator("svg.chart-svg").first();
  await chartSvg.focus();
  await chartSvg.press("ArrowLeft");
  await gpu.getByRole("button", { name: "返回实时" }).first().click();
  for (const status of [
    "warming",
    "failed",
    "stale",
    "permission_denied",
    "unsupported",
  ]) {
    await page.evaluate(
      (status) => window.__gpuClient.setScenario(status),
      status,
    );
    assert((await gpu.locator(".gpu-stats").innerText()).includes("—"));
  }
  await page.screenshot({
    path: assets + "gpu-unavailable.png",
    fullPage: true,
  });
  await page.evaluate(() => window.__gpuClient.setScenario("normal"));
  await page.evaluate(() => {
    window.__gpuDisconnected = true;
    window.__gpuClient.setScenario("normal");
  });
  assert.match(await gpu.innerText(), /采集服务未连接/);
  assert.equal(
    await gpu.locator(".gpu-stats").getByText("—", { exact: true }).count(),
    6,
  );
  await page.evaluate(() => {
    window.__gpuDisconnected = false;
    window.__gpuClient.publish({
      ...window.__gpuClient.getSnapshot(),
      connected: true,
    });
  });
  await page.evaluate(() => {
    const c = window.__gpuClient;
    const s = c.getSnapshot();
    window.__gpuFixture = {
      ...s.state.gpu,
      devices: s.state.gpu.devices.filter((d) => d.id === "demo-gpu-1"),
    };
    c.setScenario("normal");
  });
  assert.match(
    await gpu.getByRole("combobox", { name: "显卡" }).innerText(),
    /AMD/,
  );
  await page.evaluate(() => {
    window.__gpuFixture = null;
    window.__gpuClient.setScenario("normal");
  });
  await page.setViewportSize({ width: 420, height: 950 });
  await page.screenshot({ path: assets + "gpu-narrow.png", fullPage: true });
  assert(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  );
  assert(await gpu.evaluate((e) => e.scrollWidth <= e.clientWidth));
  await page.evaluate(() => {
    window.__gpuFixture = {
      status: "unsupported",
      detail: "未发现受支持的物理 GPU",
      devices: [],
    };
    window.__gpuClient.setScenario("normal");
  });
  assert.match(await gpu.innerText(), /未发现受支持的物理 GPU/);
  assert.equal(await gpu.locator(".gpu-stats").count(), 0);
  assert.deepEqual(errors, []);
  console.log(
    "PASS: Sakani themes, GPU selection/keyboard, missing temperature vs zero, chart gaps/units/history, failure/disconnect, removal/no device and narrow layout.",
  );
} catch (error) {
  await page.screenshot({
    path: assets + "gpu-check-failure.png",
    fullPage: true,
  });
  console.error(errors);
  console.error(
    await page.evaluate(() => ({
      gpu: window.__gpuClient?.getSnapshot().state?.gpu,
      keys: Object.keys(window.__gpuClient?.getSnapshot().state ?? {}),
    })),
  );
  throw error;
} finally {
  await browser.close();
}
