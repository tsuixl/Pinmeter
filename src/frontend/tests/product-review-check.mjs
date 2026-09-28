import { chromium } from "@playwright/test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
const assets = fileURLToPath(
  new URL(
    "../../../docs/development/v0.1.0-main-window/assets/",
    import.meta.url,
  ),
);
const browser = await chromium.launch({ channel: "msedge", headless: true });
const page = await browser.newPage({ viewport: { width: 1280, height: 1000 } });
page.setDefaultTimeout(12000);
const errors = [];
const screenshot = (options) =>
  page.screenshot({ ...options, animations: "disabled" });
page.on("pageerror", (error) => errors.push(String(error)));
const navigate = (name) =>
  page.getByRole("button", { name, exact: true }).click();
try {
  await page.goto("http://127.0.0.1:1420/?demo=1");
  await page.locator(".overview-metric").first().waitFor();
  await page.evaluate(async () => {
    const url = performance
      .getEntriesByType("resource")
      .find((e) => e.name.includes("/src/shared/client/demo-client.ts"))?.name;
    const { DemoClient } = await import(url);
    const publish = Object.getPrototypeOf(DemoClient.prototype).publish;
    DemoClient.prototype.publish = function (value) {
      window.reviewClient = this;
      publish.call(this, value);
    };
  });
  await page.waitForFunction(() => window.reviewClient);
  await navigate("网络");
  await page
    .locator(".app-network")
    .getByRole("button", { name: "开始监控" })
    .click();
  const session = await page.evaluate(
    () => window.reviewClient.getSnapshot().state.app_network.session,
  );
  await navigate("CPU");
  await page
    .getByRole("button", { name: "应用流量监控中", exact: true })
    .waitFor();
  assert.equal(
    await page.evaluate(
      () => window.reviewClient.getSnapshot().state.app_network.session,
    ),
    session,
  );
  await navigate("网络");
  await page
    .locator(".app-network")
    .getByRole("button", { name: "停止监控" })
    .click();
  assert.equal(
    await page.evaluate(
      () => window.reviewClient.getSnapshot().state.app_network.running,
    ),
    false,
  );
  for (const theme of ["light", "dark"]) {
    await page.evaluate(async (theme) => {
      const client = window.reviewClient;
      await client.updateSettings({
        ...client.getSnapshot().state.settings,
        theme,
      });
    }, theme);
    await navigate("总览");
    await page.waitForFunction(
      (theme) => document.documentElement.dataset.theme === theme,
      theme,
    );
    await screenshot({ path: assets + `review-overview-${theme}.png` });
    await page.getByRole("button", { name: "使用帮助", exact: true }).click();
    await page.getByText("使用 Pinmeter", { exact: true }).waitFor();
    await screenshot({ path: assets + `review-help-${theme}.png` });
    await page.keyboard.press("Escape");
    await page.evaluate(() => {
      const client = window.reviewClient;
      const snapshot = client.getSnapshot();
      client.publish({
        ...snapshot,
        state: {
          ...snapshot.state,
          network_control: {
            ...snapshot.state.network_control,
            available: false,
            detail: "测试：辅助进程断开，可重试解除",
          },
        },
      });
      const release = client.releaseAllNetworkControl.bind(client);
      client.releaseAllNetworkControl = async (...args) => {
        if (!window.reviewReleaseFailed) {
          window.reviewReleaseFailed = true;
          throw new Error("测试：第一次解除失败");
        }
        await release(...args);
      };
      window.reviewReleaseFailed = false;
    });
    await page.getByRole("button", { name: /^运行状态：/ }).click();
    await page.getByText("网络限制待核对", { exact: true }).waitFor();
    const release = page.getByRole("button", {
      name: "解除全部网络限制",
      exact: true,
    });
    assert.equal(await release.isEnabled(), true);
    await release.click();
    await page.getByText("解除未完成，可重试", { exact: true }).waitFor();
    const details = await page.locator(".health-details").boundingBox();
    const sidebar = await page.locator(".sidebar").boundingBox();
    assert(
      details.x >= sidebar.x + sidebar.width,
      "Status details must not clip behind the sidebar",
    );
    await screenshot({ path: assets + `review-recovery-${theme}.png` });
    await release.click();
    await page
      .getByText("全部限制已解除，配置已保留为未启用。", { exact: true })
      .waitFor();
    await page.keyboard.press("Escape");
    await navigate("设置");
    await page
      .getByText("版本与升级", { exact: true })
      .scrollIntoViewIfNeeded();
    await screenshot({ path: assets + `review-version-${theme}.png` });
  }
  await navigate("总览");
  await page.evaluate(() => {
    const client = window.reviewClient;
    client.publish({
      ...client.getSnapshot(),
      connected: false,
      error: "测试：连接中断",
    });
    client.reconnect = async () =>
      client.publish({ ...client.getSnapshot(), connected: true, error: null });
  });
  await page
    .getByRole("button", { name: "运行状态：连接中断", exact: true })
    .click();
  await page.getByRole("button", { name: "重新连接", exact: true }).click();
  await page.waitForFunction(() => window.reviewClient.getSnapshot().connected);
  await page.keyboard.press("Escape");
  await page.setViewportSize({ width: 460, height: 850 });
  await page.getByRole("button", { name: /^运行状态：/ }).click();
  assert(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  );
  const narrowDetails = await page.locator(".health-details").boundingBox();
  assert(narrowDetails.x >= 0 && narrowDetails.x + narrowDetails.width <= 460);
  await screenshot({ path: assets + "review-status-narrow.png" });
  assert.deepEqual(errors, []);
  console.log(
    "PASS: continuous page monitoring, explicit stop, global unavailable-helper recovery/retry, reconnect, help, build information, light/dark and narrow layouts. Uses explicit demo fixtures; no system rules changed.",
  );
} finally {
  await browser.close();
}
