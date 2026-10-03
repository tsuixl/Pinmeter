import { useEffect, useState } from "react";
import type {
  TrayPanelClient,
  TrayPage,
} from "../../shared/client/tray-panel-client";
import type { TraySnapshotDto } from "../../shared/contracts/monitor";
import { applyFont, defaultFontFamily, loadFont } from "../../shared/ui/fonts";

export function useTrayPanelViewModel(
  client: TrayPanelClient,
  initial: TraySnapshotDto,
  initialError = "",
) {
  const [data, setData] = useState(initial);
  const [error, setError] = useState(initialError);
  const [visible, setVisible] = useState(true);
  useEffect(() => {
    let disposed = false;
    let remove: (() => void) | undefined;
    void client
      .onVisible((value) => {
        if (!disposed) setVisible(value);
      })
      .then((unsubscribe) => {
        if (disposed) unsubscribe();
        else remove = unsubscribe;
      })
      .catch((e) => setError(String(e)));
    void client.ready().catch((e) => setError(String(e)));
    const key = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        void client.hide().catch((e) => setError(String(e)));
      }
    };
    document.addEventListener("keydown", key);
    return () => {
      disposed = true;
      remove?.();
      document.removeEventListener("keydown", key);
    };
  }, [client]);
  useEffect(() => {
    if (!visible) return;
    let disposed = false;
    let pending = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const tick = async () => {
      if (disposed || pending || document.hidden) return;
      pending = true;
      try {
        const snapshot = await client.read();
        if (!disposed) {
          setData(snapshot);
          setError("");
        }
      } catch (e) {
        if (!disposed) setError(String(e));
      } finally {
        pending = false;
        if (!disposed && !document.hidden)
          timer = setTimeout(() => void tick(), 2000);
      }
    };
    const changed = () => {
      clearTimeout(timer);
      if (!document.hidden) void tick();
    };
    void tick();
    document.addEventListener("visibilitychange", changed);
    return () => {
      disposed = true;
      clearTimeout(timer);
      document.removeEventListener("visibilitychange", changed);
    };
  }, [client, visible]);
  useEffect(() => {
    const media = matchMedia("(prefers-color-scheme: dark)");
    const apply = () => {
      const theme =
        data.theme === "system"
          ? media.matches
            ? "dark"
            : "light"
          : data.theme;
      document.documentElement.dataset.theme = theme;
      document.documentElement.classList.toggle("dark", theme === "dark");
      document.documentElement.style.colorScheme = theme;
    };
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [data.theme]);
  useEffect(() => {
    let disposed = false;
    void client
      .fonts()
      .then((c) => loadFont(data.font_family, data.font_style, c))
      .then(() => {
        if (!disposed) applyFont(data.font_family, data.font_style);
      })
      .catch(() => {
        if (!disposed) applyFont(defaultFontFamily);
      });
    return () => {
      disposed = true;
    };
  }, [client, data.font_family, data.font_style]);
  return {
    data,
    error,
    demo: client.demo,
    open: async (page: TrayPage) => {
      try {
        await client.open(page);
      } catch (error) {
        setError(String(error));
      }
    },
    hide: async () => {
      try {
        await client.hide();
      } catch (error) {
        setError(String(error));
      }
    },
  };
}
