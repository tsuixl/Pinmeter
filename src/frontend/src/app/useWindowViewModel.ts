import { useEffect, useRef, useState } from "react";
import type { WindowClient, WindowState } from "../shared/client/window-client";
import type { ExitStatusDto } from "../shared/contracts/monitor";

export function useWindowViewModel(client?: WindowClient) {
  const [state, setState] = useState<WindowState>({
    maximized: false,
    fullscreen: false,
  });
  const [error, setError] = useState<string | null>(null);
  const [pending, setPending] = useState(false);
  const busy = useRef(false);
  const [rememberClose, setRememberClose] = useState(false);
  const [closeError, setCloseError] = useState<string | null>(null);
  const [closePending, setClosePending] = useState(false);
  const [exit, setExit] = useState<ExitStatusDto>({
    stage: "idle",
    detail: "",
  });
  useEffect(() => {
    if (!client) return;
    let active = true;
    let changed = false;
    const release = client.onExitState((value) => {
      changed = true;
      if (active) setExit(value);
    });
    void release
      .then(() => client.readExitState())
      .then((value) => {
        if (active && !changed) setExit(value);
      })
      .catch((e) => {
        if (active) setError(String(e));
      });
    return () => {
      active = false;
      void release.then((fn) => fn()).catch(() => {});
    };
  }, [client]);
  const exitAction = async (confirm: boolean) => {
    try {
      if (confirm) await client?.confirmExit();
      else await client?.cancelExit();
    } catch (e) {
      setError(String(e));
    }
  };
  useEffect(() => {
    setRememberClose(false);
    setCloseError(null);
  }, [exit.stage]);
  const resolveClose = async (action: "minimize" | "exit") => {
    if (busy.current || !client?.resolveClose) return;
    busy.current = true;
    setClosePending(true);
    setCloseError(null);
    try {
      await client.resolveClose(action, rememberClose);
    } catch (e) {
      setCloseError(String(e));
    } finally {
      busy.current = false;
      setClosePending(false);
    }
  };
  useEffect(() => {
    if (!client) return;
    let active = true;
    let revision = 0;
    const refresh = async () => {
      const request = ++revision;
      try {
        const next = await client.readState();
        if (active && request === revision) setState(next);
      } catch (e) {
        if (active) setError(`无法读取窗口状态：${String(e)}`);
      }
    };
    const release = client.onResize(() => void refresh());
    void release.then(refresh).catch((e) => {
      if (active) setError(`无法监听窗口状态：${String(e)}`);
    });
    // App applies the confirmed theme in a layout effect before revealing.
    // A hidden WebView may suspend animation frames, so do not await one here.
    void client.show().catch((e) => {
      if (active) setError(`无法显示窗口：${String(e)}`);
    });
    return () => {
      active = false;
      void release.then((unlisten) => unlisten()).catch(() => {});
    };
  }, [client]);
  const perform = async (action: "minimize" | "toggleMaximize" | "close") => {
    if (!client || busy.current) return;
    busy.current = true;
    setPending(true);
    setError(null);
    try {
      await client[action]();
    } catch (e) {
      setError(`窗口操作失败：${String(e)}`);
    } finally {
      busy.current = false;
      setPending(false);
    }
  };
  return {
    exit,
    rememberClose,
    setRememberClose,
    closeError,
    closePending,
    resolveClose,
    confirmExit: () => exitAction(true),
    cancelExit: () => exitAction(false),
    ...state,
    error,
    pending,
    native: !!client,
    mac: client?.platform === "macos",
    perform,
  };
}
