import { useEffect, useRef, useState, type MutableRefObject } from "react";
import {
  ArrowLeft,
  Bell,
  CircleHelp,
  Database,
  Gauge,
  Info,
  PanelBottom,
  Palette,
  SlidersHorizontal,
} from "lucide-react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import { settingsSections, type SettingsSection } from "./sections";
import { SetupCard } from "./SetupCard";
import type { MonitorStateDto } from "../../shared/contracts/monitor";
import { TaskbarSettingsView } from "./TaskbarSettingsView";
import { useSettingsViewModel } from "./useSettingsViewModel";
import { buildInfo } from "../../shared/client/build-info";
import { UpdateSettings } from "../updates/UpdateViews";
import { DiagnosticsCard } from "../diagnostics/DiagnosticsCard";
import { TraceCard } from "../diagnostics/TraceCard";
import { AlertsSettingsForm } from "../alerts/AlertsView";
import { useAlertsEditor } from "../alerts/useAlertsEditor";
import { HistoryDataCard } from "../history/HistoryDataCard";
import { FontCombobox } from "../../shared/ui/FontCombobox";
import type { UpdateViewModel } from "../updates/useUpdateViewModel";
import {
  Alert,
  Button,
  Card,
  Radio,
  Select,
  Switch,
  Modal,
  SidebarItem,
} from "../../shared/ui/sakani";
import "./settings.css";

const sectionIcons = {
  resident: SlidersHorizontal,
  appearance: Palette,
  monitoring: Gauge,
  taskbar: PanelBottom,
  alerts: Bell,
  data: Database,
  diagnostics: CircleHelp,
  updates: Info,
};

export function SettingsView({
  client,
  state,
  updates,
  section,
  onHistory,
  onNavigate,
  onBack,
  navigationGuardRef,
}: {
  client: MonitorClient;
  state: MonitorStateDto | null;
  updates: UpdateViewModel;
  section?: SettingsSection;
  onHistory?: () => void;
  onNavigate?: (page: "cpu" | "memory") => void;
  onBack?: () => void;
  navigationGuardRef?: MutableRefObject<((action: () => void) => void) | null>;
}) {
  const vm = useSettingsViewModel(client, state?.settings);
  const alerts = useAlertsEditor(client);
  const [active, setActive] = useState<SettingsSection>(section ?? "resident");
  const [showDetail, setShowDetail] = useState(!!section);
  const [leaveAction, setLeaveAction] = useState<(() => void) | null>(null);
  const leaving = useRef(false);
  const content = useRef<HTMLElement>(null);
  const sidebar = useRef<HTMLElement>(null);
  const focusHeading = () => {
    requestAnimationFrame(() => {
      const heading = content.current?.querySelector<HTMLElement>("h2");
      if (heading?.getClientRects().length)
        heading.focus({ preventScroll: true });
    });
  };
  const [storageThrough] = useState(Date.now);
  const [confirmReset, setConfirmReset] = useState(false);
  useEffect(() => {
    if (section) {
      setActive(section);
      setShowDetail(true);
      focusHeading();
    }
  }, [section]);
  useEffect(() => {
    content.current?.scrollTo({ top: 0 });
  }, [active]);
  const selectSection = (next: SettingsSection) => {
    setActive(next);
    setShowDetail(true);
    focusHeading();
  };
  const returnToCategories = () => {
    setShowDetail(false);
    requestAnimationFrame(() => {
      sidebar.current
        ?.querySelector<HTMLButtonElement>(
          `[data-settings-section="${active}"] button`,
        )
        ?.focus();
    });
  };
  const requestLeave = (action: () => void) => {
    if (vm.pending || alerts.pending || leaving.current) return;
    if (alerts.dirty) setLeaveAction(() => action);
    else action();
  };
  useEffect(() => {
    if (!navigationGuardRef) return;
    navigationGuardRef.current = requestLeave;
    return () => {
      if (navigationGuardRef.current === requestLeave)
        navigationGuardRef.current = null;
    };
  });
  const saveAndLeave = async () => {
    if (leaving.current || !leaveAction) return;
    leaving.current = true;
    try {
      if (await alerts.saveDraft()) {
        setLeaveAction(null);
        leaveAction();
      }
    } finally {
      leaving.current = false;
    }
  };
  const adapters = [
    { value: "auto", label: "自动选择" },
    ...(state?.interfaces.map((a) => ({
      value: a.id,
      label: a.name + (a.up ? "" : " · 已断开"),
    })) ?? []),
  ];
  if (
    vm.draft.network_id &&
    !state?.interfaces.some((a) => a.id === vm.draft.network_id)
  )
    adapters.push({
      value: vm.draft.network_id,
      label: "已保存的接口 · 不可用",
    });
  return (
    <div
      className={
        "settings-page settings-workspace" +
        (showDetail ? " settings-show-detail" : "")
      }
    >
      <aside ref={sidebar} className="settings-sidebar" aria-label="设置导航">
        <div className="settings-title">
          <Button
            variant="ghost"
            size="sm"
            disabled={vm.pending || alerts.pending}
            onClick={() => requestLeave(() => onBack?.())}
          >
            <ArrowLeft size={16} aria-hidden="true" /> 返回
          </Button>
          <h1 tabIndex={-1}>设置</h1>
        </div>
        <nav aria-label="设置分类">
          {settingsSections.map((item) => (
            <div key={item.value} data-settings-section={item.value}>
              <SidebarItem
                icon={sectionIcons[item.value]}
                label={item.label}
                active={active === item.value}
                badge={
                  item.value === "alerts" && alerts.dirty ? "未保存" : undefined
                }
                disabled={vm.pending || alerts.pending}
                onClick={() => selectSection(item.value)}
              />
            </div>
          ))}
        </nav>
      </aside>
      <section
        ref={content}
        className="settings-pane"
        aria-label={
          settingsSections.find((item) => item.value === active)?.label
        }
      >
        <div className="settings-form">
          <div className="settings-detail-heading">
            <Button
              className="settings-category-back"
              variant="ghost"
              size="sm"
              onClick={returnToCategories}
            >
              <ArrowLeft size={16} aria-hidden="true" /> 返回设置分类
            </Button>
            <h2 tabIndex={-1}>
              {settingsSections.find((item) => item.value === active)?.label}
            </h2>
          </div>
          <div className="settings-actions" role="status">
            <span>
              {active === "alerts"
                ? "提醒更改后点击保存生效"
                : vm.feedback || "设置更改后自动保存"}
            </span>
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={() => setConfirmReset(true)}
              disabled={vm.pending || alerts.pending}
            >
              恢复默认
            </Button>
          </div>
          <Modal
            open={!!leaveAction}
            title="提醒设置尚未保存"
            description="返回前保存占用提醒的更改，或放弃本次草稿。其他设置已自动保存。"
            confirmLabel="保存并返回"
            cancelLabel="取消"
            confirmLoading={alerts.pending}
            closeOnEscape={!alerts.pending}
            closeOnBackdropClick={!alerts.pending}
            onClose={() => {
              if (!leaving.current) setLeaveAction(null);
            }}
            onConfirm={() => void saveAndLeave()}
          >
            <Button
              variant="ghost"
              disabled={alerts.pending}
              onClick={() => {
                if (leaving.current) return;
                alerts.discard();
                const action = leaveAction;
                setLeaveAction(null);
                action?.();
              }}
            >
              放弃并返回
            </Button>
            {alerts.error && (
              <Alert
                color="danger"
                title="保存失败，仍保留草稿"
                description={alerts.error}
              />
            )}
          </Modal>
          <Modal
            open={confirmReset}
            title="恢复全部默认设置？"
            description="将重置外观、采样与网卡、启动与关闭方式、任务栏、提醒规则、应用流量自动记录和退出网络限制偏好。已保存的历史不会删除；当前网络规则不在此处清除。"
            confirmLabel="恢复默认"
            cancelLabel="取消"
            onClose={() => setConfirmReset(false)}
            onConfirm={() => {
              setConfirmReset(false);
              void vm.reset().then((saved) => {
                if (saved) alerts.discard();
              });
            }}
          />
          {vm.error && (
            <Alert
              color="danger"
              title="保存失败，已恢复原设置"
              description={vm.error}
            />
          )}

          {active === "appearance" && (
            <Card
              title="外观"
              description="主题作用于主窗口，字体同时作用于主窗口和任务栏"
            >
              <fieldset className="theme-options" disabled={vm.pending}>
                <legend className="sr-only">主题</legend>
                {[
                  ["system", "跟随系统"],
                  ["light", "浅色"],
                  ["dark", "深色"],
                ].map(([value, label]) => (
                  <Radio
                    key={value}
                    name="theme"
                    value={value}
                    label={label}
                    disabled={vm.pending}
                    checked={vm.draft.theme === value}
                    onChange={() => vm.change({ theme: value })}
                  />
                ))}
              </fieldset>
              <div className="settings-fields font-settings font-pickers">
                <FontCombobox
                  label="界面字体"
                  description="内置字体与本机已安装字体，支持搜索"
                  value={vm.draft.font_family}
                  disabled={vm.pending || vm.fontLoading}
                  loading={vm.fontLoading}
                  options={vm.fontOptions}
                  onChange={(value) => {
                    if (typeof value === "string") vm.changeFont(value);
                  }}
                />
                <Select
                  id="font-style"
                  label="字体样式"
                  description="按所选字体提供实际可用的粗细与斜体"
                  value={vm.draft.font_style}
                  disabled={vm.pending || vm.fontLoading}
                  options={vm.styleOptions}
                  onChange={(font_style) => vm.change({ font_style })}
                />
              </div>
              <div className="font-catalog-actions font-pickers">
                <span className="settings-note" role="status">
                  {vm.fontNotice}
                </span>
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={vm.pending || vm.fontLoading}
                  onClick={() => vm.refreshFonts()}
                >
                  刷新字体列表
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={vm.pending}
                  onClick={() => vm.changeFont("harmonyos_sans_sc")}
                >
                  恢复默认字体
                </Button>
              </div>
              <p className="font-preview" aria-label="字体预览">
                内存与网络 CPU{" "}
                <span className="font-preview-digits">0123456789</span> · 100% ·
                128 MB/s
              </p>
              <p className="settings-note">
                字体同时用于主窗口和任务栏；完整字体许可可在“关于与更新”查看。
              </p>
              <Button
                variant="ghost"
                size="sm"
                disabled={vm.pending}
                onClick={vm.resetAppearance}
              >
                仅恢复外观默认值
              </Button>
            </Card>
          )}
          {active === "resident" && (
            <>
              <Card title="启动" description="登录电脑后自动打开 Pinmeter">
                <Switch
                  label="启动到托盘"
                  checked={vm.draft.start_in_tray}
                  disabled={
                    vm.pending ||
                    (!client.getSnapshot().demo &&
                      state?.platform !== "windows")
                  }
                  onChange={(event) =>
                    vm.change({ start_in_tray: event.target.checked })
                  }
                />
                <p className="settings-note">
                  下次启动生效。保留托盘入口并继续监控，再次打开已运行的
                  Pinmeter 可恢复窗口；托盘不可用时自动显示主窗口。
                </p>
                <Switch
                  label="开机自启"
                  checked={vm.draft.autostart}
                  disabled={
                    vm.pending ||
                    (!state?.autostart.available && !vm.draft.autostart)
                  }
                  onChange={(event) =>
                    vm.change({ autostart: event.target.checked })
                  }
                />
                <p className="settings-note" role="status">
                  {state?.autostart.detail || "等待启动项状态"}
                </p>
              </Card>
              <Card title="窗口行为" description="选择点击关闭按钮时的操作">
                <Select
                  id="close-action"
                  label="关闭窗口时"
                  value={vm.draft.close_action}
                  disabled={vm.pending}
                  options={[
                    { value: "ask", label: "每次询问" },
                    { value: "minimize", label: "最小化到托盘" },
                    { value: "exit", label: "退出 Pinmeter" },
                  ]}
                  onChange={(close_action) => vm.change({ close_action })}
                />
                <p className="settings-note">
                  最小化后收起到托盘；再次打开 Pinmeter 可恢复主窗口。
                </p>
                <Button
                  type="button"
                  variant="ghost"
                  size="sm"
                  onClick={() => {
                    void client.recoverWindow().catch(() => {});
                  }}
                >
                  将窗口移回可见区域
                </Button>
              </Card>
              <SetupCard
                client={client}
                state={state}
                onSettings={selectSection}
                onHistory={() => requestLeave(() => onHistory?.())}
              />
              {state?.settings.onboarding_completed && (
                <Button
                  variant="ghost"
                  size="sm"
                  disabled={vm.pending}
                  onClick={() => vm.change({ onboarding_completed: false })}
                >
                  重新查看常驻引导
                </Button>
              )}
              {client.requestExit && (
                <Button
                  type="button"
                  variant="ghost"
                  disabled={vm.pending || alerts.pending}
                  onClick={() =>
                    requestLeave(() => {
                      void vm.exit();
                    })
                  }
                >
                  退出 Pinmeter
                </Button>
              )}
            </>
          )}
          {active === "monitoring" && (
            <Card title="监控" description="调整基础指标的刷新频率和网络来源">
              <div className="settings-fields">
                <Select
                  id="sample-interval"
                  label="采样间隔"
                  description="CPU、物理内存与网络使用同一采样周期"
                  value={String(vm.draft.interval_ms)}
                  onChange={(value) =>
                    vm.change({ interval_ms: Number(value) })
                  }
                  disabled={vm.pending}
                  options={[1000, 2000, 5000].map((ms) => ({
                    value: String(ms),
                    label: `${ms / 1000} 秒`,
                  }))}
                />
                <Select
                  id="network-interface"
                  label="网络接口"
                  description="自动模式只选择一个活动接口"
                  value={vm.draft.network_id ?? "auto"}
                  onChange={(value) =>
                    vm.change({ network_id: value === "auto" ? null : value })
                  }
                  disabled={vm.pending}
                  options={adapters}
                />
              </div>
              <p className="settings-note">
                当前接口：{state?.selected_interface?.name ?? "暂无可用网卡"}
                。最近五分钟的细粒度趋势保存在内存中，历史页另保存 24
                小时分钟汇总、7 天十五分钟汇总和 30 天小时汇总。
              </p>
              <Button
                variant="ghost"
                size="sm"
                disabled={vm.pending}
                onClick={vm.resetMonitoring}
              >
                仅恢复采样与网卡默认值
              </Button>
            </Card>
          )}
          {active === "taskbar" && (
            <TaskbarSettingsView vm={vm} state={state} />
          )}
          {active === "monitoring" && (
            <Card title="网络控制" description="决定退出时如何处理应用限制">
              <Switch
                label="退出 Pinmeter 时解除所有网络限制"
                checked={vm.draft.release_network_on_exit}
                disabled={vm.pending}
                onChange={(event) =>
                  vm.change({ release_network_on_exit: event.target.checked })
                }
              />
              <p className="settings-note">
                {vm.draft.release_network_on_exit
                  ? "正常退出时解除限速和网络禁用，保留配置但不启用。下次打开后可手动启用。"
                  : "退出后仅保留网络禁用，限速仍会停止。下次打开会重新应用已启用规则。"}
              </p>
              <p className="processor-caption">
                最小化不解除限制；强制结束或断电可能留下禁用规则。
              </p>
            </Card>
          )}
          {active === "data" && (
            <>
              <Card
                title="记录偏好"
                description="应用流量记录包含应用身份；监控数据仅保存在本机"
              >
                <Switch
                  label="启动时自动记录应用流量"
                  checked={vm.draft.record_app_traffic_on_start}
                  disabled={vm.pending}
                  onChange={(event) =>
                    vm.change({
                      record_app_traffic_on_start: event.target.checked,
                    })
                  }
                />
                <p className="settings-note">
                  下次启动生效，关闭此项不会停止本次记录或删除已保存的历史。查看趋势及按时间段导出请前往历史页。
                </p>
                <p className="settings-note">
                  IP 页按需访问第三方检测服务，对方会看到请求出口或查询的
                  IP；检查更新也需要联网。诊断导出采用字段白名单，不包含
                  IP、完整路径和进程名单。
                </p>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => requestLeave(() => onHistory?.())}
                  disabled={!onHistory}
                >
                  查看与导出历史
                </Button>
              </Card>
              <HistoryDataCard
                client={client}
                from={storageThrough - 30 * 86400000}
                through={storageThrough}
                onChanged={() => {}}
                mode="manage"
              />
            </>
          )}
          {active === "updates" && (
            <>
              <UpdateSettings vm={updates} />
              <Card
                title="关于 Pinmeter"
                description={`当前版本 ${buildInfo.version}`}
              >
                <p className="settings-note">
                  Pinmeter 采用 AGPL-3.0-only
                  许可。第三方组件与字体许可随完整运行目录提供。
                </p>
                <p className="settings-note">
                  本软件使用 HarmonyOS Sans 字体，版权所有 © 2021
                  华为终端有限公司。
                </p>
                <p className="settings-note">
                  便携版本须保留整个运行目录；完全退出旧版后再打开新版。迁移目录后，请重新设置开机自启。
                </p>
              </Card>
            </>
          )}
          {active === "alerts" && (
            <AlertsSettingsForm
              vm={alerts}
              onNavigate={(page) => requestLeave(() => onNavigate?.(page))}
            />
          )}
          {active === "diagnostics" && (
            <>
              <DiagnosticsCard client={client} />
              <TraceCard client={client} />
              <Card title="使用帮助" description="查看读数来源与恢复常驻窗口">
                <p className="settings-note">
                  CPU、内存等详情页提供指标说明；“—”表示尚无有效读数，请查看对应状态原因。托盘可重新打开窗口，位置异常时可在通用设置恢复。
                </p>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => selectSection("resident")}
                >
                  查看窗口与启动设置
                </Button>
              </Card>
            </>
          )}
        </div>
      </section>
    </div>
  );
}
