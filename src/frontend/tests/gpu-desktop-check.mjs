import { chromium } from "@playwright/test";
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { pollState } from "./desktop-helpers.mjs";

const assets = fileURLToPath(
  new URL(
    "../../../docs/development/v0.1.0-gpu-monitoring/assets/",
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
const errors = [];
page.on("pageerror", (e) => errors.push(String(e)));
try {
  const ready = await pollState(
    invoke,
    (s) =>
      s.gpu.status === "normal" &&
      s.gpu.devices.length === 2 &&
      s.gpu.devices.every((d) => d.readings.usage.status === "normal"),
  );
  assert(ready.gpu.devices.some((d) => d.name.includes("NVIDIA")));
  assert(ready.gpu.devices.some((d) => d.name.includes("AMD")));
  assert(ready.history.some((f) => f.gpus.length === 2));
  for (const device of ready.gpu.devices) {
    for (const [key, reading] of Object.entries(device.readings)) {
      if (
        (key === "temperature" && device.name.includes("AMD")) ||
        (key === "vr_soc_temperature" && device.name.includes("NVIDIA"))
      ) {
        assert.equal(reading.status, "unsupported");
        assert.equal(reading.value, null);
      } else {
        assert.equal(reading.status, "normal", `${device.name}/${key}`);
        assert(Number.isFinite(reading.value));
      }
    }
    assert.equal(device.readings.dedicated_used.unit, "GiB");
    assert.equal(device.readings.core_clock.unit, "MHz");
    assert.equal(device.readings.temperature.unit, "°C");
  }
  for (const theme of ["light", "dark"]) {
    const state = await invoke("get_monitor_state");
    await invoke("update_settings", {
      settings: { ...state.settings, theme },
      expectedRevision: state.settings.revision,
    });
    await page.getByRole("button", { name: "GPU", exact: true }).click();
    await page.waitForFunction(
      (t) => document.documentElement.dataset.theme === t,
      theme,
    );
    await page.waitForTimeout(500);
    const gpu = page.getByRole("region", { name: "GPU 详情" });
    assert.equal(await gpu.locator(".gpu-stats > *").count(), 6);
    const select = gpu.getByRole("combobox", { name: "显卡" });
    await select.click();
    await page.getByRole("option", { name: /NVIDIA/ }).click();
    await page.waitForTimeout(350);
    assert.match(await gpu.locator(".gpu-stats").innerText(), /°C/);
    await page.screenshot({ path: assets + `gpu-native-${theme}.png` });
    await gpu.locator(".gpu-charts").scrollIntoViewIfNeeded();
    await page.screenshot({ path: assets + `gpu-native-${theme}-charts.png` });
    await select.scrollIntoViewIfNeeded();
    await select.click();
    await page.getByRole("option", { name: /AMD/ }).click();
    await page.waitForTimeout(350);
    assert.match(
      await gpu.locator(".gpu-stats").innerText(),
      /未提供核心温度，显示 VR SoC 传感器读数/,
    );
    assert(
      (await gpu.locator('[data-metric="cpu_temperature"] .series').count()) >
        0,
    );
    await page.screenshot({
      path: assets + `gpu-native-${theme}-vrsoc.png`,
    });
  }
  await page.getByRole("button", { name: "网络", exact: true }).click();
  await page.getByRole("button", { name: "GPU", exact: true }).click();
  assert.equal((await invoke("get_monitor_state")).app_network.running, false);
  assert.deepEqual(errors, []);
  console.log(
    JSON.stringify(
      {
        result:
          "PASS: native GPU readings/history, both adapters, missing AMD core temperature, themes, page switching without network capture",
        gpu: ready.gpu,
      },
      null,
      2,
    ),
  );
} finally {
  const state = await invoke("get_monitor_state");
  await invoke("update_settings", {
    settings: { ...initial.settings, revision: state.settings.revision },
    expectedRevision: state.settings.revision,
  });
  await browser.close();
}
