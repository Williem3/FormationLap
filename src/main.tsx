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

  return new TauriNativeBridge();
}

void resolveNativeBridge().then((bridge) => {
  createRoot(root).render(
    <StrictMode>
      <App bridge={bridge} />
    </StrictMode>,
  );
});
