import type {
  FrameDto,
  MonitorBatchDto,
  MonitorStateDto,
} from "../contracts/monitor";
export function mergeHistory(
  previous: FrameDto[],
  incoming: FrameDto[],
): FrameDto[] {
  const map = new Map(previous.map((frame) => [frame.cursor, frame]));
  for (const frame of incoming) map.set(frame.cursor, frame);
  const frames = [...map.values()].sort((a, b) => a.elapsed_ms - b.elapsed_ms);
  const latest = frames.at(-1)?.elapsed_ms ?? 0;
  return frames
    .filter((frame) => frame.elapsed_ms >= latest - 300_000)
    .slice(-301);
}
export function acceptBatch(
  previous: MonitorStateDto | null,
  batch: MonitorBatchDto,
): MonitorStateDto {
  if (batch.state.protocol_version !== 1)
    throw new Error("应用协议版本不匹配，请重启应用");
  const reset =
    batch.kind === "bootstrap" ||
    previous?.session_id !== batch.state.session_id;
  return {
    ...batch.state,
    settings:
      previous?.session_id === batch.state.session_id &&
      BigInt(previous.settings.revision) > BigInt(batch.state.settings.revision)
        ? previous.settings
        : batch.state.settings,
    ip: batch.state.ip ?? (reset ? undefined : previous?.ip),
    history: mergeHistory(
      reset ? [] : (previous?.history ?? []),
      batch.state.history,
    ),
  };
}
