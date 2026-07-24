import "@fontsource-variable/inter/wght.css";
import "@fontsource/barlow-condensed/500.css";
import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./app/App";
import { TauriNativeBridge } from "./native-bridge/tauri-native-bridge";

const root = document.getElementById("root");

if (!root) {
  throw new Error("Formation Lap root element is missing");
}

createRoot(root).render(
  <StrictMode>
    <App bridge={new TauriNativeBridge()} />
  </StrictMode>,
);
