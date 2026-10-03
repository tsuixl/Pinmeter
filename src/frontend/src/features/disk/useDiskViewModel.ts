import { useCallback, useState } from "react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import { usePageQuery } from "../monitoring/usePageQuery";
import { plotFrame } from "../monitoring/plot-frame";

export function useDiskViewModel(client: MonitorClient) {
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const load = useCallback(
    () =>
      client.getDiskSnapshot
        ? client.getDiskSnapshot(selectedId)
        : Promise.reject(new Error("当前客户端不支持磁盘采样")),
    [client, selectedId],
  );
  const { data, error } = usePageQuery(client, load);
  // Never show the old disk's numbers while the new selection is in flight.
  const matching = selectedId === null || data?.selected_id === selectedId;
  const current =
    !error && matching
      ? data?.devices.find((d) => d.id === data.selected_id)
      : undefined;
  const options =
    data?.devices.map((d) => ({ value: d.id, label: `磁盘 ${d.id}` })) ?? [];
  if (selectedId && !options.some((o) => o.value === selectedId))
    options.push({
      value: selectedId,
      label: `磁盘 ${selectedId}（当前不可用）`,
    });
  return {
    data,
    error,
    current,
    options,
    selectedId: selectedId ?? data?.selected_id ?? "",
    setSelectedId,
    history: matching
      ? (data?.history ?? []).map((f) =>
          plotFrame(f.at_ms, f.elapsed_ms, f.generation, {
            cpu: f.point?.activity,
            download: f.point?.read,
            upload: f.point?.write,
          }),
        )
      : [],
  };
}
