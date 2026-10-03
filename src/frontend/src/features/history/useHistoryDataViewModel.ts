import { useCallback, useRef, useState, useSyncExternalStore } from "react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import { usePageQuery } from "../monitoring/usePageQuery";
export type HistoryScope = "basic" | "applications";
type Confirmation = {
  action: "clear" | "export";
  scope: HistoryScope;
  from: number;
  through: number;
  includePaths: boolean;
};

export function useHistoryDataViewModel(
  client: MonitorClient,
  from: number,
  through: number,
  onChanged: () => void,
) {
  const [scope, setScope] = useState<HistoryScope>("basic");
  const [includePaths, setIncludePaths] = useState(false);
  const [confirmation, setConfirmation] = useState<Confirmation | null>(null);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const [revision, setRevision] = useState(0);
  const busy = useRef(false);
  const snapshot = useSyncExternalStore(client.subscribe, client.getSnapshot);
  const load = useCallback(
    () =>
      client.getHistoryStorage
        ? client.getHistoryStorage()
        : Promise.reject(new Error("当前客户端不支持历史管理")),
    [client, revision],
  );
  const query = usePageQuery(client, load, 30000);
  const operate = async (action: () => Promise<void>) => {
    if (busy.current) return;
    busy.current = true;
    setPending(true);
    setError("");
    setMessage("");
    try {
      await action();
    } catch (reason) {
      setError(String(reason));
    } finally {
      busy.current = false;
      setPending(false);
    }
  };
  return {
    ...query,
    error: error || query.error,
    message,
    pending,
    scope,
    setScope,
    includePaths,
    setIncludePaths,
    confirmation,
    canClear: !!client.clearHistory,
    canExport: !!client.exportHistory,
    running: snapshot.state?.app_network.running ?? false,
    prepare: (action: "clear" | "export") => {
      setError("");
      setConfirmation({
        action,
        scope,
        from,
        through,
        includePaths: scope === "applications" && includePaths,
      });
    },
    close: () => {
      if (!busy.current) setConfirmation(null);
    },
    confirm: () =>
      operate(async () => {
        if (!confirmation) return;
        if (confirmation.action === "clear") {
          if (!client.clearHistory) throw new Error("历史清除不可用");
          await client.clearHistory(confirmation.scope);
          setMessage(
            "所选历史及损坏备份已清除；继续采集会产生新记录。用户导出的文件不受影响。",
          );
          setRevision((value) => value + 1);
          onChanged();
        } else {
          if (!client.exportHistory) throw new Error("历史导出不可用");
          const result = await client.exportHistory(
            confirmation.scope,
            confirmation.from,
            confirmation.through,
            confirmation.includePaths,
          );
          setMessage(`已导出到 ${result.path}`);
        }
        setConfirmation(null);
      }),
    stopRecording: () =>
      operate(async () => {
        if (!client.setAppNetworkMonitoring)
          throw new Error("应用记录操作不可用");
        await client.setAppNetworkMonitoring(false);
        setMessage(
          "已停止本次应用流量记录；已保存历史保留。下次启动自动记录由设置决定。",
        );
      }),
  };
}
