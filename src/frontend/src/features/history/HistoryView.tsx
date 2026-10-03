import type { MonitorClient } from "../../shared/client/monitor-client";
import { Alert, Badge, Card, Select, StatCard } from "../../shared/ui/sakani";
import { icons } from "../../shared/ui/icons";
import { TrendChart } from "../monitoring/TrendChart";
import type { ChartSeries } from "../monitoring/chart";
import { useHistoryViewModel } from "./useHistoryViewModel";
import { bytes, coverage } from "./history-model";
import "./history.css";
import { AppHistoryView } from "./AppHistoryView";
const usage: ChartSeries[] = [
  {
    id: "cpu",
    label: "CPU",
    color: "var(--color-chart-5)",
    maxGapMs: 90_000,
    read: (f) => f.cpu,
  },
  {
    id: "memory",
    label: "内存",
    color: "var(--color-chart-2)",
    maxGapMs: 90_000,
    read: (f) => f.memory,
  },
];
const network: ChartSeries[] = [
  {
    id: "download",
    label: "下载",
    color: "var(--color-chart-5)",
    maxGapMs: 90_000,
    network: true,
    read: (f) => f.download,
  },
  {
    id: "upload",
    label: "上传",
    color: "var(--color-chart-2)",
    maxGapMs: 90_000,
    network: true,
    read: (f) => f.upload,
  },
];
export function HistoryView({ client }: { client: MonitorClient }) {
  const vm = useHistoryViewModel(client);
  const recorded = vm.data && vm.data.network_coverage_ms > 0;
  return (
    <section className="history-page" aria-label="本地历史">
      <div className="section-heading">
        <Select
          label="趋势范围"
          value={vm.range}
          options={[
            { value: "3600000", label: "最近 1 小时" },
            { value: "21600000", label: "最近 6 小时" },
            { value: "86400000", label: "最近 24 小时" },
          ]}
          onChange={(value) => {
            vm.setRange(value);
            vm.setAnchor(null);
          }}
        />
        <Badge variant="neutral">本地保存 · 分钟平均值</Badge>
      </div>
      {vm.error && (
        <Alert color="danger" title="无法读取本地历史" description={vm.error} />
      )}
      {(!vm.data || vm.data.loading) && (
        <Alert
          color="info"
          title="正在读取本地历史"
          description="首次使用会从当前采样开始积累，应用未运行的时间保留空缺。"
        />
      )}
      {vm.data?.notice && (
        <Alert color="info" title="历史恢复提示" description={vm.data.notice} />
      )}
      {vm.data?.persistence_error && (
        <Alert
          color="warning"
          title="历史暂未保存到磁盘"
          description={`${vm.data.persistence_error}。实时监控仍可使用。`}
        />
      )}
      {vm.data &&
        (vm.data.lost_samples !== "0" ||
          vm.data.clock_discontinuities !== "0") && (
          <Alert
            color="warning"
            title="部分历史不完整"
            description={`写入队列遗漏 ${vm.data.lost_samples} 个样本；时钟回拨跳过 ${vm.data.clock_discontinuities} 个样本。统计仅包含已记录的有效区间。`}
          />
        )}
      <div className="history-stats">
        <StatCard
          title="今日下载"
          value={recorded ? bytes(vm.data!.received) : "—"}
          description="运行期间所选网卡的有效收流量"
          variant="icon"
          icon={icons.download}
        />
        <StatCard
          title="今日上传"
          value={recorded ? bytes(vm.data!.transmitted) : "—"}
          description="运行期间所选网卡的有效发流量"
          variant="icon"
          icon={icons.upload}
        />
        <StatCard
          title="今日流量采集覆盖"
          value={coverage(vm.data?.network_coverage_ms ?? 0)}
          description="缺失、休眠与未运行时间不补计"
          variant="icon"
          icon={icons.history}
        />
      </div>
      <AppHistoryView client={client} />
      <Card className="trend-card">
        <div className="section-heading">
          <h2>CPU 与内存 · 分钟平均</h2>
          <div className="legend">
            <span>
              <i />
              CPU
            </span>
            <span>
              <i className="secondary" />
              内存
            </span>
          </div>
        </div>
        <TrendChart
          history={vm.frames}
          series={usage}
          range={Number(vm.range)}
          anchor={vm.anchor}
          onAnchor={vm.setAnchor}
          sampleSpacing={60_000}
          label="CPU 与内存分钟历史"
        />
      </Card>
      <Card className="trend-card">
        <div className="section-heading">
          <h2>网速 · 分钟平均</h2>
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
        </div>
        <TrendChart
          history={vm.frames}
          series={network}
          range={Number(vm.range)}
          anchor={vm.anchor}
          onAnchor={vm.setAnchor}
          sampleSpacing={60_000}
          label="下载与上传分钟历史"
        />
      </Card>
      <div className="history-notes muted">
        <p>
          今日统计从本地零点开始，仅包含 Pinmeter
          运行期间所选网卡的有效区间；切换网卡后合计各次所选网卡流量，不代表全机全天账单。跨零点采样按时间比例分配字节。
        </p>
        <p>
          今日记录的网卡：{vm.data?.networks.join("、") || "暂无有效流量"}
          。每分钟保存一次，正常退出时再次保存；强制退出可能丢失尚未保存的一分钟。
        </p>
        <p>
          {vm.data?.saved_at_ms
            ? `最近保存：${new Date(vm.data.saved_at_ms).toLocaleString()}`
            : "等待首次保存"}{" "}
          · 保留最近 24 小时 · 全部存储在本机
        </p>
      </div>
    </section>
  );
}
