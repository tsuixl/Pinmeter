export const settingsSections = [
  { value: "resident", label: "通用" },
  { value: "appearance", label: "外观" },
  { value: "monitoring", label: "监控" },
  { value: "taskbar", label: "任务栏" },
  { value: "alerts", label: "占用提醒" },
  { value: "data", label: "数据与隐私" },
  { value: "diagnostics", label: "诊断与帮助" },
  { value: "updates", label: "关于与更新" },
] as const;
export type SettingsSection = (typeof settingsSections)[number]["value"];
