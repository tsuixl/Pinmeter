import { useRef, useState } from "react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import type { DiagnosticPreviewDto } from "../../shared/contracts/monitor";

export function useDiagnosticsViewModel(client: MonitorClient) {
  const [open, setOpen] = useState(false);
  const [preview, setPreview] = useState<DiagnosticPreviewDto | null>(null);
  const [pending, setPending] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState("");
  const [message, setMessage] = useState("");
  const generation = useRef(0);
  const exporting = useRef(false);
  const prepare = async () => {
    if (pending || exporting.current) return;
    const request = ++generation.current;
    setOpen(true);
    setPreview(null);
    setError("");
    setMessage("");
    setPending(true);
    try {
      if (!client.prepareDiagnostics) throw new Error("当前无法获取诊断");
      const value = await client.prepareDiagnostics();
      if (request === generation.current) setPreview(value);
    } catch (reason) {
      if (request === generation.current) setError(String(reason));
    } finally {
      if (request === generation.current) setPending(false);
    }
  };
  const close = () => {
    if (exporting.current) return;
    generation.current++;
    setOpen(false);
    setPending(false);
  };
  const save = async () => {
    if (!preview || exporting.current || !client.exportDiagnostics) return;
    exporting.current = true;
    setSaving(true);
    setError("");
    try {
      const result = await client.exportDiagnostics(preview.token);
      setMessage(
        result.saved ? "已导出到：" + result.path : "演示模式未写入文件",
      );
    } catch (reason) {
      setError(String(reason));
    } finally {
      exporting.current = false;
      setSaving(false);
    }
  };
  return {
    open,
    preview,
    pending,
    saving,
    error,
    message,
    prepare,
    close,
    save,
  };
}
