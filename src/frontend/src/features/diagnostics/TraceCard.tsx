import { useCallback, useRef, useState } from "react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import { usePageQuery } from "../monitoring/usePageQuery";
import { Alert, Button, Card, Modal, Select } from "../../shared/ui/sakani";

function useTraceViewModel(client: MonitorClient) {
  const [revision, setRevision] = useState(0);
  const load = useCallback(() => {
    void revision;
    if (!client.getTraceSnapshot)
      return Promise.reject(new Error("此运行环境尚不支持排障记录"));
    return client.getTraceSnapshot();
  }, [client, revision]);
  const query = usePageQuery(client, load);
  const busy = useRef(false);
  const [pending, setPending] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  const perform = async (operation: () => Promise<unknown>) => {
    if (busy.current) return;
    busy.current = true;
    setPending(true);
    setError("");
    setMessage("");
    try {
      await operation();
      setRevision((v) => v + 1);
    } catch (error) {
      setError(String(error));
    } finally {
      busy.current = false;
      setPending(false);
    }
  };
  return {
    ...query,
    pending,
    message,
    error: error || query.error,
    start: (seconds: number) =>
      perform(async () => {
        if (!client.startTrace) throw new Error("排障记录不可用");
        await client.startTrace(seconds);
      }),
    stop: () =>
      perform(async () => {
        if (!client.stopTrace) throw new Error("排障记录不可用");
        await client.stopTrace();
      }),
    save: () =>
      perform(async () => {
        if (!client.exportTrace || !query.data) throw new Error("请重新预览");
        const result = await client.exportTrace(query.data.token);
        setMessage(result.saved ? `已保存：${result.path}` : "未导出文件");
      }),
  };
}
export function TraceCard({ client }: { client: MonitorClient }) {
  const vm = useTraceViewModel(client);
  const [seconds, setSeconds] = useState("60");
  const [preview, setPreview] = useState(false);
  return (
    <>
      <Card
        title="限时排障记录"
        description="出现卡顿时主动开始，记录结束后再决定是否导出"
      >
        <p className="settings-note">
          记录 CPU、内存、网速与当时 CPU 占用前 3 个进程的名称和
          PID，不含完整路径或
          IP。开始前仅保留已有基础趋势，无法追溯当时的进程。记录期间即使收起窗口，进程采样也会持续到期限结束；这会增加采集开销。
        </p>
        <Select
          id="trace-duration"
          label="记录时长"
          value={seconds}
          onChange={setSeconds}
          disabled={vm.pending || vm.data?.active}
          options={[
            { value: "30", label: "30 秒" },
            { value: "60", label: "1 分钟" },
            { value: "120", label: "2 分钟" },
          ]}
        />
        <div className="settings-actions">
          {vm.data?.active ? (
            <Button
              variant="outline"
              disabled={vm.pending}
              onClick={() => void vm.stop()}
            >
              停止记录（剩余 {vm.data.remaining_seconds} 秒）
            </Button>
          ) : (
            <Button
              disabled={vm.pending || !client.startTrace}
              onClick={() => void vm.start(Number(seconds))}
            >
              开始记录
            </Button>
          )}
          <Button
            variant="outline"
            disabled={vm.pending || vm.data?.active || !vm.data?.content}
            onClick={() => setPreview(true)}
          >
            预览排障记录
          </Button>
          <span role="status">
            {vm.data ? `已记录 ${vm.data.samples} 个观测点` : "等待排障状态"}
          </span>
        </div>
        {vm.error && (
          <Alert color="danger" title="操作未完成" description={vm.error} />
        )}
        {vm.message && <p role="status">{vm.message}</p>}
      </Card>
      <Modal
        open={preview}
        title="排障记录预览"
        description="此文件包含应用名称和 PID。确认后保存到本机下载目录，不会上传。"
        className="diagnostic-modal"
        confirmLabel="导出记录"
        cancelLabel="关闭"
        confirmLoading={vm.pending}
        onClose={() => {
          if (!vm.pending) setPreview(false);
        }}
        onConfirm={() => void vm.save()}
      >
        <pre className="diagnostic-preview" tabIndex={0}>
          {vm.data?.content}
        </pre>
        {vm.error && (
          <Alert color="danger" title="导出未完成" description={vm.error} />
        )}
        {vm.message && <p role="status">{vm.message}</p>}
      </Modal>
    </>
  );
}
