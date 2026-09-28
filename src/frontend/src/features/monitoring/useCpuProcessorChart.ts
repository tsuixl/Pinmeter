import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { ProcessorDto } from "../../shared/contracts/monitor";

export function cpuUsageStrength(processor: ProcessorDto) {
  const { status, value } = processor.usage;
  return status === "normal" && value !== null && Number.isFinite(value)
    ? 0.12 + (Math.max(0, Math.min(100, value)) / 100) * 0.88
    : null;
}

export function useCpuProcessorChart(processors: ProcessorDto[]) {
  const container = useRef<HTMLDivElement>(null);
  const tooltip = useRef<HTMLDivElement>(null);

  const [focusedId, setFocusedId] = useState<string | null>(null);
  const [active, setActive] = useState<{
    id: string;
    target: HTMLButtonElement;
    kind: "pointer" | "focus";
  } | null>(null);
  const [position, setPosition] = useState({ left: 0, top: 0 });

  const current = processors.find((processor) => processor.id === active?.id);
  const tabId = processors.some((processor) => processor.id === focusedId)
    ? focusedId
    : processors[0]?.id;

  useLayoutEffect(() => {
    const node = container.current;
    if (!node) return;
    const observer = new ResizeObserver(() => {
      setActive(null);
    });
    observer.observe(node);
    return () => observer.disconnect();
  }, []);

  useEffect(() => {
    const onScroll = () =>
      setActive((previous) => {
        if (!previous) return null;
        const rect = previous.target.getBoundingClientRect();
        const bounds = container.current?.getBoundingClientRect();
        if (
          !bounds ||
          rect.right <= Math.max(0, bounds.left) ||
          rect.left >= Math.min(window.innerWidth, bounds.right) ||
          rect.bottom <= Math.max(0, bounds.top) ||
          rect.top >= Math.min(window.innerHeight, bounds.bottom)
        )
          return null;
        return (previous.kind === "focus" &&
          document.activeElement === previous.target) ||
          previous.target.matches(":hover")
          ? { ...previous }
          : null;
      });
    const close = () => setActive(null);
    window.addEventListener("scroll", onScroll, true);
    window.addEventListener("resize", close);
    return () => {
      window.removeEventListener("scroll", onScroll, true);
      window.removeEventListener("resize", close);
    };
  }, []);

  useLayoutEffect(() => {
    if (!current || !active?.target.isConnected || !tooltip.current) return;
    const cell = (
      active.target.querySelector(".cpu-bar-value") ?? active.target
    ).getBoundingClientRect();
    const box = tooltip.current.getBoundingClientRect();
    const left = Math.max(
      8,
      Math.min(
        cell.left + cell.width / 2 - box.width / 2,
        window.innerWidth - box.width - 8,
      ),
    );
    const top =
      cell.top >= box.height + 8
        ? cell.top - box.height - 8
        : Math.min(cell.bottom + 8, window.innerHeight - box.height - 8);
    setPosition({ left, top: Math.max(8, top) });
  }, [
    active,
    current?.usage.text,
    current?.usage.status,
    current?.usage.detail,
    current?.id,
  ]);

  const move = (index: number, key: string) => {
    const next =
      key === "ArrowRight"
        ? index + 1
        : key === "ArrowLeft"
          ? index - 1
          : key === "Home"
            ? 0
            : key === "End"
              ? processors.length - 1
              : null;
    if (next === null) return false;
    const bounded = Math.max(0, Math.min(processors.length - 1, next));
    container.current
      ?.querySelectorAll<HTMLButtonElement>(".cpu-bar-column")
      [bounded]?.focus();
    return true;
  };

  return {
    container,
    tooltip,

    active,
    current,
    position,
    tabId,
    setActive,
    setFocusedId,
    move,
  };
}
