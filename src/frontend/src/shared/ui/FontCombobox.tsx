import { useLayoutEffect, useRef, useState } from "react";
import { Combobox, Input, type ComboboxProps } from "@sakaniui/react";

/** Keep the font picker readable even when a symbol font is selected. */
export function FontCombobox(props: ComboboxProps) {
  const root = useRef<HTMLDivElement>(null);
  const [query, setQuery] = useState("");
  const matches = props.options.filter((option) =>
    option.label.toLocaleLowerCase().includes(query.trim().toLocaleLowerCase()),
  );
  const selected = props.options.find((option) => option.value === props.value);
  const select = (value: string | string[]) => {
    setQuery("");
    props.onChange?.(value);
  };
  useLayoutEffect(() => {
    const control = root.current?.querySelector('[role="combobox"]');
    if (!control) return;
    control.setAttribute("aria-label", props.label ?? "界面字体");
  }, [props.label]);
  return (
    <div ref={root} className="font-search-field">
      <Input
        label="搜索字体"
        aria-label="搜索字体"
        placeholder="输入中文或英文字体名称…"
        value={query}
        disabled={props.disabled}
        onChange={(event) => {
          setQuery(event.target.value);
          const trigger =
            root.current?.querySelector<HTMLElement>('[role="combobox"]');
          if (trigger?.getAttribute("aria-expanded") === "false")
            trigger.click();
        }}
        onKeyDown={(event) => {
          if (event.key === "Enter") {
            const first = matches.find((option) => !option.disabled);
            if (first) {
              event.preventDefault();
              select(first.value);
              const trigger =
                root.current?.querySelector<HTMLElement>('[role="combobox"]');
              if (trigger?.getAttribute("aria-expanded") === "true")
                trigger.click();
            }
          } else if (event.key === "Escape") {
            setQuery("");
          }
        }}
      />
      <Combobox
        {...props}
        options={matches}
        value={
          matches.some((option) => option.value === props.value)
            ? props.value
            : ""
        }
        placeholder={selected?.label ?? props.placeholder}
        description={
          query ? `找到 ${matches.length} 项字体` : props.description
        }
        onChange={select}
      />
    </div>
  );
}
