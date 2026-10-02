import { describe, expect, it } from "vitest";
import { acceptUpdateSnapshot } from "../src/shared/client/update-client";
import { DemoUpdateClient } from "../src/shared/client/demo-update-client";

describe("更新状态", () => {
  it("较迟返回的初始快照不能覆盖下载就绪事件", () => {
    const client = new DemoUpdateClient("ready");
    const latest = { ...client.getSnapshot(), revision: "12" };
    const old = { ...latest, revision: "11", stage: "checking" };
    expect(acceptUpdateSnapshot(latest, old)).toBe(latest);
    expect(acceptUpdateSnapshot(null, latest)).toBe(latest);
  });
  it("收起更新提示不标记安装后公告为已读", async () => {
    const client = new DemoUpdateClient("notice");
    await client.preference("dismiss");
    expect(client.getSnapshot().unread).toHaveLength(1);
    expect(client.getSnapshot().dismissed_version).toBe("0.1.3");
    await client.preference("read");
    expect(client.getSnapshot().unread).toHaveLength(0);
  });
});
