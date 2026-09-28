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
const browser = await chromium.connectOverCDP("http://127.0.0.1:18800");
const context = await browser.newContext({
  viewport: { width: 1100, height: 1000 },
});
const page = await context.newPage();
page.setDefaultTimeout(10000);
const errors = [];
page.on("pageerror", (error) => errors.push(String(error)));
const cells = page.locator(".cpu-bar-column");
async function fixture(count, values = [0, 10, 25, 50, 75, 100], failed = -1) {
  await page.evaluate(
    ({ count, values, failed }) => {
      const client = window.__cpuTestClient;
      const template =
        client.getSnapshot().state.cpu_processors.processors[0]?.usage ??
        window.__cpuTemplate;
      window.__cpuTemplate = template;
      window.__cpuFixture = {
        status: count ? "normal" : "warming",
        detail: count ? "" : "等待逻辑处理器采样",
        processors: Array.from({ length: count }, (_, index) => {
          const value = values[index % values.length];
          return {
            id: `0,${index}`,
            usage: {
              ...template,
              status: index === failed ? "failed" : "normal",
              value: index === failed ? null : value,
              text: index === failed ? "—" : value.toFixed(1),
              detail: index === failed ? "读取失败" : "",
            },
          };
        }),
      };
      client.setScenario("normal");
    },
    { count, values, failed },
  );
  await page.waitForFunction(
    (count) => document.querySelectorAll(".cpu-bar-column").length === count,
    count,
  );
  await page.waitForTimeout(80);
}
try {
  await page.goto("http://127.0.0.1:1420/?demo=1");
  await page.getByRole("button", { name: "CPU", exact: true }).click();
  await cells.first().waitFor();
  await page.evaluate(async () => {
    const url = performance
      .getEntriesByType("resource")
      .find((entry) =>
        entry.name.includes("/src/shared/client/demo-client.ts"),
      ).name;
    const { DemoClient } = await import(url);
    const original = DemoClient.prototype.processors;
    DemoClient.prototype.processors = function () {
      window.__cpuTestClient = this;
      return window.__cpuFixture ?? original.call(this);
    };
  });
  await page.waitForFunction(() => !!window.__cpuTestClient);
  for (const theme of ["light", "dark"]) {
    await page.getByRole("button", { name: "设置", exact: true }).click();
    await page
      .getByText(theme === "light" ? "浅色" : "深色", { exact: true })
      .click();
    await waitForSettingsSave(page);
    await page.waitForFunction(
      (theme) => document.documentElement.dataset.theme === theme,
      theme,
    );
    await page.getByRole("button", { name: "CPU", exact: true }).click();
    await fixture(32);
    await page.waitForTimeout(350);
    await page.screenshot({ path: `${assets}cpu-bar-${theme}.png` });
    const colors = await cells.evaluateAll((nodes) =>
      nodes.map(
        (node) => getComputedStyle(node.querySelector(".cpu-bar-fill")).opacity,
      ),
    );
    await cells.nth(3).hover();
    await page.getByRole("tooltip").waitFor();
    assert.match(await page.getByRole("tooltip").innerText(), /50.0 %/);
    assert.deepEqual(
      await cells.evaluateAll((nodes) =>
        nodes.map(
          (node) =>
            getComputedStyle(node.querySelector(".cpu-bar-fill")).opacity,
        ),
      ),
      colors,
    );
    await fixture(32, [0, 10, 25, 63.2, 75, 100]);
    assert.match(await page.getByRole("tooltip").innerText(), /63.2 %/);
    await page.screenshot({ path: `${assets}cpu-bar-${theme}-hover.png` });
    await cells.first().focus();
    await page.keyboard.press("ArrowRight");
    assert.equal(
      await page.locator(":focus").getAttribute("data-processor-id"),
      "0,1",
    );
    await page.screenshot({ path: `${assets}cpu-bar-${theme}-focus.png` });
    await page.keyboard.press("Escape");
    assert.equal(await page.getByRole("tooltip").count(), 0);
  }
  for (const width of [420, 760, 1100]) {
    await page.setViewportSize({ width, height: 800 });
    for (const count of [1, 2, 6, 12, 16, 24, 32, 48, 64, 96, 128, 256]) {
      await fixture(count);
      assert.equal(await cells.count(), count);
      const geometry = await cells.first().boundingBox();
      assert.equal(geometry.height, 320);
      assert(geometry.width >= 64, `${count} at ${width}: ${geometry.width}`);
      assert(
        await page
          .locator(".processor-scroll")
          .evaluate((node) => node.scrollWidth >= node.clientWidth),
      );
      assert(
        await page
          .locator(".content")
          .evaluate((node) => node.scrollWidth <= node.clientWidth + 1),
      );
      if (
        (count === 32 && width === 420) ||
        (count === 128 && width === 1100)
      ) {
        await page.locator(".cpu-processors").scrollIntoViewIfNeeded();
        await page.screenshot({
          path: `${assets}cpu-bar-${count}-${width}.png`,
        });
      }
    }
  }
  await fixture(6, [0, 1, 3, 25, 50, 100]);
  for (let i = 0; i < 6; i++) {
    const height = (await cells.nth(i).locator(".cpu-bar-fill").boundingBox())
      .height;
    assert(Math.abs(height - [0, 1, 3, 25, 50, 100][i] * 2.64) < 0.1);
    assert.equal(
      await cells.nth(i).locator(".cpu-bar-label").innerText(),
      "CPU 0," + i,
    );
  }
  await page.locator(".cpu-processors").scrollIntoViewIfNeeded();
  await page.screenshot({ path: assets + "cpu-bar-low-load.png" });
  await fixture(128, [0, 10, 50, 100], 1);
  await page.setViewportSize({ width: 420, height: 600 });
  await cells.first().focus();
  await page.keyboard.press("End");
  assert.equal(
    await page.locator(":focus").getAttribute("data-processor-id"),
    "0,127",
  );
  await page.waitForFunction(
    () => document.querySelector(".processor-scroll").scrollLeft > 0,
  );
  await page.getByRole("tooltip").waitFor();
  const tooltip = await page.getByRole("tooltip").boundingBox();
  assert(
    tooltip.x >= 0 &&
      tooltip.x + tooltip.width <= 420 &&
      tooltip.y >= 0 &&
      tooltip.y + tooltip.height <= 600,
  );
  await page.screenshot({ path: `${assets}cpu-bar-narrow-scroll.png` });
  const axisBefore = await page.locator(".cpu-bar-axis").boundingBox();
  const scrollBefore = await page
    .locator(".processor-scroll")
    .evaluate((n) => n.scrollLeft);
  await fixture(128, [3, 10, 50, 100]);
  assert.equal(
    await page.locator(".processor-scroll").evaluate((n) => n.scrollLeft),
    scrollBefore,
  );
  await page.keyboard.press("Home");
  assert.equal(
    await page.locator(":focus").getAttribute("data-processor-id"),
    "0,0",
  );
  await page.waitForFunction(
    () => document.querySelector(".processor-scroll").scrollLeft === 0,
  );
  assert.equal(
    (await page.locator(".cpu-bar-axis").boundingBox()).x,
    axisBefore.x,
  );
  await page.keyboard.press("End");
  await fixture(6);
  assert.equal(await page.getByRole("tooltip").count(), 0);
  await page.evaluate(() => {
    window.__cpuFixture = null;
  });
  for (const label of [
    "采样中",
    "不支持",
    "权限不足",
    "采集失败",
    "数据过期",
  ]) {
    await page.getByRole("combobox", { name: "演示状态" }).click();
    await page.getByRole("option", { name: label, exact: true }).click();
    assert.equal(
      await page.locator(".cpu-bar-value").filter({ hasText: "—" }).count(),
      16,
    );
    assert.equal(
      await page.locator('.cpu-bar-column[data-valid="true"]').count(),
      0,
    );
    await cells.first().hover();
    assert((await page.getByRole("tooltip").innerText()).includes(label));
  }
  await page.screenshot({ path: `${assets}cpu-bar-stale.png` });
  await fixture(0);
  assert.equal(await cells.count(), 0);
  assert.equal(await page.getByRole("tooltip").count(), 0);
  assert(
    (await page.locator(".cpu-processors").innerText()).includes(
      "等待逻辑处理器采样",
    ),
  );
  assert.deepEqual(errors, []);
  console.log(
    "PASS: CPU bars 1–256 processors / 420–1100px, fixed scale, live hover, keyboard, tooltip bounds, removal, empty and five invalid states in light/dark.",
  );
} finally {
  await context.close();
  await browser.close();
}
