import { usePageUiState } from "../../shared/state/page-ui-state";
import {
  Building2,
  Check,
  Copy,
  Globe,
  Home,
  MapPin,
  Network,
  RefreshCw,
  ShieldCheck,
  Smartphone,
} from "lucide-react";
import { Alert, Badge, Button, Card, IconButton } from "../../shared/ui/sakani";
import type { IpClient } from "../../shared/client/ip-client";
import type { IpExitDto, IpProfileDto } from "../../shared/contracts/monitor";
import { exitLabels, queryLabels, useIpViewModel } from "./useIpViewModel";
import { CheckPanel } from "./CheckPanel";
import { IpDetails } from "./IpDetails";
import { formatTime, scoreTone } from "./presentation";
import license from "../../../../backend/platform/src/ip/third-party/one-ip.LICENSE?raw";
import notice from "../../../../backend/platform/src/ip/third-party/NOTICE?raw";
import "./ip.css";
function ExitCard({
  exit,
  profile,
  selected,
  copied,
  onSelect,
  onCopy,
}: {
  exit: IpExitDto;
  profile: IpProfileDto | undefined;
  selected: boolean;
  copied: boolean;
  onSelect: () => void;
  onCopy: () => void;
}) {
  const data = profile?.data;
  const score = data?.score;
  const kinds = [
    { key: "residential", label: "住宅网络", Icon: Home },
    { key: "datacenter", label: "数据中心", Icon: Building2 },
    { key: "mobile", label: "移动网络", Icon: Smartphone },
  ];
  return (
    <Card className={"ip-exit " + (selected ? "ip-selected" : "")}>
      <div className="ip-exit-content">
        <div className="section-heading">
          <h2>
            <Globe size={17} />
            {exitLabels[exit.id]}
          </h2>
          <span className="ip-actions">
            {exit.status === "loading" && <Badge>检测中</Badge>}
            <IconButton
              icon={copied ? Check : Copy}
              size="sm"
              variant="ghost"
              aria-label={"复制" + exitLabels[exit.id]}
              title="复制 IP"
              disabled={!exit.address}
              onClick={onCopy}
            />
          </span>
        </div>
        <div className="ip-exit-summary">
          <div className="ip-exit-identity">
            <p className="ip-address number">{exit.address ?? "—"}</p>
            <p className="ip-location">
              <MapPin size={15} />
              {data
                ? [data.country, data.region, data.city]
                    .filter(Boolean)
                    .join(" · ") || "位置未知"
                : "等待位置资料"}
            </p>
            <p className="ip-provider">
              <Network size={15} />
              {data?.isp ?? "运营商未知"}
              {data?.asn != null ? " · AS" + data.asn : ""}
            </p>
            <div className="ip-kind-tags">
              {kinds
                .filter((k) =>
                  data?.flags.some((f) => f.key === k.key && f.value === true),
                )
                .map(({ key, label, Icon }) => (
                  <Badge key={key}>
                    <span className="ip-state-label">
                      <Icon size={12} />
                      {label}
                    </span>
                  </Badge>
                ))}
            </div>
          </div>
          <div
            className={"ip-score ip-tone-" + scoreTone(score)}
            title="第三方 IP 信誉参考，不代表网速或账号安全"
          >
            <ShieldCheck size={17} />
            <strong className="number">{score ?? "—"}</strong>
            <span>IP 信誉</span>
          </div>
        </div>
        {(exit.error || profile?.error) && (
          <p className="ip-inline-error">{exit.error ?? profile?.error}</p>
        )}
        <div className="ip-exit-bottom">
          <small
            className="ip-fetch-time"
            title={exit.route + " · " + exit.source}
          >
            {(profile?.valid_at_ms ?? exit.valid_at_ms) == null
              ? "尚未获取数据"
              : "数据获取于 " +
                formatTime(profile?.valid_at_ms ?? exit.valid_at_ms)}
          </small>
          <Button
            size="sm"
            variant="ghost"
            disabled={!exit.address}
            aria-pressed={selected}
            onClick={onSelect}
          >
            查看资料
          </Button>
        </div>
      </div>
    </Card>
  );
}
export function IpView({ client }: { client: IpClient }) {
  const [detailsOpen, setDetailsOpen] = usePageUiState("ip.details", false);
  const vm = useIpViewModel(client);
  const ipv6 = vm.exits.find((e) => e.id === "ipv6");
  const select = (id: string) => {
    vm.setSelectedId(id);
    setDetailsOpen(true);
    requestAnimationFrame(() =>
      document.querySelector(".ip-details")?.scrollIntoView({ block: "start" }),
    );
  };
  const groups = vm.state?.checks ?? [];
  const ai = groups.find((g) => g.id === "ai");
  const running = vm.state?.running || groups.some((g) => g.running);
  return (
    <section className="ip-page" aria-label="IP 检测与资料">
      <div className="ip-toolbar">
        <p className="processor-caption">出口概览与网络检查</p>
        <Button
          size="sm"
          variant="outline"
          disabled={!vm.connected || running || vm.cooldown > 0}
          onClick={vm.refresh}
          leftIcon={<RefreshCw size={14} />}
        >
          {running
            ? "正在检测…"
            : vm.cooldown > 0
              ? vm.cooldown + " 秒后可刷新"
              : "重新检测"}
        </Button>
      </div>
      {!vm.connected && (
        <Alert
          color="warning"
          title="查询服务未连接"
          description="恢复连接后可检测当前出口，已有内容保留原获取日期。"
        />
      )}
      {vm.error && (
        <Alert color="warning" title="操作未完成" description={vm.error} />
      )}
      <div className="ip-exits">
        {["domestic", "ipv4"]
          .map((id) => vm.exits.find((e) => e.id === id))
          .filter((e): e is IpExitDto => !!e)
          .map((exit) => (
            <ExitCard
              key={exit.id}
              exit={exit}
              profile={vm.state?.profiles.find(
                (p) => p.address === exit.address,
              )}
              selected={vm.selected?.id === exit.id}
              copied={!!exit.address && vm.copied === exit.address}
              onCopy={() => exit.address && vm.copy(exit.address)}
              onSelect={() => select(exit.id)}
            />
          ))}
      </div>
      {ipv6 && (
        <div className="ip-ipv6">
          <span className="ip-state-label">
            <Globe size={16} />
            <strong>IPv6</strong>
          </span>
          <span className="ip-ipv6-address number">
            {ipv6.address ?? ipv6.error ?? queryLabels[ipv6.status]}
          </span>
          <small className="ip-fetch-time">
            {ipv6.valid_at_ms == null
              ? "尚未获取数据"
              : "获取于 " + formatTime(ipv6.valid_at_ms)}
          </small>
          <span className="ip-actions">
            <Button
              size="sm"
              variant="ghost"
              disabled={!ipv6.address}
              aria-pressed={vm.selected?.id === "ipv6"}
              onClick={() => select("ipv6")}
            >
              查看资料
            </Button>
            <IconButton
              icon={Copy}
              size="sm"
              variant="ghost"
              aria-label="复制公网 IPv6"
              title="复制 IPv6"
              disabled={!ipv6.address}
              onClick={() => ipv6.address && vm.copy(ipv6.address)}
            />
          </span>
        </div>
      )}
      {ai && (
        <CheckPanel
          group={ai}
          connected={vm.connected}
          onRefresh={vm.refreshChecks}
        />
      )}
      <div className="ip-overview-grid">
        {groups
          .filter((g) => g.id !== "ai")
          .map((g) => (
            <CheckPanel
              key={g.id}
              group={g}
              connected={vm.connected}
              onRefresh={vm.refreshChecks}
            />
          ))}
      </div>
      <IpDetails
        profile={vm.profile}
        open={detailsOpen}
        onOpen={setDetailsOpen}
      />
      <p role="status" className="ip-copy-feedback">
        {vm.copied ? "IP 地址已复制" : ""}
      </p>
      <details className="ip-license">
        <summary>数据来源与开源说明</summary>
        <p>
          出口：ipify、国内检测目标；IP
          资料：Net.Coffee；服务状态来自对应官方来源。只查询公网信息，不上传硬件、进程或账号数据。请求遵循当前应用代理策略，不代表浏览器路由；IP
          信誉仅供参考。
        </p>
        <p>
          检测与适配代码移植自 zhihui-hu/one-ip（5686f3f1），遵循
          AGPL-3.0。品牌图标随应用本地打包。
        </p>
        <pre>{notice}</pre>
        <pre>{license}</pre>
      </details>
    </section>
  );
}
