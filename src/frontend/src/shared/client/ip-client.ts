import type { IpStateDto } from "../contracts/monitor";
import type { MonitorClient } from "./monitor-client";
export interface IpClient {
  getSnapshot(): IpStateDto | undefined;
  isConnected(): boolean;
  subscribe(listener: () => void): () => void;
  setActive(active: boolean): Promise<void>;
  refresh(): Promise<void>;
  refreshChecks(section: string): Promise<void>;
  copy(address: string): Promise<void>;
}
export function ipClient(client: MonitorClient): IpClient {
  return {
    getSnapshot: () => client.getSnapshot().state?.ip,
    isConnected: () => client.getSnapshot().connected,
    subscribe: (listener) => client.subscribe(listener),
    setActive: (active) =>
      client.setIpViewActive?.(active) ?? Promise.resolve(),
    refresh: () =>
      client.refreshIp?.() ??
      Promise.reject(new Error("当前环境未连接 IP 查询服务")),
    copy: (address) => navigator.clipboard.writeText(address),
    refreshChecks: (section) =>
      client.refreshIpChecks?.(section) ??
      Promise.reject(new Error("当前环境未连接检测服务")),
  };
}
