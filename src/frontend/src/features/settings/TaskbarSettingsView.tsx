import { ArrowUp, ArrowDown } from "lucide-react";
import type { MonitorStateDto } from "../../shared/contracts/monitor";
import {
  Card,
  Switch,
  Select,
  Checkbox,
  IconButton,
  Button,
} from "../../shared/ui/sakani";
import type { SettingsViewModel } from "./useSettingsViewModel";
import { selectedMetricCount, type TaskbarMetric } from "./taskbar-model";
import { TaskbarPreview } from "./TaskbarPreview";
const groups: Record<
  string,
  { name: string; metrics: [TaskbarMetric, string][] }
> = {
  network: { name: "网速", metrics: [["network", "下载和上传"]] },
  cpu: {
    name: "CPU",
    metrics: [
      ["cpu", "CPU 使用率"],
      ["cpu_temperature", "CPU 温度"],
    ],
  },
  gpu: {
    name: "GPU",
    metrics: [
      ["gpu", "GPU 使用率"],
      ["gpu_temperature", "GPU 温度"],
    ],
  },
  memory: { name: "内存", metrics: [["memory", "内存使用率"]] },
};
export function TaskbarSettingsView({
  vm,
  state,
}: {
  vm: SettingsViewModel;
  state: MonitorStateDto | null;
}) {
  const value = vm.draft.taskbar;
  const disabled = vm.pending || !state?.desktop.supported;
  const devices = state?.gpu.devices ?? [];
  const gpuOptions = [
    { value: "auto", label: "自动选择" },
    ...devices.map((d) => ({ value: d.id, label: d.name })),
  ];
  if (value.gpu_id && !devices.some((d) => d.id === value.gpu_id))
    gpuOptions.push({
      value: value.gpu_id,
      label: "已保存的显卡 · 当前不可用",
    });
  return (
    <Card title="任务栏显示" description="选择读数、显示顺序和显卡">
      <Switch
        label="启用任务栏显示"
        checked={value.enabled}
        disabled={vm.pending || (!state?.desktop.supported && !value.enabled)}
        onChange={(event) =>
          vm.changeTaskbar({ enabled: event.target.checked, hidden: false })
        }
      />
      <div className="settings-fields">
        <Select
          id="taskbar-layout"
          label="读数布局"
          value={value.layout}
          disabled={disabled}
          options={[
            { value: "double", label: "双行紧凑" },
            { value: "single", label: "单行横排" },
          ]}
          onChange={(layout) => vm.changeTaskbar({ layout })}
        />
        <Select
          id="taskbar-gpu"
          label="任务栏显卡"
          value={value.gpu_id ?? "auto"}
          options={gpuOptions}
          disabled={disabled}
          onChange={(id) =>
            vm.changeTaskbar({ gpu_id: id === "auto" ? null : id })
          }
        />
      </div>
      <p className="settings-note">
        使用率和温度可独立显示。启用时至少保留一个指标；空间不足时优先显示排在前面的指标组。
      </p>
      <div className="taskbar-order" aria-label="任务栏指标顺序">
        {value.order.map((key, index) => {
          const group = groups[key];
          if (!group) return null;
          return (
            <div key={key} className="taskbar-order-row">
              <strong>{group.name}</strong>
              <div className="taskbar-order-metrics">
                {group.metrics.map(([field, label]) => (
                  <Checkbox
                    key={field}
                    label={label}
                    checked={value[field]}
                    disabled={
                      disabled ||
                      (value.enabled &&
                        value[field] &&
                        selectedMetricCount(value) === 1)
                    }
                    onChange={(event) =>
                      vm.changeTaskbar({ [field]: event.target.checked })
                    }
                  />
                ))}
              </div>
              <div className="taskbar-order-actions">
                <IconButton
                  variant="ghost"
                  size="sm"
                  icon={ArrowUp}
                  aria-label={"上移" + group.name}
                  disabled={disabled || index === 0}
                  onClick={() => vm.moveTaskbar(key, -1)}
                />
                <IconButton
                  variant="ghost"
                  size="sm"
                  icon={ArrowDown}
                  aria-label={"下移" + group.name}
                  disabled={disabled || index === value.order.length - 1}
                  onClick={() => vm.moveTaskbar(key, 1)}
                />
              </div>
            </div>
          );
        })}
      </div>
      <TaskbarPreview settings={value} />
      <p className="settings-note">
        指定显卡不可用时保留选择，读数显示 —。温度后的 * 表示 VR SoC
        测温点，详情可悬停查看。
      </p>
      <p className="settings-note" role="status">
        {state?.desktop.detail || "等待任务栏能力检测"}
      </p>
      {value.hidden && (
        <Button
          size="sm"
          variant="outline"
          disabled={disabled}
          onClick={() => vm.changeTaskbar({ hidden: false })}
        >
          恢复读数
        </Button>
      )}
    </Card>
  );
}
