import { afterEach, expect, it, vi } from "vitest";
import { loadFont } from "../src/shared/ui/fonts";

afterEach(() => vi.unstubAllGlobals());

it("prepares all bundled weights before a font preference can be saved", async () => {
  const load = vi.fn().mockResolvedValue([{}]);
  vi.stubGlobal("document", { fonts: { load } });
  await loadFont("harmonyos_sans_sc");
  expect(load.mock.calls.map(([query]) => query)).toEqual([
    '400 14px "Pinmeter HarmonyOS Sans SC"',
    '500 14px "Pinmeter HarmonyOS Sans SC"',
    '700 14px "Pinmeter HarmonyOS Sans SC"',
  ]);
});

it("does not mistake an undeclared or failed font for a successful load", async () => {
  const load = vi.fn().mockResolvedValue([]);
  vi.stubGlobal("document", { fonts: { load } });
  await expect(loadFont("harmonyos_sans_sc")).rejects.toThrow("字体资源未加载");
  load.mockRejectedValue(new Error("font decode failed"));
  await expect(loadFont("geist")).rejects.toThrow("font decode failed");
  await expect(loadFont("unknown")).rejects.toThrow("无效的界面字体");
  await expect(loadFont("system")).resolves.toBeUndefined();
});
