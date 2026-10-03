import { afterEach, expect, it, vi } from "vitest";
import { loadFont } from "../src/shared/ui/fonts";
import {
  builtinFontCatalog,
  cssQuote,
  fontLabel,
  fontStyleOptions,
} from "../src/shared/fonts";

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
  await expect(loadFont("unknown")).rejects.toThrow("所选字体已不可用");
  await expect(loadFont("system")).resolves.toBeUndefined();
});

it("loads the installed face by its real name and preserves weight, italic and width", async () => {
  const constructed = vi.fn();
  class Face {
    constructor(
      family: string,
      source: string,
      descriptors: FontFaceDescriptors,
    ) {
      constructed(family, source, descriptors);
    }
    load() {
      return Promise.resolve(this);
    }
  }
  const add = vi.fn(),
    remove = vi.fn();
  vi.stubGlobal("FontFace", Face);
  vi.stubGlobal("document", { fonts: { add, delete: remove } });
  const family = {
    id: "installed:Font test",
    name: "测试字体",
    canonical_name: "Font test",
    source: "installed",
    styles: [
      {
        id: "700:2:3",
        name: "Narrow Bold Italic",
        weight: 700,
        slant: 2,
        stretch: 3,
        local_names: ['Font"BoldItalic', "Font Bold Italic"],
      },
    ],
  };
  await loadFont(family.id, "700:2:3", {
    families: [family],
    system_available: true,
    detail: "",
  });
  expect(constructed.mock.calls[0][1]).toBe(
    'local("Font\\"BoldItalic"), local("Font Bold Italic")',
  );
  expect(constructed.mock.calls[0][2]).toEqual({
    weight: "700",
    style: "italic",
    stretch: "75%",
  });
  expect(add).toHaveBeenCalledOnce();
  expect(remove).not.toHaveBeenCalled();
  expect(fontLabel(family)).toBe("测试字体 · Font test");
  expect(cssQuote('A\\B"C\n')).toBe('"A\\\\B\\"C "');
});

it("does not offer invented styles or load a removed preference", async () => {
  const styles = fontStyleOptions(builtinFontCatalog.families[0]);
  expect(styles.map((style) => style.value)).toEqual([
    "auto",
    "400:0:5",
    "500:0:5",
    "700:0:5",
  ]);
  await expect(loadFont("harmonyos_sans_sc", "500:2:5")).rejects.toThrow(
    "样式已不可用",
  );
  await expect(loadFont("installed:Removed", "400:0:5")).rejects.toThrow(
    "字体已不可用",
  );
});

it("bounds prepared local faces even if saving a preference never succeeds", async () => {
  class Face {
    load() {
      return Promise.resolve(this);
    }
  }
  const faces = new Set();
  vi.stubGlobal("FontFace", Face);
  vi.stubGlobal("document", {
    fonts: {
      add: (face: unknown) => faces.add(face),
      delete: (face: unknown) => faces.delete(face),
    },
  });
  for (let i = 0; i < 12; i++) {
    const family = {
      id: `installed:Cache test ${i}`,
      name: `Cache test ${i}`,
      canonical_name: `Cache test ${i}`,
      source: "installed",
      styles: [
        {
          id: "400:0:5",
          name: "Regular",
          weight: 400,
          slant: 0,
          stretch: 5,
          local_names: [`CacheTest${i}`],
        },
      ],
    };
    await loadFont(family.id, "400:0:5", {
      families: [family],
      system_available: true,
      detail: "",
    });
  }
  expect(faces.size).toBeLessThanOrEqual(6);
});
