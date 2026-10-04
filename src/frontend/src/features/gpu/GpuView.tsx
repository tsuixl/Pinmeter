import { useEffect } from "react";
import { Alert, Badge, Card, Select, StatCard } from "../../shared/ui/sakani";
import { icons } from "../../shared/ui/icons";
import { statusLabels } from "../monitoring/useMonitorViewModel";
import { TrendChart } from "../monitoring/TrendChart";
import { useGpuViewModel, type GpuKey } from "./useGpuViewModel";
import "./gpu.css";
import { usePageUiState } from "../../shared/state/page-ui-state";

export function GpuView({
  vm,
  range,
}: {
  vm: ReturnType<typeof useGpuViewModel>;
  range: number;
}) {
  const [previousRange, setPreviousRange] = usePageUiState("gpu.range", range);
  useEffect(() => {
    if (previousRange !== range) {
      vm.setAnchor(null);
      setPreviousRange(range);
    }
  }, [range, previousRange, setPreviousRange, vm.setAnchor]);
  const text = (key: GpuKey) => {
    const r = vm.reading(key);
    return r.status === "normal" ? `${r.text} ${r.unit}` : "—";
  };
  const cards: [GpuKey, string, typeof icons.gpu][] = [
    ["usage", "GPU 使用率", icons.gpu],
    [vm.temperature.key, vm.temperature.label, icons.temperature],
    ["dedicated_used", "专用 GPU 内存", icons.memory],
    ["shared_used", "共享 GPU 内存", icons.memory],
    ["core_clock", "核心频率", icons.gauge],
    ["memory_clock", "显存频率", icons.gauge],
  ];
  return (
    <section className="gpu-page" aria-label="GPU 详情">
      {vm.devices.length > 0 && (
        <div className="gpu-heading">
          <Select
            label="显卡"
            value={vm.selected?.id ?? ""}
            options={vm.devices.map((d) => ({ value: d.id, label: d.name }))}
            onChange={vm.setSelectedId}
          />
          <Badge variant="neutral">
            {vm.devices.length} 张显卡 · {statusLabels[vm.status]}
          </Badge>
        </div>
      )}
      {vm.status !== "normal" && (
        <Alert
          color="info"
          title={statusLabels[vm.status]}
          description={vm.detail}
        />
      )}
      {vm.selected && (
        <>
          <div className="gpu-stats">
            {cards.map(([key, title, icon]) => (
              <StatCard
                key={key}
                title={title}
                value={text(key)}
                variant="icon"
                icon={icon}
                description={
                  vm.reading(key).status !== "normal"
                    ? vm.reading(key).detail ||
                      statusLabels[vm.reading(key).status]
                    : key === "usage"
                      ? "最忙的 GPU 引擎"
                      : key === "dedicated_used"
                        ? `驱动报告容量 ${text("memory_total")}`
                        : key === "shared_used"
                          ? "当前借用的系统内存"
                          : key === vm.temperature.key
                            ? vm.temperature.description
                            : "驱动报告的当前频率"
                }
              />
            ))}
          </div>
          <Card className="trend-card">
            <div className="section-heading">
              <h2>GPU 使用率与温度</h2>
              <div className="legend">
                <span>
                  <i />
                  使用率
                </span>
                <span>
                  <i className="secondary" />
                  {vm.temperature.label}
                </span>
              </div>
            </div>
            <TrendChart
              key={`${vm.selected.id}-usage`}
              history={vm.history("usage", vm.temperature.key, true)}
              keys={["cpu", "cpu_temperature"]}
              range={range}
              anchor={vm.anchor}
              onAnchor={vm.setAnchor}
              label="GPU 使用率与温度"
              seriesLabels={{
                cpu: "GPU 使用率",
                cpu_temperature: vm.temperature.label,
              }}
            />
          </Card>
          <div className="gpu-charts">
            <Card className="trend-card">
              <div className="section-heading">
                <h2>GPU 内存</h2>
                <div className="legend">
                  <span>
                    <i />
                    专用
                  </span>
                  <span>
                    <i className="secondary" />
                    共享
                  </span>
                </div>
              </div>
              <TrendChart
                key={`${vm.selected.id}-memory`}
                history={vm.history("dedicated_used", "shared_used")}
                keys={["cpu", "memory"]}
                range={range}
                anchor={vm.anchor}
                onAnchor={vm.setAnchor}
                label="GPU 内存"
                seriesLabels={{ cpu: "专用内存", memory: "共享内存" }}
                axis={{
                  unit: "GiB",
                  divisor: 1073741824,
                  step: 536870912,
                  minimum: 536870912,
                }}
              />
            </Card>
            <Card className="trend-card">
              <div className="section-heading">
                <h2>GPU 频率</h2>
                <div className="legend">
                  <span>
                    <i />
                    核心
                  </span>
                  <span>
                    <i className="secondary" />
                    显存
                  </span>
                </div>
              </div>
              <TrendChart
                key={`${vm.selected.id}-clock`}
                history={vm.history("core_clock", "memory_clock")}
                keys={["cpu", "memory"]}
                range={range}
                anchor={vm.anchor}
                onAnchor={vm.setAnchor}
                label="GPU 频率"
                seriesLabels={{ cpu: "核心频率", memory: "显存频率" }}
                axis={{ unit: "MHz", divisor: 1, step: 1000, minimum: 1000 }}
              />
            </Card>
          </div>
          <div className="explanation">
            <h2>关于这张显卡</h2>
            <p>
              使用率取最忙引擎，不累加不同引擎。专用与共享内存采用 Windows D3D
              统计；容量为驱动报告值，核显可能使用预留系统内存，不将共享内存当作独立显存。
            </p>
            <p>
              未提供核心温度时，可显示另行标注的 GPU VR SoC
              温度，两者测温点不同。 频率按驱动读数显示，显存 MHz
              不等同于有效数据传输率。
            </p>
            <p>
              来源：{vm.reading("usage").source}；
              {vm.reading(vm.temperature.key).source}。GPU
              独立约每秒采样，历史随监控间隔保存。
            </p>
          </div>
        </>
      )}
    </section>
  );
}
