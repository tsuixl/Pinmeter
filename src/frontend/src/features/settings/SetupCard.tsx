import { useRef, useState } from "react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import type { MonitorStateDto } from "../../shared/contracts/monitor";
import { Alert, Button, Card } from "../../shared/ui/sakani";
import type { SettingsSection } from "./sections";

function useSetupViewModel(client: MonitorClient) {
  const busy = useRef(false);
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const complete = async () => {
    const settings = client.getSnapshot().state?.settings;
    if (!settings || busy.current) return;
    busy.current = true;
    setPending(true);
    setError("");
    try {
      await client.updateSettings({ ...settings, onboarding_completed: true });
    } catch (error) {
      setError(String(error));
    } finally {
      busy.current = false;
      setPending(false);
    }
  };
  return { complete, pending, error };
}

export function SetupCard({
  client,
  state,
  onSettings,
  onHistory,
}: {
  client: MonitorClient;
  state: MonitorStateDto | null;
  onSettings: (section: SettingsSection) => void;
  onHistory: () => void;
}) {
  const vm = useSetupViewModel(client);
  if (!state || state.settings.onboarding_completed) return null;
  return (
    <Card
      title="让 Pinmeter 按你的习惯常驻"
      description="以下功能按需开启，也可以稍后在设置中调整。"
    >
      <p className="settings-note">
        Windows
        启动时统一请求管理员授权。基础读数与温度等能力分别显示状态；缺少传感器驱动只影响相应指标，可到
        CPU 页查看原因。
      </p>
      <p className="settings-note">
        最小化后收起到托盘，点击托盘图标打开快捷面板，再从面板进入总览；再次打开
        Pinmeter
        也能恢复主窗口。应用流量记录默认关闭，开启后会将应用身份与流量保存在本机。
      </p>
      <div className="settings-actions">
        <Button
          variant="outline"
          size="sm"
          onClick={() => onSettings("taskbar")}
        >
          选择任务栏指标
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={() => onSettings("resident")}
        >
          设置启动与关闭方式
        </Button>
        <Button variant="outline" size="sm" onClick={onHistory}>
          了解流量记录
        </Button>
        <Button
          variant="ghost"
          size="sm"
          disabled={vm.pending}
          onClick={() => void vm.complete()}
        >
          暂不设置
        </Button>
        <Button
          size="sm"
          disabled={vm.pending}
          onClick={() => void vm.complete()}
        >
          完成设置
        </Button>
      </div>
      {vm.error && (
        <Alert color="danger" title="未能保存引导状态" description={vm.error} />
      )}
    </Card>
  );
}
