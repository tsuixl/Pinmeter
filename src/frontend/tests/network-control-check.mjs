import { waitForSettingsSave } from "./settings-helpers.mjs";
import { chromium } from "@playwright/test";
import assert from "node:assert/strict";
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
const assets = fileURLToPath(
  new URL(
    "../../../docs/development/v0.1.0-app-network-control/assets/",
    import.meta.url,
  ),
);
await mkdir(assets, { recursive: true });
const browser = await chromium.launch({ channel: "chrome", headless: true });
const context = await browser.newContext({
  viewport: { width: 1150, height: 980 },
});
const page = await context.newPage();
page.setDefaultTimeout(8000);
const errors = [];
page.on("pageerror", (e) => errors.push(String(e)));
try {
  await page.goto("http://127.0.0.1:1420/?demo=1");
  await page.evaluate(async () => {
    const url = performance
      .getEntriesByType("resource")
      .find((e) => e.name.includes("/src/shared/client/demo-client.ts"))?.name;
    const { DemoClient } = await import(url);
    const original = DemoClient.prototype.setAppNetworkMonitoring;
    DemoClient.prototype.setAppNetworkMonitoring = function (enabled) {
      window.__controlClient = this;
      return original.call(this, enabled);
    };
  });
  const chooseLimit = async (combo) => {
    await combo.click();
    const panel = await combo.getAttribute("aria-controls");
    await page
      .locator(`[id="${panel}"]`)
      .getByRole("option", { name: "设置上限", exact: true })
      .click();
    await page.locator(`[id="${panel}"]`).waitFor({ state: "hidden" });
  };
  for (const theme of ["light", "dark"]) {
    await page.getByRole("button", { name: "设置", exact: true }).click();
    await page
      .getByText(theme === "light" ? "浅色" : "深色", { exact: true })
      .click();
    await waitForSettingsSave(page);
    await page.getByRole("button", { name: "网络", exact: true }).click();
    const ranking = page.locator(".app-network");
    await ranking.getByRole("button", { name: "开始监控" }).click();
    const row = ranking
      .locator("tbody tr")
      .filter({ hasText: "浏览器（演示）" })
      .first();
    await row.click({ button: "right" });
    await page.getByRole("menu", { name: "浏览器（演示） 网络控制" }).waitFor();
    assert.equal(await page.getByRole("menuitem").count(), 3);
    await page.screenshot({ path: `${assets}/${theme}-context-menu.png` });
    await page.keyboard.press("Escape");
    await row.getByRole("button", { name: "浏览器（演示） 网络控制" }).click();
    await page.getByRole("menuitem", { name: "设置限速…" }).click();
    const dialog = page.getByRole("alertdialog", { name: "设置应用限速" });
    await chooseLimit(
      dialog.getByRole("combobox", { name: "下载上限", exact: true }),
    );
    await dialog.getByRole("spinbutton", { name: "下载上限数值" }).fill("0");
    await dialog.getByRole("button", { name: "保存", exact: true }).click();
    await dialog.getByRole("alert").waitFor();
    await page.screenshot({ path: `${assets}/${theme}-invalid-limit.png` });
    await dialog.getByRole("spinbutton", { name: "下载上限数值" }).fill("256");
    await chooseLimit(
      dialog.getByRole("combobox", { name: "上传上限", exact: true }),
    );
    await dialog.getByRole("spinbutton", { name: "上传上限数值" }).fill("64");
    assert.equal(await dialog.getByRole("alert").count(), 0);
    await page.screenshot({ path: `${assets}/${theme}-limit-dialog.png` });
    await page.evaluate(() => {
      const client = window.__controlClient;
      window.__normalChange = client.changeNetworkControl;
      client.changeNetworkControl = (change) =>
        new Promise((_, reject) => {
          window.__pendingChange = change;
          window.__rejectControl = reject;
        });
    });
    await dialog.getByRole("button", { name: "保存", exact: true }).click();
    assert.equal(
      await dialog
        .getByRole("spinbutton", { name: "下载上限数值" })
        .isDisabled(),
      true,
    );
    await page.screenshot({ path: `${assets}/${theme}-pending-dialog.png` });
    await page.evaluate(async () => {
      const client = window.__controlClient;
      await window.__normalChange.call(client, window.__pendingChange);
      const snapshot = client.getSnapshot();
      client.publish({
        ...snapshot,
        state: {
          ...snapshot.state,
          network_control: {
            ...snapshot.state.network_control,
            rules: snapshot.state.network_control.rules.map((r) =>
              r.id === window.__pendingChange.id
                ? {
                    ...r,
                    status: "failed",
                    limiting: false,
                    detail: "驱动不可用（测试故障）",
                  }
                : r,
            ),
          },
        },
      });
      window.__rejectControl(new Error("驱动不可用（测试故障）"));
    });
    await dialog.getByRole("alert").waitFor();
    assert.equal(
      await dialog
        .getByRole("spinbutton", { name: "下载上限数值" })
        .inputValue(),
      "256",
    );
    await page.screenshot({ path: `${assets}/${theme}-failure-dialog.png` });
    await page.evaluate(() => {
      window.__controlClient.changeNetworkControl = window.__normalChange;
    });
    await dialog.getByRole("button", { name: "保存", exact: true }).click();
    await dialog.waitFor({ state: "hidden" });
    assert.match(await row.innerText(), /下载 ≤ 256.0 KB\/s/);
    const openActions = async () =>
      row.getByRole("button", { name: "浏览器（演示） 网络控制" }).click();
    await openActions();
    await page.getByRole("menuitem", { name: "禁用网络", exact: true }).click();
    assert.match(await row.innerText(), /网络已禁用/);
    await openActions();
    await page.getByRole("menuitem", { name: "恢复网络", exact: true }).click();
    assert.match(await row.innerText(), /上传 ≤ 64.0 KB\/s/);
    await ranking.getByRole("button", { name: "展开 浏览器（演示）" }).click();
    await ranking
      .locator("tbody tr")
      .filter({ hasText: "PID 1000" })
      .click({ button: "right" });
    assert.equal(await page.getByRole("menu").count(), 0);
    await ranking.getByRole("button", { name: "停止监控" }).click();
    await page.getByRole("button", { name: "CPU", exact: true }).click();
    await page.getByRole("button", { name: "网络", exact: true }).click();
    const rules = page.getByRole("region", { name: "已配置网络规则" });
    await rules.scrollIntoViewIfNeeded();
    assert.match(await rules.innerText(), /上传 ≤ 64.0 KB\/s/);
    await page.screenshot({ path: `${assets}/${theme}-saved-rules.png` });
    await rules
      .getByRole("button", { name: "浏览器（演示） 网络控制" })
      .click();
    await page.keyboard.press("End");
    assert.equal(
      await page.evaluate(() => document.activeElement.textContent),
      "删除规则并解除限制",
    );
    await page.keyboard.press("Enter");
    assert.match(await rules.innerText(), /尚未设置限制/);
    await page.setViewportSize({ width: 460, height: 900 });
    await ranking.getByRole("button", { name: "开始监控" }).click();
    await row.getByRole("button", { name: "浏览器（演示） 网络控制" }).click();
    const menuBox = await page.getByRole("menu").boundingBox();
    assert(menuBox.x >= 0 && menuBox.x + menuBox.width <= 460);
    await page.getByRole("menuitem", { name: "设置限速…" }).click();
    await page.screenshot({ path: `${assets}/${theme}-narrow-dialog.png` });
    await dialog.getByRole("button", { name: "取消", exact: true }).click();
    await ranking.getByRole("button", { name: "停止监控" }).click();
    await page.setViewportSize({ width: 1150, height: 980 });
  }
  await page.getByRole("button", { name: "开始监控" }).click();
  await page.evaluate(() => {
    const client = window.__controlClient,
      snapshot = client.getSnapshot();
    client.publish({
      ...snapshot,
      state: {
        ...snapshot.state,
        network_control: {
          ...snapshot.state.network_control,
          available: false,
          detail: "网络控制不可用（测试状态）",
        },
      },
    });
  });
  const disabled = page.getByRole("button", {
    name: "浏览器（演示） 网络控制",
  });
  assert.equal(await disabled.isDisabled(), true);
  await page
    .getByRole("region", { name: "已配置网络规则" })
    .scrollIntoViewIfNeeded();
  await page.screenshot({ path: `${assets}/dark-unavailable.png` });
  assert.deepEqual(errors, []);
  console.log(
    "PASS: context/ellipsis menu, limits, block/restore, saved rule access across pages, validation, pending/failure/retry, keyboard, light/dark, narrow viewport (demo backend).",
  );
} finally {
  await browser.close();
}
