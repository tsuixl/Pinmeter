import type { TableColumn } from "@sakaniui/react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import type { ProcessRowDto } from "../../shared/contracts/monitor";
import {
  Alert,
  Badge,
  Card,
  SegmentedControl,
  Table,
} from "../../shared/ui/sakani";
import { statusLabels } from "../monitoring/useMonitorViewModel";
import { useProcessViewModel } from "./useProcessViewModel";
import "./processes.css";
const columns: TableColumn<ProcessRowDto>[] = [
  {
    key: "name",
    header: "进程",
    render: (row) => (
      <span className="process-name" title={row.name}>
        {row.name}
      </span>
    ),
  },
  { key: "pid", header: "PID", render: (row) => row.pid },
  {
    key: "cpu",
    header: "CPU",
    render: (row) =>
      row.cpu === null
        ? statusLabels[row.cpu_status]
        : `${row.cpu.toFixed(1)}%`,
  },
  {
    key: "working_set",
    header: "内存工作集",
    render: (row) =>
      row.working_set === null
        ? statusLabels[row.memory_status]
        : `${(row.working_set / 1048576).toFixed(1)} MiB`,
  },
];
export function ProcessView({ client }: { client: MonitorClient }) {
  const vm = useProcessViewModel(client);
  return (
    <section className="process-page" aria-label="进程排行">
      <div className="section-heading">
        <h2>资源占用 Top 10</h2>
        <SegmentedControl
          value={vm.sort}
          onChange={vm.setSort}
          options={[
            { value: "cpu", label: "CPU" },
            { value: "memory", label: "内存" },
          ]}
        />
      </div>
      <div className="process-meta">
        <Badge variant="neutral">{vm.data?.total ?? "—"} 个进程</Badge>
        <Badge variant="neutral">每 2 秒刷新 · 只读</Badge>
        {vm.data?.sampled_at_ms && (
          <span className="muted">
            更新于 {new Date(vm.data.sampled_at_ms).toLocaleTimeString()}
          </span>
        )}
      </div>
      {vm.error ? (
        <Alert color="danger" title="进程查询失败" description={vm.error} />
      ) : !vm.data || vm.data.status !== "normal" ? (
        <Alert
          color="info"
          title={statusLabels[vm.data?.status ?? "warming"]}
          description={vm.data?.detail ?? "正在建立进程 CPU 基线"}
        />
      ) : null}
      {!!vm.data?.unreadable && (
        <Alert
          color="info"
          title={`${vm.data.unreadable} 个进程的部分指标无法读取`}
          description="系统保护、权限或进程退出会影响读取。无法读取的指标不参与对应排行，也不记为零。"
        />
      )}
      {vm.data?.truncated && (
        <Alert
          color="warning"
          title="进程列表已截断"
          description="仅采集前 4096 个进程，当前排行可能不完整。"
        />
      )}
      {vm.rows.length ? (
        <Table<ProcessRowDto>
          columns={columns}
          rows={vm.rows}
          rowKey={(row) => row.id}
        />
      ) : (
        <Card>
          <p className="muted">
            暂无可排行的有效读数。CPU 需要两次采样，内存可在首次采样后查看。
          </p>
        </Card>
      )}
      <p className="muted">
        CPU
        已按全机逻辑处理器数归一化。内存工作集包含共享页，不能将进程工作集相加视为系统已用内存。本页仅查看数据，切页或隐藏窗口后停止采集。
      </p>
    </section>
  );
}
