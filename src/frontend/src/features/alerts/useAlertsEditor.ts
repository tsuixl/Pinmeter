import { useEffect, useRef, useState, useSyncExternalStore } from "react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import type { AlertsConfigDto } from "../../shared/contracts/monitor";
import { useAlertsViewModel } from "./useAlertsViewModel";

export function alertSaveRevision(
  base: AlertsConfigDto,
  baseRevision: string,
  current?: { alerts: AlertsConfigDto; revision: string },
) {
  // An unrelated immediate setting may advance the shared revision while a
  // reminder draft is open. Rebase only when the reminder rules are unchanged.
  return current && JSON.stringify(current.alerts) === JSON.stringify(base)
    ? current.revision
    : baseRevision;
}

export function useAlertsEditor(client: MonitorClient) {
  const vm = useAlertsViewModel(client);
  const snapshot = useSyncExternalStore(client.subscribe, client.getSnapshot);
  const settings = snapshot.state?.settings;
  const [draft, setDraft] = useState<AlertsConfigDto | null>(null);
  const [base, setBase] = useState<AlertsConfigDto | null>(null);
  const [baseRevision, setBaseRevision] = useState("");
  const [dirty, setDirty] = useState(false);
  const [saved, setSaved] = useState(false);
  const saving = useRef(false);
  const confirmed =
    settings &&
    (!vm.data || BigInt(settings.revision) > BigInt(vm.data.settings_revision))
      ? { config: settings.alerts, revision: settings.revision }
      : vm.data
        ? { config: vm.data.config, revision: vm.data.settings_revision }
        : null;
  useEffect(() => {
    if (!dirty && confirmed) {
      setDraft(confirmed.config);
      setBase(confirmed.config);
      setBaseRevision(confirmed.revision);
    }
  }, [confirmed?.config, confirmed?.revision, dirty]);
  return {
    ...vm,
    draft,
    dirty,
    saved,
    change: (value: AlertsConfigDto) => {
      if (saving.current || vm.pending) return;
      setDraft(value);
      setDirty(true);
      setSaved(false);
    },
    discard: () => {
      if (saving.current || vm.pending) return;
      if (confirmed) {
        setDraft(confirmed.config);
        setBase(confirmed.config);
        setBaseRevision(confirmed.revision);
      }
      setDirty(false);
      setSaved(false);
    },
    saveDraft: async () => {
      if (saving.current || vm.pending || !draft || !base) return false;
      saving.current = true;
      try {
        const revision = alertSaveRevision(
          base,
          baseRevision,
          client.getSnapshot().state?.settings,
        );
        if (!(await vm.save(draft, revision))) return false;
        setDirty(false);
        setSaved(true);
        return true;
      } finally {
        saving.current = false;
      }
    },
  };
}

export type AlertsEditor = ReturnType<typeof useAlertsEditor>;
