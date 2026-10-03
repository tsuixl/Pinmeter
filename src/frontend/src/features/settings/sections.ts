export const settingsSections = [
  { value: "appearance", label: "外观" },
  { value: "resident", label: "启动与窗口" },
  { value: "monitoring", label: "监控" },
  { value: "taskbar", label: "任务栏" },
  { value: "data", label: "数据与隐私" },
  { value: "alerts", label: "占用提醒" },
  { value: "updates", label: "版本与更新" },
  { value: "diagnostics", label: "诊断与帮助" },
] as const;
export type SettingsSection = (typeof settingsSections)[number]["value"];
