import type { MonitorClient } from "../../shared/client/monitor-client";
import {
  Alert,
  Button,
  Card,
  Checkbox,
  Modal,
  Select,
} from "../../shared/ui/sakani";
import { bytes } from "./history-model";
import {
  type HistoryScope,
  useHistoryDataViewModel,
} from "./useHistoryDataViewModel";

export function HistoryDataCard({
  client,
  from,
  through,
  onChanged,
  mode = "all",
}: {
  client: MonitorClient;
  from: number;
  through: number;
  onChanged: () => void;
  mode?: "all" | "manage" | "export";
}) {
  const vm = useHistoryDataViewModel(client, from, through, onChanged);
  const name = (scope: HistoryScope) =>
    scope === "basic" ? "基础指标历史" : "应用流量历史";
  return (
    <>
      <Card
        title={mode === "export" ? "导出历史" : "本地存储"}
        description={
          mode === "export"
            ? "按当前所选时段导出已有记录"
            : "历史只存储在本机，不自动上传。仅应用流量记录需要主动开启。"
        }
      >
        <div className="history-data-content">
          {mode !== "export" && (
            <>
              <p>
                基础历史保存 CPU /
                内存有效均值和峰值、所选网卡收发字节及覆盖时长；应用历史保存应用名称、完整可执行路径、收发字节及覆盖，不保存网络内容、IP、逐
                PID 事件或图标。
              </p>
              <p>
                保留最近24小时的分钟明细、7天的15分钟汇总、30天的小时汇总；旧版已丢弃的时段无法恢复。基础文件上限{" "}
                {vm.data ? bytes(vm.data.basic_limit_bytes) : "读取中"}
                ，应用文件上限{" "}
                {bytes(vm.data?.applications_limit_bytes ?? "33554432")}。
              </p>
              <p>
                当前文件与损坏备份占用：基础{" "}
                {vm.data?.basic_bytes === null || !vm.data
                  ? "未知"
                  : bytes(vm.data.basic_bytes)}{" "}
                · 应用{" "}
                {vm.data?.applications_bytes === null || !vm.data
                  ? "未知"
                  : bytes(vm.data.applications_bytes)}
                。占用按已落盘文件计，不包含尚未保存的内存样本和用户导出文件。
              </p>
              {vm.data?.detail && (
                <p className="settings-note">{vm.data.detail}</p>
              )}
            </>
          )}
          <div className="history-data-actions">
            <Select
              label={mode === "export" ? "导出的数据" : "管理的数据"}
              value={vm.scope}
              options={[
                { value: "basic", label: "基础指标历史" },
                { value: "applications", label: "应用流量历史" },
              ]}
              onChange={(value) => vm.setScope(value as HistoryScope)}
            />
            {mode !== "manage" && (
              <Button
                variant="outline"
                disabled={!vm.canExport || vm.pending}
                onClick={() => vm.prepare("export")}
              >
                导出所选时段 CSV
              </Button>
            )}
            {mode !== "export" && (
              <Button
                variant="outline"
                disabled={!vm.canClear || vm.pending}
                onClick={() => vm.prepare("clear")}
              >
                清除所选历史
              </Button>
            )}
            {mode !== "export" && vm.running && (
              <Button
                variant="outline"
                disabled={vm.pending}
                onClick={() => void vm.stopRecording()}
              >
                停止应用流量记录
              </Button>
            )}
          </div>
          {mode !== "manage" && vm.scope === "applications" && (
            <Checkbox
              label="导出中包含应用完整路径"
              checked={vm.includePaths}
              onChange={(event) => vm.setIncludePaths(event.target.checked)}
            />
          )}
          {mode !== "manage" && (
            <p className="settings-note">
              导出范围使用页面顶部趋势选择：{new Date(from).toLocaleString()} —{" "}
              {new Date(through).toLocaleString()}
              。CSV保存到系统下载文件夹，时间为Unix毫秒，字节为完整整数；缺失保持空白。完整路径可能包含用户名等私人信息。
            </p>
          )}
          {vm.error && !vm.confirmation && (
            <Alert
              color="danger"
              title="历史管理未完成"
              description={vm.error}
            />
          )}
          {vm.message && (
            <p role="status" className="history-data-result">
              {vm.message}
            </p>
          )}
        </div>
      </Card>
      <Modal
        open={!!vm.confirmation}
        variant={
          vm.confirmation?.action === "clear" ? "destructive" : "default"
        }
        title={
          vm.confirmation?.action === "clear"
            ? `清除${name(vm.confirmation.scope)}`
            : "确认历史导出"
        }
        confirmLabel={
          vm.confirmation?.action === "clear" ? "确认清除" : "导出 CSV"
        }
        cancelLabel="取消"
        confirmLoading={vm.pending}
        closeOnBackdropClick={!vm.pending}
        closeOnEscape={!vm.pending}
        onClose={vm.close}
        onConfirm={() => void vm.confirm()}
      >
        {vm.confirmation?.action === "clear" ? (
          <p>
            删除所选历史文件中的全部30天记录及其损坏备份，无法撤销。其他类型历史、用户导出的CSV及网络控制规则保持不变。继续采集会产生新数据；若希望停止应用流量记录，请先使用“停止应用流量记录”。
          </p>
        ) : (
          <p>
            {vm.confirmation &&
              `${name(vm.confirmation.scope)}：${new Date(vm.confirmation.from).toLocaleString()} — ${new Date(vm.confirmation.through).toLocaleString()}。${vm.confirmation.includePaths ? "包含应用完整路径，可能暴露用户名。" : "不包含应用完整路径。"}`}
            仅导出已有时间桶，不补齐未记录时段；文件留在本机，不自动分享。
          </p>
        )}
        {vm.error && (
          <Alert color="danger" title="操作未完成" description={vm.error} />
        )}
      </Modal>
    </>
  );
}
