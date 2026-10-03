import type { FrameDto, ReadingDto } from "../../shared/contracts/monitor";
export type MetricKey =
  "cpu" | "cpu_temperature" | "memory" | "download" | "upload";
export type ChartSeries = {
  id: string;
  label: string;
  color: string;
  temperature?: boolean;
  network?: boolean;
  maxGapMs?: number;
  read: (frame: FrameDto) => ReadingDto;
};
export function temperatureDomain(values: number[]) {
  return [
    Math.floor(Math.min(0, ...values) / 20) * 20,
    Math.ceil(Math.max(100, ...values) / 20) * 20,
  ] as const;
}
/** Close only this valid segment; never fill across a missing sample. */
export function areaPath(path: string) {
  const points = [...path.matchAll(/[ML]([\d.]+),([\d.]+)/g)];
  if (!points.length) return "";
  return `${path} L${points.at(-1)![1]},180 L${points[0][1]},180 Z`;
}
export function networkCeiling(values: number[]) {
  const maximum = Math.max(1000, ...values);
  const scale = 10 ** Math.floor(Math.log10(maximum));
  return ([1, 2, 5, 10].find((step) => step * scale >= maximum) ?? 10) * scale;
}
export function chartPaths(
  frames: FrameDto[],
  key: MetricKey | ChartSeries,
  start: number,
  end: number,
  ceiling: number,
  floor = 0,
) {
  const paths: string[] = [];
  let path = "";
  let previous: FrameDto | undefined;
  for (const frame of frames) {
    if (frame.elapsed_ms < start || frame.elapsed_ms > end) continue;
    const reading = typeof key === "string" ? frame[key] : key.read(frame);
    const network =
      typeof key === "string"
        ? key === "download" || key === "upload"
        : key.network;
    const interrupted =
      previous &&
      (frame.generation !== previous.generation ||
        (network &&
          (frame.network_generation !== previous.network_generation ||
            frame.network_id !== previous.network_id)) ||
        frame.elapsed_ms - previous.elapsed_ms >
          (typeof key === "string" ? 15_000 : (key.maxGapMs ?? 15_000)));
    if (interrupted || reading.status !== "normal" || reading.value === null) {
      if (path) paths.push(path);
      path = "";
    }
    if (reading.status === "normal" && reading.value !== null) {
      const x = ((frame.elapsed_ms - start) / (end - start)) * 1000;
      const y =
        180 -
        Math.min(1, Math.max(0, (reading.value - floor) / (ceiling - floor))) *
          180;
      path += `${path ? " L" : "M"}${x.toFixed(2)},${y.toFixed(2)}`;
    }
    previous = frame;
  }
  if (path) paths.push(path);
  return paths;
}
export function clampAnchor(
  anchor: number,
  first: number,
  last: number,
  range: number,
) {
  return Math.min(last, Math.max(Math.min(last, first + range), anchor));
}
