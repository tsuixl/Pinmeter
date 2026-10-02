import type { UpdateSnapshotDto } from "../contracts/monitor";

export interface UpdateClient {
  getSnapshot(): UpdateSnapshotDto | null;
  subscribe(listener: () => void): () => void;
  start(): () => void;
  check(): Promise<void>;
  download(): Promise<void>;
  install(confirmed: boolean): Promise<void>;
  preference(
    action: "automatic" | "read" | "dismiss",
    value?: boolean,
  ): Promise<void>;
  openDownloads(): Promise<void>;
}
export function acceptUpdateSnapshot(
  current: UpdateSnapshotDto | null,
  next: UpdateSnapshotDto,
) {
  return !current || BigInt(next.revision) > BigInt(current.revision)
    ? next
    : current;
}
