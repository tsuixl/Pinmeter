import { ArrowDownToLine, X } from "lucide-react";
import {
  Alert,
  Badge,
  Button,
  Card,
  IconButton,
  Modal,
  Progress,
  Switch,
  Toast,
} from "../../shared/ui/sakani";
import { buildInfo } from "../../shared/client/build-info";
import type { UpdateViewModel } from "./useUpdateViewModel";
import "./updates.css";

export function UpdateBanner({
  vm,
  collapsed,
}: {
  vm: UpdateViewModel;
  collapsed: boolean;
}) {
  const s = vm.state;
  if (!s?.target || s.dismissed_version === s.target.version) return null;
  const title =
    s.stage === "ready"
      ? "更新已就绪"
      : s.stage === "downloading"
        ? "正在下载更新"
        : s.stage === "failed"
          ? "更新未完成"
          : "发现新版本";
  if (collapsed)
    return (
      <div className="update-rail">
        <IconButton
          variant="ghost"
          size="sm"
          aria-label={title + "，查看更新内容"}
          title={title}
          icon={ArrowDownToLine}
          onClick={() => vm.open("preview")}
        />
      </div>
    );
  return (
    <div className="update-banner">
      <div className="update-banner-heading">
        <strong>{title}</strong>
        <IconButton
          variant="ghost"
          size="sm"
          aria-label="稍后再说"
          title="稍后再说"
          icon={X}
          onClick={() => void vm.dismiss()}
        />
      </div>
      <span>Pinmeter {s.target.version}</span>
      {s.stage === "downloading" && (
        <Progress value={s.progress ?? undefined} label="更新下载进度" />
      )}
      <Button size="sm" variant="outline" onClick={() => vm.open("preview")}>
        查看更新
      </Button>
    </div>
  );
}
export function UpdateSettings({ vm }: { vm: UpdateViewModel }) {
  const s = vm.state;
  return (
    <Card title="版本与更新" description={"Pinmeter " + buildInfo.version}>
      <div className="update-settings-status" role="status">
        {s?.target && (
          <Badge>
            {s.stage === "ready" ? "已就绪 " : "新版本 "}
            {s.target.version}
          </Badge>
        )}
        <p>{s?.detail || "可手动检查更新，也可开启后台自动检查。"}</p>
        {s?.last_check_ms && (
          <span className="settings-note">
            上次检查：{new Date(s.last_check_ms).toLocaleString()}
          </span>
        )}
      </div>
      <div className="update-actions">
        <Button
          size="sm"
          variant="outline"
          disabled={!s || vm.busy}
          loading={s?.stage === "checking"}
          onClick={() => void vm.check()}
        >
          检查更新
        </Button>
        {s?.target && (
          <Button size="sm" onClick={() => vm.open("preview")}>
            查看更新内容
          </Button>
        )}
        <Button
          size="sm"
          variant="ghost"
          disabled={!s}
          onClick={() => vm.open("history")}
        >
          版本公告
        </Button>
      </div>
      {s?.stage === "downloading" && (
        <Progress value={s.progress ?? undefined} label="更新下载进度" />
      )}
      <Switch
        label="自动检查更新"
        checked={s?.automatic_check ?? true}
        disabled={
          !s ||
          vm.saving ||
          vm.busy ||
          !["installed", "demo"].includes(s.installation)
        }
        onChange={(event) => void vm.automatic(event.target.checked)}
      />
      <p className="settings-note">
        每天最多检查一次，下载和安装由你决定。便携测试包仅支持手动检查。
      </p>
      <p className="settings-note">
        {s?.can_install
          ? "安装前会按现有退出设置处理网络限制，监控将短暂中断。"
          : "当前为便携或开发运行包，在线安装未开放。请下载完整运行包，完全退出旧版后再启动。"}
      </p>
      {!s?.can_install && (
        <Button
          size="sm"
          variant="ghost"
          onClick={() => void vm.openDownloads()}
        >
          前往发行下载页
        </Button>
      )}
      {vm.error && (
        <Alert color="danger" title="操作未完成" description={vm.error} />
      )}
      <details className="update-build-info">
        <summary>构建信息</summary>
        <p>源码标识：{buildInfo.revision}</p>
        <p>
          构建时间：
          {buildInfo.time
            ? new Date(buildInfo.time).toLocaleString()
            : "开发环境"}
        </p>
      </details>
    </Card>
  );
}
export function UpdateDialogs({
  vm,
  visible,
}: {
  vm: UpdateViewModel;
  visible: boolean;
}) {
  const s = vm.state;
  const preview = vm.dialog === "preview";
  const downloading = s?.stage === "downloading";
  const preparing = s?.stage === "preparing";
  const failure =
    vm.error || (preview && s?.stage === "failed" ? s.detail : "");
  const confirmLabel = !preview
    ? "知道了"
    : !s?.can_install
      ? "前往下载"
      : preparing
        ? "正在准备安装…"
        : downloading
          ? "正在下载…"
          : s?.stage === "ready"
            ? s.confirmation_required
              ? "保留禁用并更新"
              : "安装并重启"
            : "下载更新";
  return (
    <>
      <Modal
        open={!!vm.dialog && visible}
        title={
          preview
            ? "更新 Pinmeter"
            : vm.dialog === "unread"
              ? "本次更新内容"
              : "版本公告"
        }
        description={
          preview && s?.target
            ? "当前 " + s.current_version + " → 新版本 " + s.target.version
            : undefined
        }
        className="update-notice-modal"
        confirmLabel={confirmLabel}
        cancelLabel={preview ? "稍后" : "关闭"}
        confirmLoading={preparing || downloading || vm.saving}
        closeOnEscape={!preparing && !vm.saving}
        closeOnBackdropClick={!preparing && !vm.saving}
        onClose={() => void vm.close()}
        onConfirm={() => {
          if (vm.saving || preparing || downloading) return;
          void (preview ? vm.primary() : vm.close());
        }}
      >
        <div className="update-notice-body">
          {vm.notes.map((release) => (
            <article key={release.version} className="release-notes">
              <div className="release-meta">
                <Badge>v{release.version}</Badge>
                <span>{release.date}</span>
              </div>
              <h3>{release.summary}</h3>
              {release.sections.map((section, i) => (
                <section key={i}>
                  <h4>{section.title}</h4>
                  <ul>
                    {section.items.map((item, j) => (
                      <li key={j}>{item}</li>
                    ))}
                  </ul>
                </section>
              ))}
            </article>
          ))}
          {!vm.notes.length && <p>暂无可用公告，可前往发行页面查看。</p>}
        </div>
        {preview && downloading && (
          <div className="update-progress">
            <Progress value={s?.progress ?? undefined} label="更新下载进度" />
            <span>
              {s?.progress == null ? "正在接收更新包…" : s.progress + "%"}
            </span>
            <p>关闭弹窗后仍会继续下载。退出程序后需重新检查和下载。</p>
          </div>
        )}
        {preview && s?.stage === "ready" && (
          <p className="settings-note">
            更新已下载并通过签名校验。安装将短暂停止监控，并按设置中的退出规则处理网络限制。
          </p>
        )}
        {preview && s?.confirmation_required && (
          <Alert
            color="warning"
            title="更新期间保留网络禁用？"
            description={s.detail}
          />
        )}
        {failure && (
          <Alert color="danger" title="操作未完成" description={failure} />
        )}
      </Modal>
      {vm.toast && visible && (
        <div className="update-toast">
          <Toast
            status={s?.stage === "failed" ? "error" : "info"}
            title="检查更新"
            description={vm.toast}
            onDismiss={vm.clearToast}
          />
        </div>
      )}
    </>
  );
}
