import { useRef, useState, useSyncExternalStore } from "react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import { useNetworkControlViewModel } from "../app-network-control/useNetworkControlViewModel";
import { monitorHealth } from "./health";

export function useMonitorStatusViewModel(
  client: MonitorClient,
  suspended: boolean,
) {
  const snapshot = useSyncExternalStore(client.subscribe, client.getSnapshot);
  const control = useNetworkControlViewModel(client, suspended);
  const busy = useRef(false);
  const [retrying, setRetrying] = useState(false);
  const [retryError, setRetryError] = useState("");
  const reconnect = async () => {
    if (suspended || busy.current || !client.reconnect) return;
    busy.current = true;
    setRetrying(true);
    setRetryError("");
    try {
      await client.reconnect();
    } catch (error) {
      setRetryError(String(error));
    } finally {
      busy.current = false;
      setRetrying(false);
    }
  };
  return {
    snapshot,
    health: monitorHealth(snapshot),
    control,
    retrying,
    retryError,
    canReconnect: !!client.reconnect,
    reconnect,
  };
}
