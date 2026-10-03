import { useState } from "react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import { Alert, Badge, Card, Select, StatCard } from "../../shared/ui/sakani";
import { icons } from "../../shared/ui/icons";
import { statusLabels } from "../monitoring/useMonitorViewModel";
import { TrendChart } from "../monitoring/TrendChart";
import { useDiskViewModel } from "./useDiskViewModel";
import "./disk.css";

export function DiskView({ client }: { client: MonitorClient }) {
  const vm = useDiskViewModel(client);
  const [anchor, setAnchor] = useState<number | null>(null);
  return (
    <section className="disk-page" aria-label="磁盘监控">
      <div className="section-heading">
        <Select
          label="物理磁盘"
          value={vm.selectedId}
          options={vm.options}
          onChange={(id) => {
            vm.setSelectedId(id);
            setAnchor(null);
          }}
          disabled={!vm.options.length}
        />
        <Badge variant="neutral">2 秒采样 · 最近 5 分钟</Badge>
      </div>
      {vm.error ? (
        <Alert color="danger" title="磁盘查询失败" description={vm.error} />
      ) : !vm.data || vm.data.status !== "normal" ? (
        <Alert
          color="info"
          title={statusLabels[vm.data?.status ?? "warming"]}
          description={vm.data?.detail ?? "正在建立磁盘采样基线"}
        />
      ) : null}
      {vm.data?.status === "normal" && !vm.current && (
        <Alert
          color="info"
          title="所选磁盘当前不可用"
          description="设备可能已移除。请选择其他磁盘，或等待设备重新连接。"
        />
      )}
      <div className="disk-stats">
        {(
          [
            ["read", "读取速度", icons.download],
            ["write", "写入速度", icons.upload],
            ["activity", "活动时间", icons.gauge],
          ] as const
        ).map(([key, title, icon]) => {
          const r = vm.current?.[key];
          return (
            <StatCard
              key={key}
              title={title}
              value={r?.status === "normal" ? `${r.text} ${r.unit}` : "—"}
              description={
                r?.status === "normal"
                  ? key === "activity"
                    ? "磁盘非空闲时间占比"
                    : "物理磁盘每秒字节量"
                  : r?.detail || statusLabels[r?.status ?? "warming"]
              }
              variant="icon"
              icon={icon}
            />
          );
        })}
      </div>
      <Card className="trend-card">
        <div className="section-heading">
          <h2>磁盘读写趋势</h2>
          <div className="legend">
            <span>
              <i />
              读取
            </span>
            <span>
              <i className="secondary" />
              写入
            </span>
          </div>
        </div>
        <TrendChart
          history={vm.history}
          keys={["download", "upload"]}
          seriesLabels={{ download: "读取", upload: "写入" }}
          range={300_000}
          anchor={anchor}
          onAnchor={setAnchor}
          label="磁盘读取与写入速度"
        />
      </Card>
      <Card className="trend-card">
        <h2>活动时间</h2>
        <TrendChart
          history={vm.history}
          keys={["cpu"]}
          seriesLabels={{ cpu: "活动时间" }}
          range={300_000}
          anchor={anchor}
          onAnchor={setAnchor}
          label="磁盘活动时间"
        />
      </Card>
      <p className="muted">
        统计物理磁盘读写，不等同卷容量或某个进程的
        I/O。仅查看本页时采集；切页或隐藏窗口后暂停，间断保留空缺。
      </p>
    </section>
  );
}
