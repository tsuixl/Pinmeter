import type {
  ArchiveSnapshotDto,
  PeriodSummaryDto,
} from "../../shared/contracts/monitor";
import { Alert, Card, Table } from "../../shared/ui/sakani";
import { bytes, coverage, historyPeaks } from "./history-model";

export function HistoryInsights({ data }: { data: ArchiveSnapshotDto }) {
  const peaks = historyPeaks(data.buckets);
  const period = (value: PeriodSummaryDto | null, label: string) => ({
    id: label,
    label,
    cpu:
      value?.cpu_average === null || !value
        ? "—"
        : `${value.cpu_average.toFixed(1)}%`,
    memory:
      value?.memory_average === null || !value
        ? "—"
        : `${value.memory_average.toFixed(1)}%`,
    traffic: value?.network_coverage_ms
      ? `${bytes(value.received)} / ${bytes(value.transmitted)}`
      : "—",
    covered: value
      ? `CPU ${coverage(value.cpu_coverage_ms)} · 内存 ${coverage(value.memory_coverage_ms)} · 网络 ${coverage(value.network_coverage_ms)}`
      : "超出30天保留范围",
  });
  const periods = [
    period(data.current_period, "所选周期"),
    period(data.previous_period, "前一同长周期"),
  ];
  return (
    <>
      <Card
        title="采样峰值"
        description="所选范围内的有效均值与最高已记录样本；采样间隔内的瞬时峰值可能未被捕获。"
      >
        <div className="history-table-scroll">
          <Table
            rows={peaks}
            rowKey={(row) => row.id}
            columns={[
              { key: "label", header: "指标" },
              { key: "average", header: "有效均值" },
              { key: "peak", header: "采样峰值" },
              { key: "occurred", header: "发生时间" },
              { key: "covered", header: "有效覆盖" },
            ]}
          />
        </div>
        <p className="settings-note">
          旧网速平均值不能还原峰值；旧 CPU /
          内存峰值的发生时间可能未知。峰值只解释指标变化，不代表已经知道当时的占用进程。
        </p>
      </Card>
      <Card
        title="与前一周期比较"
        description={`使用完整时间桶：${new Date(data.current_period.from_ms).toLocaleString()} — ${new Date(data.current_period.through_ms).toLocaleString()}`}
      >
        <div className="history-table-scroll">
          <Table
            rows={periods}
            rowKey={(row) => row.id}
            columns={[
              { key: "label", header: "周期" },
              { key: "cpu", header: "CPU均值" },
              { key: "memory", header: "内存均值" },
              { key: "traffic", header: "下载 / 上传" },
              { key: "covered", header: "有效覆盖" },
            ]}
          />
        </div>
        {!data.previous_period && (
          <Alert
            color="info"
            title="无法比较完整的前30天"
            description="本地最多保留30天，不扩展采集或推算缺失的前一周期。"
          />
        )}
        <p className="settings-note">
          两期覆盖时长可能不同，流量差额不等于完整周期的增减；未运行和旧版已过期的数据无法补回。
        </p>
      </Card>
    </>
  );
}
