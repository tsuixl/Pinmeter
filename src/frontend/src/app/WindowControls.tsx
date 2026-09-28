import { Copy, Minus, Square, X } from "lucide-react";
import { IconButton } from "../shared/ui/sakani";
import type { useWindowViewModel } from "./useWindowViewModel";

export function WindowControls({
  vm,
}: {
  vm: ReturnType<typeof useWindowViewModel>;
}) {
  if (!vm.native || vm.mac) return null;
  return (
    <div className="window-controls" role="group" aria-label="窗口控制">
      <IconButton
        icon={Minus}
        variant="ghost"
        size="sm"
        aria-label="最小化"
        disabled={vm.pending}
        onClick={() => void vm.perform("minimize")}
      />
      <IconButton
        icon={vm.maximized ? Copy : Square}
        variant="ghost"
        size="sm"
        aria-label={vm.maximized ? "还原窗口" : "最大化"}
        disabled={vm.pending}
        onClick={() => void vm.perform("toggleMaximize")}
      />
      <IconButton
        icon={X}
        variant="ghost"
        size="sm"
        aria-label="关闭"
        disabled={vm.pending}
        onClick={() => void vm.perform("close")}
      />
    </div>
  );
}
