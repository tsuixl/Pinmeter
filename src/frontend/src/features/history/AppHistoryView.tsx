import type { TableColumn } from "@sakaniui/react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import type { AppHistoryRowDto } from "../../shared/contracts/monitor";
import {
  Alert,
  Badge,
  Button,
  Card,
  Input,
  Select,
  Switch,
  Table,
} from "../../shared/ui/sakani";
import { icons } from "../../shared/ui/icons";
import { TrendChart } from "../monitoring/TrendChart";
import type { ChartSeries } from "../monitoring/chart";
import { networkStatusLabels } from "../app-network/useAppNetworkViewModel";
import { bytes, coverage } from "./history-model";
import type { HistorySort } from "./app-history-model";
import { useAppHistoryViewModel } from "./useAppHistoryViewModel";
import "./app-history.css";
const series: ChartSeries[] = [
  {
    id: "download",
    label: "下载",
    color: "var(--color-chart-5)",
    network: true,
    maxGapMs: 90_000,
    read: (f) => f.download,
  },
  {
    id: "upload",
    label: "上传",
    color: "var(--color-chart-2)",
    network: true,
    maxGapMs: 90_000,
    read: (f) => f.upload,
  },
];
export function AppHistoryView({ client }: { client: MonitorClient }) {
  const vm = useAppHistoryViewModel(client);
  const columns: TableColumn<AppHistoryRowDto>[] = [
    {
      key: "name",
      header: "应用",
      width: "40%",
      render: (row) => (
        <Button
          variant={vm.selectedId === row.id ? "secondary" : "ghost"}
          size="sm"
          className="app-history-name"
          title={row.path || row.name}
          aria-label={`查看 ${row.name} 的流量趋势${row.path ? `，${row.path}` : ""}`}
          aria-pressed={vm.selectedId === row.id}
          onClick={() => vm.select(row.id)}
          leftIcon={<icons.app size={16} />}
        >
          <span>{row.name}</span>
        </Button>
      ),
    },
    { key: "received", header: "下载", render: (row) => bytes(row.received) },
    {
      key: "transmitted",
      header: "上传",
      render: (row) => bytes(row.transmitted),
    },
    { key: "total", header: "合计", render: (row) => bytes(row.total) },
  ];
  return (
    <Card className="app-history-card">
      <div className="app-history-content">
        <div className="section-heading">
          <div>
            <h2>应用流量明细</h2>
            <p className="processor-caption">
              全机可观测 TCP/UDP · 最近 24 小时保留在本机
            </p>
          </div>
          <div className="app-history-actions">
            <Badge
              variant={
                vm.running && vm.status === "normal" ? "success" : "neutral"
              }
            >
              {vm.running && vm.status === "normal"
                ? "记录中"
                : vm.status === "disabled"
                  ? vm.data?.rows.length
                    ? "已暂停"
                    : "未开启"
                  : (networkStatusLabels[vm.status] ?? vm.status)}
            </Badge>
            <Button
              variant={vm.running ? "secondary" : "primary"}
              disabled={vm.pending || !vm.canRecord}
              onClick={() => void vm.record()}
            >
              {vm.pending ? "正在处理…" : vm.running ? "停止记录" : "开始记录"}
            </Button>
          </div>
        </div>
        <Switch
          label="启动时自动记录应用流量"
          checked={vm.autoRecord}
          disabled={vm.pending || !vm.canRecord}
          onChange={(event) => void vm.changeAuto(event.target.checked)}
        />
        <p className="processor-caption">
          和网络页共用同一采集。切页、托盘继续记录，停止或重启保留已保存历史。自动记录开关在下次启动生效；旧总量无法补拆成应用明细。
        </p>
        {vm.error && (
          <Alert
            color="danger"
            title="应用历史操作失败"
            description={vm.error}
          />
        )}
        {vm.status !== "normal" && vm.status !== "disabled" && (
          <Alert
            color="info"
            title={networkStatusLabels[vm.status] ?? vm.status}
            description={vm.detail || "正在获取应用采集状态"}
          />
        )}
        {vm.data?.notice && (
          <Alert
            color="info"
            title="历史恢复提示"
            description={vm.data.notice}
          />
        )}
        {vm.data?.persistence_error && (
          <Alert
            color="warning"
            title="应用历史暂未保存"
            description={vm.data.persistence_error}
          />
        )}
        {vm.data &&
          (vm.data.incomplete ||
            vm.data.lost_windows !== "0" ||
            vm.data.clock_discontinuities !== "0") && (
            <Alert
              color="warning"
              title="部分应用历史不完整"
              description={`数据仅代表已观测部分；本次归档队列遗漏 ${vm.data.lost_windows} 个窗口，本时段有 ${vm.data.skipped_windows} 个窗口无法定位时间；累计 ${vm.data.clock_discontinuities} 个窗口因时钟回拨跳过。无效区间保留空白。`}
            />
          )}
        {vm.data?.limited && (
          <Alert
            color="info"
            title="部分应用已合并展示"
            description="超出明细上限的流量计入“其他已归属”，仍保留在应用观测合计中。"
          />
        )}
        <div className="app-history-toolbar">
          <Select
            label="应用统计范围"
            value={vm.range}
            onChange={vm.setRange}
            options={[
              { value: "today", label: "今日" },
              { value: "1h", label: "最近 1 小时" },
              { value: "6h", label: "最近 6 小时" },
              { value: "24h", label: "最近 24 小时" },
            ]}
          />
          <Input
            label="查找应用"
            placeholder="应用名称或路径"
            value={vm.search}
            onChange={(e) => vm.setSearch(e.target.value)}
          />
          <Select
            label="应用排序"
            value={vm.sort}
            onChange={(value) => vm.setSort(value as HistorySort)}
            options={[
              { value: "total", label: "合计从高到低" },
              { value: "received", label: "下载从高到低" },
              { value: "transmitted", label: "上传从高到低" },
              { value: "name", label: "应用名称" },
            ]}
          />
        </div>
        {vm.data &&
        (vm.data.observed_ms > 0 ||
          vm.data.received !== "0" ||
          vm.data.transmitted !== "0") ? (
          <p className="app-history-summary">
            已观测：下载 {bytes(vm.data.received)} · 上传{" "}
            {bytes(vm.data.transmitted)} · 有效采集覆盖{" "}
            {coverage(vm.data.covered_ms)}
          </p>
        ) : vm.data ? (
          <p className="app-history-summary">本时段尚未记录应用流量</p>
        ) : null}
        {vm.data?.loading || !vm.data ? (
          <p className="muted" role="status">
            {vm.error
              ? "暂时无法读取应用历史，稍后自动重试。"
              : "正在读取应用历史…"}
          </p>
        ) : vm.rows.length ? (
          <Table<AppHistoryRowDto>
            columns={columns}
            rows={vm.rows}
            rowKey={(row) => row.id}
          />
        ) : (
          <p className="muted">
            {vm.search
              ? "没有匹配的应用。"
              : vm.running
                ? "当前时段暂无已归档流量。开启后开始积累，每 5 秒刷新明细。"
                : "当前时段没有应用明细，点击“开始记录”后开始积累。"}
          </p>
        )}
        {vm.rows.length < vm.totalRows && (
          <Button variant="secondary" size="sm" onClick={vm.showMore}>
            显示更多（{vm.rows.length}/{vm.totalRows}）
          </Button>
        )}
        {vm.selectedId && (
          <div className="app-history-trend">
            <div className="section-heading">
              <h3>{vm.selected?.selected_name ?? "应用"} · 分钟平均网速</h3>
              <Button variant="ghost" size="sm" onClick={() => vm.select(null)}>
                收起趋势
              </Button>
            </div>
            {vm.selected?.selected_name ? (
              <>
                <div className="legend">
                  <span>
                    <i />
                    下载
                  </span>
                  <span>
                    <i className="secondary" />
                    上传
                  </span>
                </div>
                <TrendChart
                  history={vm.frames}
                  series={series}
                  range={Math.max(
                    60_000,
                    vm.selected.through_ms - vm.selected.from_ms,
                  )}
                  anchor={vm.anchor}
                  onAnchor={vm.setAnchor}
                  sampleSpacing={60_000}
                  emptyState={{
                    title: "此时段没有有效应用样本",
                    detail: "未记录和采集缺失保留空白，不补算为零",
                  }}
                  label={`${vm.selected.selected_name} 历史流量趋势`}
                />
              </>
            ) : (
              <p className="muted">
                {vm.selected
                  ? "该应用已超出保留范围或没有可用历史。"
                  : "正在读取应用趋势…"}
              </p>
            )}
          </div>
        )}
        <p className="processor-caption">
          按可执行文件路径合并多进程，同名应用可悬停查看路径。应用统计包含可观测的局域网和回环流量，代理/VPN
          可能显示为代理进程；不与上方所选网卡总量直接对账。未归属表示无法确认应用，采样时间按分钟汇总。
        </p>
        <p className="processor-caption">
          {vm.data?.saved_at_ms
            ? `已保存至：${new Date(vm.data.saved_at_ms).toLocaleString()}`
            : "等待首次保存"}{" "}
          · 每分钟及正常退出保存，未开启和未运行时段无法补录
        </p>
      </div>
    </Card>
  );
}
