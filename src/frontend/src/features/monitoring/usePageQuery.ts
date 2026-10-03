import { useEffect, useState, useSyncExternalStore } from "react";
import type { MonitorClient } from "../../shared/client/monitor-client";

/** One in-flight request; unmounting and hidden documents stop renewing backend leases. */
export function usePageQuery<T>(
  client: MonitorClient,
  load: () => Promise<T>,
  interval = 2_000,
  enabled = true,
) {
  const visible = useSyncExternalStore(
    client.subscribe,
    () => client.getSnapshot().nativeVisible !== false,
  );
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState("");
  useEffect(() => {
    if (!visible || !enabled) return;
    let disposed = false;
    let pending = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const tick = async () => {
      if (disposed || pending || document.hidden) return;
      pending = true;
      try {
        const value = await load();
        if (!disposed && !document.hidden) {
          setData(value);
          setError("");
        }
      } catch (e) {
        if (!disposed) setError(String(e));
      } finally {
        pending = false;
        if (!disposed && !document.hidden)
          timer = setTimeout(() => void tick(), interval);
      }
    };
    const visibility = () => {
      clearTimeout(timer);
      if (!document.hidden) void tick();
    };
    void tick();
    document.addEventListener("visibilitychange", visibility);
    return () => {
      disposed = true;
      clearTimeout(timer);
      document.removeEventListener("visibilitychange", visibility);
    };
  }, [load, interval, visible, enabled]);
  return { data, error };
}
