import { useId, type CSSProperties } from "react";
import { createPortal } from "react-dom";
import { Card } from "../../shared/ui/sakani";
import type {
  ProcessorDto,
  ReadingStatus,
} from "../../shared/contracts/monitor";
import { statusLabels } from "./useMonitorViewModel";
import { cpuUsageStrength, useCpuProcessorChart } from "./useCpuProcessorChart";

export function CpuProcessorList({
  processors,
  status,
  detail,
}: {
  processors: ProcessorDto[];
  status: ReadingStatus;
  detail: string;
}) {
  const vm = useCpuProcessorChart(processors);
  const tooltipId = useId();
  const captionId = useId();
  const current = vm.current;
  const currentValid = current && cpuUsageStrength(current) !== null;
  return (
    <Card className="cpu-processors">
      <div className="section-heading">
        <h2>逻辑处理器</h2>
        <span className="processor-caption">
          当前占用{processors.length > 0 ? ` · ${processors.length} 个` : ""}
        </span>
      </div>
      <p id={captionId} className="processor-caption">
        每柱一个逻辑处理器，按编号排列。左右滚动查看更多。
      </p>
      {status !== "normal" && (
        <p className="processor-caption">
          {statusLabels[status]}
          {detail ? ` · ${detail}` : ""}
        </p>
      )}
      <div className="cpu-bar-chart">
        {processors.length > 0 && (
          <div className="cpu-bar-axis" aria-hidden="true">
            {[100, 75, 50, 25, 0].map((value) => (
              <span
                key={value}
                style={{ top: `${24 + (100 - value) * 2.64}px` }}
              >
                {value}%
              </span>
            ))}
          </div>
        )}
        <div
          ref={vm.container}
          className="processor-scroll"
          role="group"
          aria-label="逻辑处理器占用率柱状图"
          aria-describedby={captionId}
        >
          {processors.length > 0 && (
            <div
              className="cpu-bar-grid"
              style={{ "--cpu-count": processors.length } as CSSProperties}
            >
              {processors.map((processor, index) => {
                const strength = cpuUsageStrength(processor);
                const value =
                  strength === null
                    ? statusLabels[processor.usage.status]
                    : `${processor.usage.text} %`;
                return (
                  <button
                    type="button"
                    key={processor.id}
                    className="cpu-bar-column"
                    data-processor-id={processor.id}
                    data-valid={strength !== null}
                    data-active={vm.active?.id === processor.id && !!current}
                    style={
                      {
                        "--cpu-strength": strength ?? 0,
                        "--cpu-usage":
                          strength === null
                            ? 0
                            : Math.max(
                                0,
                                Math.min(100, processor.usage.value!),
                              ),
                      } as CSSProperties
                    }
                    aria-label={`逻辑处理器 ${processor.id}，${value}`}
                    aria-describedby={
                      current?.id === processor.id ? tooltipId : undefined
                    }
                    tabIndex={vm.tabId === processor.id ? 0 : -1}
                    onMouseMove={(event) => {
                      const target = event.currentTarget;
                      vm.setActive((previous) =>
                        previous?.id === processor.id &&
                        previous.kind === "pointer"
                          ? previous
                          : { id: processor.id, target, kind: "pointer" },
                      );
                    }}
                    onMouseLeave={() =>
                      vm.setActive((previous) =>
                        previous?.kind === "pointer" &&
                        previous.id === processor.id
                          ? null
                          : previous,
                      )
                    }
                    onFocus={(event) => {
                      vm.setFocusedId(processor.id);
                      vm.setActive({
                        id: processor.id,
                        target: event.currentTarget,
                        kind: "focus",
                      });
                    }}
                    onBlur={() =>
                      vm.setActive((previous) =>
                        previous?.kind === "focus" &&
                        previous.id === processor.id
                          ? null
                          : previous,
                      )
                    }
                    onClick={(event) =>
                      vm.setActive({
                        id: processor.id,
                        target: event.currentTarget,
                        kind: "focus",
                      })
                    }
                    onKeyDown={(event) => {
                      if (event.key === "Escape") {
                        vm.setActive(null);
                        event.preventDefault();
                      } else if (vm.move(index, event.key))
                        event.preventDefault();
                    }}
                  >
                    <span className="cpu-bar-plot" aria-hidden="true">
                      <span className="cpu-bar-fill" />
                      <span className="cpu-bar-value number">
                        {strength === null ? "—" : processor.usage.text + "%"}
                      </span>
                    </span>
                    <span className="cpu-bar-label" aria-hidden="true">
                      CPU {processor.id}
                    </span>
                  </button>
                );
              })}
            </div>
          )}
        </div>
      </div>
      {current &&
        createPortal(
          <div
            ref={vm.tooltip}
            id={tooltipId}
            role="tooltip"
            className="cpu-bar-tooltip"
            style={vm.position}
          >
            <span className="cpu-bar-tooltip-title">
              逻辑处理器 {current.id}
            </span>
            <div className="cpu-bar-tooltip-row">
              <i
                className={currentValid ? "" : "unavailable"}
                aria-hidden="true"
              />
              <span>{currentValid ? "占用率" : "状态"}</span>
              <strong className="number">
                {currentValid
                  ? `${current.usage.text} %`
                  : statusLabels[current.usage.status]}
              </strong>
            </div>
            {!currentValid && current.usage.detail && (
              <span className="cpu-bar-tooltip-detail">
                {current.usage.detail}
              </span>
            )}
          </div>,
          document.body,
        )}
    </Card>
  );
}
