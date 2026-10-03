import { useEffect, useState } from "react";
import type { MonitorClient } from "../../shared/client/monitor-client";
import type {
  AlertEventDto,
  AlertsConfigDto,
} from "../../shared/contracts/monitor";
import {
  Alert,
  Badge,
  Button,
  Card,
  Input,
  Switch,
} from "../../shared/ui/sakani";
import { useAlertsViewModel } from "./useAlertsViewModel";
import "./alerts.css";

type Props = {
  client: MonitorClient;
  onNavigate: (page: "cpu" | "memory") => void;
};
const title = (event: AlertEventDto) =>
  `${event.metric === "cpu" ? "CPU" : "内存"} 持续高占用`;
const description = (event: AlertEventDto) =>
  `${new Date(event.at_ms).toLocaleString()} · 持续 ${event.duration_seconds} 秒达到 ${event.threshold_percent}% 阈值，触发时 ${event.value.toFixed(1)}%。`;
const timeText = (minutes: number) =>
  `${String(Math.floor(minutes / 60)).padStart(2, "0")}:${String(minutes % 60).padStart(2, "0")}`;
const parseTime = (value: string) => {
  const [hours, minutes] = value.split(":").map(Number);
  return hours * 60 + minutes;
};

export function AlertsBanner({ client, onNavigate }: Props) {
  const vm = useAlertsViewModel(client);
  const event = vm.data?.events.find((item) => !item.acknowledged);
  if (!event) return null;
  return (
    <section className="alerts-banner" aria-label="占用提醒">
      <Alert
        color="warning"
        title={title(event)}
        description={`${description(event)}这是已记录的事件，当前占用请查看详情。`}
      />
      <div className="alerts-actions">
        <Button
          size="sm"
          variant="outline"
          disabled={vm.pending}
          onClick={() => {
            void vm.acknowledge(event.id);
            onNavigate(event.metric);
          }}
        >
          查看{event.metric === "cpu" ? " CPU" : "内存"}详情
        </Button>
        <Button
          size="sm"
          variant="ghost"
          disabled={vm.pending}
          onClick={() => void vm.acknowledge(event.id)}
        >
          知道了
        </Button>
        {(vm.data?.unread_count ?? 0) > 1 && (
          <span className="muted">
            还有 {(vm.data?.unread_count ?? 1) - 1} 条待查看
          </span>
        )}
      </div>
      {vm.error && (
        <p role="status" className="settings-note">
          {vm.error}
        </p>
      )}
    </section>
  );
}

export function AlertsView({ client, onNavigate }: Props) {
  const vm = useAlertsViewModel(client);
  const [draft, setDraft] = useState<AlertsConfigDto | null>(null);
  const [baseRevision, setBaseRevision] = useState("");
  const [dirty, setDirty] = useState(false);
  const [saved, setSaved] = useState(false);
  useEffect(() => {
    if (vm.data && !dirty) {
      setDraft(vm.data.config);
      setBaseRevision(vm.data.settings_revision);
    }
  }, [vm.data, dirty]);
  const change = (value: AlertsConfigDto) => {
    setDraft(value);
    setDirty(true);
    setSaved(false);
  };
  if (!vm.data || !draft)
    return (
      <Alert
        color={vm.error ? "danger" : "info"}
        title={vm.error ? "提醒设置读取失败" : "正在读取提醒设置"}
        description={vm.error || "正在获取后端已确认的规则和本次运行记录。"}
      />
    );
  return (
    <section className="alerts-page" aria-label="占用提醒设置">
      <Alert
        color="info"
        title="提醒默认关闭"
        description={vm.data.delivery_detail}
      />
      {vm.error && (
        <Alert color="danger" title="操作未完成" description={vm.error} />
      )}
      {vm.data.quiet_now && (
        <Alert
          color="neutral"
          title="当前处于静默状态"
          description="静默时不生成提醒；结束后重新计算持续时间，不补发积压事件。"
        />
      )}
      <form
        className="alerts-config"
        onSubmit={async (event) => {
          event.preventDefault();
          if (await vm.save(draft, baseRevision)) {
            setDirty(false);
            setSaved(true);
          }
        }}
      >
        {(["cpu", "memory"] as const).map((metric) => {
          const rule = draft[metric];
          const name = metric === "cpu" ? "CPU" : "内存";
          const setRule = (patch: Partial<typeof rule>) =>
            change({ ...draft, [metric]: { ...rule, ...patch } });
          return (
            <Card
              key={metric}
              title={`${name} 高占用`}
              description="一次连续高占用仅提醒一次，恢复后再次达到条件才会提醒"
            >
              <div className="alerts-rule">
                <Switch
                  label={`开启 ${name} 提醒`}
                  checked={rule.enabled}
                  disabled={vm.pending}
                  onChange={(event) =>
                    setRule({ enabled: event.target.checked })
                  }
                />
                <div className="alerts-fields">
                  <Input
                    id={`alert-${metric}-threshold`}
                    label="占用阈值（%）"
                    type="number"
                    min={1}
                    max={100}
                    step={1}
                    required
                    disabled={vm.pending || !rule.enabled}
                    value={rule.threshold_percent}
                    onChange={(event) =>
                      setRule({ threshold_percent: Number(event.target.value) })
                    }
                  />
                  <Input
                    id={`alert-${metric}-duration`}
                    label="持续时间（秒）"
                    type="number"
                    min={5}
                    max={600}
                    step={1}
                    required
                    disabled={vm.pending || !rule.enabled}
                    value={rule.duration_seconds}
                    onChange={(event) =>
                      setRule({ duration_seconds: Number(event.target.value) })
                    }
                  />
                  <Input
                    id={`alert-${metric}-cooldown`}
                    label="两次提醒最短间隔（分钟）"
                    type="number"
                    min={1}
                    max={1440}
                    step={1}
                    required
                    disabled={vm.pending || !rule.enabled}
                    value={rule.cooldown_seconds / 60}
                    onChange={(event) =>
                      setRule({
                        cooldown_seconds: Number(event.target.value) * 60,
                      })
                    }
                  />
                </div>
              </div>
            </Card>
          );
        })}
        <Card
          title="每日静默时段"
          description={
            vm.data.local_time_available
              ? "按本机时间生效，支持跨午夜"
              : "当前平台尚不支持本地静默时段；可使用不带静默的提醒"
          }
        >
          <div className="alerts-rule">
            <Switch
              label="启用静默时段"
              checked={draft.quiet.enabled}
              disabled={
                vm.pending ||
                (!vm.data.local_time_available && !draft.quiet.enabled)
              }
              onChange={(event) =>
                change({
                  ...draft,
                  quiet: { ...draft.quiet, enabled: event.target.checked },
                })
              }
            />
            <div className="alerts-fields">
              <Input
                id="alerts-quiet-start"
                label="开始时间"
                type="time"
                required
                disabled={
                  vm.pending ||
                  !draft.quiet.enabled ||
                  !vm.data.local_time_available
                }
                value={timeText(draft.quiet.start_minute)}
                onChange={(event) =>
                  change({
                    ...draft,
                    quiet: {
                      ...draft.quiet,
                      start_minute: parseTime(event.target.value),
                    },
                  })
                }
              />
              <Input
                id="alerts-quiet-end"
                label="结束时间"
                type="time"
                required
                disabled={
                  vm.pending ||
                  !draft.quiet.enabled ||
                  !vm.data.local_time_available
                }
                value={timeText(draft.quiet.end_minute)}
                onChange={(event) =>
                  change({
                    ...draft,
                    quiet: {
                      ...draft.quiet,
                      end_minute: parseTime(event.target.value),
                    },
                  })
                }
              />
            </div>
          </div>
        </Card>
        <div className="alerts-actions">
          <Button
            type="submit"
            size="sm"
            disabled={!dirty || vm.pending}
            loading={vm.pending}
          >
            保存提醒设置
          </Button>
          <Button
            type="button"
            size="sm"
            variant="ghost"
            disabled={!dirty || vm.pending}
            onClick={() => {
              setDraft(vm.data!.config);
              setBaseRevision(vm.data!.settings_revision);
              setDirty(false);
              setSaved(false);
            }}
          >
            放弃未保存更改
          </Button>
          <span className="muted" role="status">
            {saved ? "已保存" : dirty ? "存在未保存更改" : "规则已与后端同步"}
          </span>
        </div>
      </form>
      <Card
        title="本次运行的提醒"
        description="最多保留最近 20 条；历史事件不代表当前仍处于高占用"
      >
        <div className="alerts-event-list">
          {!vm.data.events.length && (
            <p className="muted">
              暂无提醒。开启规则后，正常有效的读数达到持续条件才会记录。
            </p>
          )}
          {vm.data.events.map((event) => (
            <div className="alerts-event" key={event.id}>
              <div>
                <strong>{title(event)}</strong>{" "}
                <Badge variant={event.acknowledged ? "neutral" : "warning"}>
                  {event.acknowledged ? "已查看" : "待查看"}
                </Badge>
                <p className="muted">{description(event)}</p>
              </div>
              <Button
                size="sm"
                variant="outline"
                disabled={vm.pending}
                onClick={() => {
                  void vm.acknowledge(event.id);
                  onNavigate(event.metric);
                }}
              >
                查看详情
              </Button>
            </div>
          ))}
          {!!vm.data.events.length && (
            <Button
              size="sm"
              variant="ghost"
              disabled={vm.pending}
              onClick={() => void vm.clear(vm.data!.events[0].id)}
            >
              清除当前记录
            </Button>
          )}
        </div>
      </Card>
    </section>
  );
}
