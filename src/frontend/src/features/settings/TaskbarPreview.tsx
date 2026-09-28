import { ArrowDown, ArrowUp } from "lucide-react";
import type { TaskbarSettingsDto } from "../../shared/contracts/monitor";
import "./taskbar.css";

export function TaskbarPreview({ settings }: { settings: TaskbarSettingsDto }) {
  const hardware = [
    ...(settings.cpu ? [{ label: "CPU", usage: 2, temperature: 57 }] : []),
    ...(settings.gpu ? [{ label: "GPU", usage: 18, temperature: 49 }] : []),
  ];
  const memory = { label: "内存", usage: 44, temperature: null };
  const groups =
    hardware.length < 2
      ? [[...hardware, ...(settings.memory ? [memory] : [])]]
      : [hardware, ...(settings.memory ? [[memory]] : [])];
  return (
    <div className="taskbar-preview-container">
      <span className="settings-note">布局预览 · 示例数据</span>
      <div
        className={`taskbar-preview ${settings.layout === "single" ? "taskbar-preview-single" : ""}`}
        aria-label="任务栏读数布局预览"
      >
        <div className="taskbar-preview-group">
          <div>
            <ArrowDown size={14} />
            <strong>3.2</strong>
            <span>MB/s</span>
          </div>
          <div>
            <ArrowUp size={14} />
            <strong>128.0</strong>
            <span>KB/s</span>
          </div>
        </div>
        {groups
          .filter((group) => group.length > 0)
          .map((group) => (
            <div
              key={group[0].label}
              className={`taskbar-preview-group taskbar-preview-percent ${group.some((r) => r.temperature !== null) ? "taskbar-preview-hardware" : ""}`}
            >
              {group.map((reading) => (
                <div key={reading.label}>
                  <span>{reading.label}</span>
                  <strong>
                    {reading.usage}
                    <span>%</span>
                  </strong>
                  {reading.temperature !== null && (
                    <span className="taskbar-preview-temperature">
                      ({reading.temperature}°C)
                    </span>
                  )}
                </div>
              ))}
            </div>
          ))}
      </div>
    </div>
  );
}
