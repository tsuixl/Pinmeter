import { createRoot } from "react-dom/client";
import { Badge, Button, Card, Alert } from "../../shared/ui/sakani";
import {
  chartPaths,
  networkCeiling,
  type MetricKey,
} from "../monitoring/chart";
import { statusLabels } from "../monitoring/useMonitorViewModel";
import {
  createTrayPanelClient,
  type TrayPage,
  type TrayPanelClient,
} from "../../shared/client/tray-panel-client";
import type { TraySnapshotDto } from "../../shared/contracts/monitor";
import { useTrayPanelViewModel } from "./useTrayPanelViewModel";
import { applyFont, defaultFontFamily, loadFont } from "../../shared/ui/fonts";
import "./tray-panel.css";

const metrics: { key: MetricKey; label: string; page: TrayPage }[] = [
  { key: "cpu", label: "CPU", page: "cpu" },
  { key: "memory", label: "内存", page: "memory" },
  { key: "download", label: "下载", page: "network" },
  { key: "upload", label: "上传", page: "network" },
];
function TrayPanel({
  client,
  initial,
  initialError,
}: {
  client: TrayPanelClient;
  initial: TraySnapshotDto;
  initialError: string;
}) {
  const vm = useTrayPanelViewModel(client, initial, initialError);
  const last = vm.data.history.at(-1)?.elapsed_ms ?? 60000;
  return (
    <main className="tray-panel" aria-label="Pinmeter 快捷面板">
      <header>
        <strong>Pinmeter</strong>
        <Badge variant={vm.demo ? "warning" : "neutral"}>
          {vm.demo ? "演示数据" : "最近 1 分钟"}
        </Badge>
        <Button size="sm" variant="ghost" onClick={() => void vm.hide()}>
          收起
        </Button>
      </header>
      {vm.error ? (
        <Alert
          color="warning"
          title="暂时无法读取监控"
          description={vm.error}
        />
      ) : (
        <div className="tray-panel-grid">
          {metrics.map(({ key, label, page }) => {
            const reading = vm.data.frame?.[key];
            const network = key === "download" || key === "upload";
            const ceiling = network
              ? networkCeiling(
                  vm.data.history.flatMap((f) =>
                    f[key].status === "normal" && f[key].value !== null
                      ? [f[key].value!]
                      : [],
                  ),
                )
              : 100;
            const paths = chartPaths(
              vm.data.history,
              key,
              last - 60000,
              last,
              ceiling,
            );
            return (
              <Card key={key} className="tray-panel-stat">
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() => void vm.open(page)}
                  aria-label={`打开${label}详情`}
                >
                  <span className="tray-panel-reading">
                    <span>{label}</span>
                    <strong>
                      {reading?.text ?? "—"}{" "}
                      <small>{reading?.unit ?? (network ? "B/s" : "%")}</small>
                    </strong>
                  </span>
                </Button>
                <svg
                  viewBox="0 0 1000 180"
                  role="img"
                  aria-label={`${label}最近一分钟趋势，缺失区间留空`}
                  preserveAspectRatio="none"
                >
                  {paths.map((path, i) => (
                    <path
                      key={i}
                      d={path}
                      fill="none"
                      stroke="var(--color-chart-5)"
                      strokeWidth="2"
                      vectorEffect="non-scaling-stroke"
                    />
                  ))}
                </svg>
                <span className="processor-caption">
                  {statusLabels[reading?.status ?? "warming"]}
                </span>
              </Card>
            );
          })}
        </div>
      )}
      <footer>
        <span className="processor-caption" title={vm.data.network}>
          {vm.data.network}
        </span>
        <Button
          size="sm"
          variant="secondary"
          onClick={() => void vm.open("overview")}
        >
          打开总览
        </Button>
      </footer>
    </main>
  );
}
export async function startTrayPanel() {
  const client = await createTrayPanelClient();
  let initialError = "";
  const initial = await client.read().catch((error): TraySnapshotDto => {
    initialError = String(error);
    return {
      frame: null,
      history: [],
      theme: "system",
      font_family: defaultFontFamily,
      font_style: "auto",
      network: "暂无监控数据",
    };
  });
  try {
    await loadFont(
      initial.font_family,
      initial.font_style,
      await client.fonts(),
    );
    applyFont(initial.font_family, initial.font_style);
  } catch {
    applyFont(defaultFontFamily);
  }
  createRoot(document.getElementById("root")!).render(
    <TrayPanel client={client} initial={initial} initialError={initialError} />,
  );
}
