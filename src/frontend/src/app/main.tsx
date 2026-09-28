import { createRoot } from "react-dom/client";
import { App } from "./App";
import { TauriMonitorClient } from "../shared/client/tauri-client";
import {
  createWindowClient,
  showStartupError,
} from "../shared/client/window-client";
import "@fontsource-variable/geist";
import "../shared/ui/sakani/tokens.css";
import "@sakaniui/react/style.css";
import "./style.css";

async function bootstrap() {
  const windowClient = await createWindowClient();
  const client =
    import.meta.env.DEV && new URLSearchParams(location.search).has("demo")
      ? new (await import("../shared/client/demo-client")).DemoClient()
      : new TauriMonitorClient();
  createRoot(document.getElementById("root")!).render(
    <App client={client} windowClient={windowClient} />,
  );
}
void bootstrap().catch(showStartupError);
