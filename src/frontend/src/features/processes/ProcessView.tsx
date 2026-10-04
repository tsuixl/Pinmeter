import type { TableColumn } from "@sakaniui/react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import {
  Alert,
  Badge,
  Button,
  Card,
  Input,
  SegmentedControl,
  Table,
} from "../../shared/ui/sakani";
import { statusLabels } from "../monitoring/useMonitorViewModel";
import { icons } from "../../shared/ui/icons";
import { useProcessViewModel } from "./useProcessViewModel";
import type { ProcessDisplayRow } from "./process-model";
import "./processes.css";

export function ProcessView({
  client,
  initialSort = "cpu",
}: {
  client: MonitorClient;
  initialSort?: string;
}) {
  const vm = useProcessViewModel(client, initialSort);
  const columns: TableColumn<ProcessDisplayRow>[] = [
    {
      key: "name",
      header: vm.mode === "applications" ? "应用 / 进程" : "进程",
      render: (row) => (
        <div
          className={`process-identity${row.pid === null ? " process-group" : ""}${row.child ? " process-child" : ""}`}
        >
          {row.pid === null && (
            <Button
              variant="ghost"
              size="sm"
              aria-expanded={vm.expanded.has(row.id) || !!vm.search.trim()}
              aria-label={`${vm.expanded.has(row.id) ? "收起" : "展开"} ${row.name} 的进程`}
              onClick={() => vm.toggle(row.id)}
              disabled={!!vm.search.trim()}
            >
              {vm.expanded.has(row.id) || vm.search.trim() ? (
                <icons.chevronDown size={16} />
              ) : (
                <icons.chevronRight size={16} />
              )}
            </Button>
          )}
          <span className="process-name" title={row.name}>
            {row.name}
          </span>
          {row.pid === null && (
            <small className="processor-caption">
              {row.process_count} 个进程 · ID {row.id}
            </small>
          )}
        </div>
      ),
    },
    { key: "pid", header: "PID", render: (row) => row.pid ?? "—" },
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
    {
      key: "id",
      header: "操作",
      render: (row) => (
        <div className="process-actions">
          {!row.child && (
            <Button
              variant={vm.pinned?.id === row.id ? "secondary" : "ghost"}
              size="sm"
              aria-pressed={vm.pinned?.id === row.id}
              onClick={() => vm.pin(row)}
            >
              {vm.pinned?.id === row.id ? "已固定" : "固定"}
            </Button>
          )}
          <Button
            variant="ghost"
            size="sm"
            aria-label={`复制 ${row.name} 的名称`}
            onClick={() => void vm.copy(row.name)}
          >
            复制名称
          </Button>
          {row.pid !== null && (
            <Button
              variant="ghost"
              size="sm"
              aria-label={`复制 PID ${row.pid}`}
              onClick={() => void vm.copy(String(row.pid))}
            >
              复制 PID
            </Button>
          )}
        </div>
      ),
    },
  ];
  return (
    <section className="process-page" aria-label="进程排行">
      <div className="section-heading">
        <h2>资源占用</h2>
        <div aria-label="排行指标">
          <SegmentedControl
            value={vm.sort}
            onChange={vm.setSort}
            options={[
              { value: "cpu", label: "CPU" },
              { value: "memory", label: "内存" },
            ]}
          />
        </div>
        <Button variant="secondary" onClick={() => vm.setPaused(!vm.paused)}>
          {vm.paused ? "恢复刷新" : "暂停刷新"}
        </Button>
      </div>
      <div className="process-toolbar">
        <div aria-label="查看方式">
          <SegmentedControl
            value={vm.mode}
            onChange={vm.setMode}
            options={[
              { value: "applications", label: "按应用" },
              { value: "processes", label: "按进程" },
            ]}
          />
        </div>
        <Input
          label="查找应用或进程"
          placeholder="名称或 PID"
          value={vm.search}
          onChange={(e) => vm.setSearch(e.target.value)}
        />
      </div>
      <div className="process-meta">
        <Badge variant="neutral">本次采集 {vm.data?.total ?? "—"} 个进程</Badge>
        <Badge variant={vm.paused ? "warning" : "neutral"}>
          {vm.paused ? "已暂停 · 显示冻结快照" : "每 2 秒刷新 · 只读"}
        </Badge>
        {vm.data?.sampled_at_ms && (
          <span className="muted">
            采样于 {new Date(vm.data.sampled_at_ms).toLocaleTimeString()}
          </span>
        )}
      </div>
      {vm.paused && (
        <Alert
          color="info"
          title="已暂停刷新"
          description="显示暂停时的快照，不代表当前读数；五秒内停止本页采集，恢复后重新取得当前数据。"
        />
      )}
      {vm.pinned && (
        <div className="process-meta">
          <Button variant="secondary" size="sm" onClick={vm.clearPin}>
            取消固定 {vm.pinned.name}
          </Button>
          {vm.pinnedMissing && (
            <span className="muted">
              目标未出现在当前筛选中，可能已退出或身份已变化。
            </span>
          )}
        </div>
      )}
      {vm.copyMessage && (
        <p className="muted" role="status">
          {vm.copyMessage}
        </p>
      )}
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
          description="系统保护、权限或进程退出会影响读取。应用有任一成员指标无效时，汇总指标也不标为有效，不记为零。"
        />
      )}
      {vm.data?.truncated && (
        <Alert
          color="warning"
          title="进程列表已截断"
          description="仅采集前 4096 个进程，搜索及排行可能不完整。"
        />
      )}
      <p className="processor-caption">
        搜索覆盖本次已采集进程；当前匹配 {vm.total} 个
        {vm.mode === "applications" ? "应用" : "进程"}。默认显示前 10
        项，固定项优先，展开项显示其进程。
      </p>
      {vm.rows.length ? (
        <Table<ProcessDisplayRow>
          columns={columns}
          rows={vm.rows}
          rowKey={(row) => row.id}
        />
      ) : (
        <Card>
          <p className="muted">
            {vm.search
              ? "本次采集中没有匹配项。"
              : "尚未取得进程列表。CPU 需要两次采样，内存可在首次采样后查看。"}
          </p>
        </Card>
      )}
      {vm.hasMore && (
        <Button variant="secondary" onClick={vm.showMore}>
          显示更多
        </Button>
      )}
      <p className="muted">
        CPU
        已按全机逻辑处理器数归一化。仅同一可执行文件的进程合并，同名不同来源保持独立；身份未知时单独显示。工作集合计包含重复共享页，不等于系统已用内存。本页仅查看数据，切页或隐藏后停止页面采集。
      </p>
    </section>
  );
}
