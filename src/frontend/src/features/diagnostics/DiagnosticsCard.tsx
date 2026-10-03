import type { MonitorClient } from "../../shared/client/monitor-client";
import { Alert, Button, Card, Modal } from "../../shared/ui/sakani";
import { useDiagnosticsViewModel } from "./useDiagnosticsViewModel";
import "./diagnostics.css";

export function DiagnosticsCard({ client }: { client: MonitorClient }) {
  const vm = useDiagnosticsViewModel(client);
  return (
    <>
      <Card title="诊断与反馈" description="生成本地诊断快照，方便定位问题">
        <p className="settings-note">
          包含版本、系统、采集状态和配置摘要。不包含
          IP、用户名、完整路径、进程名单或日志原文，也不会自动上传。
        </p>
        <Button
          size="sm"
          variant="outline"
          loading={vm.pending}
          onClick={() => void vm.prepare()}
        >
          预览诊断
        </Button>
      </Card>
      <Modal
        open={vm.open}
        title="诊断快照"
        description="确认内容后，可导出到系统下载文件夹。"
        className="diagnostic-modal"
        confirmLabel={vm.preview ? "导出诊断" : "重新生成"}
        cancelLabel="关闭"
        confirmLoading={vm.pending || vm.saving}
        closeOnBackdropClick={!vm.saving}
        closeOnEscape={!vm.saving}
        onClose={vm.close}
        onConfirm={() => void (vm.preview ? vm.save() : vm.prepare())}
      >
        {vm.pending && <p role="status">正在读取当前诊断状态…</p>}
        {vm.preview && (
          <pre
            className="diagnostic-preview"
            tabIndex={0}
            aria-label="诊断内容"
          >
            {vm.preview.content}
          </pre>
        )}
        {vm.error && (
          <Alert color="danger" title="操作未完成" description={vm.error} />
        )}
        {vm.message && (
          <p className="diagnostic-result" role="status">
            {vm.message}
          </p>
        )}
      </Modal>
    </>
  );
}
