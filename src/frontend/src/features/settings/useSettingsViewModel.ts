import { useRef, useState } from "react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import type { SettingsDto } from "../../shared/contracts/monitor";
const defaults: SettingsDto = {
  start_in_tray: false,
  autostart: false,
  close_action: "ask",
  revision: "0",
  theme: "system",
  interval_ms: 1000,
  network_id: null,
  release_network_on_exit: true,
  taskbar: {
    enabled: false,
    hidden: false,
    layout: "double",
    cpu: true,
    gpu: true,
    memory: true,
  },
};
export function useSettingsViewModel(
  client: MonitorClient,
  confirmed?: SettingsDto,
) {
  const [draft, setDraft] = useState<SettingsDto>(confirmed ?? defaults);
  const [pending, setPending] = useState(false);
  const busy = useRef(false);
  const [feedback, setFeedback] = useState("");
  const [error, setError] = useState<string | null>(null);
  const change = (patch: Partial<SettingsDto>) => {
    if (busy.current) return;
    const previous = client.getSnapshot().state?.settings;
    if (!previous) {
      setError("采集服务未连接，无法保存设置");
      return;
    }
    busy.current = true;
    setPending(true);
    setError(null);
    const next = { ...previous, ...patch, revision: previous.revision };
    setDraft(next);
    setFeedback("正在保存…");
    void client
      .updateSettings(next)
      .then((saved) => {
        const current = client.getSnapshot().state?.settings;
        setDraft(
          current && BigInt(current.revision) > BigInt(saved.revision)
            ? current
            : saved,
        );
        setFeedback(
          client.getSnapshot().demo ? "已更新演示设置" : "已自动保存",
        );
      })
      .catch((error) => {
        setDraft(client.getSnapshot().state?.settings ?? previous);
        setError(String(error));
        setFeedback("保存失败，已恢复原设置");
      })
      .finally(() => {
        busy.current = false;
        setPending(false);
      });
  };
  return {
    draft: pending ? draft : (confirmed ?? defaults),
    pending: pending || !confirmed,
    feedback,
    error,
    change,
    exit: async () => {
      try {
        await client.requestExit?.();
      } catch (error) {
        setError(String(error));
      }
    },
    reset: () => change(defaults),
  };
}
