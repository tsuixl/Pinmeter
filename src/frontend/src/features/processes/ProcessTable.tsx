import {
  useLayoutEffect,
  useRef,
  useState,
  type CSSProperties,
  type ReactNode,
  type PointerEvent,
} from "react";
import type { TableColumn } from "@sakaniui/react";
import { ArrowDown, ArrowUp, ArrowUpDown, GripVertical } from "lucide-react";
import { Button, Table } from "../../shared/ui/sakani";
import type {
  ProcessDisplayRow,
  ProcessSortDirection,
  ProcessSortKey,
} from "./process-model";
import {
  clampColumnWidth,
  defaultColumnWidths,
  maximumColumnWidth,
  minimumColumnWidths,
  moveProcessColumn,
  processColumnOrder,
  type ProcessColumnKey,
} from "./process-columns";

const sortKeys: Partial<Record<ProcessColumnKey, ProcessSortKey>> = {
  name: "name",
  pid: "pid",
  cpu: "cpu",
  working_set: "memory",
};
type DropPosition = { key: ProcessColumnKey; after: boolean };
type HeaderProps = {
  column: ProcessColumnKey;
  label: string;
  width: number;
  sort: ProcessSortKey;
  direction: ProcessSortDirection;
  onSort: (key: ProcessSortKey) => void;
  onWidth: (key: ProcessColumnKey, width: number) => void;
  onMove: (key: ProcessColumnKey, offset: number) => void;
  onDragStart: (key: ProcessColumnKey) => void;
  onDragEnd: () => void;
  onTarget: (target: DropPosition) => void;
  onDrop: (target: DropPosition) => void;
  dragging: ProcessColumnKey | null;
  drop: DropPosition | null;
};
function ColumnHeader(props: HeaderProps) {
  const { column, label, width, sort, direction } = props;
  const root = useRef<HTMLDivElement>(null);
  const resize = useRef<{ x: number; width: number; pointer: number } | null>(
    null,
  );
  const ignoreClickUntil = useRef(0);
  const sortKey = sortKeys[column];
  const active = sortKey === sort;
  const next = active
    ? direction === "asc"
      ? "降序"
      : "升序"
    : sortKey === "name" || sortKey === "pid"
      ? "升序"
      : "降序";
  const Arrow = active
    ? direction === "asc"
      ? ArrowUp
      : ArrowDown
    : ArrowUpDown;
  useLayoutEffect(() => {
    if (!props.dragging && ignoreClickUntil.current === Infinity) {
      ignoreClickUntil.current = performance.now() + 300;
    }
  }, [props.dragging]);
  useLayoutEffect(() => {
    // Synchronize semantics only; React and Sakani retain ownership of all cells.
    const cell = root.current?.closest("th");
    if (!cell) return;
    cell.scope = "col";
    if (active)
      cell.setAttribute(
        "aria-sort",
        direction === "asc" ? "ascending" : "descending",
      );
    else cell.removeAttribute("aria-sort");
  }, [active, direction]);
  const targetAt = (x: number): DropPosition => {
    const bounds = root.current!.closest("th")!.getBoundingClientRect();
    return { key: column, after: x > bounds.left + bounds.width / 2 };
  };
  const endResize = (event: PointerEvent<HTMLSpanElement>, cancel = false) => {
    const start = resize.current;
    if (!start || start.pointer !== event.pointerId) return;
    resize.current = null;
    if (cancel) props.onWidth(column, start.width);
    if (event.currentTarget.hasPointerCapture(event.pointerId))
      event.currentTarget.releasePointerCapture(event.pointerId);
    ignoreClickUntil.current = performance.now() + 300;
  };
  return (
    <div
      ref={root}
      className={`process-column-header${props.dragging === column ? " is-dragging" : ""}${props.drop?.key === column ? (props.drop.after ? " drop-after" : " drop-before") : ""}`}
      data-column={column}
      onDragOver={(event) => {
        if (!props.dragging) return;
        event.preventDefault();
        event.dataTransfer.dropEffect = "move";
        props.onTarget(targetAt(event.clientX));
      }}
      onDrop={(event) => {
        if (!props.dragging) return;
        event.preventDefault();
        event.stopPropagation();
        ignoreClickUntil.current = performance.now() + 300;
        props.onDrop(targetAt(event.clientX));
      }}
    >
      <Button
        variant="ghost"
        size="sm"
        className="process-column-title"
        draggable
        aria-label={
          sortKey
            ? `${label}，${active ? `当前${direction === "asc" ? "升序" : "降序"}，` : ""}点击${next}排序`
            : `${label}，可拖动调整列顺序`
        }
        title={
          sortKey
            ? "点击排序；拖动标题调整列顺序；Alt+左右方向键移动列"
            : "拖动标题调整列顺序；Alt+左右方向键移动列"
        }
        leftIcon={<GripVertical size={14} aria-hidden="true" />}
        rightIcon={sortKey ? <Arrow size={14} aria-hidden="true" /> : undefined}
        onClick={() => {
          if (sortKey && performance.now() >= ignoreClickUntil.current)
            props.onSort(sortKey);
        }}
        onKeyDown={(event) => {
          if (
            event.altKey &&
            (event.key === "ArrowLeft" || event.key === "ArrowRight")
          ) {
            event.preventDefault();
            props.onMove(column, event.key === "ArrowLeft" ? -1 : 1);
          }
        }}
        onDragStart={(event) => {
          if (resize.current) {
            event.preventDefault();
            return;
          }
          event.dataTransfer.effectAllowed = "move";
          event.dataTransfer.setData("text/plain", column);
          ignoreClickUntil.current = Infinity;
          props.onDragStart(column);
        }}
        onDragEnd={() => {
          ignoreClickUntil.current = performance.now() + 300;
          props.onDragEnd();
        }}
      >
        <span className="process-column-label">{label}</span>
      </Button>
      <span
        role="separator"
        tabIndex={0}
        aria-orientation="vertical"
        aria-label={`调整${label}列宽`}
        aria-valuemin={minimumColumnWidths[column]}
        aria-valuemax={maximumColumnWidth}
        aria-valuenow={width}
        aria-valuetext={`${width} 像素`}
        className="process-column-resize"
        title="拖动调整列宽；左右方向键微调，Home 恢复最小宽度"
        draggable={false}
        onClick={(event) => event.stopPropagation()}
        onPointerDown={(event) => {
          if (event.button !== 0) return;
          event.preventDefault();
          event.stopPropagation();
          event.currentTarget.focus();
          resize.current = {
            x: event.clientX,
            width,
            pointer: event.pointerId,
          };
          event.currentTarget.setPointerCapture(event.pointerId);
        }}
        onPointerMove={(event) => {
          const start = resize.current;
          if (start?.pointer === event.pointerId)
            props.onWidth(column, start.width + event.clientX - start.x);
        }}
        onPointerUp={(event) => endResize(event)}
        onPointerCancel={(event) => endResize(event, true)}
        onLostPointerCapture={() => {
          resize.current = null;
        }}
        onKeyDown={(event) => {
          if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
            event.preventDefault();
            event.stopPropagation();
            props.onWidth(
              column,
              width +
                (event.key === "ArrowLeft" ? -1 : 1) *
                  (event.shiftKey ? 48 : 16),
            );
          } else if (event.key === "Home") {
            event.preventDefault();
            props.onWidth(column, minimumColumnWidths[column]);
          }
        }}
      />
    </div>
  );
}

export function ProcessTable({
  columns,
  rows,
  sort,
  direction,
  onSort,
}: {
  columns: TableColumn<ProcessDisplayRow>[];
  rows: ProcessDisplayRow[];
  sort: ProcessSortKey;
  direction: ProcessSortDirection;
  onSort: (key: ProcessSortKey) => void;
}) {
  const scroller = useRef<HTMLDivElement>(null);
  const [order, setOrder] = useState<ProcessColumnKey[]>([
    ...processColumnOrder,
  ]);
  const [widths, setWidths] = useState(defaultColumnWidths);
  const [dragging, setDragging] = useState<ProcessColumnKey | null>(null);
  const draggingRef = useRef<ProcessColumnKey | null>(null);
  const [drop, setDrop] = useState<DropPosition | null>(null);
  const reset = () => {
    setOrder([...processColumnOrder]);
    setWidths(
      defaultColumnWidths(
        Math.max(0, (scroller.current?.clientWidth ?? 1002) - 2),
      ),
    );
  };
  useLayoutEffect(() => {
    setWidths(
      defaultColumnWidths(
        Math.max(0, (scroller.current?.clientWidth ?? 1002) - 2),
      ),
    );
  }, []);
  const clearDrag = () => {
    draggingRef.current = null;
    setDragging(null);
    setDrop(null);
  };
  const adapted: Array<
    Omit<TableColumn<ProcessDisplayRow>, "header"> & { header: ReactNode }
  > = order.map((key) => {
    const column = columns.find((c) => c.key === key)!;
    return {
      ...column,
      width: `${widths[key]}px`,
      header: (
        <ColumnHeader
          key={key}
          column={key}
          label={column.header}
          width={widths[key]}
          sort={sort}
          direction={direction}
          onSort={onSort}
          dragging={dragging}
          drop={drop}
          onWidth={(key, value) =>
            setWidths((current) => ({
              ...current,
              [key]: clampColumnWidth(key, value),
            }))
          }
          onMove={(key, offset) =>
            setOrder((current) => {
              const target = current[current.indexOf(key) + offset];
              return target
                ? moveProcessColumn(current, key, target, offset > 0)
                : current;
            })
          }
          onDragStart={(key) => {
            draggingRef.current = key;
            setDragging(key);
          }}
          onDragEnd={clearDrag}
          onTarget={setDrop}
          onDrop={(target) => {
            const source = draggingRef.current;
            if (source)
              setOrder((current) =>
                moveProcessColumn(current, source, target.key, target.after),
              );
            clearDrag();
          }}
        />
      ),
    };
  });
  return (
    <div className="process-table-container">
      <div className="process-table-tools">
        <p className="muted">点击表头排序，拖动标题换列，拖动列边界调宽。</p>
        <Button variant="ghost" size="sm" onClick={reset}>
          恢复默认列
        </Button>
      </div>
      <div
        ref={scroller}
        className="process-table-scroll"
        role="region"
        aria-label="可调整列的进程表格"
        tabIndex={0}
        style={
          {
            "--process-table-width": `${Object.values(widths).reduce((a, b) => a + b, 0)}px`,
          } as CSSProperties
        }
      >
        {/* Pinned Sakani 0.3.1 renders header nodes directly in th. Its declaration
          incorrectly narrows them to string. Keep this single compatibility cast;
          recheck column interactions when upgrading, and disable stacked headers. */}
        <Table<ProcessDisplayRow>
          responsive="default"
          className="process-interactive-table"
          columns={adapted as TableColumn<ProcessDisplayRow>[]}
          rows={rows}
          rowKey={(row) => row.id}
        />
      </div>
    </div>
  );
}
