import { chromium } from "@playwright/test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { pollState } from "./desktop-helpers.mjs";

const assets = fileURLToPath(
  new URL(
    "../../../docs/development/v0.1.0-main-window/assets/",
    import.meta.url,
  ),
);
const browser = await chromium.connectOverCDP("http://127.0.0.1:9223");
const page = browser.contexts()[0].pages()[0];
const invoke = (command, args = {}) =>
  page.evaluate(
    ({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args),
    { command, args },
  );
const initial = await invoke("get_monitor_state");
try {
  const ready = await pollState(
    invoke,
    (s) =>
      s.frame?.cpu.status === "normal" &&
      s.cpu_temperature.status === "normal" &&
      s.gpu.devices.some((d) => d.readings.usage.status === "normal"),
  );
  await page.getByRole("button", { name: "总览", exact: true }).click();
  const cards = page.locator(".overview-metric");
  assert.equal(await cards.count(), 6);
  assert.match(await cards.nth(0).innerText(), /%/);
  assert.match(await cards.nth(1).innerText(), /°C/);
  assert.match(await cards.nth(2).innerText(), /%/);
  assert.match(await cards.last().innerText(), /↓.*\/s\n↑.*\/s/);
  for (const theme of ["light", "dark"]) {
    const state = await invoke("get_monitor_state");
    await invoke("update_settings", {
      settings: { ...state.settings, theme },
      expectedRevision: state.settings.revision,
    });
    await page.waitForFunction(
      (t) => document.documentElement.dataset.theme === t,
      theme,
    );
    await page.mouse.move(0, 0);
    await page.locator(".content").evaluate((n) => {
      n.scrollTop = 0;
    });
    await page.waitForTimeout(350);
    assert(
      await page
        .locator(".content")
        .evaluate((n) => n.scrollWidth <= n.clientWidth + 1),
    );
    await page.screenshot({
      path: assets + `overview-six-metrics-native-${theme}.png`,
    });
    const chart = page.locator(".overview-trends .chart-svg");
    await page
      .locator(".overview-trends")
      .evaluate((n) => n.scrollIntoView({ block: "end" }));
    await page.waitForTimeout(350);
    const box = await chart.boundingBox();
    await page.mouse.move(box.x + box.width - 2, box.y + box.height / 2);
    await page.getByRole("tooltip").waitFor();
    assert.equal(
      await page.getByRole("tooltip").locator(".tooltip-row").count(),
      5,
    );
    await page.waitForTimeout(100);
    await page.screenshot({
      path: assets + `overview-six-metrics-native-${theme}-trend.png`,
    });
  }
  const selectedName = await page
    .getByRole("combobox", { name: "显卡" })
    .innerText();
  await cards.nth(2).click();
  assert.equal(
    await page.getByRole("combobox", { name: "显卡" }).innerText(),
    selectedName,
  );
  await page.getByRole("button", { name: "总览", exact: true }).click();
  console.log(
    JSON.stringify({
      result:
        "PASS: native six metrics, real temperature/network, five-series tooltip, shared GPU choice, themes and no overflow",
      temperature: ready.cpu_temperature.text,
      gpus: ready.gpu.devices.map((d) => d.name),
    }),
  );
} finally {
  const state = await invoke("get_monitor_state");
  await invoke("update_settings", {
    settings: { ...initial.settings, revision: state.settings.revision },
    expectedRevision: state.settings.revision,
  });
  await browser.close();
}
