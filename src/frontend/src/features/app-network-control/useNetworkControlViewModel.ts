import { useEffect, useState, useSyncExternalStore } from "react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import type { NetworkRuleDto } from "../../shared/contracts/monitor";

export type Target = Pick<NetworkRuleDto, "id" | "name" | "path">;
export type ControlAction =
  "limits" | "block" | "restore" | "clear" | "enable" | "disable";
export type LimitField = { mode: string; value: string; unit: string };
export function parseLimit(field: LimitField): number | null {
  if (field.mode === "unlimited") return null;
  const value = Number(field.value);
  const bytes = value * (field.unit === "MB/s" ? 1_000_000 : 1000);
  if (
    !field.value.trim() ||
    !Number.isFinite(bytes) ||
    bytes < 16_000 ||
    bytes > 1_000_000_000
  )
    throw new Error(
      "限速范围为 16 KB/s 至 1000 MB/s；解除限速请选择“不限制”。",
    );
  return Math.round(bytes);
}
export function limitField(rate: number | null | undefined): LimitField {
  return {
    mode: rate == null ? "unlimited" : "limited",
    value:
      rate == null ? "" : String(rate / (rate >= 1_000_000 ? 1_000_000 : 1000)),
    unit: rate != null && rate >= 1_000_000 ? "MB/s" : "KB/s",
  };
}
export function controllable(target: Target) {
  return (
    /[\\/]([^\\/]+)\.exe$/i.test(target.path) &&
    !/[\\/]windows[\\/]|[\\/]pinmeter[^\\/]*\.exe$/i.test(target.path)
  );
}
export function useNetworkControlViewModel(
  client: MonitorClient,
  suspended = false,
) {
  const snapshot = useSyncExternalStore(client.subscribe, client.getSnapshot);
  const data = snapshot.state?.network_control;
  const releaseOnExit =
    snapshot.state?.settings.release_network_on_exit ?? true;
  const [menu, setMenu] = useState<{
    target: Target;
    revision: string;
    x: number;
    y: number;
    trigger: HTMLElement | null;
  } | null>(null);
  const [editor, setEditor] = useState<{
    target: Target;
    revision: string;
  } | null>(null);
  const [down, setDown] = useState<LimitField>(limitField(null));
  const [up, setUp] = useState<LimitField>(limitField(null));
  const [pendingId, setPendingId] = useState<string | null>(null);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const [retry, setRetry] = useState<{
    id: string;
    revision: string;
    download: number | null;
    upload: number | null;
  } | null>(null);
  useEffect(() => {
    if (!retry || !data || !editor || editor.target.id !== retry.id) return;
    const current = data.rules.find((r) => r.id === retry.id);
    // A failed OS apply can still have durably saved this exact request.
    // Only rebase that one revision; unrelated newer edits must still conflict.
    if (
      BigInt(data.revision) === BigInt(retry.revision) + 1n &&
      current?.download === retry.download &&
      current?.upload === retry.upload
    ) {
      setEditor({ ...editor, revision: data.revision });
      setRetry(null);
    }
  }, [data, editor, retry]);
  const available = !!(
    !suspended &&
    snapshot.connected &&
    data?.available &&
    client.changeNetworkControl
  );
  const canRelease =
    !suspended &&
    snapshot.connected &&
    !!data?.supported &&
    !!client.releaseAllNetworkControl;
  const rule = (id: string) => data?.rules.find((r) => r.id === id);
  const closeMenu = () => {
    menu?.trigger?.focus({ preventScroll: true });
    setMenu(null);
  };
  const openMenu = (
    target: Target,
    x: number,
    y: number,
    trigger: HTMLElement | null,
  ) => {
    if (!available || pendingId || !controllable(target)) return;
    setError("");
    setMenu({ target: { ...target }, revision: data!.revision, x, y, trigger });
  };
  const act = async (
    target: Target,
    action: ControlAction,
    revision: string,
    download: number | null = null,
    upload: number | null = null,
  ) => {
    if (!available || pendingId) return false;
    setPendingId(target.id);
    setRetry(null);
    setError("");
    setMessage(`正在为 ${target.name} 应用设置…`);
    try {
      await client.changeNetworkControl!({
        id: target.id,
        action,
        download,
        upload,
        expectedRevision: revision,
      });
      setMessage(
        action === "enable"
          ? `${target.name} 已启用保存的规则。`
          : action === "disable"
            ? `${target.name} 已停用限制，配置已保留。`
            : action === "block"
              ? `${target.name} 已禁用网络。${releaseOnExit ? "退出时自动解除。" : "退出后仍保持禁用。"}`
              : action === "restore"
                ? `${target.name} 已解除 Pinmeter 的网络禁用，请查看原限速的实际状态。`
                : action === "clear"
                  ? `${target.name} 已清除 Pinmeter 的限制。`
                  : `${target.name} 的限速设置已保存${rule(target.id)?.enabled === false ? "，规则仍未启用" : "并应用"}。`,
      );
      return true;
    } catch (e) {
      setMessage("");
      setError(String(e));
      if (action === "limits")
        setRetry({ id: target.id, revision, download, upload });
      return false;
    } finally {
      setPendingId(null);
    }
  };
  const edit = () => {
    if (!menu) return;
    setRetry(null);
    const current = rule(menu.target.id);
    setDown(limitField(current?.download));
    setUp(limitField(current?.upload));
    setEditor({ target: menu.target, revision: menu.revision });
    closeMenu();
  };
  const save = async () => {
    if (!editor) return;
    try {
      const download = parseLimit(down),
        upload = parseLimit(up);
      if (await act(editor.target, "limits", editor.revision, download, upload))
        setEditor(null);
    } catch (e) {
      setError(e instanceof Error ? e.message : String(e));
    }
  };
  const releaseAll = async () => {
    if (!canRelease || pendingId || !data || !client.releaseAllNetworkControl)
      return;
    setPendingId("__all__");
    setError("");
    setMessage("正在解除全部限制…");
    try {
      await client.releaseAllNetworkControl(data.revision);
      setMessage("全部限制已解除，配置已保留为未启用。");
    } catch (error) {
      setMessage("");
      setError(String(error));
    } finally {
      setPendingId(null);
    }
  };
  return {
    suspended,
    releaseOnExit,
    releaseAll,
    data,
    available,
    canRelease,
    rule,
    menu,
    openMenu,
    closeMenu,
    editor,
    edit,
    closeEditor: () => {
      if (!pendingId) {
        setEditor(null);
        setError("");
        setRetry(null);
      }
    },
    down,
    up,
    setDown: (value: LimitField) => {
      setDown(value);
      setError("");
    },
    setUp: (value: LimitField) => {
      setUp(value);
      setError("");
    },
    pendingId,
    message,
    error,
    act,
    save,
    demo: snapshot.demo,
  };
}
export type ControlViewModel = ReturnType<typeof useNetworkControlViewModel>;
