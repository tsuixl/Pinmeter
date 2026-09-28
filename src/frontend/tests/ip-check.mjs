import { chromium, expect } from "@playwright/test";
import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
const assets = fileURLToPath(
  new URL(
    "../../../docs/development/v0.1.0-ip-inspection/assets/",
    import.meta.url,
  ),
);
await mkdir(assets, { recursive: true });
const browser = await chromium.launch({ channel: "chrome", headless: true });
const context = await browser.newContext({
  viewport: { width: 1280, height: 1000 },
  permissions: ["clipboard-read", "clipboard-write"],
});
const page = await context.newPage();
page.setDefaultTimeout(15000);
const errors = [];
try {
  if (process.argv.includes("--references")) {
    for (const story of [
      "composite-card--default",
      "composite-card--dark-mode",
      "core-button--loading",
      "core-badge--dark-mode",
      "composite-alert--danger",
      "composite-table--default",
      "composite-table--dark-mode",
      "core-skeleton--card-placeholder",
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
      await page.screenshot({
        path: assets + `reference-${story}.png`,
        fullPage: true,
      });
    }
  }
  page.on("pageerror", (e) => errors.push(String(e)));
  await page.goto(
    `${process.env.PINMETER_UI_URL ?? "http://127.0.0.1:1421"}/?demo=1`,
  );
  await page.evaluate(async () => {
    const entry = performance
      .getEntriesByType("resource")
      .find((e) => e.name.includes("/src/shared/client/demo-client.ts"));
    const { DemoClient } = await import(entry.name);
    const original = DemoClient.prototype.publish;
    DemoClient.prototype.publish = function (snapshot) {
      window.__ipClient = this;
      if (window.__ipFixture) snapshot.state.ip = window.__ipFixture;
      if (window.__ipOffline) snapshot.connected = false;
      return original.call(this, snapshot);
    };
  });
  await page.waitForFunction(() => window.__ipClient);
  await page.getByRole("button", { name: "IP", exact: true }).click();
  await page.getByRole("heading", { name: "公网 IPv4" }).waitFor();
  assert.equal(await page.getByLabel("趋势时间范围").count(), 0);
  for (const title of ["网络连通性", "AI 访问概览", "官方服务状态"])
    await page.getByRole("heading", { name: title, exact: true }).waitFor();
  assert.equal(await page.locator(".ip-brand img").count(), 22);
  assert(
    await page
      .locator(".ip-brand img")
      .evaluateAll((imgs) =>
        imgs.every(
          (i) =>
            i.complete &&
            i.naturalWidth > 0 &&
            new URL(i.src).origin === location.origin,
        ),
      ),
  );
  assert.equal(await page.locator(".ip-details").getAttribute("open"), null);
  const assertAiFirst = async () => {
    const ai = await page.locator(".ip-check-ai").boundingBox();
    const network = await page.locator(".ip-check-connectivity").boundingBox();
    assert(ai.y + ai.height <= network.y);
  };
  await assertAiFirst();
  for (const theme of ["light", "dark"]) {
    await page.evaluate((theme) => {
      const c = window.__ipClient;
      return c.updateSettings({ ...c.getSnapshot().state.settings, theme });
    }, theme);
    await page.waitForFunction(
      (t) => document.documentElement.dataset.theme === t,
      theme,
    );
    await page.waitForTimeout(2000);
    await page.locator("main").evaluate((e) => (e.scrollTop = 0));
    await page.screenshot({ path: assets + "ip-" + theme + ".png" });
    await page
      .getByRole("heading", { name: "AI 访问概览" })
      .evaluate((e) => e.scrollIntoView({ block: "start" }));
    await page.screenshot({ path: assets + "ip-" + theme + "-checks.png" });
    await page.locator(".ip-details > summary").click();
    await page
      .getByRole("heading", { name: "地理位置 · 多源对比" })
      .evaluate((e) => e.scrollIntoView({ block: "start" }));
    await page.screenshot({ path: assets + "ip-" + theme + "-details.png" });
    await page.locator(".ip-details > summary").click();
  }
  for (const code of [200, 302, 403, 429, 503]) {
    await page.evaluate((code) => {
      const s = structuredClone(window.__ipClient.getSnapshot().state.ip);
      const g = s.checks.find((g) => g.id === "ai");
      const r = g.rows.find((r) => r.id === "chatgpt");
      Object.assign(r, {
        status: "ready",
        http_status: code,
        latency_ms: 350,
        error: null,
        samples: [350],
      });
      g.retry_after_ms = code === 429 ? 120000 : 0;
      window.__ipFixture = s;
    }, code);
    await page.waitForFunction(
      (code) =>
        window.__ipClient
          .getSnapshot()
          .state.ip.checks.find((g) => g.id === "ai")
          .rows.find((r) => r.id === "chatgpt").http_status === code,
      code,
    );
    const expected = "350 ms";
    await page
      .locator(".ip-check-ai")
      .getByText(expected, { exact: true })
      .waitFor();
    assert.equal(
      await page
        .locator(".ip-check-ai")
        .getByText("未连通", { exact: true })
        .count(),
      0,
    );
    if (code === 429)
      await expect(
        page.getByRole("button", { name: "刷新AI 访问概览" }),
      ).toBeDisabled();
    await page.getByRole("button", { name: "查看ChatGPT检测详情" }).click();
    await page
      .getByRole("region", { name: "ChatGPT检测详情" })
      .getByText("最近有效耗时 350 ms", { exact: false })
      .waitFor();
    assert.equal(
      await page
        .locator(".ip-check-ai")
        .getByText(/受限|重定向|HTTP 响应/)
        .count(),
      0,
    );
    await expect(
      page
        .locator(".ip-check-ai .ip-state-label")
        .filter({ hasText: "350 ms" })
        .locator("svg"),
    ).toHaveClass(/circle-check/);
    if (code === 403) {
      await page
        .locator(".ip-check-ai")
        .evaluate((e) => e.scrollIntoView({ block: "start" }));
      await page.screenshot({ path: assets + "ip-ai-response.png" });
    }
    await page.getByRole("button", { name: "查看ChatGPT检测详情" }).click();
  }
  await page.evaluate(() => {
    const g = window.__ipFixture.checks.find((g) => g.id === "ai");
    const r = g.rows.find((r) => r.id === "chatgpt");
    Object.assign(r, {
      status: "stale",
      error: "测试：数据源请求超时",
      samples: [null],
    });
  });
  await page
    .locator(".ip-check-ai")
    .getByText("未取得响应", { exact: true })
    .waitFor();
  await page.getByRole("button", { name: "查看ChatGPT检测详情" }).click();
  await page
    .getByRole("region", { name: "ChatGPT检测详情" })
    .getByText("测试：数据源请求超时", { exact: true })
    .waitFor();
  await page.getByRole("button", { name: "查看ChatGPT检测详情" }).click();
  await page.evaluate(() => {
    window.__ipFixture = null;
  });
  await page.getByRole("button", { name: "查看OpenAI检测详情" }).click();
  await page
    .getByRole("region", { name: "OpenAI检测详情" })
    .getByText("演示事件：部分服务延迟", { exact: false })
    .waitFor();
  await page.getByRole("button", { name: "查看OpenAI检测详情" }).click();
  await page.evaluate(() => {
    window.__ipClient.refreshIpChecks = async (section) => {
      window.__refreshedSection = section;
    };
  });
  await page.getByRole("button", { name: "刷新AI 访问概览" }).click();
  await page.waitForFunction(() => window.__refreshedSection === "ai");
  await page
    .getByRole("button", { name: "复制公网 IPv4", exact: true })
    .click();
  await page.getByRole("status").getByText("IP 地址已复制").waitFor();
  assert.equal(
    await page.evaluate(() => navigator.clipboard.readText()),
    "1.1.1.1",
  );
  await page.evaluate(() => {
    Object.defineProperty(navigator.clipboard, "writeText", {
      configurable: true,
      value: async () => {
        throw Error("denied");
      },
    });
  });
  await page
    .getByRole("button", { name: "复制公网 IPv4", exact: true })
    .click();
  await page.getByRole("alert").filter({ hasText: "复制" }).waitFor();
  await page.evaluate(() => {
    window.__ipFixture = structuredClone(
      window.__ipClient.getSnapshot().state.ip,
    );
    window.__ipFixture.running = true;
    window.__ipFixture.exits[0].status = "loading";
    window.__ipFixture.profiles[0].status = "loading";
    window.__ipFixture.checks[0].running = true;
    window.__ipFixture.checks[0].rows[0].status = "loading";
    window.__ipFixture.checks[0].rows[0].samples = [80, null];
  });
  await page.getByRole("button", { name: "正在检测…" }).waitFor();
  assert(await page.getByRole("button", { name: "正在检测…" }).isDisabled());
  await page.locator("main").evaluate((e) => (e.scrollTop = 0));
  await page.screenshot({ path: assets + "ip-loading.png" });
  const validAt = await page.evaluate(
    () => window.__ipFixture.profiles[0].valid_at_ms,
  );
  await page.evaluate(() => {
    const s = structuredClone(window.__ipFixture);
    s.running = false;
    s.exits[0].status = "stale";
    s.profiles[0].status = "stale";
    s.profiles[0].error = null;
    s.retry_after_ms = 60000;
    s.checks[0].running = false;
    window.__ipFixture = s;
  });
  await page.getByRole("button", { name: /秒后可刷新/ }).waitFor();
  assert.equal(await page.getByText(/IP 资料.*数据过期/).count(), 0);
  assert.equal(
    await page
      .locator(".ip-page")
      .getByText("数据过期", { exact: true })
      .count(),
    0,
  );
  assert.equal(
    await page.evaluate(
      () => window.__ipClient.getSnapshot().state.ip.profiles[0].valid_at_ms,
    ),
    validAt,
  );
  assert(
    (await page.locator(".ip-fetch-time").allTextContents()).some((t) =>
      t.includes("数据获取于"),
    ),
  );
  await page.screenshot({ path: assets + "ip-stale.png" });
  await page.evaluate(() => {
    const s = structuredClone(window.__ipFixture);
    s.profiles[0].error = "测试：本轮资料获取失败";
    s.profiles[0].data.score = null;
    const r = s.checks[2].rows[0];
    r.status = "failed";
    r.service_state = null;
    r.valid_at_ms = null;
    r.error = "测试：官方来源暂不可用";
    r.incidents = [];
    window.__ipFixture = s;
  });
  await page.getByText("测试：本轮资料获取失败", { exact: true }).waitFor();
  await page
    .locator(".ip-check-services")
    .getByText("获取失败", { exact: true })
    .waitFor();
  assert.equal(
    await page.locator(".ip-score").last().innerText(),
    "—\nIP 信誉",
  );
  await page.getByRole("button", { name: "查看OpenAI检测详情" }).click();
  await page
    .getByRole("region", { name: "OpenAI检测详情" })
    .getByText("测试：官方来源暂不可用", { exact: true })
    .waitFor();
  await page.screenshot({ path: assets + "ip-check-failure.png" });
  await page.getByRole("button", { name: "查看OpenAI检测详情" }).click();
  await page.setViewportSize({ width: 880, height: 700 });
  await assertAiFirst();
  await page.locator("main").evaluate((e) => (e.scrollTop = 0));
  await page.screenshot({ path: assets + "ip-default-window.png" });
  assert(
    await page
      .locator("main")
      .evaluate((e) => e.scrollWidth <= e.clientWidth + 1),
  );
  await page.setViewportSize({ width: 420, height: 920 });
  await assertAiFirst();
  await page.locator("main").evaluate((e) => (e.scrollTop = 0));
  await page.screenshot({ path: assets + "ip-narrow.png" });
  await page
    .getByRole("heading", { name: "AI 访问概览" })
    .evaluate((e) => e.scrollIntoView({ block: "start" }));
  await page.screenshot({ path: assets + "ip-narrow-checks.png" });
  assert(
    await page
      .locator("main")
      .evaluate((e) => e.scrollWidth <= e.clientWidth + 1),
  );
  await page.evaluate(() => {
    window.__ipOffline = true;
  });
  await page.getByText("查询服务未连接", { exact: true }).waitFor();
  assert(
    await page.getByRole("button", { name: "刷新官方服务状态" }).isDisabled(),
  );
  await page
    .getByRole("button", { name: "查看Claude检测详情" })
    .first()
    .focus();
  await page.keyboard.press("Enter");
  await page.getByRole("region", { name: "Claude检测详情" }).waitFor();
  assert.deepEqual(errors, []);
  console.log(
    "PASS: three check panels, 22 local icons, per-section refresh, events, dates without stale banner, unknown/failure, copy, themes, keyboard, 880/420px layout.",
  );
} finally {
  await context.close();
  await browser.close();
}
