import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import type { UpdateClient } from "../../shared/client/update-client";

export type NoticeMode = "preview" | "history" | "unread";
export function useUpdateViewModel(client: UpdateClient, visible: boolean) {
  const state = useSyncExternalStore(client.subscribe, client.getSnapshot);
  const [dialog, setDialog] = useState<NoticeMode | null>(null);
  const [error, setError] = useState("");
  const [toast, setToast] = useState("");
  const [saving, setSaving] = useState(false);
  const autoShown = useRef(false);
  const lastManual = useRef<string | null>(null);
  const awaitingManual = useRef(false);
  useEffect(() => client.start(), [client]);
  useEffect(() => {
    if (visible && state?.unread.length && !dialog && !autoShown.current) {
      autoShown.current = true;
      setDialog("unread");
    }
  }, [state, visible, dialog]);
  useEffect(() => {
    if (!state) return;
    if (
      lastManual.current !== null &&
      lastManual.current !== state.manual_request
    )
      awaitingManual.current = true;
    lastManual.current = state.manual_request;
    if (awaitingManual.current && state.stage !== "checking") {
      awaitingManual.current = false;
      setToast(
        state.detail || (state.target ? "发现新版本" : "当前已是最新版本"),
      );
    }
  }, [state]);
  useEffect(() => {
    if (!toast) return;
    const timer = setTimeout(() => setToast(""), 6000);
    return () => clearTimeout(timer);
  }, [toast]);
  const run = async (action: () => Promise<void>) => {
    setError("");
    try {
      await action();
    } catch (reason) {
      setError(String(reason));
    }
  };
  const open = (mode: NoticeMode) => {
    autoShown.current = true;
    setError("");
    setDialog(mode);
  };
  const close = async () => {
    if (state?.stage === "preparing" || saving) return;
    if (dialog === "unread") {
      setSaving(true);
      try {
        await client.preference("read");
      } catch (reason) {
        setError(String(reason));
        setSaving(false);
        return;
      }
      setSaving(false);
    }
    setDialog(null);
    setError("");
  };
  const preference = async (
    action: "automatic" | "dismiss",
    value?: boolean,
  ) => {
    if (saving) return;
    setSaving(true);
    await run(() => client.preference(action, value));
    setSaving(false);
  };
  const busy =
    !!state && ["checking", "downloading", "preparing"].includes(state.stage);
  const notes =
    dialog === "preview"
      ? state?.target_history
      : dialog === "unread"
        ? state?.unread
        : state?.history;
  const primary = async () => {
    if (!state || busy) return;
    if (!state.can_install) {
      await run(() => client.openDownloads());
      return;
    }
    if (state.stage === "ready")
      await run(() => client.install(state.confirmation_required));
    else await run(() => client.download());
  };
  return {
    state,
    dialog,
    notes: notes ?? [],
    error,
    toast,
    saving,
    busy,
    open,
    close,
    primary,
    clearToast: () => setToast(""),
    check: () => run(() => client.check()),
    openDownloads: () => run(() => client.openDownloads()),
    automatic: (value: boolean) => preference("automatic", value),
    dismiss: () => preference("dismiss"),
  };
}
export type UpdateViewModel = ReturnType<typeof useUpdateViewModel>;
