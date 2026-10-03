import { createRoot } from "react-dom/client";
import { App } from "./App";
import { TauriMonitorClient } from "../shared/client/tauri-client";
import { TauriUpdateClient } from "../shared/client/tauri-update-client";
import {
  createWindowClient,
  showStartupError,
} from "../shared/client/window-client";
import "@fontsource-variable/geist";
import "../shared/ui/sakani/tokens.css";
import "@sakaniui/react/style.css";
import "../shared/ui/fonts.css";
import "./style.css";
import { applyFont, defaultFontFamily, loadFont } from "../shared/ui/fonts";

async function bootstrap() {
  const windowClient = await createWindowClient();
  const font = windowClient?.initialFontFamily ?? defaultFontFamily;
  // The native window stays hidden during its initial font load. App reports a
  // resource failure and keeps a readable system fallback instead of blocking startup.
  try {
    await loadFont(font);
    applyFont(font);
  } catch {
    applyFont("system");
  }
  const demo =
    import.meta.env.DEV && new URLSearchParams(location.search).has("demo");
  const updateClient = demo
    ? new (
        await import("../shared/client/demo-update-client")
      ).DemoUpdateClient(new URLSearchParams(location.search).get("update"))
    : new TauriUpdateClient();
  const client =
    import.meta.env.DEV && new URLSearchParams(location.search).has("demo")
      ? new (await import("../shared/client/demo-client")).DemoClient()
      : new TauriMonitorClient();
  createRoot(document.getElementById("root")!).render(
    <App
      client={client}
      windowClient={windowClient}
      updateClient={updateClient}
    />,
  );
}
void bootstrap().catch(showStartupError);
