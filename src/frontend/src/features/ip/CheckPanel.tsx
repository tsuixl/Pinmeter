import { useEffect, useState } from "react";
import { usePageUiState } from "../../shared/state/page-ui-state";
import {
  Activity,
  Bot,
  CircleCheck,
  CircleHelp,
  CircleX,
  Clock3,
  RefreshCw,
  Server,
  TriangleAlert,
} from "lucide-react";
import { Badge, Button, Card, IconButton } from "../../shared/ui/sakani";
import type {
  IpCheckGroupDto,
  IpCheckRowDto,
} from "../../shared/contracts/monitor";
import { BrandIcon } from "./BrandIcon";
import { formatTime, names, serviceLabels } from "./presentation";

function outcome(row: IpCheckRowDto, service: boolean) {
  if (row.status === "loading")
    return { text: "检测中", tone: "neutral", Icon: Clock3 } as const;
  if (row.status === "idle")
    return { text: "待检测", tone: "neutral", Icon: CircleHelp } as const;
  if (row.error || row.status === "failed" || row.status === "unsupported")
    return {
      text: service
        ? "获取失败"
        : !row.samples.some((sample) => sample != null)
          ? "未取得响应"
          : "部分失败",
      tone: "warning",
      Icon: CircleX,
    } as const;
  if (row.status === "stale")
    return {
      text: service
        ? (serviceLabels[row.service_state ?? ""] ?? "待确认")
        : row.latency_ms == null
          ? "待确认"
          : `${row.latency_ms} ms`,
      tone: "neutral",
      Icon: Clock3,
    } as const;
  if (service) {
    if (row.service_state === "operational")
      return { text: "正常运行", tone: "success", Icon: CircleCheck } as const;
    if (!row.service_state)
      return { text: "待确认", tone: "neutral", Icon: CircleHelp } as const;
    return {
      text: serviceLabels[row.service_state] ?? "待确认",
      tone: "warning",
      Icon: TriangleAlert,
    } as const;
  }
  return {
    text: row.latency_ms == null ? "待确认" : `${row.latency_ms} ms`,
    tone:
      row.latency_ms == null
        ? "neutral"
        : row.latency_ms < 400
          ? "success"
          : "warning",
    Icon: row.latency_ms == null ? CircleHelp : CircleCheck,
  } as const;
}
export function CheckPanel({
  group,
  connected,
  onRefresh,
}: {
  group: IpCheckGroupDto;
  connected: boolean;
  onRefresh: (id: string) => void;
}) {
  const [cooldown, setCooldown] = useState(0);
  useEffect(() => {
    const until = performance.now() + group.retry_after_ms;
    const update = () =>
      setCooldown(Math.max(0, Math.ceil((until - performance.now()) / 1000)));
    update();
    const timer = window.setInterval(update, 1000);
    return () => clearInterval(timer);
  }, [group]);
  const service = group.id === "services";
  const title = service
    ? "官方服务状态"
    : group.id === "ai"
      ? "AI 访问概览"
      : "网络连通性";
  const Icon = service ? Server : group.id === "ai" ? Bot : Activity;
  const [selected, setSelected] = usePageUiState<string | undefined>(
    `ip.check.${group.id}`,
    undefined,
  );
  const detail = group.rows.find((r) => r.id === selected);
  return (
    <Card className={`ip-check-panel ip-check-${group.id}`}>
      <div className="section-heading">
        <h2>
          <Icon size={18} />
          {title}
        </h2>
        <IconButton
          icon={RefreshCw}
          variant="ghost"
          size="sm"
          aria-label={`刷新${title}`}
          title={cooldown ? `${cooldown} 秒后可刷新` : `刷新${title}`}
          disabled={!connected || group.running || cooldown > 0}
          onClick={() => onRefresh(group.id)}
        />
      </div>
      <div className="ip-check-grid">
        {group.rows.map((row) => {
          const state = outcome(row, service);
          const StatusIcon = state.Icon;
          return (
            <div className="ip-check-item" key={row.id}>
              <Button
                variant="ghost"
                size="sm"
                className="ip-check-target"
                leftIcon={<BrandIcon id={row.id} />}
                onClick={() =>
                  setSelected(selected === row.id ? undefined : row.id)
                }
                aria-expanded={selected === row.id}
                aria-label={`查看${names[row.id] ?? row.id}检测详情`}
              >
                {names[row.id] ?? row.id}
              </Button>
              <Badge variant={state.tone}>
                <span
                  className="ip-state-label"
                  title={
                    row.valid_at_ms == null
                      ? "尚未取得有效结果"
                      : "数据获取于 " + formatTime(row.valid_at_ms)
                  }
                >
                  <StatusIcon size={12} />
                  {state.text}
                </span>
              </Badge>
              {group.id === "connectivity" && (
                <div
                  className="ip-samples"
                  aria-label={`${names[row.id]}：${row.samples.filter((s) => s != null).length}/${row.samples.length} 次有效响应`}
                >
                  {Array.from({ length: 8 }, (_, i) => (
                    <span
                      key={i}
                      className={`ip-sample ${row.samples[i] === undefined ? "pending" : row.samples[i] === null ? "failed" : row.samples[i]! < 400 ? "success" : "slow"}`}
                      title={
                        row.samples[i] === undefined
                          ? "尚未采样"
                          : row.samples[i] === null
                            ? "请求失败"
                            : `${row.samples[i]} ms`
                      }
                    />
                  ))}
                </div>
              )}
            </div>
          );
        })}
      </div>
      {detail && (
        <div
          className="ip-check-detail"
          role="region"
          aria-label={`${names[detail.id]}检测详情`}
        >
          <strong>{names[detail.id]}</strong>
          <small>数据获取于 {formatTime(detail.valid_at_ms)}</small>
          {detail.error && <p>{detail.error}</p>}
          {!service && (
            <>
              <p>
                {detail.latency_ms == null
                  ? "尚无有效耗时"
                  : `最近有效耗时 ${detail.latency_ms} ms`}
                {detail.samples.length
                  ? ` · 本轮 ${detail.samples.filter((s) => s != null).length}/${detail.samples.length} 次有效响应`
                  : ""}
              </p>
              {group.id === "ai" && (
                <p>
                  使用当前应用路由探测公开资源；浏览器的代理、连接和站点防护结果可能不同。
                </p>
              )}
            </>
          )}
          {service && (
            <>
              <p>
                最近获取的官方状态：
                {serviceLabels[detail.service_state ?? ""] ?? "待确认"}
              </p>
              {detail.updated_at && <p>官方更新时间：{detail.updated_at}</p>}
              <p className="processor-caption">
                来源：{detail.source ?? "尚未取得官方数据"}
              </p>
              {detail.incidents.length > 0 && (
                <ul>
                  {detail.incidents.map((i, index) => (
                    <li key={index}>
                      {i.name} · {serviceLabels[i.state] ?? i.state}
                    </li>
                  ))}
                </ul>
              )}
              {detail.components.length > 0 && (
                <details>
                  <summary>服务组件（{detail.components.length}）</summary>
                  <ul>
                    {detail.components.map((c, i) => (
                      <li key={i}>
                        {c.name} · {serviceLabels[c.state] ?? c.state}
                      </li>
                    ))}
                  </ul>
                </details>
              )}
            </>
          )}
        </div>
      )}
      <p className="ip-check-caption">
        {service
          ? "来自官方状态来源 · 点击服务查看组件与事件"
          : group.id === "ai"
            ? "收到响应的耗时 · 不代表登录或对话可用"
            : "HTTP 响应耗时中位数 · 圆点为本轮实际采样"}
      </p>
    </Card>
  );
}
