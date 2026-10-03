import type { TaskbarSettingsDto } from "../../shared/contracts/monitor";
export type TaskbarMetric =
  "network" | "cpu" | "cpu_temperature" | "gpu" | "gpu_temperature" | "memory";
export function taskbarGroups(settings: TaskbarSettingsDto): number[][] {
  const enabled: Record<string, boolean> = {
    network: settings.network,
    cpu: settings.cpu || settings.cpu_temperature,
    gpu: settings.gpu || settings.gpu_temperature,
    memory: settings.memory,
  };
  const indices: Record<string, number[]> = {
    network: [0, 1],
    cpu: [2],
    memory: [3],
    gpu: [4],
  };
  const groups: number[][] = [];
  let pending: number[] = [];
  for (const key of settings.order) {
    if (!enabled[key]) continue;
    if (key === "network") {
      if (pending.length) {
        groups.push(pending);
        pending = [];
      }
      groups.push(indices[key]);
    } else
      for (const index of indices[key] ?? []) {
        pending.push(index);
        if (pending.length === 2) {
          groups.push(pending);
          pending = [];
        }
      }
  }
  if (pending.length) groups.push(pending);
  return settings.layout === "single" ? groups.flat().map((i) => [i]) : groups;
}
export function moveTaskbarGroup(
  settings: TaskbarSettingsDto,
  key: string,
  direction: number,
): TaskbarSettingsDto {
  const order = [...settings.order];
  const index = order.indexOf(key);
  const next = index + direction;
  if (index < 0 || next < 0 || next >= order.length) return settings;
  [order[index], order[next]] = [order[next], order[index]];
  return { ...settings, order };
}
export function selectedMetricCount(settings: TaskbarSettingsDto) {
  return [
    settings.network,
    settings.cpu,
    settings.cpu_temperature,
    settings.gpu,
    settings.gpu_temperature,
    settings.memory,
  ].filter(Boolean).length;
}
