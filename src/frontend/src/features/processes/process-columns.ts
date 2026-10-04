export const processColumnOrder = [
  "name",
  "pid",
  "cpu",
  "working_set",
  "id",
] as const;
export type ProcessColumnKey = (typeof processColumnOrder)[number];
export const minimumColumnWidths: Record<ProcessColumnKey, number> = {
  name: 220,
  pid: 128,
  cpu: 128,
  working_set: 184,
  id: 184,
};
export const maximumColumnWidth = 1200;
export function clampColumnWidth(key: ProcessColumnKey, width: number) {
  return Math.max(
    minimumColumnWidths[key],
    Math.min(
      maximumColumnWidth,
      Number.isFinite(width) ? Math.round(width) : minimumColumnWidths[key],
    ),
  );
}
export function defaultColumnWidths(
  available = 1000,
): Record<ProcessColumnKey, number> {
  const extra = Math.max(0, Math.round(available) - 844);
  const shares = { name: 0.4, pid: 0.05, cpu: 0.1, working_set: 0.2, id: 0.25 };
  return Object.fromEntries(
    processColumnOrder.map((key) => [
      key,
      clampColumnWidth(key, minimumColumnWidths[key] + extra * shares[key]),
    ]),
  ) as Record<ProcessColumnKey, number>;
}
export function moveProcessColumn(
  order: readonly ProcessColumnKey[],
  source: ProcessColumnKey,
  target: ProcessColumnKey,
  after: boolean,
) {
  if (source === target || !order.includes(source) || !order.includes(target))
    return [...order];
  const next = order.filter((key) => key !== source);
  next.splice(next.indexOf(target) + Number(after), 0, source);
  return next;
}
