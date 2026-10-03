import type {
  DiskSnapshotDto,
  ReadingDto,
  ReadingStatus,
} from "../contracts/monitor";
export function demoDisk(
  id: string | null,
  status: ReadingStatus,
): DiskSnapshotDto {
  const now = Date.now();
  const selected = id ?? "0 C:";
  const reading = (value: number, unit: string, at: number): ReadingDto => ({
    value: status === "normal" ? value : null,
    text:
      status === "normal"
        ? (unit === "%" ? value : value / 1e6).toFixed(1)
        : "—",
    unit: unit === "%" ? "%" : "MB/s",
    status,
    valid_at_ms: status === "normal" ? at : null,
    source: "演示物理磁盘",
    semantic: "demo",
    detail: status === "normal" ? "" : "演示异常状态",
  });
  const point = (at: number) => ({
    id: selected,
    read: reading(12e6 + 10e6 * Math.sin(at / 8000), "B/s", at),
    write: reading(2e6 + 2e6 * Math.sin(at / 11000), "B/s", at),
    activity: reading(25 + 20 * Math.sin(at / 8000), "%", at),
  });
  return {
    status,
    detail: "演示物理磁盘数据",
    selected_id: selected,
    devices: [point(now)],
    history: Array.from({ length: 151 }, (_, i) => {
      const at = now - (150 - i) * 2000;
      return {
        at_ms: at,
        elapsed_ms: at,
        generation: "demo",
        point: point(at),
      };
    }),
  };
}
