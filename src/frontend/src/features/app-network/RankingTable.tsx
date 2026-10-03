import { type MouseEvent, useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import type { TableColumn } from "@sakaniui/react";
import { ArrowDown, ArrowUp, ArrowUpDown } from "lucide-react";
import { Button, Table } from "../../shared/ui/sakani";
import type {
  AppNetworkSort,
  DisplayRow,
  SortKey,
} from "./useAppNetworkViewModel";

const labels: Record<SortKey, string> = {
  name: "应用名称",
  download: "下载速度",
  upload: "上传速度",
};
const sortable = (key: string): key is SortKey => key in labels;

// Sakani 0.3.1 has string-only headers and no header render/attribute slots.
// Portals add official buttons to empty headers without replacing the table.
// Keep the default table layout so headers remain available on narrow screens.
export function RankingTable({
  columns,
  rows,
  sort,
  onSort,
  onContextMenu,
}: {
  columns: TableColumn<DisplayRow>[];
  rows: DisplayRow[];
  sort: AppNetworkSort;
  onSort: (key: SortKey) => void;
  onContextMenu?: (row: DisplayRow, event: MouseEvent) => void;
}) {
  const root = useRef<HTMLDivElement>(null);
  const [headers, setHeaders] = useState<
    Array<{ cell: HTMLTableCellElement; slot: HTMLSpanElement | null }>
  >([]);
  useLayoutEffect(() => {
    const cells = Array.from(
      root.current!.querySelectorAll<HTMLTableCellElement>("thead th"),
    );
    // React owns the table cell's header text, even when it is an empty string.
    // Mount each portal into its own leaf rather than competing for the cell's children.
    const slots = cells.map((cell, index) => {
      const slot = sortable(columns[index]?.key ?? "")
        ? document.createElement("span")
        : null;
      if (slot) cell.append(slot);
      return { cell, slot };
    });
    setHeaders(slots);
    return () => slots.forEach(({ slot }) => slot?.remove());
  }, []);
  useLayoutEffect(() => {
    headers.forEach(({ cell: header }, index) => {
      header.scope = "col";
      if (columns[index].key === sort.key)
        header.setAttribute(
          "aria-sort",
          sort.order === "asc" ? "ascending" : "descending",
        );
      else header.removeAttribute("aria-sort");
    });
  }, [headers, columns, sort]);
  return (
    <div
      ref={root}
      className="app-network-table"
      onContextMenu={(event) => {
        const row = (event.target as HTMLElement).closest(
          "tbody tr",
        ) as HTMLTableRowElement | null;
        if (row && root.current?.contains(row) && rows[row.sectionRowIndex])
          onContextMenu?.(rows[row.sectionRowIndex], event);
      }}
    >
      <Table<DisplayRow>
        responsive="default"
        rowKey={(row) => row.id}
        rows={rows}
        columns={columns.map((column) =>
          sortable(column.key) ? { ...column, header: "" } : column,
        )}
      />
      {headers.map(({ slot }, index) => {
        const key = columns[index].key;
        if (!sortable(key) || !slot) return null;
        const active = sort.key === key;
        const Icon = active
          ? sort.order === "asc"
            ? ArrowUp
            : ArrowDown
          : ArrowUpDown;
        const next = active
          ? sort.order === "asc"
            ? "降序"
            : "升序"
          : key === "name"
            ? "升序"
            : "降序";
        return createPortal(
          <Button
            variant="ghost"
            size="sm"
            className="app-network-sort"
            aria-label={`${labels[key]}，点击${next}排序`}
            title={`${labels[key]}，点击${next}排序`}
            rightIcon={<Icon size={14} aria-hidden="true" />}
            onClick={() => onSort(key)}
          >
            {labels[key]}
          </Button>,
          slot,
          key,
        );
      })}
    </div>
  );
}
