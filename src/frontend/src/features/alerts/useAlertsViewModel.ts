import { useMemo, useSyncExternalStore } from "react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import { alertsStore } from "./alerts-store";

export function useAlertsViewModel(client: MonitorClient) {
  const store = useMemo(() => alertsStore(client), [client]);
  const state = useSyncExternalStore(
    store.subscribe,
    store.getSnapshot,
    store.getSnapshot,
  );
  return {
    ...state,
    refresh: store.refresh,
    save: store.save,
    acknowledge: store.acknowledge,
    clear: store.clear,
  };
}
