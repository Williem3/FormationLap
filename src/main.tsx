import "@fontsource-variable/inter/wght.css";
import "@fontsource/barlow-condensed/500.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./app/App";
import type { NativeBridge } from "./native-bridge/native-bridge";
import { TauriNativeBridge } from "./native-bridge/tauri-native-bridge";

const root = document.getElementById("root");

if (!root) {
  throw new Error("Formation Lap root element is missing");
}

async function resolveNativeBridge(): Promise<NativeBridge> {
  const preview = new URLSearchParams(window.location.search).get("preview");
  if (
    import.meta.env.DEV &&
    (preview === "m2-wizard" || preview === "m2-editor")
  ) {
    const { createM2PreviewBridge } = await import("./preview/m2-preview");
    return createM2PreviewBridge(preview);
  }
  if (import.meta.env.DEV && preview === "m3-dashboard") {
    const { createM3PreviewBridge } = await import("./preview/m3-preview");
    return createM3PreviewBridge();
  }
  if (import.meta.env.DEV && preview === "m4-session") {
    const { createM4PreviewBridge } = await import("./preview/m4-preview");
    const previewState =
      new URLSearchParams(window.location.search).get("state") ?? "prestart";
    return createM4PreviewBridge(previewState);
  }
  if (import.meta.env.DEV && preview === "m5-wizard") {
    const { createM5PreviewBridge } = await import("./preview/m5-preview");
    return createM5PreviewBridge();
  }

  return new TauriNativeBridge();
}

void resolveNativeBridge().then((bridge) => {
  createRoot(root).render(
    <StrictMode>
      <App bridge={bridge} />
    </StrictMode>,
  );
});
