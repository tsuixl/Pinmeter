import type {
  AppHistorySnapshotDto,
  AppHistoryRowDto,
} from "../contracts/monitor";
const hour = 3600_000;
const apps = [
  ["浏览器.exe", "c:\\program files\\browser\\浏览器.exe", 6_000_000, 200_000],
  ["云盘.exe", "c:\\program files\\drive\\云盘.exe", 400_000, 4_000_000],
  ["下载器.exe", "c:\\tools\\下载器.exe", 2_000_000, 90_000],
  ["工具.exe", "c:\\tools\\工具.exe", 100_000, 30_000],
  ["工具.exe", "d:\\tools\\工具.exe", 90_000, 20_000],
] as const;
export function demoAppHistory(
  range: string,
  appId: string | null,
  dayStart: number,
  recordedFrom: number | null,
  recordedThrough: number,
  incomplete = false,
): AppHistorySnapshotDto {
  const now = Date.now();
  const from =
    Math.floor(
      Math.max(
        now - 24 * hour,
        range === "today"
          ? dayStart
          : now - (range === "1h" ? 1 : range === "6h" ? 6 : 24) * hour,
      ) / 60_000,
    ) * 60_000;
  const observed =
    recordedFrom === null
      ? 0
      : Math.max(
          0,
          Math.min(now, recordedThrough) - Math.max(from, recordedFrom),
        );
  const minutes = observed / 60_000;
  const rows: AppHistoryRowDto[] = observed
    ? apps.map(([name, path, rx, tx]) => {
        const received = BigInt(Math.floor(rx * minutes));
        const transmitted = BigInt(Math.floor(tx * minutes));
        return {
          id: `app:${path}`,
          name,
          path,
          received: String(received),
          transmitted: String(transmitted),
          total: String(received + transmitted),
        };
      })
    : [];
  if (observed)
    rows.push({
      id: "unknown",
      name: "未归属",
      path: "",
      received: String(Math.floor(80_000 * minutes)),
      transmitted: "0",
      total: String(Math.floor(80_000 * minutes)),
    });
  const selected = apps.find(([, path]) => appId === `app:${path}`);
  const selectedName = appId === "unknown" ? "未归属" : (selected?.[0] ?? null);
  return {
    loading: false,
    notice: "",
    persistence_error: "",
    saved_at_ms: recordedFrom === null ? null : recordedThrough,
    lost_windows: "0",
    clock_discontinuities: "0",
    range,
    from_ms: from,
    through_ms: now,
    received: String(rows.reduce((n, row) => n + BigInt(row.received), 0n)),
    transmitted: String(
      rows.reduce((n, row) => n + BigInt(row.transmitted), 0n),
    ),
    covered_ms: observed,
    observed_ms: observed,
    incomplete,
    limited: false,
    skipped_windows: "0",
    rows,
    selected_id: appId,
    selected_name: recordedFrom === null ? null : selectedName,
    points: selectedName
      ? Array.from(
          { length: Math.min(1441, Math.floor((now - from) / 60_000) + 1) },
          (_, i) => {
            const at = Math.min(now, from + (i + 1) * 60_000);
            const valid =
              recordedFrom !== null &&
              at >= recordedFrom &&
              at <= recordedThrough &&
              !(at > now - 2 * hour && at < now - 1.5 * hour);
            return {
              at_ms: at,
              download: valid
                ? ((selected?.[2] ?? 80_000) / 60) *
                  (1 + Math.sin(i / 12) * 0.5)
                : null,
              upload: valid
                ? ((selected?.[3] ?? 0) / 60) * (1 + Math.cos(i / 15) * 0.3)
                : null,
            };
          },
        )
      : [],
  };
}
