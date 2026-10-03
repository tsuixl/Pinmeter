export const defaultFontFamily = "harmonyos_sans_sc";
export const fontOptions = [
  { value: defaultFontFamily, label: "HarmonyOS Sans SC（默认）" },
  { value: "geist", label: "Geist" },
  { value: "system", label: "系统默认" },
];

const stacks: Record<string, string> = {
  harmonyos_sans_sc:
    '"Pinmeter HarmonyOS Sans SC", "Microsoft YaHei UI", system-ui, sans-serif',
  geist:
    '"Geist Variable", "Geist", "Microsoft YaHei UI", system-ui, sans-serif',
  system:
    'system-ui, -apple-system, "Segoe UI", "Microsoft YaHei UI", sans-serif',
};

export function fontStack(font: string): string {
  return stacks[font] ?? stacks[defaultFontFamily];
}

// The alias deliberately skips local(): every installation uses our original TTFs.
export async function loadFont(font: string): Promise<void> {
  if (!(font in stacks)) throw new Error("无效的界面字体");
  if (font === "system") return;
  const family =
    font === "geist" ? "Geist Variable" : "Pinmeter HarmonyOS Sans SC";
  const faces = await Promise.all(
    [400, 500, 700].map((weight) =>
      document.fonts.load(`${weight} 14px "${family}"`, "内存 CPU 0123456789"),
    ),
  );
  if (faces.some((loaded) => loaded.length === 0)) {
    throw new Error("字体资源未加载，请检查完整运行目录");
  }
}

export function applyFont(font: string): void {
  document.documentElement.style.setProperty("--font-sans", fontStack(font));
  document.documentElement.dataset.font = font;
}
