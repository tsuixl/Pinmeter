import { waitForSettingsSave } from "./settings-helpers.mjs";
import { chromium } from "@playwright/test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";

const assets = fileURLToPath(
  new URL(
    "../../../docs/development/v0.1.0-main-window/assets/",
    import.meta.url,
  ),
);
const browser = process.env.BROWSER_CDP
  ? await chromium.connectOverCDP(process.env.BROWSER_CDP)
  : await chromium.launch({ channel: "msedge", headless: true });
const context = await browser.newContext({
  viewport: { width: 1280, height: 1000 },
});
const page = await context.newPage();
const errors = [];
page.on("pageerror", (error) => errors.push(String(error)));
const cards = page.locator(".overview-metric");
const plot = page.locator(".overview-trends");
const labels = [
  "CPU 使用率",
  "CPU 温度",
  "GPU 使用率",
  "GPU 核心温度",
  "内存",
  "网速",
];
try {
  await page.goto("http://127.0.0.1:1420/?demo=1");
  await cards.first().waitFor();
  assert.equal(await cards.count(), 6);
  for (let i = 0; i < labels.length; i++)
    assert.equal(
      await cards.nth(i).getAttribute("aria-label"),
      `查看${labels[i]}详情`,
    );
  assert.equal(await page.getByText("当前连接", { exact: true }).count(), 0);
  assert.equal(
    await plot
      .locator("svg g[data-metric]")
      .evaluateAll((nodes) => new Set(nodes.map((n) => n.dataset.metric)).size),
    3,
  );
  await plot.getByRole("tab", { name: "温度", exact: true }).click();
  assert.equal(
    await plot
      .locator("svg g[data-metric]")
      .evaluateAll((nodes) => new Set(nodes.map((n) => n.dataset.metric)).size),
    2,
  );
  await plot.getByRole("tab", { name: "全部", exact: true }).click();
  assert.equal(
    await plot
      .locator("svg g[data-metric]")
      .evaluateAll((nodes) => new Set(nodes.map((n) => n.dataset.metric)).size),
    5,
  );
  assert.equal(
    await plot
      .locator(".series")
      .evaluateAll(
        (nodes) => new Set(nodes.map((n) => getComputedStyle(n).stroke)).size,
      ),
    5,
  );
  assert.match(await plot.locator(".temperature-axis").innerText(), /100 °C/);
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
    await page.getByRole("button", { name: "总览", exact: true }).click();
    await plot.getByRole("tab", { name: "全部", exact: true }).click();
    await page.waitForTimeout(350);
    await page.screenshot({
      path: assets + `overview-six-metrics-${theme}.png`,
    });
  }
  const box = await plot.locator(".chart-svg").boundingBox();
  await page.mouse.move(box.x + box.width * 0.9, box.y + 80);
  await page.getByRole("tooltip").waitFor();
  assert.equal(
    await page.getByRole("tooltip").locator(".tooltip-row").count(),
    5,
  );
  await plot.getByRole("button", { name: "CPU 温度", exact: true }).click();
  assert.equal(
    await plot.locator('svg [data-metric="cpu_temperature"]').count(),
    0,
  );
  assert.equal(
    await plot
      .getByRole("button", { name: "CPU 温度", exact: true })
      .getAttribute("aria-pressed"),
    "false",
  );
  await plot.getByRole("button", { name: "CPU 温度", exact: true }).click();
  for (const button of await plot.locator(".overview-legend button").all())
    await button.click();
  await plot.getByText("选择图例以显示趋势").waitFor();
  for (const button of await plot.locator(".overview-legend button").all())
    await button.click();

  await page.getByRole("combobox", { name: "显卡" }).click();
  await page.getByRole("option", { name: /AMD/ }).click();
  assert.match(await cards.nth(2).innerText(), /0(?:\.00)? %/);
  assert.match(await cards.nth(3).innerText(), /GPU VR SoC 温度/);
  assert.match(await cards.nth(3).innerText(), /未提供核心温度/);
  assert(
    (await plot.locator('svg [data-metric="gpu_temperature"]').count()) > 0,
  );
  assert(
    (await plot.locator('svg [data-metric="cpu_temperature"]').count()) > 0,
  );
  await cards.nth(2).click();
  assert.match(
    await page.getByRole("combobox", { name: "显卡" }).innerText(),
    /AMD/,
  );
  await page.getByRole("combobox", { name: "显卡" }).click();
  await page.getByRole("option", { name: /NVIDIA/ }).click();
  await page.getByRole("button", { name: "总览", exact: true }).click();
  await plot.getByRole("tab", { name: "全部", exact: true }).click();
  assert.match(
    await page.getByRole("combobox", { name: "显卡" }).innerText(),
    /NVIDIA/,
  );
  await page.getByRole("tab", { name: "1 分钟", exact: true }).click();
  await plot.locator(".chart-svg").focus();
  await page.keyboard.press("ArrowLeft");
  await plot.getByRole("button", { name: "返回实时" }).click();

  await page.evaluate(async () => {
    const url = performance
      .getEntriesByType("resource")
      .find((e) => e.name.includes("/src/shared/client/demo-client.ts")).name;
    const { DemoClient } = await import(url);
    const publish = DemoClient.prototype.publish;
    DemoClient.prototype.publish = function (snapshot) {
      window.__overviewClient = this;
      publish.call(this, window.__overviewTransform?.(snapshot) ?? snapshot);
    };
  });
  await page.waitForFunction(() => !!window.__overviewClient);
  await page.evaluate(() => {
    window.__overviewTransform = (s) => ({
      ...s,
      state: {
        ...s.state,
        frame: {
          ...s.state.frame,
          download: {
            ...s.state.frame.download,
            value: 9999999999,
            text: "9999.9",
            unit: "MB/s",
          },
          upload: {
            ...s.state.frame.upload,
            value: 8888888888,
            text: "8888.8",
            unit: "MB/s",
          },
        },
      },
    });
    window.__overviewClient.setScenario("normal");
  });
  for (const width of [1800, 1280, 880, 620, 420]) {
    await page.setViewportSize({ width, height: 1000 });
    await cards.last().scrollIntoViewIfNeeded();
    assert(
      await page
        .locator(".content")
        .evaluate((n) => n.scrollWidth <= n.clientWidth + 1),
      `content overflow at ${width}`,
    );
    assert(
      await cards.evaluateAll((nodes) =>
        nodes.every((n) => n.scrollWidth <= n.clientWidth + 1),
      ),
      `card overflow at ${width}`,
    );
    const sizes = await cards.evaluateAll((nodes) =>
      nodes.map((n) => ({
        width: n.clientWidth,
        height: n.clientHeight,
        font: Math.max(
          ...[...n.querySelectorAll("*")].map((child) =>
            parseFloat(getComputedStyle(child).fontSize),
          ),
        ),
      })),
    );
    assert.equal(
      sizes[0].font,
      sizes[5].font,
      `network font changed at ${width}`,
    );
    assert(
      sizes.every((s) => Math.abs(s.width - sizes[0].width) <= 1),
      `unequal widths at ${width}`,
    );
    assert(
      sizes.every((s) => Math.abs(s.height - sizes[0].height) <= 1),
      `unequal heights at ${width}`,
    );
    assert.match(
      await cards.last().innerText(),
      /↓ 9999.9 MB\/s\n↑ 8888.8 MB\/s/,
    );
    assert(
      await cards.last().evaluate((n) => {
        const value = n.firstElementChild.children[1];
        return (
          value.clientHeight <=
          parseFloat(getComputedStyle(value).lineHeight) * 2 + 1
        );
      }),
      `network values wrapped beyond two lines at ${width}`,
    );
    if (width === 420)
      await page.screenshot({
        path: assets + "overview-six-metrics-narrow.png",
      });
  }
  await page.setViewportSize({ width: 1280, height: 1000 });
  await page.evaluate(() => {
    window.__overviewTransform = (s) => ({
      ...s,
      state: {
        ...s.state,
        history: s.state.history.map((f) => ({
          ...f,
          gpus: f.gpus.map((g) => ({
            ...g,
            readings: {
              ...g.readings,
              temperature: {
                ...g.readings.temperature,
                value: 125,
                text: "125.0",
                status: "normal",
              },
            },
          })),
        })),
      },
    });
    window.__overviewClient.setScenario("normal");
  });
  assert.match(await plot.locator(".temperature-axis").innerText(), /140 °C/);
  assert.match(await plot.locator(".chart-axis").first().innerText(), /100 %/);
  await page.evaluate(() => {
    window.__overviewTransform = (s) => ({
      ...s,
      state: {
        ...s.state,
        gpu: {
          ...s.state.gpu,
          devices: s.state.gpu.devices.filter((d) => d.id === "demo-gpu-1"),
        },
      },
    });
    window.__overviewClient.setScenario("normal");
  });
  await page.waitForFunction(() =>
    document
      .querySelector('.overview-trends [role="combobox"]')
      ?.textContent.includes("AMD"),
  );
  await page.evaluate(() => {
    window.__overviewTransform = (s) => ({ ...s, connected: false });
    window.__overviewClient.setScenario("normal");
  });
  assert((await cards.allTextContents()).every((text) => text.includes("—")));
  assert.deepEqual(errors, []);
  console.log(
    "PASS: six equal-size cards, unchanged font, responsive widths, five colors/axes/tooltip, legend states, shared GPU selection/removal, independent missing values and disconnection.",
  );
} finally {
  await context.close();
  await browser.close();
}
