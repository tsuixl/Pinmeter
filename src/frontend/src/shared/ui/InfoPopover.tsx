import { useId, useLayoutEffect, useRef, type ReactNode } from "react";
import { IconButton, Popover } from "./sakani";
import { icons } from "./icons";
import "./InfoPopover.css";

/** Keep explanatory prose behind the same information control as metric details. */
export function InfoPopover({
  title,
  label = `查看${title}`,
  children,
}: {
  title: string;
  label?: string;
  children: ReactNode;
}) {
  const root = useRef<HTMLSpanElement>(null);
  const id = useId();
  useLayoutEffect(() => {
    let observed: HTMLElement | null = null;
    const position = () => {
      const panel =
        root.current?.querySelector<HTMLElement>('[role="dialog"]') ?? null;
      const trigger = root.current?.querySelector<HTMLButtonElement>("button");
      trigger?.setAttribute("aria-expanded", String(!!panel));
      if (panel !== observed) {
        resized.disconnect();
        observed = panel;
        if (panel) resized.observe(panel);
      }
      if (!panel || !trigger) return;
      panel.id = id;
      panel.setAttribute("aria-label", title);
      const anchor = trigger.getBoundingClientRect();
      const content = panel.querySelector<HTMLElement>(".info-popover-content");
      const below = window.innerHeight - anchor.bottom - 24;
      const above = anchor.top - 24;
      const useAbove = above > below;
      const room = Math.max(above, below);
      if (content) {
        const chrome = panel.offsetHeight - content.offsetHeight;
        content.style.maxHeight = `${Math.max(80, Math.min(420, window.innerHeight * 0.55, room - chrome))}px`;
      }
      const box = panel.getBoundingClientRect();
      panel.style.left = `${Math.max(16, Math.min(anchor.left + (anchor.width - box.width) / 2, window.innerWidth - box.width - 16))}px`;
      const desiredTop = useAbove
        ? anchor.top - box.height - 8
        : anchor.bottom + 8;
      panel.style.top = `${Math.max(16, Math.min(desiredTop, window.innerHeight - box.height - 16))}px`;
    };
    // Sakani owns open/close/focus. Only adapt its geometry to the desktop scroll host.
    const resized = new ResizeObserver(position);
    const mutated = new MutationObserver(position);
    if (root.current)
      mutated.observe(root.current, { childList: true, subtree: true });
    position();
    window.addEventListener("resize", position);
    document.addEventListener("scroll", position, true);
    return () => {
      resized.disconnect();
      mutated.disconnect();
      window.removeEventListener("resize", position);
      document.removeEventListener("scroll", position, true);
    };
  }, [id, title]);
  return (
    <span ref={root} className="info-popover-anchor">
      <Popover
        className="info-popover"
        title={title}
        placement="bottom"
        trigger={
          <IconButton
            type="button"
            icon={icons.info}
            variant="ghost"
            size="sm"
            aria-label={label}
            aria-haspopup="dialog"
            aria-controls={id}
          />
        }
      >
        <div
          className="explanation info-popover-content"
          role="region"
          aria-label={`${title}内容`}
          tabIndex={0}
        >
          {children}
        </div>
      </Popover>
    </span>
  );
}
