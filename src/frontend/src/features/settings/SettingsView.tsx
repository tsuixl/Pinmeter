import type { MonitorClient } from "../../shared/client/monitor-client";
import type { MonitorStateDto } from "../../shared/contracts/monitor";
import { TaskbarPreview } from "./TaskbarPreview";
import { useSettingsViewModel } from "./useSettingsViewModel";
import { buildInfo } from "../../shared/client/build-info";
import { UpdateSettings } from "../updates/UpdateViews";
import { DiagnosticsCard } from "../diagnostics/DiagnosticsCard";
import type { UpdateViewModel } from "../updates/useUpdateViewModel";
import {
  Alert,
  Button,
  Card,
  Radio,
  Select,
  Switch,
  Checkbox,
} from "../../shared/ui/sakani";
export function SettingsView({
  client,
  state,
  updates,
}: {
  client: MonitorClient;
  state: MonitorStateDto | null;
  updates: UpdateViewModel;
}) {
  const vm = useSettingsViewModel(client, state?.settings);
  const adapters = [
    { value: "auto", label: "自动选择" },
    ...(state?.interfaces.map((a) => ({
      value: a.id,
      label: a.name + (a.up ? "" : " · 已断开"),
    })) ?? []),
  ];
  if (
    vm.draft.network_id &&
    !state?.interfaces.some((a) => a.id === vm.draft.network_id)
  )
    adapters.push({
      value: vm.draft.network_id,
      label: "已保存的接口 · 不可用",
    });
  return (
    <div className="settings-form">
      <div className="settings-actions" role="status">
        <span>{vm.feedback || "设置更改后自动保存"}</span>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={vm.reset}
          disabled={vm.pending}
        >
          恢复默认
        </Button>
      </div>
      {vm.error && (
        <Alert
          color="danger"
          title="保存失败，已恢复原设置"
          description={vm.error}
        />
      )}

      <Card title="外观" description="标题栏与内容使用同一主题">
        <fieldset className="theme-options" disabled={vm.pending}>
          <legend className="sr-only">主题</legend>
          {[
            ["system", "跟随系统"],
            ["light", "浅色"],
            ["dark", "深色"],
          ].map(([value, label]) => (
            <Radio
              key={value}
              name="theme"
              value={value}
              label={label}
              disabled={vm.pending}
              checked={vm.draft.theme === value}
              onChange={() => vm.change({ theme: value })}
            />
          ))}
        </fieldset>
      </Card>
      <Card title="启动" description="登录电脑后自动打开 Pinmeter">
        <Switch
          label="启动到托盘"
          checked={vm.draft.start_in_tray}
          disabled={
            vm.pending ||
            (!client.getSnapshot().demo && state?.platform !== "windows")
          }
          onChange={(event) =>
            vm.change({ start_in_tray: event.target.checked })
          }
        />
        <p className="settings-note">
          下次启动生效。保留托盘入口并继续监控，再次打开已运行的 Pinmeter
          可恢复窗口；托盘不可用时自动显示主窗口。
        </p>
        <Switch
          label="开机自启"
          checked={vm.draft.autostart}
          disabled={
            vm.pending || (!state?.autostart.available && !vm.draft.autostart)
          }
          onChange={(event) => vm.change({ autostart: event.target.checked })}
        />
        <p className="settings-note" role="status">
          {state?.autostart.detail || "等待启动项状态"}
        </p>
      </Card>
      <Card title="窗口行为" description="选择点击关闭按钮时的操作">
        <Select
          id="close-action"
          label="关闭窗口时"
          value={vm.draft.close_action}
          disabled={vm.pending}
          options={[
            { value: "ask", label: "每次询问" },
            { value: "minimize", label: "最小化到托盘" },
            { value: "exit", label: "退出 Pinmeter" },
          ]}
          onChange={(close_action) => vm.change({ close_action })}
        />
      </Card>
      <Card title="监控" description="调整基础指标的刷新频率和网络来源">
        <div className="settings-fields">
          <Select
            id="sample-interval"
            label="采样间隔"
            description="CPU、物理内存与网络使用同一采样周期"
            value={String(vm.draft.interval_ms)}
            onChange={(value) => vm.change({ interval_ms: Number(value) })}
            disabled={vm.pending}
            options={[1000, 2000, 5000].map((ms) => ({
              value: String(ms),
              label: `${ms / 1000} 秒`,
            }))}
          />
          <Select
            id="network-interface"
            label="网络接口"
            description="自动模式只选择一个活动接口"
            value={vm.draft.network_id ?? "auto"}
            onChange={(value) =>
              vm.change({ network_id: value === "auto" ? null : value })
            }
            disabled={vm.pending}
            options={adapters}
          />
        </div>
        <p className="settings-note">
          当前接口：{state?.selected_interface?.name ?? "暂无可用网卡"}
          。最近五分钟历史仅保存在内存中。
        </p>
      </Card>
      <Card
        title="任务栏显示"
        description="在主屏任务栏查看网速、CPU / GPU 使用率与温度、内存"
      >
        <Switch
          label="启用任务栏显示"
          checked={vm.draft.taskbar.enabled}
          disabled={
            vm.pending ||
            (!state?.desktop.supported && !vm.draft.taskbar.enabled)
          }
          onChange={(event) =>
            vm.change({
              taskbar: {
                ...vm.draft.taskbar,
                enabled: event.target.checked,
                hidden: false,
              },
            })
          }
        />
        <p className="settings-note">
          开关和选项更改后立即生效。最小化到托盘后仍会继续监控。
        </p>
        <div className="settings-fields">
          <Select
            id="taskbar-layout"
            label="读数布局"
            value={vm.draft.taskbar.layout}
            disabled={
              vm.pending ||
              !vm.draft.taskbar.enabled ||
              !state?.desktop.supported
            }
            options={[
              { value: "double", label: "双行紧凑" },
              { value: "single", label: "单行横排" },
            ]}
            onChange={(layout) =>
              vm.change({ taskbar: { ...vm.draft.taskbar, layout } })
            }
          />
          <fieldset
            className="taskbar-metrics"
            disabled={
              vm.pending ||
              !vm.draft.taskbar.enabled ||
              !state?.desktop.supported
            }
          >
            <legend>显示指标</legend>
            <span className="settings-note">下载与上传始终显示</span>
            <Checkbox
              label="CPU 使用率与温度"
              checked={vm.draft.taskbar.cpu}
              disabled={
                vm.pending ||
                !vm.draft.taskbar.enabled ||
                !state?.desktop.supported
              }
              onChange={(event) =>
                vm.change({
                  taskbar: { ...vm.draft.taskbar, cpu: event.target.checked },
                })
              }
            />
            <Checkbox
              label="GPU 使用率与温度"
              checked={vm.draft.taskbar.gpu}
              disabled={
                vm.pending ||
                !vm.draft.taskbar.enabled ||
                !state?.desktop.supported
              }
              onChange={(event) =>
                vm.change({
                  taskbar: { ...vm.draft.taskbar, gpu: event.target.checked },
                })
              }
            />
            <Checkbox
              label="内存使用率"
              checked={vm.draft.taskbar.memory}
              disabled={
                vm.pending ||
                !vm.draft.taskbar.enabled ||
                !state?.desktop.supported
              }
              onChange={(event) =>
                vm.change({
                  taskbar: {
                    ...vm.draft.taskbar,
                    memory: event.target.checked,
                  },
                })
              }
            />
          </fieldset>
        </div>
        <p className="settings-note">
          温度不可用时显示 —。GPU 默认选择显存容量最大的设备；温度后的 * 表示 VR
          SoC 测温点，悬停可查看详情。
        </p>
        <TaskbarPreview settings={vm.draft.taskbar} />
        <p className="settings-note" role="status">
          {state?.desktop.detail || "等待任务栏能力检测"}
        </p>
        {vm.draft.taskbar.hidden && (
          <Button
            type="button"
            size="sm"
            variant="outline"
            disabled={vm.pending}
            onClick={() =>
              vm.change({ taskbar: { ...vm.draft.taskbar, hidden: false } })
            }
          >
            恢复读数
          </Button>
        )}
      </Card>
      <Card title="网络控制" description="决定退出时如何处理应用限制">
        <Switch
          label="退出 Pinmeter 时解除所有网络限制"
          checked={vm.draft.release_network_on_exit}
          disabled={vm.pending}
          onChange={(event) =>
            vm.change({ release_network_on_exit: event.target.checked })
          }
        />
        <p className="settings-note">
          {vm.draft.release_network_on_exit
            ? "正常退出时解除限速和网络禁用，保留配置但不启用。下次打开后可手动启用。"
            : "退出后仅保留网络禁用，限速仍会停止。下次打开会重新应用已启用规则。"}
        </p>
        <p className="processor-caption">
          最小化不解除限制；强制结束或断电可能留下禁用规则。
        </p>
      </Card>
      <UpdateSettings vm={updates} />
      <DiagnosticsCard client={client} />
      <div className="about">
        <div>
          <strong>
            Pinmeter <span>{buildInfo.version}</span>
          </strong>
          <p>最小化后收起到托盘；点击托盘图标可恢复窗口。</p>
        </div>
        <Button
          type="button"
          variant="ghost"
          size="sm"
          onClick={() => {
            void client.recoverWindow().catch(() => {});
          }}
        >
          将窗口移回可见区域
        </Button>
      </div>
      {client.requestExit && (
        <Button
          type="button"
          variant="ghost"
          onClick={() => {
            void vm.exit();
          }}
        >
          退出 Pinmeter
        </Button>
      )}
    </div>
  );
}
