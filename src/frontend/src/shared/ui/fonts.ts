import type {
  FontCatalogDto,
  FontFamilyDto,
  FontStyleDto,
} from "../contracts/monitor";
import { builtinFontCatalog, cssQuote } from "../fonts";
import harmonyRegular from "../../../../shared/fonts/harmonyos-sans-sc/HarmonyOS_Sans_SC_Regular.ttf?url";
import harmonyMedium from "../../../../shared/fonts/harmonyos-sans-sc/HarmonyOS_Sans_SC_Medium.ttf?url";
import harmonyBold from "../../../../shared/fonts/harmonyos-sans-sc/HarmonyOS_Sans_SC_Bold.ttf?url";
import geistCss from "@fontsource-variable/geist/index.css?inline";

export const defaultFontFamily = "harmonyos_sans_sc";
const stacks: Record<string, string> = {
  harmonyos_sans_sc:
    '"Pinmeter HarmonyOS Sans SC", "Microsoft YaHei UI", system-ui, sans-serif',
  geist:
    '"Geist Variable", "Geist", "Pinmeter HarmonyOS Sans SC", system-ui, sans-serif',
  system:
    'system-ui, -apple-system, "Segoe UI", "Pinmeter HarmonyOS Sans SC", sans-serif',
};
type LoadedFont = {
  stack: string;
  faces: FontFace[];
  sheet?: HTMLStyleElement;
  ready: Promise<void>;
};
const loaded = new Map<string, LoadedFont>();
let nextAlias = 0;
const key = (font: string, style: string) => JSON.stringify([font, style]);
let activeFontKey = key(defaultFontFamily, "auto");
function trimFonts(keep: string) {
  for (const [id, entry] of loaded) {
    if (loaded.size <= 6) break;
    if (id === keep || id === activeFontKey) continue;
    loaded.delete(id);
    void entry.ready
      .then(() => {
        entry.faces.forEach((face) => document.fonts.delete(face));
        entry.sheet?.remove();
      })
      .catch(() => {});
  }
}

export function fontStack(font: string, style = "auto"): string {
  return (
    loaded.get(key(font, style))?.stack ??
    (Object.hasOwn(stacks, font) ? stacks[font] : stacks[defaultFontFamily])
  );
}

const cssSlant = (style: FontStyleDto) =>
  style.slant === 2 ? "italic" : style.slant === 1 ? "oblique" : "normal";
const cssStretch = (style: FontStyleDto) =>
  `${[0, 50, 62.5, 75, 87.5, 100, 112.5, 125, 150, 200][style.stretch]}%`;
function nearest(styles: FontStyleDto[], weight: number): FontStyleDto {
  return [...styles].sort(
    (a, b) =>
      Number(a.slant !== 0) - Number(b.slant !== 0) ||
      Math.abs(a.stretch - 5) - Math.abs(b.stretch - 5) ||
      Math.abs(a.weight - weight) - Math.abs(b.weight - weight),
  )[0];
}
function localFaces(
  alias: string,
  family: FontFamilyDto,
  style: string,
): FontFace[] {
  const faces =
    style === "auto"
      ? family.styles
      : family.styles.filter((face) => face.id === style);
  return faces.map(
    (face) =>
      new FontFace(
        alias,
        face.local_names.map((name) => `local(${cssQuote(name)})`).join(", "),
        {
          weight: String(face.weight),
          style: cssSlant(face),
          stretch: cssStretch(face),
        },
      ),
  );
}

export async function loadFont(
  font: string,
  style = "auto",
  catalog: FontCatalogDto = builtinFontCatalog,
): Promise<void> {
  const family = catalog.families.find((f) => f.id === font);
  if (!family) throw new Error("所选字体已不可用，请刷新字体列表");
  const selected = family.styles.find((face) => face.id === style);
  if (style !== "auto" && !selected)
    throw new Error("所选字体样式已不可用，请重新选择");
  if (style === "auto" && Object.hasOwn(stacks, font)) {
    if (font === "system") return;
    const name =
      font === "geist" ? "Geist Variable" : "Pinmeter HarmonyOS Sans SC";
    const faces = await Promise.all(
      [400, 500, 700].map((weight) =>
        document.fonts.load(`${weight} 14px "${name}"`, "内存 CPU 0123456789"),
      ),
    );
    if (faces.some((f) => f.length === 0))
      throw new Error("字体资源未加载，请检查完整运行目录");
    return;
  }
  const id = key(font, style);
  const existing = loaded.get(id);
  if (existing) return existing.ready;
  const alias = `Pinmeter Selected Font ${++nextAlias}`;
  const entry: LoadedFont = {
    stack: `${cssQuote(alias)}, ${stacks[defaultFontFamily]}`,
    faces: [],
    ready: Promise.resolve(),
  };
  loaded.set(id, entry);
  entry.ready = (async () => {
    if (font === "geist") {
      const sheet = document.createElement("style");
      sheet.textContent = geistCss
        .replaceAll("Geist Variable", alias)
        .replaceAll("100 900", String(selected!.weight));
      document.head.append(sheet);
      entry.sheet = sheet;
      const faces = await document.fonts.load(
        `400 14px ${cssQuote(alias)}`,
        "CPU 0123456789",
      );
      if (!faces.length) throw new Error("Geist 样式加载失败");
    } else {
      if (font === "harmonyos_sans_sc") {
        const urls: Record<number, string> = {
          400: harmonyRegular,
          500: harmonyMedium,
          700: harmonyBold,
        };
        entry.faces = [
          new FontFace(alias, `url(${cssQuote(urls[selected!.weight])})`, {
            weight: String(selected!.weight),
          }),
        ];
      } else {
        entry.faces = localFaces(alias, family, style);
      }
      entry.faces.forEach((face) => document.fonts.add(face));
      const needed =
        style === "auto"
          ? [
              ...new Set(
                [400, 500, 700].map((weight) =>
                  family.styles.indexOf(nearest(family.styles, weight)),
                ),
              ),
            ]
          : [0];
      await Promise.all(needed.map((index) => entry.faces[index].load()));
    }
  })().catch((error) => {
    entry.faces.forEach((face) => document.fonts.delete(face));
    entry.sheet?.remove();
    if (loaded.get(id) === entry) loaded.delete(id);
    console.warn("Selected font face could not be loaded", error);
    throw new Error("所选字面无法在当前界面加载，请尝试其他样式或字体");
  });
  trimFonts(id);
  return entry.ready;
}

export function applyFont(font: string, style = "auto"): void {
  document.documentElement.style.setProperty(
    "--font-sans",
    fontStack(font, style),
  );
  document.documentElement.dataset.font = font;
  document.documentElement.dataset.fontStyle = style;
  activeFontKey = key(font, style);
  trimFonts(activeFontKey);
}
