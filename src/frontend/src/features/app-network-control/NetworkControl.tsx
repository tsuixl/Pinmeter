import { useLayoutEffect, useRef, useState } from "react";
import { createPortal } from "react-dom";
import { MoreHorizontal, Gauge, Unplug, Plug, RotateCcw } from "lucide-react";
import {
  Badge,
  Button,
  Input,
  Menu,
  MenuItem,
  Modal,
  Select,
} from "../../shared/ui/sakani";
import { formatTraffic } from "../app-network/useAppNetworkViewModel";
import {
  controllable,
  type ControlViewModel,
  type LimitField,
  type Target,
} from "./useNetworkControlViewModel";
import "./network-control.css";

export function RuleStatus({ vm, id }: { vm: ControlViewModel; id: string }) {
  const rule = vm.rule(id);
  if (vm.pendingId === id || rule?.status === "applying")
    return <small className="processor-caption">正在应用…</small>;
  if (!rule) return null;
  if (rule.status === "disabled")
    return <Badge variant="neutral">未启用</Badge>;
  if (rule.status !== "applied")
    return (
      <small className="processor-caption" title={rule.detail}>
        {!rule.enabled
          ? "解除未确认"
          : rule.status === "partial"
            ? "部分生效"
            : "设置未生效"}{" "}
        · {rule.detail}
      </small>
    );
  if (rule.blocked) return <Badge variant="danger">网络已禁用</Badge>;
  return (
    <small className="processor-caption" title={rule.detail}>
      {[
        rule.download != null ? `下载 ≤ ${formatTraffic(rule.download)}` : "",
        rule.upload != null ? `上传 ≤ ${formatTraffic(rule.upload)}` : "",
      ]
        .filter(Boolean)
        .join(" · ") || "未设置限制"}
    </small>
  );
}
export function NetworkActions({
  vm,
  target,
}: {
  vm: ControlViewModel;
  target: Target;
}) {
  return (
    <Button
      variant="ghost"
      size="sm"
      aria-label={`${target.name} 网络控制`}
      aria-haspopup="menu"
      disabled={!vm.available || !!vm.pendingId || !controllable(target)}
      title={
        !controllable(target)
          ? "系统或 Pinmeter 组件不支持网络控制"
          : "网络控制"
      }
      onClick={(e) => {
        const r = e.currentTarget.getBoundingClientRect();
        vm.openMenu(target, r.right - 224, r.bottom, e.currentTarget);
      }}
    >
      <MoreHorizontal size={16} aria-hidden="true" />
    </Button>
  );
}
function ControlMenu({ vm }: { vm: ControlViewModel }) {
  const root = useRef<HTMLDivElement>(null);
  const [position, setPosition] = useState({ x: vm.menu!.x, y: vm.menu!.y });
  useLayoutEffect(() => {
    const node = root.current!;
    const box = node.getBoundingClientRect();
    setPosition({
      x: Math.max(8, Math.min(vm.menu!.x, window.innerWidth - box.width - 8)),
      y: Math.max(8, Math.min(vm.menu!.y, window.innerHeight - box.height - 8)),
    });
    node
      .querySelector<HTMLElement>(
        '[role="menuitem"]:not([aria-disabled="true"])',
      )
      ?.focus({ preventScroll: true });
    const outside = (e: PointerEvent) => {
      if (!node.contains(e.target as Node)) vm.closeMenu();
    };
    const close = () => vm.closeMenu();
    document.addEventListener("pointerdown", outside);
    window.addEventListener("resize", close);

    return () => {
      document.removeEventListener("pointerdown", outside);
      window.removeEventListener("resize", close);
    };
  }, []);
  const menu = vm.menu!;
  const rule = vm.rule(menu.target.id);
  const restore = !!(
    (rule?.enabled && rule?.blocked) ||
    rule?.inbound_blocked ||
    rule?.outbound_blocked
  );
  const select = (
    action: "block" | "restore" | "clear" | "enable" | "disable",
  ) => {
    vm.closeMenu();
    void vm.act(menu.target, action, menu.revision);
  };
  return createPortal(
    <div
      ref={root}
      className="network-control-menu"
      style={{ left: position.x, top: position.y }}
      onContextMenu={(e) => e.preventDefault()}
      onKeyDown={(e) => {
        if (e.key === "Escape" || e.key === "Tab") {
          if (e.key === "Escape") e.preventDefault();
          vm.closeMenu();
          return;
        }
        if (!["ArrowDown", "ArrowUp", "Home", "End"].includes(e.key)) return;
        e.preventDefault();
        const items = Array.from(
          root.current!.querySelectorAll<HTMLElement>(
            '[role="menuitem"]:not([aria-disabled="true"])',
          ),
        );
        if (!items.length) return;
        const at = items.indexOf(document.activeElement as HTMLElement);
        const next =
          e.key === "Home"
            ? 0
            : e.key === "End"
              ? items.length - 1
              : (at + (e.key === "ArrowDown" ? 1 : -1) + items.length) %
                items.length;
        items[next]?.focus();
      }}
    >
      <Menu aria-label={`${menu.target.name} 网络控制`} minWidth={224}>
        {rule && (
          <MenuItem
            onSelect={() => select(rule.enabled ? "disable" : "enable")}
          >
            {rule.enabled ? "停用规则（保留配置）" : "启用已保存规则"}
          </MenuItem>
        )}
        <MenuItem icon={<Gauge size={16} />} onSelect={vm.edit}>
          设置限速…
        </MenuItem>
        <MenuItem
          icon={restore ? <Plug size={16} /> : <Unplug size={16} />}
          state={restore ? "default" : "destructive"}
          onSelect={() => select(restore ? "restore" : "block")}
        >
          {restore ? "恢复网络" : "禁用网络"}
        </MenuItem>
        <MenuItem
          icon={<RotateCcw size={16} />}
          state={rule ? "default" : "disabled"}
          onSelect={() => select("clear")}
        >
          删除规则并解除限制
        </MenuItem>
        <p className="processor-caption network-control-menu-note">
          {vm.releaseOnExit
            ? "退出时解除限制，保留配置但不启用。"
            : "退出后仍保持网络禁用，限速停止。"}
        </p>
      </Menu>
    </div>,
    document.body,
  );
}
function RateField({
  label,
  field,
  change,
  disabled,
}: {
  label: string;
  field: LimitField;
  change: (value: LimitField) => void;
  disabled: boolean;
}) {
  return (
    <div className="network-control-rate">
      <Select
        label={label}
        value={field.mode}
        onChange={(mode) => change({ ...field, mode })}
        disabled={disabled}
        options={[
          { label: "不限制", value: "unlimited" },
          { label: "设置上限", value: "limited" },
        ]}
      />
      {field.mode === "limited" && (
        <div className="network-control-rate-value">
          <Input
            label={`${label}数值`}
            type="number"
            inputMode="decimal"
            min={0}
            step="any"
            value={field.value}
            disabled={disabled}
            onChange={(e) => change({ ...field, value: e.target.value })}
          />
          <Select
            label={`${label}单位`}
            value={field.unit}
            disabled={disabled}
            onChange={(unit) => change({ ...field, unit })}
            options={[
              { label: "KB/s", value: "KB/s" },
              { label: "MB/s", value: "MB/s" },
            ]}
          />
        </div>
      )}
    </div>
  );
}
export function NetworkControlPanel({ vm }: { vm: ControlViewModel }) {
  return (
    <>
      <section className="network-control-rules" aria-label="已配置网络规则">
        <div className="section-heading">
          <h3>已配置规则</h3>
          <Badge>{vm.data?.rules.length ?? 0}</Badge>
          <Button
            variant="outline"
            size="sm"
            disabled={!vm.canRelease || !!vm.pendingId}
            onClick={() => void vm.releaseAll()}
          >
            解除全部限制
          </Button>
        </div>
        {!vm.available && (
          <p className="processor-caption">
            {vm.data?.detail || "当前环境尚未连接应用网络控制"}
          </p>
        )}
        {vm.data?.rules.map((rule) => (
          <div
            className="network-control-rule"
            key={rule.id}
            onContextMenu={(e) => {
              if (vm.available && controllable(rule)) {
                e.preventDefault();
                vm.openMenu(
                  rule,
                  e.clientX,
                  e.clientY,
                  e.currentTarget.querySelector("button"),
                );
              }
            }}
          >
            <div>
              <span>{rule.name}</span>
              <small
                className="processor-caption network-control-path"
                title={rule.path}
              >
                {rule.path}
              </small>
              <RuleStatus vm={vm} id={rule.id} />
              <small className="processor-caption">
                已保存：
                {[
                  rule.blocked ? "禁用网络" : "",
                  rule.download != null
                    ? `下载 ≤ ${formatTraffic(rule.download)}`
                    : "",
                  rule.upload != null
                    ? `上传 ≤ ${formatTraffic(rule.upload)}`
                    : "",
                ]
                  .filter(Boolean)
                  .join(" · ") || "无限制"}
              </small>
            </div>
            <NetworkActions vm={vm} target={rule} />
          </div>
        ))}
        {!vm.data?.rules.length && (
          <p className="processor-caption">
            尚未设置限制。右键应用或点击 ⋯，设置限速或禁用网络。
          </p>
        )}
        <p className="processor-caption">
          规则作用于同一路径的全部进程。停止监控或切换页面不取消限制；退出
          Pinmeter{" "}
          {vm.releaseOnExit
            ? "解除全部限制，配置保留为未启用。"
            : "停止限速，网络禁用仍保留。"}
          代理、VPN 与本机回环需单独核对。
        </p>
        {vm.message && (
          <p className="processor-caption" role="status">
            {vm.message}
          </p>
        )}
        {vm.error && !vm.editor && (
          <p className="network-control-error" role="alert">
            {vm.error}
          </p>
        )}
      </section>
      {vm.menu && !vm.suspended && (
        <ControlMenu
          key={`${vm.menu.target.id}-${vm.menu.x}-${vm.menu.y}`}
          vm={vm}
        />
      )}
      <Modal
        open={!!vm.editor && !vm.suspended}
        title="设置应用限速"
        description={vm.editor?.target.name}
        confirmLabel="保存"
        cancelLabel="取消"
        onClose={vm.closeEditor}
        onConfirm={() => void vm.save()}
        confirmLoading={!!vm.pendingId}
        closeOnBackdropClick={!vm.pendingId}
        closeOnEscape={!vm.pendingId}
      >
        <div className="network-control-form">
          <p className="processor-caption network-control-path">
            {vm.editor?.target.path}
          </p>
          <RateField
            label="下载上限"
            field={vm.down}
            change={vm.setDown}
            disabled={!!vm.pendingId}
          />
          <RateField
            label="上传上限"
            field={vm.up}
            change={vm.setUp}
            disabled={!!vm.pendingId}
          />
          <p className="processor-caption">
            同一程序的全部进程共享上限。范围：16 KB/s 至 1000 MB/s。退出
            Pinmeter 后停止限速。
            {vm.releaseOnExit
              ? "配置保留为未启用，下次可手动启用。"
              : "下次启动重新应用已启用的规则。"}
          </p>
          {vm.editor && vm.rule(vm.editor.target.id)?.blocked && (
            <p className="processor-caption">
              此应用已禁用网络；保存限速不会解除禁用，恢复网络后使用这些上限。
            </p>
          )}
          {vm.editor && vm.rule(vm.editor.target.id)?.enabled === false && (
            <p className="processor-caption">
              此规则未启用；保存仅更新数值，请从菜单启用已保存规则。
            </p>
          )}
          {vm.error && (
            <p className="network-control-error" role="alert">
              {vm.error}
            </p>
          )}
        </div>
      </Modal>
    </>
  );
}
