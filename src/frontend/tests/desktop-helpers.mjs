import { setTimeout } from "node:timers/promises";
export async function pollState(invoke, predicate, timeout = 20000) {
  const deadline = Date.now() + timeout;
  while (Date.now() < deadline) {
    const state = await invoke("get_monitor_state");
    if (predicate(state)) return state;
    await setTimeout(100);
  }
  throw new Error("Timed out waiting for desktop state");
}
