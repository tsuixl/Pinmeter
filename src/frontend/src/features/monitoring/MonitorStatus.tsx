import type { MonitorClient } from "../../shared/client/monitor-client";
import { Alert, Badge, Button, Popover } from "../../shared/ui/sakani";
import type { Page } from "./useMonitorViewModel";
import { useMonitorStatusViewModel } from "./useMonitorStatusViewModel";

export function MonitorStatus({
  client,
  suspended,
  onNavigate,
}: {
  client: MonitorClient;
  suspended: boolean;
  onNavigate: (page: Page) => void;
}) {
  const {
    snapshot,
    health,
    control,
    retrying,
    retryError,
    canReconnect,
    reconnect,
  } = useMonitorStatusViewModel(client, suspended);
  return (
    <div className="monitor-status">
      <Popover
        title="运行状态"
        placement="bottom-start"
        trigger={
          <Button
            size="sm"
            variant="ghost"
            aria-label={`运行状态：${health.label}`}
            aria-haspopup="dialog"
          >
            运行状态 · {health.label}
          </Button>
        }
      >
        <div className="health-details">
          {!snapshot.connected ? (
            <>
              <Alert
                color="warning"
                title="监控连接中断"
                description={
                  snapshot.error || "正在重新连接，恢复前隐藏旧读数。"
                }
              />
              {canReconnect && (
                <Button
                  size="sm"
                  loading={retrying}
                  disabled={retrying || suspended}
                  onClick={() => void reconnect()}
                >
                  重新连接
                </Button>
              )}
            </>
          ) : health.issues.length ? (
            health.issues.map((issue, index) => (
              <div key={`${issue.title}-${index}`}>
                <Alert
                  color="warning"
                  title={issue.title}
                  description={issue.detail}
                />
                <Button
                  size="sm"
                  variant="ghost"
                  onClick={() => onNavigate(issue.page)}
                >
                  查看{issue.title}
                </Button>
              </div>
            ))
          ) : (
            <p>
              {health.label === "等待采样"
                ? "正在准备读数，预热完成后自动更新。"
                : "已连接采集服务，当前可用指标未报告异常。"}
            </p>
          )}
          {retryError && (
            <Alert
              color="danger"
              title="重新连接失败"
              description={retryError}
            />
          )}
          <p>
            网络异常时，可尝试解除本安装的全部限速和禁用；配置保留为未启用。
          </p>
          <Button
            size="sm"
            variant="outline"
            loading={!!control.pendingId}
            disabled={!control.canRelease || !!control.pendingId}
            onClick={() => void control.releaseAll()}
          >
            解除全部网络限制
          </Button>
          {control.error && (
            <Alert
              color="danger"
              title="解除未完成，可重试"
              description={control.error}
            />
          )}
          {control.message && <p role="status">{control.message}</p>}
        </div>
      </Popover>
      {snapshot.connected && snapshot.state?.app_network.running && (
        <Button size="sm" variant="ghost" onClick={() => onNavigate("network")}>
          应用流量监控中
        </Button>
      )}
      {health.restrictions > 0 && (
        <Button size="sm" variant="ghost" onClick={() => onNavigate("network")}>
          <Badge variant="warning">
            {snapshot.connected ? "网络限制" : "上次网络限制"} ·{" "}
            {health.restrictions}
          </Badge>
        </Button>
      )}
    </div>
  );
}
