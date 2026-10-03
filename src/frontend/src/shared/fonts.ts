import type {
  FontCatalogDto,
  FontFamilyDto,
  FontStyleDto,
} from "./contracts/monitor";

const face = (name: string, weight: number): FontStyleDto => ({
  id: `${weight}:0:5`,
  name,
  weight,
  slant: 0,
  stretch: 5,
  local_names: [],
});
export const builtinFontCatalog: FontCatalogDto = {
  families: [
    {
      id: "harmonyos_sans_sc",
      name: "HarmonyOS Sans SC（内置默认）",
      canonical_name: "HarmonyOS Sans SC",
      source: "builtin",
      styles: [face("Regular", 400), face("Medium", 500), face("Bold", 700)],
    },
    {
      id: "geist",
      name: "Geist（内置）",
      canonical_name: "Geist",
      source: "builtin",
      styles: [
        "Thin",
        "ExtraLight",
        "Light",
        "Regular",
        "Medium",
        "SemiBold",
        "Bold",
        "ExtraBold",
        "Black",
      ].map((name, i) => face(name, (i + 1) * 100)),
    },
    {
      id: "system",
      name: "系统默认",
      canonical_name: "",
      source: "system",
      styles: [],
    },
  ],
  system_available: false,
  detail: "尚未读取系统字体",
};

export function fontLabel(family: FontFamilyDto): string {
  if (family.source !== "installed") return family.name;
  return family.name === family.canonical_name
    ? `${family.name}（系统）`
    : `${family.name} · ${family.canonical_name}`;
}
export function fontStyleOptions(family?: FontFamilyDto) {
  return [
    { value: "auto", label: "默认排版" },
    ...(family?.styles.map((style) => ({
      value: style.id,
      label: `${style.name} · ${style.weight}`,
    })) ?? []),
  ];
}
export function cssQuote(value: string): string {
  return `"${value
    .replace(/\\/g, "\\\\")
    .replace(/"/g, '\\"')
    .replace(/[\r\n\f]/g, " ")}"`;
}
