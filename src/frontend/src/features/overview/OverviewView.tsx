import { Button, Card, Select, SegmentedControl } from "../../shared/ui/sakani";
import type { useGpuViewModel } from "../gpu/useGpuViewModel";
import type {
  Page,
  useMonitorViewModel,
} from "../monitoring/useMonitorViewModel";
import { TrendChart } from "../monitoring/TrendChart";
import { useOverviewViewModel } from "./useOverviewViewModel";
import "./overview.css";

export function OverviewView({
  monitor,
  gpu,
  onNavigate,
}: {
  monitor: ReturnType<typeof useMonitorViewModel>;
  gpu: ReturnType<typeof useGpuViewModel>;
  onNavigate: (page: Page) => void;
}) {
  const vm = useOverviewViewModel(monitor, gpu);
  return (
    <section className="overview-page" aria-label="系统总览">
      <div className="overview-metrics">
        {vm.cards.map((card) => (
          <button
            type="button"
            key={card.id}
            className={`metric-link overview-metric${card.id === "network" ? " overview-network" : ""}`}
            data-metric={card.id}
            aria-label={`查看${card.title}详情`}
            onClick={() => onNavigate(card.page)}
          >
            <Card interactive className="overview-resource-card">
              <div className="overview-card-content">
                <h2 className="overview-card-title">{card.title}</h2>
                <div className="overview-readings">
                  {card.readings.map((reading) => (
                    <div
                      key={reading.id}
                      className={`overview-reading${reading.secondary ? " overview-reading-secondary" : ""}`}
                      data-reading={reading.id}
                      data-status={reading.status}
                    >
                      <span className="overview-reading-label">
                        {reading.label}
                      </span>
                      <span className="overview-reading-value">
                        {reading.value}
                      </span>
                      {reading.statusLabel && (
                        <span
                          className="overview-reading-status"
                          title={reading.detail || undefined}
                        >
                          {reading.statusLabel}
                        </span>
                      )}
                    </div>
                  ))}
                </div>
                <p className="overview-card-description">{card.description}</p>
              </div>
            </Card>
          </button>
        ))}
      </div>
      <Card className="trend-card overview-trends">
        <div className="section-heading">
          <h2>资源趋势</h2>
          {gpu.devices.length > 0 && (
            <Select
              label="显卡"
              value={gpu.selected?.id ?? ""}
              options={gpu.devices.map((device) => ({
                value: device.id,
                label: device.name,
              }))}
              onChange={gpu.setSelectedId}
            />
          )}
        </div>
        <div className="overview-groups" aria-label="趋势指标分组">
          <SegmentedControl
            value={vm.group}
            onChange={vm.setGroup}
            options={[
              { value: "usage", label: "使用率" },
              { value: "temperature", label: "温度" },
              { value: "all", label: "全部" },
            ]}
          />
        </div>
        <div className="overview-legend" aria-label="显示的资源趋势">
          {vm.groupSeries.map((item) => (
            <Button
              key={item.id}
              size="sm"
              variant={vm.hidden.includes(item.id) ? "ghost" : "outline"}
              aria-pressed={!vm.hidden.includes(item.id)}
              onClick={() => vm.toggle(item.id)}
            >
              <span className="overview-legend-label">
                <i
                  className="tooltip-dot"
                  style={{
                    background: vm.hidden.includes(item.id)
                      ? "var(--color-fg-muted)"
                      : item.color,
                  }}
                  aria-hidden="true"
                />
                {item.label}
              </span>
            </Button>
          ))}
        </div>
        <TrendChart
          key={gpu.selected?.id ?? "no-gpu"}
          history={monitor.history}
          series={vm.visibleSeries}
          range={monitor.range}
          anchor={monitor.anchor}
          onAnchor={monitor.setAnchor}
          label="资源趋势"
        />
      </Card>
    </section>
  );
}
