import { chromium } from "@playwright/test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { copyFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { pollState } from "./desktop-helpers.mjs";

const root = fileURLToPath(new URL("../../../", import.meta.url));
const assets = `${root}docs/development/v0.1.0-main-window/assets/`;
const browser = await chromium.connectOverCDP("http://127.0.0.1:9223");
const page = browser.contexts()[0].pages()[0];
const invoke = (command, args = {}) =>
  page.evaluate(
    ({ command, args }) => window.__TAURI_INTERNALS__.invoke(command, args),
    { command, args },
  );
const native = (command) =>
  invoke(`plugin:window|${command}`, { label: "main" });
const windowCheck = (action, args = []) =>
  JSON.parse(
    execFileSync(
      "powershell",
      [
        "-ExecutionPolicy",
        "Bypass",
        "-File",
        "tools/window-check.ps1",
        "-Action",
        action,
        ...args,
      ],
      { cwd: root, encoding: "utf8" },
    ),
  );
const capture = async (name) => {
  const window = windowCheck("capture");
  await copyFile(
    `${root}src/backend/target/measurements/release-window.png`,
    `${assets}${name}.png`,
  );
  return window;
};
await page.getByRole("button", { name: "最小化", exact: true }).waitFor();
await page.getByText("本机实时数据", { exact: true }).waitFor();
const initial = (await invoke("get_monitor_state")).settings;
const wasMaximized = await native("is_maximized");
if (wasMaximized)
  await page.getByRole("button", { name: "还原窗口", exact: true }).click();
const original = windowCheck("inspect");
const errors = [];
page.on("pageerror", (error) => errors.push(String(error)));
try {
  assert.equal(await native("is_decorated"), false);
  await page.getByRole("button", { name: "最大化", exact: true }).click();
  await page.getByRole("button", { name: "还原窗口", exact: true }).waitFor();
  assert.equal(await native("is_maximized"), true);
  await page.getByRole("button", { name: "还原窗口", exact: true }).click();
  await page.getByRole("button", { name: "最大化", exact: true }).waitFor();
  // The Tauri drag-region handler must handle a real double click.
  await page.locator(".topbar").dblclick({ position: { x: 210, y: 24 } });
  await page.getByRole("button", { name: "还原窗口", exact: true }).waitFor();
  assert.equal(await native("is_maximized"), true);
  await page.getByRole("button", { name: "还原窗口", exact: true }).click();
  await page.getByRole("button", { name: "最大化", exact: true }).waitFor();
  for (const theme of ["light", "dark", "system"]) {
    const state = await invoke("get_monitor_state");
    await invoke("update_settings", {
      settings: { ...state.settings, theme },
      expectedRevision: state.settings.revision,
    });
    const expected = theme === "system" ? await native("theme") : theme;
    await page.waitForFunction(
      (value) => document.documentElement.dataset.theme === value,
      expected,
    );
    await page.waitForTimeout(350);
    let position;
    for (const label of ["总览", "CPU", "内存", "网络", "设置"]) {
      await page.getByRole("button", { name: label, exact: true }).click();
      const box = await page
        .getByRole("group", { name: "窗口控制" })
        .boundingBox();
      position ??= box;
      assert.deepEqual(
        box,
        position,
        "window controls must stay fixed across pages",
      );
    }
    await page.getByRole("button", { name: "总览", exact: true }).click();
    const result = await capture(`titlebar-native-${theme}`);
    console.log(
      `PASS: ${theme} (${expected}), fixed controls, no native caption, DPI ${result.Dpi}`,
    );
    await page.getByRole("button", { name: "关闭", exact: true }).hover();
    await page.waitForTimeout(250);
    await capture(`titlebar-${theme}-hover`);
    await page.getByRole("button", { name: "最小化", exact: true }).focus();
    await page.keyboard.press("Tab");
    assert.equal(
      await page.evaluate(() =>
        document.activeElement?.getAttribute("aria-label"),
      ),
      "最大化",
    );
    await capture(`titlebar-${theme}-focus`);
    // Visual fixture only: keep the disabled state on screen long enough to
    // compare the unmodified Sakani styling. This does not simulate an IPC failure.
    if (theme !== "system") {
      const buttons = page.locator(".window-controls button");
      try {
        await buttons.evaluateAll((elements) =>
          elements.forEach((button) => {
            button.disabled = true;
          }),
        );
        await page.waitForTimeout(250);
        await capture(`titlebar-${theme}-disabled`);
      } finally {
        await buttons.evaluateAll((elements) =>
          elements.forEach((button) => {
            button.disabled = false;
          }),
        );
      }
    }
  }
  for (const width of [840, 1760, 2200]) {
    windowCheck("move", [
      "-X",
      "100",
      "-Y",
      "100",
      "-Width",
      String(width),
      "-Height",
      "1200",
    ]);
    await page.waitForTimeout(350);
    assert.equal(
      await page.evaluate(
        () => document.documentElement.scrollWidth > innerWidth,
      ),
      false,
    );
    const controls = await page
      .getByRole("group", { name: "窗口控制" })
      .boundingBox();
    const viewport = await page.evaluate(() => ({
      width: innerWidth,
      height: innerHeight,
    }));
    assert(controls.x >= 0 && controls.x + controls.width <= viewport.width);
    assert(controls.y >= 0 && controls.y + controls.height <= 48);
    assert.equal(
      await page.getByRole("button", { name: "关闭", exact: true }).isEnabled(),
      true,
    );
    await capture(`titlebar-width-${width}`);
  }
  const before = await invoke("get_monitor_state");
  await page.getByRole("button", { name: "最小化", exact: true }).click();
  await page.waitForTimeout(600);
  assert.equal(await native("is_minimized"), true);
  await page.evaluate(() => {
    window.titlebarMutations = 0;
    window.titlebarObserver = new MutationObserver(
      () => window.titlebarMutations++,
    );
    window.titlebarObserver.observe(document.querySelector("main"), {
      subtree: true,
      childList: true,
      characterData: true,
    });
  });
  await pollState(
    invoke,
    (state) => BigInt(state.frame.cursor) > BigInt(before.frame.cursor) + 1n,
  );
  assert.equal(await page.evaluate(() => window.titlebarMutations), 0);
  await invoke("perform_desktop_action", { action: "activate" });
  await page.waitForFunction(() => window.titlebarMutations > 0);
  await page.evaluate(() => window.titlebarObserver.disconnect());
  assert.equal(
    (await invoke("get_monitor_state")).session_id,
    before.session_id,
  );
  assert.deepEqual(errors, []);
  console.log(
    "PASS: maximize/restore, drag-region double click, keyboard focus, narrow/expanded layout, minimize pauses rendering while sampling continues, restore resumes.",
  );
} finally {
  await invoke("perform_desktop_action", { action: "activate" });
  if (await native("is_maximized"))
    await page.getByRole("button", { name: "还原窗口", exact: true }).click();
  windowCheck("move", [
    "-X",
    String(original.Left),
    "-Y",
    String(original.Top),
    "-Width",
    String(original.Width),
    "-Height",
    String(original.Height),
  ]);
  if (wasMaximized)
    await page.getByRole("button", { name: "最大化", exact: true }).click();
  const state = await invoke("get_monitor_state");
  await invoke("update_settings", {
    settings: { ...initial, revision: state.settings.revision },
    expectedRevision: state.settings.revision,
  });
  await browser.close();
}
