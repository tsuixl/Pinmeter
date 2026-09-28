import {
  CircuitBoard,
  Clock3,
  Copy,
  Info,
  Cpu,
  Gpu,
  HardDrive,
  MemoryStick,
  Monitor,
  Network,
  Volume2,
} from "lucide-react";
import { Alert, Badge, Button, Card } from "../../shared/ui/sakani";
import type { MonitorClient } from "../../shared/client/monitor-client";
import type { MonitorStateDto } from "../../shared/contracts/monitor";
import { useHardwareViewModel } from "./useHardwareViewModel";
import "./hardware.css";

const categoryIcons = {
  board: CircuitBoard,
  cpu: Cpu,
  memory: MemoryStick,
  gpu: Gpu,
  display: Monitor,
  disk: HardDrive,
  audio: Volume2,
  network: Network,
};

export function HardwareView({
  client,
  state,
  connected,
}: {
  client: MonitorClient;
  state: MonitorStateDto | null;
  connected: boolean;
}) {
  const vm = useHardwareViewModel(client, state, connected);
  return (
    <section
      className={`hardware-page${vm.expanded ? " hardware-expanded" : ""}`}
      aria-label="硬件信息"
    >
      <div className="hardware-toolbar">
        <p>本机型号、系统与关键硬件参数。</p>
        <div className="hardware-actions">
          <span role="status" className="hardware-copy-status">
            {vm.copyState}
          </span>
          <Button
            variant="outline"
            size="sm"
            leftIcon={<Info size={16} />}
            aria-expanded={vm.expanded}
            aria-controls="hardware-rows"
            onClick={vm.toggleExpanded}
          >
            {vm.expanded ? "收起详情" : "详细信息"}
          </Button>
          <Button
            variant="outline"
            size="sm"
            disabled={vm.loading || !!vm.issue}
            leftIcon={<Copy size={16} />}
            onClick={() => void vm.copy()}
          >
            复制信息
          </Button>
        </div>
      </div>
      {vm.issue && (
        <Alert
          color="warning"
          title="硬件信息暂不可用"
          description={vm.issue}
        />
      )}
      <div className="hardware-summary">
        <Card>
          <div className="hardware-summary-label">
            <CircuitBoard size={18} />
            <span>设备信息</span>
          </div>
          <h2>{vm.device}</h2>
          {vm.expanded && <p>{vm.deviceDetail}</p>}
        </Card>
        <Card>
          <div className="hardware-summary-label">
            <Monitor size={18} />
            <span>操作系统</span>
          </div>
          <h2>{vm.info?.system || (vm.loading ? "正在读取…" : "—")}</h2>
          {vm.expanded && <p>{vm.info?.system_detail || "版本与系统架构"}</p>}
        </Card>
        <Card>
          <div className="hardware-summary-label">
            <Clock3 size={18} />
            <span>系统运行时间</span>
          </div>
          <h2 className="number">{vm.uptime}</h2>
          {vm.expanded && <p>{vm.bootLabel}</p>}
        </Card>
      </div>
      <Card className="hardware-list">
        <div className="section-heading">
          <h2>硬件清单</h2>
          {(vm.expanded || vm.loading || vm.issue) && (
            <Badge variant="neutral">
              {vm.loading
                ? "正在读取"
                : vm.issue
                  ? "暂不可用"
                  : "本次启动时读取"}
            </Badge>
          )}
        </div>
        <dl id="hardware-rows" className="hardware-rows" aria-busy={vm.loading}>
          {vm.sections.map((section) => {
            const Icon = categoryIcons[section.id];
            return (
              <div
                className="hardware-row"
                key={section.id}
                data-hardware={section.id}
              >
                <dt>
                  {vm.expanded && <Icon size={18} strokeWidth={1.5} />}
                  {section.title}
                </dt>
                <dd>
                  {section.items.length ? (
                    vm.expanded ? (
                      <ul>
                        {section.items.map((item, index) => (
                          <li key={`${item.name}-${index}`}>
                            <span className="hardware-name">{item.name}</span>
                            {item.details.length > 0 && (
                              <span className="hardware-details">
                                {item.details.join(" · ")}
                              </span>
                            )}
                          </li>
                        ))}
                      </ul>
                    ) : (
                      <span
                        className="hardware-row-summary"
                        title={section.summary}
                      >
                        {section.summary}
                      </span>
                    )
                  ) : (
                    <span className="hardware-empty">
                      {section.error || section.empty}
                    </span>
                  )}
                  {section.items.length > 0 && section.error && (
                    <span className="hardware-details">部分信息未能读取</span>
                  )}
                </dd>
              </div>
            );
          })}
        </dl>
      </Card>
      {vm.expanded && (
        <p className="hardware-footnote">
          硬件信息在应用启动时读取。显示器分辨率为设备首选模式，尺寸为设备报告的物理尺寸估算。
        </p>
      )}
    </section>
  );
}
