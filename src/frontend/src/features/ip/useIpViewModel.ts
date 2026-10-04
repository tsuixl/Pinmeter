import { useEffect, useState, useSyncExternalStore } from "react";
import type { IpClient } from "../../shared/client/ip-client";
import { usePageUiState } from "../../shared/state/page-ui-state";
export const exitLabels: Record<string, string> = {
  ipv4: "公网 IPv4",
  ipv6: "公网 IPv6",
  domestic: "国内目标出口",
};
export const queryLabels: Record<string, string> = {
  idle: "尚未检测",
  loading: "查询中",
  ready: "已更新",
  failed: "查询失败",
  stale: "上次结果",
  unsupported: "暂不支持",
};
export function useIpViewModel(client: IpClient) {
  const state = useSyncExternalStore(client.subscribe, client.getSnapshot);
  const connected = useSyncExternalStore(client.subscribe, client.isConnected);
  const [selectedId, setSelectedId] = usePageUiState<string | undefined>(
    "ip.selected",
    undefined,
  );
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState<string | null>(null);
  const [cooldown, setCooldown] = useState(0);
  useEffect(() => {
    let active = true;
    void client.setActive(true).catch((e) => {
      if (active) setError(String(e));
    });
    return () => {
      active = false;
      void client.setActive(false).catch(() => {});
    };
  }, [client]);
  useEffect(() => {
    const until = performance.now() + (state?.retry_after_ms ?? 0);
    const update = () =>
      setCooldown(Math.max(0, Math.ceil((until - performance.now()) / 1000)));
    update();
    const timer = window.setInterval(update, 1000);
    return () => clearInterval(timer);
  }, [state]);
  useEffect(() => {
    if (!copied) return;
    const timer = window.setTimeout(() => setCopied(null), 2000);
    return () => clearTimeout(timer);
  }, [copied]);
  const exits = state?.exits ?? [];
  const selected =
    exits.find((e) => e.id === selectedId) ??
    exits.find((e) => e.status === "ready") ??
    exits.find((e) => e.address) ??
    exits[0];
  const profile = state?.profiles.find((p) => p.address === selected?.address);
  useEffect(() => {
    if (selectedId === undefined && selected?.address)
      setSelectedId(selected.id);
  }, [selectedId, selected?.id, selected?.address]);
  const run = async (action: () => Promise<void>) => {
    setError(null);
    try {
      await action();
    } catch (e) {
      setError(String(e));
    }
  };
  return {
    state,
    connected,
    exits,
    selected,
    profile,
    error,
    copied,
    cooldown,
    setSelectedId,
    refresh: () => run(() => client.refresh()),
    refreshChecks: (section: string) =>
      run(() => client.refreshChecks(section)),
    copy: (address: string) =>
      run(async () => {
        setCopied(null);
        try {
          await client.copy(address);
        } catch {
          throw new Error("复制失败，请检查剪贴板权限后重试。");
        }
        setCopied(address);
      }),
  };
}
