import { useId, useRef, useState } from "react";
import type { FrameDto } from "../../shared/contracts/monitor";
import {
  chartPaths,
  clampAnchor,
  type MetricKey,
  type ChartSeries,
  networkCeiling,
  areaPath,
  temperatureDomain,
} from "./chart";
import { Button } from "../../shared/ui/sakani";
import { statusLabels } from "./useMonitorViewModel";
const labels: Record<MetricKey, string> = {
  cpu: "CPU",
  cpu_temperature: "CPU 温度",
  memory: "内存",
  download: "下载",
  upload: "上传",
};
export function TrendChart({
  history,
  keys = [],
  series,
  range,
  anchor,
  onAnchor,
  label,
  seriesLabels,
  axis,
}: {
  history: FrameDto[];
  keys?: MetricKey[];
  series?: ChartSeries[];
  range: number;
  anchor: number | null;
  onAnchor: (value: number | null) => void;
  label: string;
  seriesLabels?: Partial<Record<MetricKey, string>>;
  axis?: { unit: string; divisor: number; step: number; minimum: number };
}) {
  const items: ChartSeries[] =
    series ??
    keys.map((key, i) => ({
      id: key,
      label: seriesLabels?.[key] ?? labels[key],
      color: `var(--color-chart-${i ? 2 : 5})`,
      temperature: key === "cpu_temperature",
      network: key === "download" || key === "upload",
      read: (frame) => frame[key],
    }));
  const id = useId().replace(/:/g, "");
  const last = history.at(-1)?.elapsed_ms ?? range;
  const first = history[0]?.elapsed_ms ?? 0;
  const end = anchor === null ? last : clampAnchor(anchor, first, last, range);
  const start = end - range;
  const visible = history.filter(
    (f) => f.elapsed_ms >= start && f.elapsed_ms <= end,
  );
  const valid = visible.filter((f) =>
    items.some(
      (item) => item.read(f).status === "normal" && item.read(f).value !== null,
    ),
  );
  const values = valid.flatMap((f) =>
    items.flatMap((item) => {
      const reading = item.read(f);
      return reading.status === "normal" && reading.value !== null
        ? [reading.value]
        : [];
    }),
  );
  const network = items.some((item) => item.network);
  const ceiling = axis
    ? Math.ceil(Math.max(axis.minimum, ...values) / axis.step) * axis.step
    : network
      ? networkCeiling(values)
      : 100;
  const hasTemperature = items.some((item) => item.temperature);
  const [temperatureFloor, temperatureCeiling] = temperatureDomain(
    visible.flatMap((frame) =>
      items
        .filter((item) => item.temperature)
        .flatMap((item) => {
          const reading = item.read(frame);
          return reading.status === "normal" && reading.value !== null
            ? [reading.value]
            : [];
        }),
    ),
  );
  const bounds = (item: ChartSeries) =>
    item.temperature ? [temperatureFloor, temperatureCeiling] : [0, ceiling];
  const divisor =
    axis?.divisor ??
    (network ? (ceiling >= 1e6 ? 1e6 : ceiling >= 1000 ? 1000 : 1) : 1);
  const unit =
    axis?.unit ??
    (network
      ? divisor === 1e6
        ? "MB/s"
        : divisor === 1000
          ? "KB/s"
          : "B/s"
      : "%");
  const drag = useRef<{ x: number; end: number } | null>(null);
  const [hoverAt, setHoverAt] = useState<number | null>(null);
  const nearest =
    hoverAt === null
      ? null
      : valid.reduce<FrameDto | null>(
          (a, b) =>
            !a ||
            Math.abs(b.elapsed_ms - hoverAt) < Math.abs(a.elapsed_ms - hoverAt)
              ? b
              : a,
          null,
        );
  const hover =
    nearest &&
    hoverAt !== null &&
    Math.abs(nearest.elapsed_ms - hoverAt) <= 5000
      ? nearest
      : null;
  const hoverX = hover ? (hover.elapsed_ms - start) / range : 0;
  const wall = (elapsed: number) =>
    new Date(
      (history.at(-1)?.at_ms ?? Date.now()) - (last - elapsed),
    ).toLocaleTimeString([], { hour12: false });
  const emptyStatus = items
    .map(
      (item) =>
        statusLabels[
          history.length
            ? item.read(history[history.length - 1]).status
            : "warming"
        ],
    )
    .filter((s, i, a) => a.indexOf(s) === i)
    .join(" / ");
  return (
    <div className={`trend${hasTemperature ? " dual-axis" : ""}`}>
      <div className="chart-axis">
        {[1, 0.75, 0.5, 0.25, 0].map((fraction, i) => (
          <span key={fraction} style={{ top: `${(1 - fraction) * 100}%` }}>
            {Number(((ceiling * fraction) / divisor).toFixed(2))}
            {i === 0 ? ` ${unit}` : ""}
          </span>
        ))}
      </div>
      <div className="chart-body">
        <svg
          viewBox="0 0 1000 180"
          preserveAspectRatio="none"
          className="chart-svg"
          role="img"
          aria-label={`${label}，左右拖动或方向键查看历史`}
          tabIndex={0}
          onPointerDown={(e) => {
            drag.current = { x: e.clientX, end };
            e.currentTarget.setPointerCapture(e.pointerId);
          }}
          onPointerUp={() => {
            drag.current = null;
          }}
          onPointerCancel={() => {
            drag.current = null;
            setHoverAt(null);
          }}
          onPointerLeave={() => setHoverAt(null)}
          onBlur={() => setHoverAt(null)}
          onPointerMove={(e) => {
            const rect = e.currentTarget.getBoundingClientRect();
            if (drag.current) {
              onAnchor(
                clampAnchor(
                  drag.current.end -
                    ((e.clientX - drag.current.x) / rect.width) * range,
                  first,
                  last,
                  range,
                ),
              );
              setHoverAt(null);
            } else
              setHoverAt(
                start +
                  Math.max(
                    0,
                    Math.min(1, (e.clientX - rect.left) / rect.width),
                  ) *
                    range,
              );
          }}
          onKeyDown={(e) => {
            if (["ArrowLeft", "ArrowRight", "End"].includes(e.key)) {
              e.preventDefault();
              setHoverAt(null);
              onAnchor(
                e.key === "End"
                  ? null
                  : clampAnchor(
                      end + (e.key === "ArrowLeft" ? -10000 : 10000),
                      first,
                      last,
                      range,
                    ),
              );
            }
          }}
        >
          <defs>
            {items.map((item) => (
              <linearGradient
                id={`${id}-${item.id}`}
                key={item.id}
                x1="0"
                y1="0"
                x2="0"
                y2="1"
              >
                <stop
                  offset="5%"
                  stopColor={item.color}
                  stopOpacity={items.length > 2 ? ".10" : ".35"}
                />
                <stop offset="95%" stopColor={item.color} stopOpacity=".02" />
              </linearGradient>
            ))}
          </defs>
          {[0, 45, 90, 135, 180].map((y) => (
            <line
              key={y}
              x1="0"
              x2="1000"
              y1={y}
              y2={y}
              className="grid-line"
            />
          ))}
          {items
            .map((item) =>
              chartPaths(
                history,
                item,
                start,
                end,
                bounds(item)[1],
                bounds(item)[0],
              ).map((path, n) => (
                <g key={`${item.id}-${n}`} data-metric={item.id}>
                  <path d={areaPath(path)} fill={`url(#${id}-${item.id})`} />
                  <path
                    d={path}
                    className="series"
                    style={{ stroke: item.color }}
                    vectorEffect="non-scaling-stroke"
                  />
                </g>
              )),
            )
            .reverse()}
          {hover && (
            <line
              x1={hoverX * 1000}
              x2={hoverX * 1000}
              y1="0"
              y2="180"
              className="chart-crosshair"
              vectorEffect="non-scaling-stroke"
            />
          )}
        </svg>
        {hover && (
          <>
            {items.map((item) =>
              item.read(hover).status === "normal" &&
              item.read(hover).value !== null ? (
                <span
                  key={item.id}
                  className="active-point"
                  style={{
                    background: item.color,
                    left: `${hoverX * 100}%`,
                    top: `${(1 - (item.read(hover).value! - bounds(item)[0]) / (bounds(item)[1] - bounds(item)[0])) * 148}px`,
                  }}
                />
              ) : null,
            )}
            <div
              className="chart-tooltip"
              role="tooltip"
              style={{
                left: `${hoverX * 100}%`,
                transform:
                  hoverX > 0.55
                    ? "translateX(calc(-100% - 8px))"
                    : "translateX(8px)",
              }}
            >
              <span>{wall(hover.elapsed_ms)}</span>
              {items.map((item) => (
                <div className="tooltip-row" key={item.id}>
                  <i
                    className="tooltip-dot"
                    style={{ background: item.color }}
                  />
                  <span>{item.label}</span>
                  <strong>
                    {item.read(hover).status === "normal"
                      ? `${item.read(hover).text} ${item.read(hover).unit}`
                      : `— ${statusLabels[item.read(hover).status]}`}
                  </strong>
                </div>
              ))}
            </div>
          </>
        )}
        {!values.length && (
          <div className="chart-empty">
            <strong>
              {items.length ? "等待有效采样" : "选择图例以显示趋势"}
            </strong>
            {items.length > 0 && <span>{emptyStatus} · 不显示为零值</span>}
          </div>
        )}
        <div className="chart-times">
          {[0, 0.25, 0.5, 0.75, 1].map((f) => (
            <span key={f}>
              {f === 1 && anchor === null ? "现在" : wall(start + f * range)}
            </span>
          ))}
        </div>
      </div>
      {hasTemperature && (
        <div
          className="chart-axis temperature-axis"
          aria-label="温度刻度，摄氏度"
        >
          {[1, 0.75, 0.5, 0.25, 0].map((fraction, i) => (
            <span key={fraction} style={{ top: `${(1 - fraction) * 100}%` }}>
              {temperatureFloor +
                (temperatureCeiling - temperatureFloor) * fraction}
              {i === 0 ? " °C" : ""}
            </span>
          ))}
        </div>
      )}
      <div className="chart-hint">
        <span>
          {hover ? wall(hover.elapsed_ms) : "拖动查看历史 · 缺失数据保留空白"}
        </span>
        {anchor !== null && (
          <Button variant="ghost" size="sm" onClick={() => onAnchor(null)}>
            返回实时
          </Button>
        )}
      </div>
    </div>
  );
}
