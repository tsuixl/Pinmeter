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
import { builtinFontCatalog } from "../shared/fonts";

async function bootstrap() {
  if (new URLSearchParams(location.search).get("panel") === "1") {
    await (await import("../features/tray-panel/TrayPanel")).startTrayPanel();
    return;
  }
  const windowClient = await createWindowClient();
  const font = windowClient?.initialFontFamily ?? defaultFontFamily;
  const fontStyle = windowClient?.initialFontStyle ?? "auto";
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
  // Preload the saved face before the hidden native window is revealed.
  try {
    const catalog = (await client.getFontCatalog?.()) ?? builtinFontCatalog;
    await loadFont(font, fontStyle, catalog);
    applyFont(font, fontStyle);
  } catch {
    try {
      await loadFont(defaultFontFamily);
    } catch {
      /* readable CSS fallback */
    }
    applyFont(defaultFontFamily);
  }
  createRoot(document.getElementById("root")!).render(
    <App
      client={client}
      windowClient={windowClient}
      updateClient={updateClient}
    />,
  );
}
void bootstrap().catch(showStartupError);
