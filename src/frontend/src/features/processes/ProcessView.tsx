import type { TableColumn } from "@sakaniui/react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import {
  Badge,
  Button,
  Input,
  IconButton,
  SegmentedControl,
} from "../../shared/ui/sakani";
import { statusLabels } from "../monitoring/useMonitorViewModel";
import { icons } from "../../shared/ui/icons";
import { InfoPopover } from "../../shared/ui/InfoPopover";
import { useProcessViewModel } from "./useProcessViewModel";
import type { ProcessDisplayRow } from "./process-model";
import { ProcessTable } from "./ProcessTable";
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
            <IconButton
              variant="ghost"
              size="sm"
              icon={
                vm.expanded.has(row.id) || vm.search.trim()
                  ? icons.chevronDown
                  : icons.chevronRight
              }
              aria-expanded={vm.expanded.has(row.id) || !!vm.search.trim()}
              aria-label={`${vm.expanded.has(row.id) || vm.search.trim() ? "收起" : "展开"} ${row.name} 的进程`}
              onClick={() => vm.toggle(row.id)}
              disabled={!!vm.search.trim()}
            />
          )}
          <span className="process-name" title={row.name}>
            {row.name}
          </span>
          {row.pid === null && (
            <small className="processor-caption">
              {row.process_count} 个进程
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
        <div className="info-heading">
          <h2>资源占用</h2>
          <InfoPopover title="资源占用说明">
            {vm.error && <p>查询失败：{vm.error}</p>}
            {vm.data?.detail && <p>采集状态：{vm.data.detail}</p>}
            {!vm.data && !vm.error && (
              <p>正在建立进程 CPU 基线，等待采集返回。</p>
            )}
            {vm.paused && (
              <p>
                当前已暂停，显示暂停时的快照，不代表当前读数；五秒内停止本页采集，恢复后重新取得当前数据。
              </p>
            )}
            {!!vm.data?.unreadable && (
              <p>
                {vm.data.unreadable}{" "}
                个进程的部分指标无法读取。系统保护、权限或进程退出会影响读取；应用任一成员指标无效时，汇总指标也不标为有效，不记为零。
              </p>
            )}
            {vm.data?.truncated && (
              <p>
                进程列表已截断：仅采集前 4096 个进程，搜索及排行可能不完整。
              </p>
            )}
            {vm.pinnedMissing && (
              <p>固定目标未出现在当前筛选中，可能已退出或身份已变化。</p>
            )}
            <p>
              CPU
              需要两次采样建立基线，已按全机逻辑处理器数归一化；内存可在首次采样后查看。
            </p>
            <p>
              仅同一可执行文件的进程合并，同名不同来源保持独立；身份未知时单独显示。工作集合计包含重复共享页，不等于系统已用内存。
            </p>
            <p>
              搜索覆盖本次已采集进程，默认显示前 10
              项，固定项优先。展开应用可查看子进程；搜索时自动展开匹配项。
            </p>
            <p>
              点击表头切换升降排序，拖动标题换列，拖动列边界调宽。标题支持 Alt +
              左右方向键移动；列边界支持左右方向键微调、Shift 加大步进、Home
              恢复最小宽度。“恢复默认列”只重置列顺序和宽度。
            </p>
            <p>
              应用汇总没有 PID，按 PID
              排序时父应用顺序保持稳定，只调整各组子进程。无效值始终置后，固定项优先。
            </p>
            <p>
              本页只读，每 2
              秒刷新。暂停冻结当前快照；切页、隐藏或暂停后五秒内停止本页采集，恢复时重新取得当前数据。
            </p>
          </InfoPopover>
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
          {vm.paused ? "已暂停" : "每 2 秒刷新 · 只读"}
        </Badge>
        {vm.error ? (
          <span role="alert">
            <Badge variant="danger">查询失败</Badge>
          </span>
        ) : (
          (!vm.data || vm.data.status !== "normal") && (
            <Badge
              variant={
                vm.data?.status === "failed"
                  ? "danger"
                  : vm.data?.status === "warming" || !vm.data
                    ? "neutral"
                    : "warning"
              }
            >
              {statusLabels[vm.data?.status ?? "warming"]}
            </Badge>
          )
        )}
        {!!vm.data?.unreadable && (
          <Badge variant="warning">读取不全 {vm.data.unreadable} 个</Badge>
        )}
        {vm.data?.truncated && <Badge variant="warning">列表已截断</Badge>}
        <span className="muted">
          匹配 {vm.total} 个{vm.mode === "applications" ? "应用" : "进程"}
        </span>
        {vm.data?.sampled_at_ms && (
          <span className="muted">
            采样于 {new Date(vm.data.sampled_at_ms).toLocaleTimeString()}
          </span>
        )}
      </div>
      {vm.pinned && (
        <div className="process-meta">
          <Button variant="secondary" size="sm" onClick={vm.clearPin}>
            取消固定 {vm.pinned.name}
          </Button>
          {vm.pinnedMissing && <Badge variant="warning">固定目标未找到</Badge>}
        </div>
      )}
      {vm.copyMessage && (
        <p className="muted" role="status">
          {vm.copyMessage}
        </p>
      )}
      <ProcessTable
        columns={columns}
        rows={vm.rows}
        sort={vm.sort}
        direction={vm.sortDirection}
        onSort={vm.sortBy}
      />
      {!vm.rows.length && (
        <p className="muted">
          {vm.search ? "没有匹配的应用或进程。" : "尚未取得进程列表。"}
        </p>
      )}
      {vm.hasMore && (
        <Button variant="secondary" onClick={vm.showMore}>
          显示更多
        </Button>
      )}
    </section>
  );
}
