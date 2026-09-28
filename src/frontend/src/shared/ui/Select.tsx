import { useLayoutEffect, useRef } from "react";
import { Select as SakaniSelect, type SelectProps } from "@sakaniui/react";

/** Sakani 0.3.1 labels a div with htmlFor, which does not name its combobox.
 * Add the accessible name without changing its markup, styles or interaction. */
export function Select(props: SelectProps) {
  const root = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    root.current
      ?.querySelector('[role="combobox"]')
      ?.setAttribute(
        "aria-label",
        props.label ?? props.placeholder ?? "选择选项",
      );
  }, [props.label, props.placeholder]);
  return (
    <div ref={root}>
      <SakaniSelect {...props} />
    </div>
  );
}
