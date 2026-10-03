import { useEffect, useState } from "react";

/** One in-flight request; unmounting and hidden documents stop renewing backend leases. */
export function usePageQuery<T>(load: () => Promise<T>, interval = 2_000) {
  const [data, setData] = useState<T | null>(null);
  const [error, setError] = useState("");
  useEffect(() => {
    let disposed = false;
    let pending = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const tick = async () => {
      if (disposed || pending || document.hidden) return;
      pending = true;
      try {
        const value = await load();
        if (!disposed) {
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
  }, [load, interval]);
  return { data, error };
}
