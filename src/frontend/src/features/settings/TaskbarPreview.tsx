import { ArrowDown, ArrowUp } from "lucide-react";
import type { TaskbarSettingsDto } from "../../shared/contracts/monitor";
import { taskbarGroups } from "./taskbar-model";
import "./taskbar.css";
export function TaskbarPreview({ settings }: { settings: TaskbarSettingsDto }) {
  const readings = [
    {
      label: "下载",
      value: "3.2",
      unit: "MB/s",
      temperature: null,
      show: true,
    },
    {
      label: "上传",
      value: "128.0",
      unit: "KB/s",
      temperature: null,
      show: true,
    },
    {
      label: "CPU",
      value: "2",
      unit: "%",
      temperature: settings.cpu_temperature ? 57 : null,
      show: settings.cpu,
    },
    {
      label: "内存",
      value: "44",
      unit: "%",
      temperature: null,
      show: settings.memory,
    },
    {
      label: "GPU",
      value: "18",
      unit: "%",
      temperature: settings.gpu_temperature ? 49 : null,
      show: settings.gpu,
    },
  ];
  return (
    <div className="taskbar-preview-container">
      <span className="settings-note">布局预览 · 示例数据</span>
      <div className="taskbar-preview" aria-label="任务栏读数布局预览">
        {taskbarGroups(settings).map((indices, i) => {
          const network = indices[0] < 2;
          const anyValue = indices.some((index) => readings[index].show);
          const anyTemperature = indices.some(
            (index) => readings[index].temperature !== null,
          );
          return (
            <div
              key={i}
              className={
                "taskbar-preview-group" +
                (!network ? " taskbar-preview-percent" : "") +
                (anyTemperature ? " taskbar-preview-hardware" : "") +
                (!anyValue ? " taskbar-preview-no-values" : "")
              }
            >
              {indices.map((index) => {
                const r = readings[index];
                return (
                  <div key={index}>
                    {index === 0 ? (
                      <ArrowDown size={14} />
                    ) : index === 1 ? (
                      <ArrowUp size={14} />
                    ) : (
                      <span>{r.label}</span>
                    )}
                    {anyValue && (
                      <strong>
                        {r.show ? r.value : ""}
                        {index >= 2 && r.show && <span>%</span>}
                      </strong>
                    )}
                    {network && <span>{r.unit}</span>}
                    {r.temperature !== null && (
                      <span className="taskbar-preview-temperature">
                        ({r.temperature}°C)
                      </span>
                    )}
                  </div>
                );
              })}
            </div>
          );
        })}
      </div>
    </div>
  );
}
