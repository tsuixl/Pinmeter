import { useEffect, useLayoutEffect, useMemo, useState } from "react";
import { IpView } from "../features/ip/IpView";
import { HardwareView } from "../features/hardware/HardwareView";
import { ipClient } from "../shared/client/ip-client";
import type { MonitorClient } from "../shared/client/monitor-client";
import {
  useMonitorViewModel,
  pageLabels,
  statusLabels,
  type Page,
} from "../features/monitoring/useMonitorViewModel";
import { TrendChart } from "../features/monitoring/TrendChart";
import { CpuProcessorList } from "../features/monitoring/CpuProcessorList";
import { OverviewView } from "../features/overview/OverviewView";
import { useGpuViewModel } from "../features/gpu/useGpuViewModel";
import { GpuView } from "../features/gpu/GpuView";
import { AppNetworkRanking } from "../features/app-network/AppNetworkRanking";
import type { MetricKey } from "../features/monitoring/chart";
import { SettingsView } from "../features/settings/SettingsView";
import {
  Alert,
  Badge,
  Button,
  Card,
  Select,
  SegmentedControl,
  SidebarItem,
  StatCard,
  IconButton,
  Popover,
  Modal,
  Checkbox,
} from "../shared/ui/sakani";
import { icons } from "../shared/ui/icons";
import pinmeterIcon from "../assets/pinmeter.png";
import type { ReadingStatus } from "../shared/contracts/monitor";
import type { WindowClient } from "../shared/client/window-client";
import { useWindowViewModel } from "./useWindowViewModel";
import { WindowControls } from "./WindowControls";
import { MonitorStatus } from "../features/monitoring/MonitorStatus";
import { buildInfo } from "../shared/client/build-info";
import { monitorHealth } from "../features/monitoring/health";
import type { UpdateClient } from "../shared/client/update-client";
import { useUpdateViewModel } from "../features/updates/useUpdateViewModel";
import { UpdateBanner, UpdateDialogs } from "../features/updates/UpdateViews";

const labels: Record<MetricKey, string> = {
  cpu: "CPU 使用率",
  cpu_temperature: "CPU 温度",
  memory: "物理内存",
  download: "下载速度",
  upload: "上传速度",
};
const rangeOptions = [
  { value: "60000", label: "1 分钟" },
  { value: "300000", label: "5 分钟" },
];
export function App({
  client,
  windowClient,
  updateClient,
}: {
  client: MonitorClient;
  windowClient?: WindowClient;
  updateClient: UpdateClient;
}) {
  const vm = useMonitorViewModel(client);
  const health = monitorHealth(vm);
  const ip = useMemo(() => ipClient(client), [client]);
  const gpuVm = useGpuViewModel(vm.state?.gpu, vm.connected, vm.history);
  const windowVm = useWindowViewModel(windowClient);
  const [helpOpen, setHelpOpen] = useState(false);
  const [documentVisible, setDocumentVisible] = useState(!document.hidden);
  useEffect(() => {
    const changed = () => setDocumentVisible(!document.hidden);
    document.addEventListener("visibilitychange", changed);
    return () => document.removeEventListener("visibilitychange", changed);
  }, []);
  const updateVisible =
    documentVisible &&
    (windowVm.exit.stage === "idle" || windowVm.exit.stage === "updating") &&
    !helpOpen;
  const updateVm = useUpdateViewModel(updateClient, updateVisible);
  const theme =
    vm.state?.settings.theme ?? windowClient?.initialTheme ?? "system";
  const [collapsed, setCollapsed] = useState(
    () => matchMedia("(max-width: 1023px)").matches,
  );
  useLayoutEffect(() => {
    const query = matchMedia("(prefers-color-scheme: dark)");
    const update = () => {
      const resolved =
        theme === "system"
          ? (vm.nativeTheme ?? (query.matches ? "dark" : "light"))
          : theme;
      document.documentElement.dataset.theme = resolved;
      document.documentElement.classList.toggle("dark", resolved === "dark");
      document.documentElement.style.colorScheme = resolved;
    };
    update();
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  }, [theme, vm.nativeTheme]);
  useEffect(() => {
    const query = matchMedia("(max-width: 1023px)");
    const update = () => setCollapsed(query.matches);
    query.addEventListener("change", update);
    return () => query.removeEventListener("change", update);
  }, []);
  const go = (page: Page) => {
    vm.setPage(page);
    vm.setAnchor(null);
    requestAnimationFrame(() =>
      document.querySelector<HTMLElement>("h1")?.focus(),
    );
  };
  const chart = (keys: MetricKey[], label: string) => (
    <TrendChart
      history={vm.history}
      keys={keys}
      range={vm.range}
      anchor={vm.anchor}
      onAnchor={vm.setAnchor}
      label={label}
    />
  );
  const value = (key: MetricKey) => {
    const r = vm.reading(key);
    return `${r.text}${r.unit ? ` ${r.unit}` : ""}`;
  };
  const panel = (keys: MetricKey[], title: string) => (
    <Card className="trend-card">
      <div className="section-heading">
        <h2>{title}</h2>
        <div className="legend">
          {keys.map((key, i) => (
            <span key={key}>
              <i className={i ? "secondary" : ""} />
              {labels[key]}
            </span>
          ))}
        </div>
      </div>
      {chart(keys, title)}
    </Card>
  );
  const nav = (page: Page) => (
    <SidebarItem
      key={page}
      icon={icons[page]}
      label={pageLabels[page]}
      active={page === vm.page}
      collapsed={collapsed}
      onClick={() => go(page)}
    />
  );
  return (
    <div
      className={`app ${collapsed ? "compact-nav" : ""} ${windowVm.mac && !windowVm.fullscreen ? "mac-window" : ""}`}
    >
      <UpdateDialogs vm={updateVm} visible={updateVisible} />
      <Modal
        open={helpOpen && windowVm.exit.stage === "idle"}
        title="使用 Pinmeter"
        confirmLabel="知道了"
        cancelLabel="关闭"
        onConfirm={() => setHelpOpen(false)}
        onClose={() => setHelpOpen(false)}
      >
        <div className="health-details">
          <p>
            总览查看当前读数，点击卡片查看趋势和指标说明。无效读数显示
            —，可从“运行状态”查看原因。
          </p>
          <p>
            在网络页点击“开始监控”查看应用流量。切页和最小化到托盘继续累计，明确停止后再次开始会重新计数。
          </p>
          <p>
            最小化后，点击系统托盘中的 Pinmeter
            图标恢复窗口；图标可能收在任务栏的隐藏图标菜单中。关闭行为可在设置中更改。
          </p>
          <p>
            需要完全退出时，从设置或托盘菜单选择“退出
            Pinmeter”。默认会先解除网络限制；最小化不会解除限制。
          </p>
          <p>
            网络出现异常时，可从任何页面的“运行状态”解除全部网络限制，并在网络页核对结果。
          </p>
        </div>
      </Modal>
      <Modal
        open={
          windowVm.exit.stage !== "idle" &&
          windowVm.exit.stage !== "updating" &&
          windowVm.exit.stage !== "choose_close"
        }
        title={
          windowVm.exit.stage === "failed"
            ? "未能解除全部限制"
            : windowVm.exit.stage === "confirm"
              ? "退出后保留网络禁用？"
              : "正在退出 Pinmeter"
        }
        description={windowVm.exit.detail}
        confirmLabel={
          windowVm.exit.stage === "failed"
            ? "重试并退出"
            : windowVm.exit.stage === "confirm"
              ? "保留禁用并退出"
              : "正在解除…"
        }
        cancelLabel="返回应用"
        confirmLoading={windowVm.exit.stage === "releasing"}
        closeOnBackdropClick={false}
        closeOnEscape={windowVm.exit.stage !== "releasing"}
        onClose={() => {
          if (windowVm.exit.stage !== "releasing") void windowVm.cancelExit();
        }}
        onConfirm={() => {
          if (windowVm.exit.stage !== "releasing") void windowVm.confirmExit();
        }}
      />
      <Modal
        open={windowVm.exit.stage === "choose_close"}
        title="关闭 Pinmeter"
        description="最小化到托盘后继续监控，点击托盘图标可恢复窗口。"
        cancelLabel="退出"
        confirmLabel="最小化到托盘"
        confirmLoading={windowVm.closePending}
        closeOnBackdropClick={false}
        closeOnEscape={!windowVm.closePending}
        onClose={() => {
          if (!windowVm.closePending) void windowVm.cancelExit();
        }}
        onCancel={() => {
          if (!windowVm.closePending) void windowVm.resolveClose("exit");
        }}
        onConfirm={() => {
          void windowVm.resolveClose("minimize");
        }}
      >
        <Checkbox
          label="记住我的选择"
          description="可以在设置中更改关闭行为。"
          checked={windowVm.rememberClose}
          disabled={windowVm.closePending}
          onChange={(event) => windowVm.setRememberClose(event.target.checked)}
        />
        {(windowVm.closeError || windowVm.exit.detail) && (
          <Alert
            color="danger"
            title="未能执行关闭操作"
            description={windowVm.closeError || windowVm.exit.detail}
          />
        )}
      </Modal>
      <aside className="sidebar">
        <div
          className="brand"
          data-tauri-drag-region={windowVm.native ? "" : undefined}
        >
          <img src={pinmeterIcon} width={24} height={24} alt="" />
          <span>
            Pinmeter<span className="brand-dot">.</span>
          </span>
        </div>
        <nav aria-label="监控页面">
          {(
            [
              "overview",
              "hardware",
              "cpu",
              "memory",
              "gpu",
              "network",
              "ip",
            ] as Page[]
          ).map(nav)}
        </nav>
        <div className="sidebar-bottom">
          <UpdateBanner vm={updateVm} collapsed={collapsed} />
          {nav("settings")}
          <div className="device">
            <icons.monitor size={18} />
            <span>
              此电脑<small>{vm.state?.platform ?? "本地监控"}</small>
            </span>
          </div>
          <span className="version">v{buildInfo.version}</span>
        </div>
      </aside>
      <div className="workspace">
        <header
          className="topbar"
          data-tauri-drag-region={windowVm.native ? "" : undefined}
        >
          <div className="breadcrumb">
            <icons.monitor size={16} />
            <span>此电脑</span>
            <span>/</span>
            <strong>{pageLabels[vm.page]}</strong>
          </div>
          <div className="topbar-actions">
            <IconButton
              icon={icons.info}
              size="sm"
              variant="ghost"
              aria-label="使用帮助"
              onClick={() => setHelpOpen(true)}
            />
            <span className="connection-status">
              <Badge variant={vm.demo ? "warning" : "neutral"}>
                {vm.demo
                  ? "演示数据"
                  : vm.connected
                    ? health.issues.length
                      ? "部分功能需关注"
                      : vm.page === "ip"
                        ? "按需联网查询"
                        : "本机实时数据"
                    : "未连接"}
              </Badge>
            </span>
            <WindowControls vm={windowVm} />
          </div>
        </header>
        <main className="content" key={vm.page}>
          <MonitorStatus
            client={client}
            suspended={windowVm.exit.stage !== "idle"}
            onNavigate={go}
          />
          {windowVm.error && (
            <Alert
              color="warning"
              title="窗口操作"
              description={windowVm.error}
            />
          )}
          <div className="page-heading">
            <h1 tabIndex={-1}>
              {vm.page === "overview" ? "系统总览" : pageLabels[vm.page]}
            </h1>
            {vm.page !== "settings" &&
              vm.page !== "ip" &&
              vm.page !== "hardware" && (
                <div aria-label="趋势时间范围">
                  <SegmentedControl
                    options={rangeOptions}
                    value={String(vm.range)}
                    onChange={(range) => {
                      vm.setRange(Number(range));
                      vm.setAnchor(null);
                    }}
                  />
                </div>
              )}
          </div>
          {(vm.error || vm.state?.diagnostic) && (
            <Alert
              color="warning"
              title="采集状态"
              description={vm.error ?? vm.state?.diagnostic ?? ""}
            />
          )}
          {vm.demo && (
            <div className="demo-controls">
              <Badge variant="warning">演示数据 · 不代表系统实际读数</Badge>
              {vm.page !== "ip" && vm.page !== "hardware" && (
                <Select
                  label="演示状态"
                  size="sm"
                  value={vm.frame?.cpu.status ?? "normal"}
                  options={Object.entries(statusLabels).map(
                    ([value, label]) => ({
                      value,
                      label,
                    }),
                  )}
                  onChange={(status) =>
                    client.setScenario?.(status as ReadingStatus)
                  }
                />
              )}
            </div>
          )}
          {vm.page === "overview" && (
            <OverviewView monitor={vm} gpu={gpuVm} onNavigate={go} />
          )}
          {vm.page === "hardware" && (
            <HardwareView
              client={client}
              state={vm.state}
              connected={vm.connected}
            />
          )}
          {(vm.page === "cpu" || vm.page === "memory") && (
            <>
              {vm.page === "cpu" &&
                (vm.driverMissing || vm.driverMessage || vm.driverError) && (
                  <div className="temperature-driver">
                    <Alert
                      color={vm.driverError ? "danger" : "info"}
                      title={
                        vm.driverError
                          ? "温度驱动操作未完成"
                          : vm.driverInstalling
                            ? "正在下载并安装驱动"
                            : vm.driverPending
                              ? "正在检测驱动"
                              : vm.driverMissing
                                ? "CPU 温度需要 PawnIO 驱动"
                                : "温度驱动状态"
                      }
                      description={
                        vm.driverError ||
                        vm.driverMessage ||
                        (vm.driverInstalling
                          ? "正在从官方来源下载、校验并安装 PawnIO，请等待操作完成。"
                          : "点击后将联网下载官方 PawnIO 驱动并安装，完成后自动检测。安装时可能需要管理员授权。")
                      }
                    />
                    {vm.driverMissing && (
                      <Button
                        loading={vm.driverInstalling}
                        disabled={vm.driverPending || !vm.connected}
                        onClick={() => void vm.installTemperatureDriver()}
                      >
                        {vm.driverInstalling ? "正在处理…" : "下载安装驱动"}
                      </Button>
                    )}
                    {(vm.driverMissing ||
                      vm.driverMessage ||
                      vm.driverError) && (
                      <Button
                        loading={vm.driverPending && !vm.driverInstalling}
                        disabled={vm.driverPending || !vm.connected}
                        onClick={() => void vm.checkTemperatureDriver()}
                      >
                        重新检测
                      </Button>
                    )}
                  </div>
                )}
              <div
                className={`detail-summary ${vm.page === "cpu" ? "cpu-summary" : ""}`}
              >
                <StatCard
                  title={labels[vm.page]}
                  value={value(vm.page)}
                  description={
                    vm.page === "cpu" && vm.reading("cpu").status === "normal"
                      ? vm.cpuModel
                      : statusLabels[vm.reading(vm.page).status]
                  }
                  variant="icon"
                  icon={icons[vm.page]}
                />
                {vm.page === "cpu" && (
                  <div className="cpu-temperature">
                    <StatCard
                      title="CPU 温度"
                      value={
                        vm.temperature.status === "normal"
                          ? vm.temperature.text + " °C"
                          : "—"
                      }
                      description={
                        vm.temperature.status === "normal"
                          ? vm.cpuModel
                          : vm.temperature.detail ||
                            statusLabels[vm.temperature.status]
                      }
                      variant="icon"
                      icon={icons.temperature}
                    />
                  </div>
                )}
                {vm.page === "memory" && (
                  <Card className="detail-meta">
                    <dl className="data-rows">
                      <>
                        <div>
                          <dt>已使用</dt>
                          <dd>
                            {vm.reading("memory").status === "normal"
                              ? vm.frame?.memory_used
                              : "—"}
                          </dd>
                        </div>
                        <div>
                          <dt>物理内存总量</dt>
                          <dd>
                            {vm.reading("memory").status === "normal"
                              ? vm.frame?.memory_total
                              : "—"}
                          </dd>
                        </div>
                      </>
                      <div>
                        <dt>采样间隔</dt>
                        <dd>
                          {(vm.state?.settings.interval_ms ?? 1000) / 1000} 秒
                        </dd>
                      </div>
                    </dl>
                  </Card>
                )}
              </div>
              {panel(
                vm.page === "cpu" ? ["cpu", "cpu_temperature"] : [vm.page],
                `${pageLabels[vm.page]} 趋势`,
              )}
              {vm.page === "cpu" && (
                <CpuProcessorList
                  processors={vm.processors}
                  status={vm.processorsStatus}
                  detail={vm.processorsDetail}
                />
              )}
              <Popover
                key={vm.page}
                className="metric-info"
                placement="top"
                title={`${pageLabels[vm.page]} 指标说明`}
                trigger={
                  <div className="metric-info-trigger">
                    <h2>关于这项指标</h2>
                    <IconButton
                      type="button"
                      icon={icons.info}
                      variant="ghost"
                      size="sm"
                      aria-label="查看指标说明"
                      aria-haspopup="dialog"
                    />
                  </div>
                }
              >
                <div className="explanation">
                  {vm.page === "cpu" && (
                    <p>
                      统计范围：全部逻辑处理器 · 采样间隔：
                      {vm.state
                        ? `${vm.state.settings.interval_ms / 1000} 秒`
                        : "等待采集服务"}
                      。列表显示当前占用，趋势图显示整机历史。
                    </p>
                  )}
                  <p>
                    {vm.page === "cpu"
                      ? "两次有效采样间的整机忙碌时间占比；与任务管理器的处理器效用口径可能不同。首次采样需要预热。"
                      : "系统物理内存已用量与总量。Windows 已用量为总物理内存减可用内存，不代表提交量。"}
                  </p>
                  <p>来源：{vm.reading(vm.page).source}</p>
                  {vm.reading(vm.page).detail && (
                    <p>{vm.reading(vm.page).detail}</p>
                  )}
                </div>
              </Popover>
            </>
          )}
          {vm.page === "gpu" && <GpuView vm={gpuVm} range={vm.range} />}
          {vm.page === "ip" && <IpView client={ip} />}
          {vm.page === "network" && (
            <>
              <div className="network-heading">
                <div className="connection-name">
                  <icons.network size={20} />
                  <strong>{vm.networkName}</strong>
                  <Badge>
                    {vm.state?.settings.network_id ? "手动选择" : "自动选择"}
                  </Badge>
                </div>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => go("settings")}
                >
                  切换网卡
                </Button>
              </div>
              <div className="two-charts">
                {(["download", "upload"] as MetricKey[]).map((key) => (
                  <Card key={key}>
                    <div className="section-heading">
                      <h2>{labels[key]}</h2>
                      <Badge>{statusLabels[vm.reading(key).status]}</Badge>
                    </div>
                    <div className="network-value number">{value(key)}</div>
                    {chart([key], labels[key])}
                  </Card>
                ))}
              </div>
              <div className="explanation">
                <h2>单个接口，清楚计量</h2>
                <p>
                  速率来自累计字节差与实际经过时间。使用十进制
                  KB/s、MB/s，不叠加其他网卡。接口切换、重连或休眠后重新预热，曲线保留断档。
                </p>
                <p>来源：{vm.reading("download").source}</p>
              </div>
              <AppNetworkRanking
                client={client}
                suspended={windowVm.exit.stage !== "idle"}
              />
            </>
          )}
          {vm.page === "settings" && (
            <SettingsView client={client} state={vm.state} updates={updateVm} />
          )}
        </main>
        <footer className="footer">
          <span>
            <i className={vm.connected ? "live-dot" : "offline-dot"} />
            {vm.demo
              ? "演示数据"
              : vm.connected
                ? vm.page === "ip"
                  ? "查询服务已连接"
                  : "本机采集"
                : "采集服务未连接"}
            {vm.page !== "ip" && vm.frame && vm.connected
              ? ` · ${["cpu", "memory", "download", "upload"].every((key) => vm.frame![key as MetricKey].status === "normal") ? "实时" : "部分指标未就绪"}`
              : ""}
          </span>
          <span>
            {vm.page === "ip"
              ? "按需查询 · 第三方来源"
              : `${(vm.state?.settings.interval_ms ?? 1000) / 1000} 秒 / 次 · 最近 5 分钟`}
          </span>
        </footer>
      </div>
    </div>
  );
}
