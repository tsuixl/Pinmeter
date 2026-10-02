import type { UpdateSnapshotDto } from "../contracts/monitor";
import type { UpdateClient } from "./update-client";
import releases from "../../../../shared/updates/releases.json";
import { buildInfo } from "./build-info";

export class DemoUpdateClient implements UpdateClient {
  private listeners = new Set<() => void>();
  private state: UpdateSnapshotDto;
  constructor(scenario: string | null = null) {
    const target = {
      version: "0.1.3",
      date: "2026-10-03",
      summary: "演示：更方便地更新 Pinmeter",
      sections: [
        {
          title: "体验优化",
          items: [
            "此公告和下载进度为演示数据，不会下载或安装任何程序。",
            "更新前后查看完整公告，后台监控保持安静。",
          ],
        },
      ],
    };
    this.state = {
      revision: "0",
      current_version: buildInfo.version,
      installation: "demo",
      can_install: true,
      stage:
        scenario === "failed"
          ? "failed"
          : scenario === "ready"
            ? "ready"
            : scenario
              ? "available"
              : "idle",
      detail:
        scenario === "failed"
          ? "演示：下载中断，请检查网络后重试。"
          : "演示更新，不会操作本机安装",
      progress: scenario === "ready" ? 100 : null,
      target: scenario ? target : null,
      target_history: scenario ? [target] : [],
      history: releases,
      unread: scenario === "notice" ? [releases[0]] : [],
      automatic_check: true,
      last_check_ms: null,
      dismissed_version: null,
      manual_request: "0",
      confirmation_required: false,
      release_url: "https://github.com/tsuixl/Pinmeter/releases",
    };
  }
  getSnapshot = () => this.state;
  subscribe = (listener: () => void) => {
    this.listeners.add(listener);
    return () => {
      this.listeners.delete(listener);
    };
  };
  private publish(patch: Partial<UpdateSnapshotDto>) {
    this.state = {
      ...this.state,
      ...patch,
      revision: String(BigInt(this.state.revision) + 1n),
    };
    this.listeners.forEach((listener) => listener());
  }
  start = () => () => {};
  async check() {
    this.publish({
      manual_request: String(BigInt(this.state.manual_request) + 1n),
      detail: "演示：检查完成",
    });
  }
  async download() {
    this.publish({ stage: "downloading", progress: 0 });
    for (const progress of [20, 50, 80, 100]) {
      await new Promise((resolve) => setTimeout(resolve, 300));
      this.publish({ progress });
    }
    this.publish({ stage: "ready", detail: "演示：更新已就绪" });
  }
  async install() {
    this.publish({ stage: "ready", detail: "演示模式不会重启或安装程序" });
  }
  async preference(action: "automatic" | "read" | "dismiss", value?: boolean) {
    if (action === "automatic")
      this.publish({ automatic_check: value ?? true });
    if (action === "read") this.publish({ unread: [] });
    if (action === "dismiss")
      this.publish({ dismissed_version: this.state.target?.version ?? null });
  }
  async openDownloads() {
    this.publish({ detail: "演示模式不会打开下载页面" });
  }
}
