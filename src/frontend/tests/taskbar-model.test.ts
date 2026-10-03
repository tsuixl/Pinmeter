import { describe, it, expect } from "vitest";
import cases from "../../shared/fixtures/taskbar-groups.json";
import {
  taskbarGroups,
  moveTaskbarGroup,
} from "../src/features/settings/taskbar-model";
import type { TaskbarSettingsDto } from "../src/shared/contracts/monitor";
const defaults: TaskbarSettingsDto = {
  enabled: true,
  hidden: false,
  layout: "double",
  network: true,
  cpu: true,
  cpu_temperature: true,
  gpu: true,
  gpu_temperature: true,
  memory: true,
  gpu_id: null,
  order: ["network", "cpu", "gpu", "memory"],
};
describe("任务栏预览与原生共享排列", () => {
  for (const [index, item] of cases.entries())
    it("分组样本 " + index, () => {
      const config: Partial<TaskbarSettingsDto> = item.settings;
      const settings = {
        ...defaults,
        ...config,
        cpu_temperature: config.cpu_temperature ?? config.cpu ?? true,
        gpu_temperature: config.gpu_temperature ?? config.gpu ?? true,
      };
      expect(taskbarGroups(settings)).toEqual(item.expected);
    });
  it("移动保持组完整且不修改已确认对象", () => {
    const next = moveTaskbarGroup(defaults, "cpu", -1);
    expect(next.order).toEqual(["cpu", "network", "gpu", "memory"]);
    expect(defaults.order[0]).toBe("network");
    expect(moveTaskbarGroup(defaults, "network", -1)).toBe(defaults);
  });
});
