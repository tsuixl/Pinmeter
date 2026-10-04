import {
  NetworkActions,
  NetworkControlPanel,
  RuleStatus,
} from "../app-network-control/NetworkControl";
import {
  useNetworkControlViewModel,
  controllable,
} from "../app-network-control/useNetworkControlViewModel";
import type { MonitorClient } from "../../shared/client/monitor-client";
import { Badge, Button, Card, Input, Progress } from "../../shared/ui/sakani";
import { InfoPopover } from "../../shared/ui/InfoPopover";
import { RankingTable } from "./RankingTable";
import { icons } from "../../shared/ui/icons";
import {
  formatTraffic,
  networkStatusLabels,
  useAppNetworkViewModel,
} from "./useAppNetworkViewModel";

export function AppNetworkRanking({
  client,
  suspended = false,
  onHistory,
  vm,
}: {
  client: MonitorClient;
  suspended?: boolean;
  onHistory?: (app?: { id: string; name: string }) => void;
  vm: ReturnType<typeof useAppNetworkViewModel>;
}) {
  const control = useNetworkControlViewModel(client, suspended);
  const displayStatus =
    vm.status === "normal" && vm.data?.incomplete ? "incomplete" : vm.status;
  return (
    <Card className="app-network">
      <div className="section-heading app-network-heading">
        <div>
          <div className="info-heading">
            <h2>应用网络排行</h2>
            <InfoPopover title="应用流量说明">
              {vm.error && <p>操作失败：{vm.error}</p>}
              {vm.detail && <p>{vm.detail}</p>}
              {(vm.data?.incomplete || vm.status === "incomplete") && (
                <p>本次累计存在采集缺失，仅表示已观测流量。</p>
              )}
              {vm.data?.limited && (
                <p>已达到明细上限；其余已归属流量仍计入合计与占比。</p>
              )}
              {vm.pinnedMissing && (
                <p>固定应用未在当前快照中；不显示为零流量。</p>
              )}
              <p>
                点击列标题排序。占比仅在已归属应用中计算，分母包含当前未显示的应用，
                不表示选定网卡或购买带宽的利用率。
              </p>
              <p>
                统计包含全机局域网及可观测回环流量；代理与 VPN
                可能归属到代理进程。这里与上方选定网卡的曲线分开计量。
              </p>
              <p>
                应用收发量同时保留最长 30
                天分层历史；重新开始只重置本次累计，已保存历史仍可查看。
              </p>
              <p>
                切页和最小化到托盘继续监控。点击停止后结束本次统计，再次开始重新累计。
              </p>
            </InfoPopover>
          </div>
          <p className="processor-caption">全机应用流量 · 本次监控</p>
        </div>
        <div className="app-network-title" role="status">
          <Badge
            variant={
              displayStatus === "failed"
                ? "danger"
                : ["incomplete", "stale", "permission_denied"].includes(
                      displayStatus,
                    )
                  ? "warning"
                  : "neutral"
            }
          >
            {networkStatusLabels[displayStatus] ?? displayStatus}
          </Badge>
          {vm.error && <Badge variant="danger">操作失败</Badge>}
          {vm.data?.incomplete && displayStatus !== "incomplete" && (
            <Badge variant="warning">累计有缺失</Badge>
          )}
          {vm.data?.limited && <Badge variant="warning">明细已达上限</Badge>}
          {vm.pinnedMissing && <Badge>固定应用未出现</Badge>}
        </div>
        <Button
          variant={vm.data?.running ? "secondary" : "primary"}
          disabled={vm.pending || (!vm.data?.running && !vm.canStart)}
          onClick={() => void vm.setMonitoring(!vm.data?.running)}
        >
          {vm.data?.running
            ? "停止监控"
            : vm.pending
              ? "正在启动…"
              : "开始监控"}
        </Button>
      </div>
      <div className="app-network-toolbar">
        <Input
          label="查找当前应用"
          placeholder="应用名称或路径"
          value={vm.search}
          onChange={(e) => vm.setSearch(e.target.value)}
        />
        <span className="processor-caption">
          已记录 {vm.totalApps} · 匹配 {vm.matchingApps}
        </span>
        {vm.pinned && (
          <Button variant="secondary" size="sm" onClick={vm.clearPin}>
            取消固定 {vm.pinned.name}
          </Button>
        )}
        {onHistory && (
          <Button variant="ghost" size="sm" onClick={() => onHistory()}>
            查看应用历史
          </Button>
        )}
      </div>
      {vm.rows.length > 0 ? (
        <RankingTable
          rows={vm.rows}
          sort={vm.sort}
          onSort={vm.setSort}
          onContextMenu={(row, event) => {
            if (row.app && control.available && controllable(row.app)) {
              event.preventDefault();
              control.openMenu(
                row.app,
                event.clientX,
                event.clientY,
                (event.target as HTMLElement)
                  .closest("tr")
                  ?.querySelector("button[aria-haspopup]") ?? null,
              );
            }
          }}
          columns={[
            {
              key: "name",
              header: "应用名称",
              width: "32%",
              render: (row) => (
                <div
                  className={`app-network-name${row.process ? " app-network-process" : ""}`}
                >
                  {row.app ? (
                    <>
                      <Button
                        variant="ghost"
                        size="sm"
                        aria-expanded={vm.expanded.has(row.id)}
                        aria-label={`${vm.expanded.has(row.id) ? "收起" : "展开"} ${row.name}`}
                        onClick={() => vm.toggle(row.id)}
                      >
                        {vm.expanded.has(row.id) ? (
                          <icons.chevronDown size={16} />
                        ) : (
                          <icons.chevronRight size={16} />
                        )}
                      </Button>
                      {row.app.icon ? (
                        <img src={row.app.icon} width={18} height={18} alt="" />
                      ) : (
                        <icons.app size={18} aria-hidden="true" />
                      )}
                    </>
                  ) : null}
                  <div>
                    <div className="app-network-title">
                      {row.app && onHistory ? (
                        <Button
                          variant={
                            vm.selectedId === row.id ? "secondary" : "ghost"
                          }
                          size="sm"
                          title={row.path || undefined}
                          aria-label={`查看 ${row.name} 的历史`}
                          onClick={() => {
                            vm.select(row.id);
                            onHistory({ id: row.id, name: row.name });
                          }}
                        >
                          {row.name}
                        </Button>
                      ) : (
                        <span title={row.path || undefined}>{row.name}</span>
                      )}
                      {row.app && (
                        <Button
                          variant={
                            vm.pinned?.id === row.id ? "secondary" : "ghost"
                          }
                          size="sm"
                          aria-pressed={vm.pinned?.id === row.id}
                          onClick={() => vm.pin(row.app!)}
                        >
                          {vm.pinned?.id === row.id ? "已固定" : "固定"}
                        </Button>
                      )}
                    </div>
                    {row.app && (
                      <small className="processor-caption">
                        {row.app.processes.length} 个已记录进程
                      </small>
                    )}
                    {row.app && <RuleStatus vm={control} id={row.id} />}
                    {row.process && (
                      <small className="processor-caption">
                        {row.observed ? "本窗口有流量" : "本窗口无流量"}
                      </small>
                    )}
                  </div>
                </div>
              ),
            },
            {
              key: "download",
              header: "下载速度",
              width: "16%",
              align: "right",
              render: (row) => (
                <span className="app-network-numbers number">
                  {formatTraffic(row.download)}
                </span>
              ),
            },
            {
              key: "upload",
              header: "上传速度",
              width: "16%",
              align: "right",
              render: (row) => (
                <span className="app-network-numbers number">
                  {formatTraffic(row.upload)}
                </span>
              ),
            },
            {
              key: "id",
              header: vm.direction === "download" ? "下载占比" : "上传占比",
              width: "16%",
              render: (row) => {
                const share =
                  row.traffic[
                    vm.direction === "download"
                      ? "download_share"
                      : "upload_share"
                  ];
                return (
                  <div className="app-network-share number">
                    <span>{share === null ? "—" : `${share.toFixed(1)}%`}</span>
                    {share !== null && (
                      <Progress
                        value={share}
                        size="sm"
                        label={`${row.name}占比`}
                      />
                    )}
                  </div>
                );
              },
            },
            {
              key: "path",
              header: "累计下载 / 上传",
              width: "20%",
              align: "right",
              render: (row) => (
                <div className="app-network-numbers number">
                  <span>
                    ↓ {formatTraffic(Number(row.traffic.received), false)}
                  </span>
                  <span>
                    ↑ {formatTraffic(Number(row.traffic.sent), false)}
                  </span>
                </div>
              ),
            },
            {
              key: "app",
              header: "操作",
              width: "64px",
              align: "right",
              render: (row) =>
                row.app ? (
                  <div className="network-control-actions">
                    <NetworkActions vm={control} target={row.app} />
                  </div>
                ) : row.process ? (
                  <span className="processor-caption">随应用</span>
                ) : null,
            },
          ]}
        />
      ) : (
        <p className="app-network-empty processor-caption">
          {vm.search
            ? "本次监控中没有匹配的应用。"
            : vm.status === "normal"
              ? "当前尚未观察到应用流量"
              : vm.status === "unsupported"
                ? "当前环境暂不支持应用网络采集"
                : "开始监控后，这里会显示正在使用网络的应用"}
        </p>
      )}
      <NetworkControlPanel vm={control} />
    </Card>
  );
}
