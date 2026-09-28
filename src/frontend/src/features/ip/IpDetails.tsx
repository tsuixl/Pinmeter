import { Alert, Badge, Card, Table } from "../../shared/ui/sakani";
import {
  Building2,
  Globe,
  Home,
  Network,
  ShieldCheck,
  Smartphone,
  ChevronDown,
} from "lucide-react";
import type {
  IpProfileDto,
  IpLocationDto,
} from "../../shared/contracts/monitor";
import { formatTime } from "./presentation";
const labels: Record<string, string> = {
  cidr: "CIDR",
  reverse_dns: "反向 DNS",
  registered_country: "注册国家",
  rpki: "RPKI",
  organization: "ASN 组织",
  asn_name: "ASN 名称",
  company: "公司",
  company_type: "公司类型",
  datacenter_name: "数据中心",
  asn_kind: "ASN 类型",
  asn_allocated: "ASN 分配日期",
  range_first: "地址范围起点",
  range_last: "地址范围终点",
  range_count: "地址数量",
  residential: "住宅网络",
  datacenter: "数据中心",
  mobile: "移动网络",
  vpn: "VPN",
  proxy: "代理",
  tor: "Tor",
  crawler: "爬虫",
  abuser: "滥用",
  public_service: "公共服务",
  related_domains: "关联域名",
  location_history: "位置历史",
  asn_history: "ASN 历史",
  company_history: "企业历史",
  dc_neighbors: "关联网络地址",
};

function AttributeIcon({ name }: { name: string }) {
  const Icon =
    name === "residential"
      ? Home
      : name === "datacenter"
        ? Building2
        : name === "mobile"
          ? Smartphone
          : ["vpn", "proxy", "tor", "crawler", "abuser"].includes(name)
            ? ShieldCheck
            : name === "registered_country"
              ? Globe
              : Network;
  return <Icon size={15} aria-hidden="true" />;
}
export function IpDetails({
  profile,
  open,
  onOpen,
}: {
  profile: IpProfileDto | undefined;
  open: boolean;
  onOpen: (open: boolean) => void;
}) {
  const data = profile?.data;
  if (!data) return null;
  return (
    <details
      className="ip-details"
      open={open}
      onToggle={(e) => onOpen(e.currentTarget.open)}
    >
      <summary>
        <ChevronDown size={16} />
        IP 详细资料<span className="processor-caption">{data.address}</span>
      </summary>
      <p className="ip-fetch-time">
        数据获取于 {formatTime(profile?.valid_at_ms)} · {data.source}
      </p>
      <div className="ip-details-content">
        <div className="ip-detail-grid">
          <Card>
            <div className="section-heading">
              <h2>网络属性与 ASN</h2>
            </div>
            <dl className="data-rows">
              {data.facts.map((f) => (
                <div key={f.key}>
                  <dt>
                    <AttributeIcon name={f.key} />
                    {labels[f.key] ?? f.key}
                  </dt>
                  <dd>{f.value ?? "未知"}</dd>
                </div>
              ))}
            </dl>
          </Card>
          <Card>
            <div className="section-heading">
              <h2>网络类型与风险标记</h2>
            </div>
            <dl className="data-rows">
              {data.flags.map((f) => (
                <div key={f.key}>
                  <dt>
                    <AttributeIcon name={f.key} />
                    {labels[f.key] ?? f.key}
                  </dt>
                  <dd>
                    <Badge
                      variant={
                        f.value === true &&
                        ["vpn", "proxy", "tor", "crawler", "abuser"].includes(
                          f.key,
                        )
                          ? "warning"
                          : "neutral"
                      }
                    >
                      {f.value == null ? "未知" : f.value ? "是" : "否"}
                    </Badge>
                  </dd>
                </div>
              ))}
            </dl>
            <p className="processor-caption">
              “否”仅表示数据源本次未标记；缺失字段保持未知。
            </p>
          </Card>
        </div>
        <Card>
          <div className="section-heading">
            <h2>地理位置 · 多源对比</h2>
          </div>
          {data.locations.length ? (
            <Table<IpLocationDto>
              rows={data.locations}
              rowKey={(_row, index) => String(index)}
              columns={[
                { key: "source", header: "来源" },
                {
                  key: "country",
                  header: "国家",
                  render: (r) => r.country ?? "未知",
                },
                {
                  key: "region",
                  header: "地区",
                  render: (r) => r.region ?? "未知",
                },
                {
                  key: "city",
                  header: "城市",
                  render: (r) => r.city ?? "未知",
                },
              ]}
            />
          ) : (
            <p className="processor-caption">数据源未提供可对比的位置。</p>
          )}
        </Card>
        {Array.from(new Set(data.records.map((r) => r.kind))).map((kind) => (
          <Card key={kind}>
            <details>
              <summary>
                {labels[kind] ?? kind}（
                {data.records.filter((r) => r.kind === kind).length}）
              </summary>
              <dl className="data-rows">
                {data.records
                  .filter((r) => r.kind === kind)
                  .map((r, i) => (
                    <div key={i}>
                      <dt>{r.value}</dt>
                      <dd>
                        {r.detail ?? "—"}
                        {r.at && (
                          <small> · {formatTime(Number(r.at) * 1000)}</small>
                        )}
                      </dd>
                    </div>
                  ))}
              </dl>
            </details>
          </Card>
        ))}
        {data.truncated && (
          <Alert
            color="info"
            title="部分记录已截断"
            description="每类最多展示 100 条上游记录。"
          />
        )}
      </div>
    </details>
  );
}
