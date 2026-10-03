import { useCallback, useEffect, useRef, useState } from "react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import type { SettingsDto } from "../../shared/contracts/monitor";
import { moveTaskbarGroup } from "./taskbar-model";
import { defaultFontFamily, loadFont } from "../../shared/ui/fonts";
import { defaultAlertsConfig } from "../../shared/client/alert-defaults";
import {
  builtinFontCatalog,
  fontLabel,
  fontStyleOptions,
} from "../../shared/fonts";
const defaults: SettingsDto = {
  alerts: defaultAlertsConfig(),
  onboarding_completed: false,
  record_app_traffic_on_start: false,
  start_in_tray: false,
  autostart: false,
  close_action: "ask",
  revision: "0",
  theme: "system",
  font_family: defaultFontFamily,
  font_style: "auto",
  interval_ms: 1000,
  network_id: null,
  release_network_on_exit: true,
  taskbar: {
    enabled: false,
    hidden: false,
    layout: "double",
    cpu: true,
    gpu: true,
    memory: true,
    network: true,
    cpu_temperature: true,
    gpu_temperature: true,
    gpu_id: null,
    order: ["network", "cpu", "gpu", "memory"],
  },
};
export function useSettingsViewModel(
  client: MonitorClient,
  confirmed?: SettingsDto,
) {
  const [draft, setDraft] = useState<SettingsDto>(confirmed ?? defaults);
  const [pending, setPending] = useState(false);
  const busy = useRef(false);
  const [feedback, setFeedback] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [fontCatalog, setFontCatalog] = useState(builtinFontCatalog);
  const [fontLoading, setFontLoading] = useState(true);
  const [fontError, setFontError] = useState<string | null>(null);
  const fontRequest = useRef(0);
  const refreshFonts = useCallback(
    (refresh = true) => {
      const request = ++fontRequest.current;
      setFontLoading(true);
      setFontError(null);
      void (
        client.getFontCatalog?.(refresh) ?? Promise.resolve(builtinFontCatalog)
      )
        .then((catalog) => {
          if (request === fontRequest.current) setFontCatalog(catalog);
        })
        .catch((error) => {
          if (request === fontRequest.current) setFontError(String(error));
        })
        .finally(() => {
          if (request === fontRequest.current) setFontLoading(false);
        });
    },
    [client],
  );
  useEffect(() => {
    refreshFonts(false);
    return () => {
      fontRequest.current++;
    };
  }, [refreshFonts]);
  const change = (patch: Partial<SettingsDto>) => {
    if (busy.current) return;
    const previous = client.getSnapshot().state?.settings;
    if (!previous) {
      setError("采集服务未连接，无法保存设置");
      return;
    }
    busy.current = true;
    setPending(true);
    setError(null);
    const next = { ...previous, ...patch, revision: previous.revision };
    setDraft(next);
    setFeedback("正在保存…");
    void (
      next.font_family !== previous.font_family ||
      next.font_style !== previous.font_style
        ? (client.getFontCatalog?.() ?? Promise.resolve(fontCatalog)).then(
            (catalog) => loadFont(next.font_family, next.font_style, catalog),
          )
        : Promise.resolve()
    )
      .then(() => client.updateSettings(next))
      .then((saved) => {
        const current = client.getSnapshot().state?.settings;
        setDraft(
          current && BigInt(current.revision) > BigInt(saved.revision)
            ? current
            : saved,
        );
        setFeedback(
          client.getSnapshot().demo ? "已更新演示设置" : "已自动保存",
        );
      })
      .catch((error) => {
        setDraft(client.getSnapshot().state?.settings ?? previous);
        setError(String(error));
        setFeedback("保存失败，已恢复原设置");
      })
      .finally(() => {
        busy.current = false;
        setPending(false);
      });
  };
  const currentDraft = pending ? draft : (confirmed ?? defaults);
  const fontOptions = fontCatalog.families.map((font) => ({
    value: font.id,
    label: fontLabel(font),
    disabled: false,
  }));
  const selectedFamily = fontCatalog.families.find(
    (font) => font.id === currentDraft.font_family,
  );
  if (!selectedFamily)
    fontOptions.push({
      value: currentDraft.font_family,
      label: `${currentDraft.font_family.replace(/^installed:/, "")}（不可用）`,
      disabled: true,
    });
  const styleOptions = fontStyleOptions(selectedFamily);
  const missingStyle = !styleOptions.some(
    (style) => style.value === currentDraft.font_style,
  );
  if (missingStyle)
    styleOptions.push({
      value: currentDraft.font_style,
      label: "已保存样式（不可用）",
    });
  return {
    fontOptions,
    styleOptions,
    fontLoading,
    refreshFonts,
    fontNotice:
      fontError ??
      (!selectedFamily || missingStyle
        ? "已保存的字体或样式当前不可用，请重新选择。"
        : fontCatalog.detail),
    changeFont: (font_family: string) =>
      change({ font_family, font_style: "auto" }),
    changeTaskbar: (patch: Partial<SettingsDto["taskbar"]>) => {
      const current = client.getSnapshot().state?.settings.taskbar;
      if (current) change({ taskbar: { ...current, ...patch } });
    },
    moveTaskbar: (key: string, direction: number) => {
      const current = client.getSnapshot().state?.settings.taskbar;
      if (current)
        change({ taskbar: moveTaskbarGroup(current, key, direction) });
    },
    draft: currentDraft,
    pending: pending || !confirmed,
    feedback,
    error,
    change,
    exit: async () => {
      try {
        await client.requestExit?.();
      } catch (error) {
        setError(String(error));
      }
    },
    reset: () => change(defaults),
    resetAppearance: () =>
      change({
        theme: defaults.theme,
        font_family: defaults.font_family,
        font_style: defaults.font_style,
      }),
    resetMonitoring: () =>
      change({
        interval_ms: defaults.interval_ms,
        network_id: defaults.network_id,
      }),
  };
}
export type SettingsViewModel = ReturnType<typeof useSettingsViewModel>;
